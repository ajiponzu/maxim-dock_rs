//! File arguments for registered applications. Never copy/move or use a command shell.
use super::{PlatformError, error};
use crate::core::{DockItem, TargetKind};
use std::path::{Path, PathBuf};
use windows::{
    Win32::{
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize, IPersistFile, STGM_READ,
        },
        UI::{
            Shell::{IShellLinkW, ShellExecuteW, ShellLink},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
    core::{Interface, PCWSTR, w},
};

struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        // SAFETY: balances this module's successful CoInitializeEx on the same thread.
        unsafe {
            CoUninitialize();
        }
    }
}
fn com() -> Result<Com, PlatformError> {
    // SAFETY: scoped to this thread; S_FALSE also needs a balancing uninitialize.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .map_err(|e| error("CoInitializeEx", e))?;
    Ok(Com)
}
fn wide(s: &str) -> Result<Vec<u16>, PlatformError> {
    if s.contains('\0') {
        return Err(PlatformError::Invalid("embedded NUL"));
    }
    Ok(s.encode_utf16().chain(Some(0)).collect())
}
fn text(buffer: &[u16]) -> Result<String, PlatformError> {
    let end = buffer
        .iter()
        .position(|c| *c == 0)
        .ok_or(PlatformError::Invalid("unterminated Shell string"))?;
    String::from_utf16(&buffer[..end])
        .map_err(|_| PlatformError::Invalid("non-Unicode Shell string"))
}
fn executable(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
}

struct LaunchPlan {
    target: String,
    parameters: String,
    directory: String,
}
fn plan(item: &DockItem, paths: &[PathBuf]) -> Result<LaunchPlan, PlatformError> {
    if item.kind != TargetKind::Path || paths.is_empty() {
        return Err(PlatformError::Invalid(
            "not an application or empty file drop",
        ));
    }
    let path = Path::new(&item.target);
    let mut target = item.target.clone();
    let mut parameters = String::new();
    let mut directory = String::new();
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
    {
        if !path.is_file() {
            return Err(PlatformError::Invalid("missing shortcut"));
        }
        let _com = com()?;
        let name = wide(&item.target)?;
        // SAFETY: COM is initialized, all owned buffers live through their calls.
        unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| error("ShellLink", e))?;
            let persist: IPersistFile = link.cast().map_err(|e| error("IPersistFile", e))?;
            persist
                .Load(PCWSTR(name.as_ptr()), STGM_READ)
                .map_err(|e| error("Load shortcut", e))?;
            let mut buffer = vec![0; 32768];
            link.GetPath(&mut buffer, std::ptr::null_mut(), 0)
                .map_err(|e| error("Shortcut target", e))?;
            target = text(&buffer)?;
            buffer.fill(0);
            link.GetArguments(&mut buffer)
                .map_err(|e| error("Shortcut arguments", e))?;
            parameters = text(&buffer)?;
            buffer.fill(0);
            link.GetWorkingDirectory(&mut buffer)
                .map_err(|e| error("Shortcut directory", e))?;
            directory = text(&buffer)?;
        }
    }
    if !executable(Path::new(&target)) {
        return Err(PlatformError::Invalid(
            "drop target must resolve to an existing exe",
        ));
    }
    for path in paths {
        if !path.is_file() {
            return Err(PlatformError::Invalid(
                "drop must contain existing files only",
            ));
        }
        let absolute = std::path::absolute(path)
            .map_err(|_| PlatformError::Invalid("invalid dropped path"))?;
        let argument = absolute
            .to_str()
            .ok_or(PlatformError::Invalid("non-Unicode dropped path"))?;
        if !parameters.is_empty() {
            parameters.push(' ');
        }
        parameters.push_str(&quote_argument(argument)?);
    }
    // Check the Win32 command-line bound before invoking Shell.
    if parameters.encode_utf16().count() + target.encode_utf16().count() + 4 >= 32767 {
        return Err(PlatformError::Invalid("too many dropped files"));
    }
    Ok(LaunchPlan {
        target,
        parameters,
        directory,
    })
}

/// Windows argv quoting: double backslashes before quotes and the closing quote.
fn quote_argument(argument: &str) -> Result<String, PlatformError> {
    if argument.contains('\0') {
        return Err(PlatformError::Invalid("embedded NUL"));
    }
    let mut quoted = String::from("\"");
    let mut slashes = 0;
    for c in argument.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        quoted.extend(std::iter::repeat_n(
            '\\',
            if c == '"' { slashes * 2 + 1 } else { slashes },
        ));
        quoted.push(c);
        slashes = 0;
    }
    quoted.extend(std::iter::repeat_n('\\', slashes * 2));
    quoted.push('"');
    Ok(quoted)
}

