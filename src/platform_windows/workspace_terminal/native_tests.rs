//! Opt-in Terminal tests. Close only windows carrying this test's newly generated UUIDs.
use super::*;
use crate::{
    core::*,
    platform_windows::{WorkspaceRunner, display_catalog, workspace_windows},
};
use std::{
    thread,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE},
};
struct Windows(Vec<WorkspaceEntry>);
impl Drop for Windows {
    fn drop(&mut self) {
        if let Ok(windows) = workspace_windows::enumerate() {
            for window in windows {
                if self.0.iter().any(|entry| matches(entry, &window)) {
                    // SAFETY: only the test's UUID-tagged, newly created Terminal windows.
                    unsafe {
                        let _ = PostMessageW(
                            Some(HWND(window.identity.handle as *mut _)),
                            WM_CLOSE,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            }
        }
    }
}
fn linux(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    format!("/mnt/{}/{}", path[..1].to_lowercase(), &path[3..])
}
#[test]
#[ignore = "opens/places/closes only UUID-tagged test Terminal windows; runs harmless scripts in PowerShell/cmd/Ubuntu"]
fn native_terminal_scripts_run_in_requested_directories_and_preserve_existing_windows() {
    let _dpi = workspace_windows::PhysicalDpi::enter().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let working = dir.path().join("日本語 project space");
    std::fs::create_dir(&working).unwrap();
    let existing = workspace_windows::enumerate()
        .unwrap()
        .into_iter()
        .filter(|w| w.executable.ends_with("\\windowsterminal.exe"))
        .map(|w| {
            let mut bounds = windows::Win32::Foundation::RECT::default();
            // SAFETY: query-only existing HWND and valid output RECT.
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::GetWindowRect(
                    HWND(w.identity.handle as *mut _),
                    &mut bounds,
                )
                .unwrap();
            }
            (w.identity, bounds)
        })
        .collect::<Vec<_>>();
    let displays = display_catalog().unwrap();
    let display = displays.iter().find(|d| !d.primary).unwrap_or(&displays[0]);
    let mut cleanup = Windows(vec![]);
    for shell in [
        TerminalShell::PowerShell,
        TerminalShell::CommandPrompt,
        TerminalShell::Wsl,
    ] {
        let output = dir.path().join(format!("{shell:?}.txt"));
        let marker = "日本語 spaces 'quotes' & ;";
        let script = match shell {
            TerminalShell::PowerShell => format!(
                "if ([Console]::IsInputRedirected) {{ exit 23 }}\nSet-Content -LiteralPath '{}' -Value @('{}', $PWD.Path) -Encoding UTF8",
                output.to_string_lossy().replace('\'', "''"),
                marker.replace('\'', "''")
            ),
            TerminalShell::CommandPrompt => format!(
                "echo cmd-marker > \"{}\"\ncd >> \"{}\"",
                output.display(),
                output.display()
            ),
            TerminalShell::Wsl => format!(
                "test -t 0 && test -t 1 || exit 23\r\nprintf '%s\\n' \"{}\" \"$PWD\" > '{}'",
                marker,
                linux(&output)
            ),
        };
        let entry = WorkspaceEntry {
            label: format!("Terminal test {shell:?}"),
            terminal: Some(TerminalSettings {
                shell,
                script,
                distribution: if shell == TerminalShell::Wsl {
                    "Ubuntu-24.04".into()
                } else {
                    String::new()
                },
                ..Default::default()
            }),
            working_directory: if shell == TerminalShell::Wsl {
                linux(&working)
            } else {
                working.to_string_lossy().into_owned()
            },
            monitor_id: display.id.clone(),
            placement: Placement::Rect {
                x: 0.5,
                y: 0.0,
                width: 0.5,
                height: 1.0,
            },
            ..Default::default()
        };
        cleanup.0.push(entry.clone());
        let mut workspace = Workspace::new("Terminal native");
        workspace.entries.push(entry.clone());
        let mut runner = WorkspaceRunner::default();
        runner.start(workspace, || {}).unwrap();
        let start = Instant::now();
        let status = loop {
            let status = runner.poll();
            if !status.running {
                break status;
            }
            assert!(start.elapsed() < Duration::from_secs(20));
            thread::sleep(Duration::from_millis(100));
        };
        assert_eq!(status.results.len(), 1, "{status:?}");
        assert!(
            status.results[0].message.contains("起動・配置完了"),
            "{status:?}"
        );
        let start = Instant::now();
        while !output.exists() {
            assert!(
                start.elapsed() < Duration::from_secs(15),
                "script did not run for {shell:?}"
            );
            thread::sleep(Duration::from_millis(100));
        }
        let bytes = std::fs::read(&output).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains(if shell == TerminalShell::CommandPrompt {
                "cmd-marker"
            } else {
                marker
            }),
            "{shell:?}: {text}"
        );
        // cmd inherits OEM encoding, so ASCII portion confirms cwd; UTF-8 shells confirm all.
        assert!(
            text.contains(if shell == TerminalShell::CommandPrompt {
                "project space"
            } else {
                &entry.working_directory
            }),
            "{shell:?}: {text}"
        );
        let window = workspace_windows::enumerate()
            .unwrap()
            .into_iter()
            .find(|w| matches(&entry, w))
            .unwrap();
        assert!(workspace_windows::placed(
            window.identity,
            entry.placement,
            display.work
        ));
        println!(
            "TERMINAL_NATIVE_PASS: {shell:?} script + cwd + submonitor placement; new Terminal remains interactive"
        );
    }
    for (identity, previous) in existing {
        let mut current = windows::Win32::Foundation::RECT::default();
        // SAFETY: test only queries the existing Terminal windows, never modifies them.
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetWindowRect(
                HWND(identity.handle as *mut _),
                &mut current,
            )
            .unwrap();
        }
        assert_eq!(current, previous, "existing Terminal must not move");
    }
}
