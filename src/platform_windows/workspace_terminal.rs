//! Only starts the GUI terminal launcher; shell startup runs inside Terminal's own PTY.
#[cfg(test)]
mod native_tests;
use crate::core::{TerminalSettings, TerminalShell, WorkspaceEntry};
use std::{
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
pub(super) fn launch(entry: &WorkspaceEntry) -> Result<(), String> {
    let settings = entry
        .terminal
        .as_ref()
        .ok_or("Terminal 設定がありません。")?;
    settings
        .validate(&entry.working_directory)
        .map_err(|e| e.to_string())?;
    if settings.shell != TerminalShell::Wsl
        && !entry.working_directory.is_empty()
        && (!Path::new(&entry.working_directory).is_absolute()
            || !Path::new(&entry.working_directory).is_dir())
    {
        return Err("Terminal の Windows 作業ディレクトリが見つかりません。".into());
    }
    let path = if settings.launcher.is_empty() {
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA がありません。")?)
            .join("Microsoft/WindowsApps/wt.exe")
    } else {
        super::workspace_launch::executable(&settings.launcher)?
    };
    if !path.is_file() {
        return Err("Windows Terminal (wt.exe) が見つかりません。インストールと実行エイリアスを確認するか、Terminal 実行ファイルを指定してください。".into());
    }
    Command::new(path)
        .args(settings.arguments(entry.id, &entry.working_directory))
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Terminal 起動失敗: {e}"))
}
pub(super) fn matches(entry: &WorkspaceEntry, window: &super::workspace_windows::Window) -> bool {
    window.executable.rsplit('\\').next() == Some("windowsterminal.exe")
        && window.title.contains(&TerminalSettings::title(entry.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_matching_requires_the_owner_and_this_recipe_title() {
        let entry = WorkspaceEntry::default();
        let mut window = super::super::workspace_windows::Window {
            identity: crate::core::WindowIdentity { handle: 1, pid: 1 },
            executable: "c:\\installed\\windowsterminal.exe".into(),
            title: TerminalSettings::title(entry.id),
        };
        assert!(matches(&entry, &window));
        window.executable = "c:\\installed\\other.exe".into();
        assert!(!matches(&entry, &window));
        window.executable = "c:\\installed\\windowsterminal.exe".into();
        window.title = TerminalSettings::title(uuid::Uuid::new_v4());
        assert!(!matches(&entry, &window));
    }
    #[test]
    fn bad_directory_and_launcher_are_rejected_before_starting_any_process() {
        let dir = tempfile::tempdir().unwrap();
        let mut entry = WorkspaceEntry {
            terminal: Some(TerminalSettings {
                launcher: dir
                    .path()
                    .join("missing.exe")
                    .to_string_lossy()
                    .into_owned(),
                ..Default::default()
            }),
            working_directory: "relative".into(),
            ..Default::default()
        };
        assert!(launch(&entry).unwrap_err().contains("作業ディレクトリ"));
        entry.working_directory.clear();
        assert!(launch(&entry).unwrap_err().contains(".exe"));
    }
}