pub fn open_files_with_app(item: &DockItem, paths: &[PathBuf]) -> Result<(), PlatformError> {
    let plan = plan(item, paths)?;
    let _com = com()?;
    let target = wide(&plan.target)?;
    let parameters = wide(&plan.parameters)?;
    let directory = wide(&plan.directory)?;
    // SAFETY: terminated UTF-16 buffers outlive this synchronous call. Shell retains none.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(target.as_ptr()),
            PCWSTR(parameters.as_ptr()),
            if plan.directory.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(directory.as_ptr())
            },
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    if code <= 32 {
        Err(PlatformError::Shell(code))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_shell_delivers_multiple_paths_to_controlled_exe_and_shortcut() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("receiver 日本語.exe");
        let status = std::process::Command::new("rustc")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/drop_receiver.rs"
            ))
            .args(["--crate-name", "drop_receiver", "-o"])
            .arg(&exe)
            .status()
            .unwrap();
        assert!(status.success(), "build controlled receiver");
        let files = [
            tmp.path().join("日本語 & first.txt"),
            tmp.path().join("second space.txt"),
        ];
        for file in &files {
            std::fs::write(file, b"unchanged").unwrap();
        }
        let output = exe.with_extension("args");
        let item = DockItem::new("receiver", exe.to_str().unwrap(), TargetKind::Path);
        open_files_with_app(&item, &files).unwrap();
        let wait_output = || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                if let Ok(args) = std::fs::read_to_string(&output) {
                    return args;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "receiver did not record argv"
                );
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        };
        let expected = files
            .iter()
            .map(|p| p.to_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(wait_output(), expected);
        std::fs::remove_file(&output).unwrap();
        let _com = com().unwrap();
        let shortcut = tmp.path().join("receiver.lnk");
        let shortcut_wide = wide(shortcut.to_str().unwrap()).unwrap();
        let exe_wide = wide(exe.to_str().unwrap()).unwrap();
        // SAFETY: initialized COM and owned buffers valid for each call.
        unsafe {
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            link.SetPath(PCWSTR(exe_wide.as_ptr())).unwrap();
            link.SetArguments(w!("--marker \"two words\"")).unwrap();
            let persist: IPersistFile = link.cast().unwrap();
            persist.Save(PCWSTR(shortcut_wide.as_ptr()), true).unwrap();
        }
        let item = DockItem::new(
            "receiver shortcut",
            shortcut.to_str().unwrap(),
            TargetKind::Path,
        );
        open_files_with_app(&item, &files).unwrap();
        assert_eq!(wait_output(), format!("--marker\ntwo words\n{expected}"));
        for file in &files {
            assert_eq!(std::fs::read(file).unwrap(), b"unchanged");
        }
    }
    #[test]
    fn quoting_handles_spaces_unicode_quotes_and_trailing_slashes() {
        assert_eq!(
            quote_argument("a b 日本語 & c.txt").unwrap(),
            "\"a b 日本語 & c.txt\""
        );
        assert_eq!(quote_argument("a\\\"b").unwrap(), "\"a\\\\\\\"b\"");
        assert_eq!(
            quote_argument("C:\\space dir\\").unwrap(),
            "\"C:\\space dir\\\\\""
        );
        assert!(quote_argument("bad\0path").is_err());
    }
    #[test]
    fn drop_plan_validates_whole_batch_without_launching_or_changing_files() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("日本語 & spaced.txt");
        std::fs::write(&file, b"unchanged").unwrap();
        let exe = std::env::current_exe().unwrap();
        let item = DockItem::new("app", exe.to_str().unwrap(), TargetKind::Path);
        let launch = plan(&item, &[file.clone(), file.clone()]).unwrap();
        assert_eq!(
            launch.parameters,
            format!("{0} {0}", quote_argument(file.to_str().unwrap()).unwrap())
        );
        assert!(plan(&item, &[file.clone(), tmp.path().join("missing")]).is_err());
        assert!(plan(&item, &[tmp.path().into()]).is_err());
        assert!(
            plan(
                &DockItem::new("url", "https://example.com", TargetKind::Url),
                std::slice::from_ref(&file)
            )
            .is_err()
        );
        assert_eq!(std::fs::read(file).unwrap(), b"unchanged");
    }
    #[test]
    fn shortcut_preserves_arguments_and_rejects_document_targets() {
        let _com = com().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let shortcut = tmp.path().join("app 日本語.lnk");
        let file = tmp.path().join("data.txt");
        std::fs::write(&file, b"data").unwrap();
        let exe = std::env::current_exe().unwrap();
        let shortcut_wide = wide(shortcut.to_str().unwrap()).unwrap();
        let exe_wide = wide(exe.to_str().unwrap()).unwrap();
        // SAFETY: initialized COM and owned terminated buffers for each synchronous call.
        unsafe {
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            link.SetPath(PCWSTR(exe_wide.as_ptr())).unwrap();
            link.SetArguments(w!("--existing \"two words\"")).unwrap();
            let persist: IPersistFile = link.cast().unwrap();
            persist.Save(PCWSTR(shortcut_wide.as_ptr()), true).unwrap();
            let item = DockItem::new("app", shortcut.to_str().unwrap(), TargetKind::Path);
            let launch = plan(&item, std::slice::from_ref(&file)).unwrap();
            assert_eq!(launch.target, exe.to_str().unwrap());
            assert!(launch.parameters.starts_with("--existing \"two words\" "));
            let document = wide(file.to_str().unwrap()).unwrap();
            link.SetPath(PCWSTR(document.as_ptr())).unwrap();
            persist.Save(PCWSTR(shortcut_wide.as_ptr()), true).unwrap();
            assert!(plan(&item, &[file]).is_err());
        }
    }
}
