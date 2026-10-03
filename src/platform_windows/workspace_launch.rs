//! Direct CreateProcess via Rust Command: argv is not evaluated by PowerShell/cmd.
use crate::core::WorkspaceEntry;
use std::{
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn executable(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if !p.is_absolute()
        || !p.is_file()
        || !p.extension().is_some_and(|s| s.eq_ignore_ascii_case("exe"))
    {
        return Err("実在する .exe の絶対パスを指定してください。".into());
    }
    std::fs::canonicalize(p).map_err(|e| e.to_string())
}
pub(super) fn launch(entry: &WorkspaceEntry) -> Result<(), String> {
    executable(&entry.executable)?;
    let mut cmd = Command::new(&entry.executable);
    cmd.args(&entry.arguments).creation_flags(0x08000000); // CREATE_NO_WINDOW: GUI apps unaffected.
    if !entry.working_directory.is_empty() {
        if !Path::new(&entry.working_directory).is_absolute()
            || !Path::new(&entry.working_directory).is_dir()
        {
            return Err("作業ディレクトリが見つかりません。".into());
        }
        cmd.current_dir(&entry.working_directory);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("起動失敗: {e}"))
}
pub(super) fn path_key(path: &str) -> String {
    path.trim_start_matches(r"\\?\")
        .replace('/', "\\")
        .to_lowercase()
}
