//! Shared window wait/placement stage, independent of the Windows/WSL launch mechanism.
use super::*;
pub(super) fn wait(
    entry: &WorkspaceEntry,
    cancel: &AtomicBool,
    target: String,
    display_id: String,
    before: std::collections::HashSet<WindowIdentity>,
    fallback: bool,
) -> Result<String, String> {
    let started = Instant::now();
    let mut tracker = CandidateTracker::default();
    while started.elapsed() < Duration::from_secs(15) {
        if cancel.load(Ordering::Acquire) {
            return Err("中止しました。起動済みアプリは終了しません。".into());
        }
        let windows = workspace_windows::enumerate().map_err(|e| e.to_string())?;
        let matches: Vec<_> = windows
            .iter()
            .filter(|w| {
                (if entry.terminal.is_some() && target.is_empty() {
                    workspace_terminal::matches(entry, w)
                } else {
                    w.executable == target
                }) && (entry.title_contains.is_empty() || w.title.contains(&entry.title_contains))
            })
            .map(|w| w.identity)
            .collect();
        match tracker.observe(&before, &matches) {
            CandidateDecision::Ambiguous => return Err("起動しましたが新規ウィンドウが複数あります。タイトル条件を指定してください。配置していません。".into()),
            CandidateDecision::Ready(identity) => {
                // Re-enumerate monitors to avoid applying stale geometry after hot unplug.
                let current = display_catalog().map_err(|e| e.to_string())?;
                let (display, missing) = select_display(&display_id, &current).ok_or("モニターがありません。")?;
                if cancel.load(Ordering::Acquire) {
                    return Err("配置を中止しました。起動済みアプリは終了しません。".into());
                }
                workspace_windows::place(identity, entry.placement, display.work).map_err(|e| e.to_string())?;
                let placement_started = Instant::now();
                while !workspace_windows::placed(identity, entry.placement, display.work) {
                    if cancel.load(Ordering::Acquire) {
                        return Err("配置待機を中止しました。起動済みアプリは終了しません。".into());
                    }
                    if placement_started.elapsed() > Duration::from_secs(2) || started.elapsed() >= Duration::from_secs(15) {
                        return Err("起動済みですが配置を確認できません。アプリのサイズ制約・権限・応答を確認してください。".into());
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                return Ok(if fallback || missing { "起動・配置完了。指定モニターがないため primary に配置しました。" } else { "起動・配置完了。" }.into());
            }
            CandidateDecision::Wait => thread::sleep(Duration::from_millis(200)),
        }
    }
    Err("起動しましたが 15 秒以内に対象の新規ウィンドウを確認できませんでした。既存ウィンドウは変更しません。".into())
}
