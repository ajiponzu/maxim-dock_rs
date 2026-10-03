//! WSL bridge: positional argv, optional login environment, no console or forced termination.
use crate::core::WorkspaceEntry;
use std::{
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};
pub(super) fn run(entry: &WorkspaceEntry, cancel: &AtomicBool) -> Result<String, String> {
    let wsl = entry.wsl.as_ref().ok_or("WSL 設定がありません。")?;
    wsl.validate(&entry.executable, &entry.working_directory)
        .map_err(|e| e.to_string())?;
    if cancel.load(Ordering::Acquire) {
        return Err("WSL の起動を中止しました。".into());
    }
    let mut command = command(entry)?;
    let mut child = spawn(&mut command, wsl.wait_for_exit)?;
    if !wsl.wait_for_exit {
        return Ok("WSL に起動要求を送りました（コマンドの終了は待ちません）。".into());
    }
    wait(&mut child, Duration::from_secs(wsl.timeout_secs), cancel)
        .map(|message| format!("{}: {message}", wsl.execution_mode()))
        .map_err(|message| format!("{}: {message}", wsl.execution_mode()))
}
fn command(entry: &WorkspaceEntry) -> Result<Command, String> {
    let wsl = entry.wsl.as_ref().ok_or("WSL 設定がありません。")?;
    let bridge = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .ok_or("Windows ディレクトリが見つかりません。")?
        .join("System32/wsl.exe");
    let mut command = Command::new(bridge);
    let arguments = wsl.arguments(
        &entry.executable,
        &entry.arguments,
        &entry.working_directory,
    );
    if !wsl.login_shell && !wsl.direct_exec {
        let (line, options) = arguments.split_last().ok_or("WSL コマンドがありません。")?;
        // WSL consumes the remainder as Linux shell source. Rust's normal Windows argv
        // quoting would surround the whole line and turn it into one executable name.
        // The core has already POSIX-quoted every user word; only that line is raw.
        command.args(options).raw_arg(line);
    } else {
        command.args(arguments);
    }
    command.creation_flags(0x08000000);
    Ok(command)
}
fn spawn(command: &mut Command, capture: bool) -> Result<Child, String> {
    command
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(if capture {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stderr(if capture {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .spawn()
        .map_err(|e| format!("WSL 起動失敗: {e}"))
}
fn wait(child: &mut Child, timeout: Duration, cancel: &AtomicBool) -> Result<String, String> {
    let started = Instant::now();
    let mut diagnostics = super::workspace_wsl_output::Diagnostics::default();
    loop {
        diagnostics.poll(child)?;
        if cancel.load(Ordering::Acquire) {
            return Err("WSL の終了待機を中止しました。起動済みコマンドは終了しません。".into());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("WSL 終了確認失敗: {e}"))?
        {
            diagnostics.poll(child)?;
            return if status.success() {
                Ok("WSL コマンド完了（終了コード 0）。".into())
            } else {
                Err(format!(
                    "WSL コマンド失敗（終了コード {}）。ディストリビューション・コマンド・権限を確認してください。{}",
                    status
                        .code()
                        .map_or_else(|| "不明".into(), |code| code.to_string()),
                    diagnostics.message()
                ))
            };
        }
        if started.elapsed() >= timeout {
            return Err(format!(
                "WSL の終了待機が {} 秒でタイムアウトしました。コマンドは強制終了しません。",
                timeout.as_secs()
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_wait_reports_success_failure_cancel_and_timeout_without_killing() {
        let cancel = AtomicBool::new(false);
        let dir = tempfile::tempdir().unwrap();
        let fixture = dir.path().join("command fixture.exe");
        assert!(
            Command::new("rustc")
                .args([
                    "--edition=2024",
                    "src/platform_windows/fixtures/workspace_command.rs",
                    "-o"
                ])
                .arg(&fixture)
                .status()
                .unwrap()
                .success()
        );
        for code in [0, 7] {
            let mut cmd = Command::new(&fixture);
            cmd.arg(code.to_string()).arg("0");
            let mut child = spawn(&mut cmd, true).unwrap();
            let result = wait(&mut child, Duration::from_secs(5), &cancel);
            assert_eq!(result.is_ok(), code == 0, "{result:?}");
        }
        let mut cmd = Command::new(&fixture);
        cmd.args(["0", "500"]);
        let mut child = spawn(&mut cmd, true).unwrap();
        assert!(
            wait(&mut child, Duration::ZERO, &cancel)
                .unwrap_err()
                .contains("タイムアウト")
        );
        assert!(child.try_wait().unwrap().is_none());
        cancel.store(true, Ordering::Release);
        assert!(
            wait(&mut child, Duration::from_secs(5), &cancel)
                .unwrap_err()
                .contains("中止")
        );
        assert!(child.try_wait().unwrap().is_none());
        child.wait().unwrap();
    }
    #[test]
    #[ignore = "read-only code --version using the local WSL user mita"]
    fn native_wsl_code_login_environment_resolves_launcher_without_console() {
        let mut entry = WorkspaceEntry {
            executable: "code".into(),
            arguments: vec!["--version".into()],
            wsl: Some(crate::core::WslSettings {
                user: "mita".into(),
                login_shell: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = run(&entry, &AtomicBool::new(false));
        assert!(result.is_ok(), "{result:?}");
        entry.wsl.as_mut().unwrap().login_shell = false;
        let result = run(&entry, &AtomicBool::new(false));
        assert!(
            result.is_ok(),
            "standard shell must resolve code: {result:?}"
        );
        entry.wsl.as_mut().unwrap().direct_exec = true;
        let error = run(&entry, &AtomicBool::new(false)).unwrap_err();
        assert!(
            error.contains("診断出力") && error.contains("execvpe(code)"),
            "{error}"
        );
        println!(
            "WSL_CODE_NATIVE_PASS: login code --version succeeds with hidden console/piped output; direct exec shows missing code diagnosis"
        );
    }
    #[test]
    #[ignore = "executes read-only commands inside Ubuntu-24.04"]
    fn native_wsl_executes_linux_command_without_window_wait() {
        let entry = WorkspaceEntry {
            executable: "/bin/true".into(),
            working_directory: "/tmp".into(),
            wsl: Some(crate::core::WslSettings {
                distribution: "Ubuntu-24.04".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let start = Instant::now();
        assert!(
            run(&entry, &AtomicBool::new(false))
                .unwrap()
                .contains("終了コード 0")
        );
        println!("WSL_NATIVE_PASS: true completed in {:?}", start.elapsed());
        let bad = WorkspaceEntry {
            executable: "/bin/false".into(),
            ..entry.clone()
        };
        assert!(
            run(&bad, &AtomicBool::new(false))
                .unwrap_err()
                .contains("失敗")
        );
        let args = vec![
            "%s\\n".into(),
            "".into(),
            "日本語 with spaces".into(),
            "a\"b".into(),
            "$(not-a-shell-command)".into(),
        ];
        let printf = WorkspaceEntry {
            executable: "/usr/bin/printf".into(),
            arguments: args,
            ..entry
        };
        let out = command(&printf)
            .unwrap()
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            "\n日本語 with spaces\na\"b\n$(not-a-shell-command)\n"
        );
        let mut login = printf.clone();
        login.wsl.as_mut().unwrap().login_shell = true;
        let out = command(&login)
            .unwrap()
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            "\n日本語 with spaces\na\"b\n$(not-a-shell-command)\n"
        );
        let missing = WorkspaceEntry {
            executable: "/maximdock-missing-command-test".into(),
            ..printf
        };
        let message = run(&missing, &AtomicBool::new(false)).unwrap_err();
        assert!(
            message.contains("診断出力") && message.contains("maximdock-missing-command-test"),
            "{message}"
        );
        let mut workspace = crate::core::Workspace::new("WSL sequential test");
        workspace.entries = vec![
            WorkspaceEntry {
                label: "fail".into(),
                ..bad
            },
            WorkspaceEntry {
                label: "must-not-start".into(),
                executable: "C:\\missing\\app.exe".into(),
                ..Default::default()
            },
        ];
        let mut runner = crate::platform_windows::WorkspaceRunner::default();
        runner.start(workspace, || {}).unwrap();
        let start = Instant::now();
        loop {
            let status = runner.poll();
            if !status.running {
                assert_eq!(status.results.len(), 2);
                assert_eq!(status.results[0].label, "fail");
                assert_eq!(status.results[1].label, "停止");
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(10));
            thread::sleep(Duration::from_millis(100));
        }
    }
}
