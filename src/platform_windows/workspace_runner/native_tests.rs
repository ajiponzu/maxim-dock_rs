//! Opt-in real HWND tests. Only temp-dir fixture processes are launched/terminated.
use super::*;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use windows::Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*};
struct Fixture {
    dir: tempfile::TempDir,
    exe: PathBuf,
    outputs: Vec<PathBuf>,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("workspace fixture.exe");
        assert!(
            Command::new("rustc")
                .args([
                    "--edition=2024",
                    "src/platform_windows/fixtures/workspace_window.rs",
                    "-o"
                ])
                .arg(&exe)
                .status()
                .unwrap()
                .success()
        );
        Self {
            dir,
            exe,
            outputs: vec![],
        }
    }
    fn entry(&mut self, name: &str, extra: &[&str]) -> WorkspaceEntry {
        let out = self
            .dir
            .path()
            .join(format!("{}-{name}.txt", self.outputs.len()));
        self.outputs.push(out.clone());
        WorkspaceEntry {
            label: name.into(),
            executable: self.exe.to_string_lossy().into(),
            working_directory: self.dir.path().to_string_lossy().into(),
            arguments: std::iter::once(out.to_string_lossy().into_owned())
                .chain(std::iter::once(name.into()))
                .chain(extra.iter().map(|s| s.to_string()))
                .collect(),
            placement: Placement::Rect {
                x: 0.5,
                y: 0.0,
                width: 0.5,
                height: 1.0,
            },
            ..Default::default()
        }
    }
    fn windows(&self, name: &str) -> Vec<workspace_windows::Window> {
        workspace_windows::enumerate()
            .unwrap()
            .into_iter()
            .filter(|w| {
                w.executable == workspace_launch::path_key(&self.exe.to_string_lossy())
                    && w.title == name
            })
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for out in &self.outputs {
            if let Ok(s) = std::fs::read_to_string(out)
                && let Some(pid) = s.lines().next().and_then(|s| s.parse::<u32>().ok())
            {
                // SAFETY: PID is supplied exclusively by this fixture in its private temp directory.
                unsafe {
                    if let Ok(h) = OpenProcess(
                        PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                        false,
                        pid,
                    ) {
                        let _ = TerminateProcess(h, 0);
                        let _ = CloseHandle(h);
                    }
                }
            }
        }
    }
}
fn wait_file(path: &Path) {
    let start = Instant::now();
    while !path.exists() {
        assert!(start.elapsed() < Duration::from_secs(4));
        thread::sleep(Duration::from_millis(20));
    }
}
fn wait_done(runner: &mut WorkspaceRunner) -> RestoreStatus {
    let start = Instant::now();
    loop {
        let s = runner.poll();
        if !s.running {
            return s;
        }
        assert!(start.elapsed() < Duration::from_secs(20));
        thread::sleep(Duration::from_millis(30));
    }
}
fn bounds(w: WindowIdentity) -> RECT {
    let mut r = RECT::default(); // SAFETY: test enumerated live HWND, writable RECT.
    unsafe {
        GetWindowRect(HWND(w.handle as *mut _), &mut r).unwrap();
    }
    r
}
#[test]
#[ignore = "opens the user's saved WSL code project in a new VS Code window; leaves it open"]
fn native_user_wsl_vscode_project_window() {
    let _dpi = workspace_windows::PhysicalDpi::enter().unwrap();
    let path = PathBuf::from(std::env::var_os("APPDATA").unwrap()).join("MaXImDock/config.toml");
    let original = std::fs::read(&path).unwrap();
    let config: Config = toml::from_str(std::str::from_utf8(&original).unwrap()).unwrap();
    let mut entry = config
        .workspaces
        .iter()
        .flat_map(|w| &w.entries)
        .find(|e| e.executable == "code" && e.wsl.is_some())
        .expect("saved WSL code entry")
        .clone();
    let wsl = entry.wsl.as_mut().unwrap();
    wsl.login_shell = false;
    wsl.direct_exec = false;
    wsl.place_window = true;
    wsl.wait_for_exit = true;
    entry.window_executable = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join("Programs/Microsoft VS Code/Code.exe")
        .to_string_lossy()
        .into_owned();
    if !entry.arguments.iter().any(|a| a == "--new-window") {
        entry.arguments.insert(0, "--new-window".into());
    }
    let target = workspace_launch::path_key(&entry.window_executable);
    let old: Vec<_> = workspace_windows::enumerate()
        .unwrap()
        .into_iter()
        .filter(|w| w.executable == target)
        .map(|w| (w.identity, bounds(w.identity)))
        .collect();
    let result = restore_entry(&entry, &AtomicBool::new(false));
    assert!(result.is_ok(), "{result:?}");
    assert!(result.unwrap().contains("起動・配置完了"));
    assert_eq!(
        std::fs::read(path).unwrap(),
        original,
        "saved user recipe must not be modified"
    );
    for (identity, previous) in &old {
        assert_eq!(
            bounds(*identity),
            *previous,
            "existing VS Code must not move"
        );
    }
    let current = workspace_windows::enumerate().unwrap();
    let new = current
        .iter()
        .find(|w| w.executable == target && !old.iter().any(|(id, _)| *id == w.identity))
        .unwrap();
    assert!(
        new.title.contains("WSL"),
        "new VS Code must be a WSL window: {}",
        new.title
    );
    println!(
        "WSL_VSCODE_PROJECT_NATIVE_PASS: {} ; saved config/existing window bounds unchanged; new project window remains open",
        new.title
    );
}
#[test]
#[ignore = "launches controlled native windows through Ubuntu-24.04; requires desktop/WSL access"]
fn native_wsl_windows_placement_and_existing_window_safety() {
    let _dpi = workspace_windows::PhysicalDpi::enter().unwrap();
    let mut fixture = Fixture::new();
    let old = fixture.entry("wsl-same-title", &[]);
    workspace_launch::launch(&old).unwrap();
    wait_file(&fixture.outputs[0]);
    let started = Instant::now();
    while fixture.windows("wsl-same-title").is_empty() {
        assert!(started.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(20));
    }
    let old_identity = fixture.windows("wsl-same-title")[0].identity;
    let old_bounds = bounds(old_identity);
    let displays = display_catalog().unwrap();
    let display = displays.iter().find(|d| !d.primary).unwrap_or(&displays[0]);
    let mut entry = fixture.entry("wsl-same-title", &[]);
    let windows_path = entry.executable.replace('\\', "/");
    entry.window_executable = entry.executable.clone();
    entry.executable = format!(
        "/mnt/{}/{}",
        windows_path[..1].to_lowercase(),
        &windows_path[3..]
    );
    entry.working_directory = "/tmp".into();
    entry.title_contains = "wsl-same-title".into();
    entry.monitor_id = display.id.clone();
    entry.wsl = Some(WslSettings {
        distribution: "Ubuntu-24.04".into(),
        wait_for_exit: false,
        place_window: true,
        ..Default::default()
    });
    let mut workspace = Workspace::new("WSL Windows window");
    workspace.entries.push(entry.clone());
    let mut runner = WorkspaceRunner::default();
    runner.start(workspace, || {}).unwrap();
    let status = wait_done(&mut runner);
    assert_eq!(status.results.len(), 1);
    assert!(
        status.results[0].message.contains("起動・配置完了"),
        "{status:?}"
    );
    assert_eq!(
        bounds(old_identity),
        old_bounds,
        "existing Windows window must remain untouched"
    );
    let new = fixture
        .windows("wsl-same-title")
        .into_iter()
        .find(|w| w.identity != old_identity)
        .unwrap();
    assert!(workspace_windows::placed(
        new.identity,
        entry.placement,
        display.work
    ));
    let output = std::fs::read_to_string(&fixture.outputs[1]).unwrap();
    assert_eq!(
        output.lines().nth(1).unwrap(),
        format!("{:?}", entry.arguments)
    );
    println!(
        "WSL_WINDOW_NATIVE_PASS: new window placed on {} with {} connected monitors; existing unchanged",
        display.name,
        displays.len()
    );
}
#[test]
#[ignore = "launches controlled native windows; run with desktop access"]
fn native_workspace_launch_placement_timeout_cancel_and_existing_window_safety() {
    let _dpi = workspace_windows::PhysicalDpi::enter().unwrap();
    let mut fixture = Fixture::new();
    let old = fixture.entry("same-title", &[]);
    workspace_launch::launch(&old).unwrap();
    wait_file(&fixture.outputs[0]);
    let start = Instant::now();
    while fixture.windows("same-title").is_empty() {
        assert!(start.elapsed() < Duration::from_secs(4));
        thread::sleep(Duration::from_millis(20));
    }
    let old_window = fixture.windows("same-title")[0].identity;
    let old_bounds = bounds(old_window);
    let mut entry = fixture.entry(
        "same-title",
        &[
            "",
            "日本語 with spaces",
            "a\"b",
            "https://example.com/?a=1&b=2",
            "C:\\folder with space\\",
        ],
    );
    entry.monitor_id = "disconnected-test-monitor".into();
    entry.title_contains = "same-title".into();
    let mut workspace = Workspace::new("native");
    workspace.entries.push(entry.clone());
    let mut runner = WorkspaceRunner::default();
    runner.start(workspace.clone(), || {}).unwrap();
    assert!(runner.start(workspace, || {}).is_err());
    let status = wait_done(&mut runner);
    assert_eq!(status.results.len(), 1);
    assert!(
        status.results[0].message.starts_with("起動・配置完了"),
        "{status:?}"
    );
    let output = std::fs::read_to_string(&fixture.outputs[1]).unwrap();
    assert_eq!(
        output.lines().nth(1).unwrap(),
        format!("{:?}", entry.arguments)
    );
    assert_eq!(output.lines().nth(2).unwrap(), entry.working_directory);
    assert_eq!(
        bounds(old_window),
        old_bounds,
        "existing window must remain untouched"
    );
    let windows = fixture.windows("same-title");
    let new = windows.iter().find(|w| w.identity != old_window).unwrap();
    let displays = display_catalog().unwrap();
    println!("WORKSPACE_NATIVE: {} connected monitor(s)", displays.len());
    let work = select_display("", &displays).unwrap().0.work;
    let expected = entry.placement.rectangle(work);
    let actual = bounds(new.identity);
    assert_eq!(
        (
            actual.left,
            actual.top,
            actual.right - actual.left,
            actual.bottom - actual.top
        ),
        (expected.left, expected.top, expected.width, expected.height)
    );
    assert!(status.results[0].message.contains("primary"));
    for (index, display) in displays.iter().enumerate() {
        let mut e = fixture.entry(&format!("monitor-{index}"), &[]);
        e.monitor_id = display.id.clone();
        e.placement = Placement::Maximize;
        let mut w = Workspace::new("maximize");
        w.entries.push(e);
        runner.start(w, || {}).unwrap();
        let s = wait_done(&mut runner);
        assert!(s.results[0].message.starts_with("起動・配置完了"), "{s:?}");
        let identity = fixture.windows(&format!("monitor-{index}"))[0].identity;
        assert!(workspace_windows::placed(
            identity,
            Placement::Maximize,
            display.work
        ));
        let r = bounds(identity);
        assert!(
            display.bounds.contains(ScreenPoint {
                x: r.left + (r.right - r.left) / 2,
                y: r.top + (r.bottom - r.top) / 2
            }),
            "maximized on the requested monitor"
        );
        // SAFETY: fixture window was just enumerated; query-only API.
        let dpi =
            unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(HWND(identity.handle as *mut _)) };
        println!("WORKSPACE_NATIVE: monitor-{index} placement verified, window DPI={dpi}");
    }
    let ambiguous = fixture.entry("ambiguous", &["--two-windows"]);
    let mut w = Workspace::new("ambiguous");
    w.entries.push(ambiguous);
    runner.start(w, || {}).unwrap();
    let s = wait_done(&mut runner);
    assert!(s.results[0].message.contains("複数"), "{s:?}");
    let timeout = fixture.entry("timeout", &["--no-window"]);
    let mut w = Workspace::new("timeout");
    w.entries.push(timeout);
    runner.start(w, || {}).unwrap();
    let start = Instant::now();
    let s = wait_done(&mut runner);
    assert!(start.elapsed() >= Duration::from_secs(15));
    assert!(s.results[0].message.contains("15 秒"), "{s:?}");
    let cancel = fixture.entry("cancel", &["--no-window"]);
    let out = fixture.outputs.last().unwrap().clone();
    let skipped = fixture.entry("skipped", &[]);
    let skipped_out = fixture.outputs.last().unwrap().clone();
    let mut w = Workspace::new("cancel");
    w.entries = vec![cancel, skipped];
    runner.start(w, || {}).unwrap();
    wait_file(&out);
    runner.cancel();
    let s = wait_done(&mut runner);
    assert!(s.results[0].message.contains("中止"), "{s:?}");
    assert!(!skipped_out.exists());
}
