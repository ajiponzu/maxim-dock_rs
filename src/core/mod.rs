mod geometry;
mod visibility;

pub use geometry::*;
pub use visibility::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockEdge {
    #[default]
    Bottom,
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

#[derive(Debug, Clone, Copy)]
pub enum TargetKind {
    Path,
    Url,
}

pub struct DockItem {
    pub label: &'static str,
    pub target: String,
    pub kind: TargetKind,
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
            let rest = target
                .strip_prefix("https://")
                .or_else(|| target.strip_prefix("http://"));
            if rest.is_some_and(|s| {
                let host = s.split(['/', '?', '#']).next().unwrap_or_default();
                !host.is_empty()
                    && !host.contains('@')
                    && !s.chars().any(|c| c.is_whitespace() || c.is_control())
            }) {
                Ok(())
            } else {
                Err("Specify http:// or https:// with a valid host name.")
            }
        }
        TargetKind::Path => Ok(()),
    }
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
        ] {
            assert!(validate_target(target, TargetKind::Url).is_err());
        }
        assert!(validate_target("https://github.com/", TargetKind::Url).is_ok());
        assert!(validate_target("missing-maximdock-target.exe", TargetKind::Path).is_err());
    }
}
