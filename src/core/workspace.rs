//! Saved launch recipes and physical placement math; no GUI or OS dependencies.
use super::{MonitorRect, ValidationError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub entries: Vec<WorkspaceEntry>,
}
impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            entries: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceEntry {
    pub id: Uuid,
    pub label: String,
    pub executable: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wsl: Option<super::WslSettings>,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub working_directory: String,
    /// Empty means primary. Otherwise a Windows monitor device interface, not HMONITOR.
    #[serde(default)]
    pub monitor_id: String,
    #[serde(default)]
    pub placement: Placement,
    /// Empty means executable. Useful for launch brokers (e.g. wt.exe).
    #[serde(default)]
    pub window_executable: String,
    #[serde(default)]
    pub title_contains: String,
}
impl Default for WorkspaceEntry {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            label: "アプリ".into(),
            executable: String::new(),
            wsl: None,
            arguments: vec![],
            working_directory: String::new(),
            monitor_id: String::new(),
            placement: Placement::default(),
            window_executable: String::new(),
            title_contains: String::new(),
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Placement {
    #[default]
    Maximize,
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
}
impl Placement {
    pub fn valid(self) -> bool {
        match self {
            Self::Maximize => true,
            Self::Rect {
                x,
                y,
                width,
                height,
            } => {
                [x, y, width, height].iter().all(|v| v.is_finite())
                    && x >= 0.0
                    && y >= 0.0
                    && width > 0.0
                    && height > 0.0
                    && x + width <= 1.000001
                    && y + height <= 1.000001
            }
        }
    }
    pub fn rectangle(self, work: MonitorRect) -> MonitorRect {
        let (x, y, w, h) = match self {
            Self::Maximize => (0.0, 0.0, 1.0, 1.0),
            Self::Rect {
                x,
                y,
                width,
                height,
            } => (x, y, width, height),
        };
        let left = (x * f64::from(work.width)).round() as i32;
        let top = (y * f64::from(work.height)).round() as i32;
        let right = ((x + w) * f64::from(work.width)).round() as i32;
        let bottom = ((y + h) * f64::from(work.height)).round() as i32;
        MonitorRect {
            left: work.left.saturating_add(left),
            top: work.top.saturating_add(top),
            width: (right - left).max(1),
            height: (bottom - top).max(1),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub bounds: MonitorRect,
    pub work: MonitorRect,
    pub primary: bool,
}
pub fn select_display<'a>(
    id: &str,
    displays: &'a [DisplayInfo],
) -> Option<(&'a DisplayInfo, bool)> {
    if !id.is_empty()
        && let Some(display) = displays.iter().find(|d| d.id.eq_ignore_ascii_case(id))
    {
        return Some((display, false));
    }
    displays
        .iter()
        .find(|d| d.primary)
        .or_else(|| displays.first())
        .map(|d| (d, !id.is_empty()))
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RestoreStatus {
    pub running: bool,
    pub workspace_name: String,
    pub results: Vec<EntryResult>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct EntryResult {
    pub label: String,
    pub message: String,
}

pub fn validate_workspaces(workspaces: &[Workspace]) -> Result<(), ValidationError> {
    let mut ids = HashSet::new();
    let printable = |s: &str, max: usize| {
        !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
    };
    let text = |s: &str| s.len() <= 32768 && !s.chars().any(char::is_control);
    if workspaces.len() > 64 {
        return Err(ValidationError("At most 64 workspaces are supported."));
    }
    for w in workspaces {
        if w.id.is_nil() || !ids.insert(w.id) || !printable(&w.name, 80) || w.entries.len() > 64 {
            return Err(ValidationError(
                "Workspace names/IDs/entry count are invalid.",
            ));
        }
        for e in &w.entries {
            if let Some(wsl) = &e.wsl {
                wsl.validate(&e.executable, &e.working_directory)?;
                if wsl.place_window && e.window_executable.trim().is_empty() {
                    return Err(ValidationError(
                        "WSL window placement requires the Windows window executable.",
                    ));
                }
            }
            if e.id.is_nil()
                || !ids.insert(e.id)
                || !printable(&e.label, 80)
                || e.executable.trim().is_empty()
                || !text(&e.executable)
                || !text(&e.working_directory)
                || !text(&e.monitor_id)
                || !text(&e.window_executable)
                || !text(&e.title_contains)
                || e.arguments.len() > 256
                || e.arguments.iter().any(|a| !text(a))
                || !e.placement.valid()
            {
                return Err(ValidationError(
                    "Invalid workspace entry: check executable, arguments and normalized placement.",
                ));
            }
            let command_units = if let Some(wsl) = &e.wsl {
                wsl.arguments(&e.executable, &e.arguments, &e.working_directory)
                    .iter()
                    .map(|a| 2 * a.encode_utf16().count() + 3)
                    .sum::<usize>()
            } else {
                e.executable.encode_utf16().count()
                    + e.arguments
                        .iter()
                        .map(|a| 2 * a.encode_utf16().count() + 3)
                        .sum::<usize>()
            };
            if command_units > 32000 {
                return Err(ValidationError("Workspace command line is too long."));
            }
        }
    }
    Ok(())
}

/// Candidate identity includes PID so a reused native handle is not mistaken for an old window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowIdentity {
    pub handle: usize,
    pub pid: u32,
}
#[derive(Debug, Default)]
pub struct CandidateTracker {
    last: Option<WindowIdentity>,
    stable: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateDecision {
    Wait,
    Ready(WindowIdentity),
    Ambiguous,
}
impl CandidateTracker {
    pub fn observe(
        &mut self,
        before: &HashSet<WindowIdentity>,
        matching: &[WindowIdentity],
    ) -> CandidateDecision {
        let candidates: Vec<_> = matching
            .iter()
            .copied()
            .filter(|w| !before.contains(w))
            .collect();
        if candidates.len() > 1 {
            self.last = None;
            self.stable = 0;
            return CandidateDecision::Ambiguous;
        }
        let current = candidates.first().copied();
        if current.is_some() && current == self.last {
            self.stable += 1;
        } else {
            self.last = current;
            self.stable = 0;
        }
        if self.stable >= 3 {
            CandidateDecision::Ready(current.unwrap())
        } else {
            CandidateDecision::Wait
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_legacy_and_recipe_round_trip() {
        let mut c = crate::core::Config::defaults("missing".into());
        let mut old = toml::Value::try_from(&c).unwrap();
        old.as_table_mut().unwrap().remove("workspaces");
        assert_eq!(
            toml::from_str::<crate::core::Config>(&toml::to_string(&old).unwrap()).unwrap(),
            c
        );
        let mut w = Workspace::new("開発");
        w.entries.push(WorkspaceEntry {
            executable: "C:\\missing\\app.exe".into(),
            arguments: vec![
                "".into(),
                "C:\\日本語 folder\\file.txt".into(),
                "https://example.com/?a=1&b=2".into(),
            ],
            ..Default::default()
        });
        c.workspaces.push(w);
        c.validate().unwrap();
        assert_eq!(
            toml::from_str::<crate::core::Config>(&toml::to_string(&c).unwrap()).unwrap(),
            c
        );
        c.workspaces[0].entries[0].placement = Placement::Rect {
            x: f64::NAN,
            y: 0.0,
            width: 0.5,
            height: 1.0,
        };
        assert!(c.validate().is_err());
    }
    #[test]
    fn placement_and_missing_monitor_use_work_area_physical_pixels() {
        let work = MonitorRect {
            left: -2560,
            top: 30,
            width: 2560,
            height: 1410,
        };
        assert_eq!(
            Placement::Rect {
                x: 0.5,
                y: 0.0,
                width: 0.5,
                height: 1.0
            }
            .rectangle(work),
            MonitorRect {
                left: -1280,
                top: 30,
                width: 1280,
                height: 1410
            }
        );
        let ds = vec![DisplayInfo {
            id: "persistent".into(),
            name: "screen".into(),
            bounds: work,
            work,
            primary: true,
        }];
        assert!(!select_display("PERSISTENT", &ds).unwrap().1);
        assert!(select_display("unplugged", &ds).unwrap().1);
        assert!(!select_display("", &ds).unwrap().1);
    }
    #[test]
    fn old_windows_never_move_and_new_candidates_must_stabilize() {
        let a = WindowIdentity { handle: 1, pid: 1 };
        let b = WindowIdentity { handle: 2, pid: 2 };
        let c = WindowIdentity { handle: 3, pid: 3 };
        let before = HashSet::from([a]);
        let mut t = CandidateTracker::default();
        assert_eq!(t.observe(&before, &[a]), CandidateDecision::Wait);
        assert_eq!(t.observe(&before, &[a, b, c]), CandidateDecision::Ambiguous);
        for _ in 0..3 {
            assert_eq!(t.observe(&before, &[a, b]), CandidateDecision::Wait);
        }
        assert_eq!(t.observe(&before, &[a, b]), CandidateDecision::Ready(b));
    }

    #[test]
    fn invalid_recipe_is_rejected_without_reassigning_ids_on_load() {
        let mut workspace = Workspace::new("development");
        workspace.entries.push(WorkspaceEntry {
            executable: "C:\\missing\\app.exe".into(),
            ..Default::default()
        });
        for case in 0..7 {
            let mut bad = workspace.clone();
            let e = &mut bad.entries[0];
            match case {
                0 => e.id = Uuid::nil(),
                1 => e.label.clear(),
                2 => e.arguments.push("bad\0argument".into()),
                3 => e.working_directory = "bad\ndirectory".into(),
                4 => e.executable.clear(),
                5 => {
                    e.placement = Placement::Rect {
                        x: 0.5,
                        y: 0.0,
                        width: 0.6,
                        height: 1.0,
                    }
                }
                _ => {
                    e.placement = Placement::Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 0.0,
                        height: 1.0,
                    }
                }
            }
            assert!(validate_workspaces(&[bad]).is_err());
        }
        assert!(validate_workspaces(&[workspace.clone(), workspace.clone()]).is_err());
        let mut value = toml::Value::try_from(&workspace).unwrap();
        value.get_mut("entries").unwrap().as_array_mut().unwrap()[0]
            .as_table_mut()
            .unwrap()
            .remove("id");
        assert!(toml::from_str::<Workspace>(&toml::to_string(&value).unwrap()).is_err());
    }
}
