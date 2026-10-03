mod config;
mod geometry;
mod visibility;
mod workspace;
mod wsl;
pub use workspace::*;
pub use wsl::*;

pub use config::*;
pub use geometry::*;
use serde::{Deserialize, Serialize};
pub use visibility::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockEdge {
    Bottom,
    #[default]
    Top,
    Left,
    Right,
}

impl DockEdge {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Bottom | Self::Top)
    }

    pub fn is_vertical(self) -> bool {
        !self.is_horizontal()
    }
}

impl std::str::FromStr for DockEdge {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "bottom" => Ok(Self::Bottom),
            "top" => Ok(Self::Top),
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err("edge must be bottom, top, left, or right"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Path,
    Url,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DockItem {
    pub id: uuid::Uuid,
    pub label: String,
    pub target: String,
    pub kind: TargetKind,
    pub icon: IconSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum IconSource {
    #[default]
    Auto,
    File {
        path: std::path::PathBuf,
    },
    Builtin {
        name: String,
    },
}

impl DockItem {
    pub fn new(label: impl Into<String>, target: impl Into<String>, kind: TargetKind) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            label: label.into(),
            target: target.into(),
            kind,
            icon: IconSource::Auto,
        }
    }
}

pub fn validate_target(target: &str, kind: TargetKind) -> Result<(), &'static str> {
    if target.is_empty() || target.contains('\0') {
        return Err("Target is empty or contains an invalid character.");
    }
    match kind {
        TargetKind::Path if !std::path::Path::new(target).exists() => {
            Err("Target not found. Check the path and permissions.")
        }
        TargetKind::Url => {
            if valid_url(target) {
                Ok(())
            } else {
                Err("Specify http:// or https:// with a valid host name.")
            }
        }
        TargetKind::Path => Ok(()),
    }
}

pub fn valid_url(target: &str) -> bool {
    (target
        .get(..7)
        .is_some_and(|p| p.eq_ignore_ascii_case("http://"))
        || target
            .get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case("https://")))
        && !target.chars().any(|c| c.is_whitespace() || c.is_control())
        && !target.contains('\\')
        && url::Url::parse(target).is_ok_and(|u| {
            matches!(u.scheme(), "http" | "https")
                && u.host_str().is_some()
                && u.username().is_empty()
                && u.password().is_none()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_and_edge_parsing() {
        for edge in [DockEdge::Bottom, DockEdge::Top] {
            assert!(edge.is_horizontal());
            assert!(!edge.is_vertical());
        }
        for edge in [DockEdge::Left, DockEdge::Right] {
            assert!(edge.is_vertical());
            assert!(!edge.is_horizontal());
        }
        assert_eq!("right".parse::<DockEdge>(), Ok(DockEdge::Right));
        assert!("unknown".parse::<DockEdge>().is_err());
    }

    #[test]
    fn invalid_targets_are_rejected_before_shell() {
        for target in [
            "",
            "https://",
            "file://example",
            "https://a b",
            "https://a\0b",
            "https:example.com",
            "http:/example.com",
            "https://user:password@example.com/",
            "https://example.com:bad/",
        ] {
            assert!(validate_target(target, TargetKind::Url).is_err());
        }
        assert!(validate_target("https://github.com/", TargetKind::Url).is_ok());
        assert!(validate_target("missing-maximdock-target.exe", TargetKind::Path).is_err());
    }
}
