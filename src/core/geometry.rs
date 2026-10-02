use super::DockEdge;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl MonitorRect {
    pub fn contains(self, point: ScreenPoint) -> bool {
        let x = i64::from(point.x) - i64::from(self.left);
        let y = i64::from(point.y) - i64::from(self.top);
        x >= 0 && y >= 0 && x < i64::from(self.width) && y < i64::from(self.height)
    }
}

pub fn is_in_hot_zone(
    edge: DockEdge,
    cursor: ScreenPoint,
    monitor: MonitorRect,
    hot_zone_px: i32,
) -> bool {
    if hot_zone_px <= 0 || !monitor.contains(cursor) {
        return false;
    }
    let x = i64::from(cursor.x) - i64::from(monitor.left);
    let y = i64::from(cursor.y) - i64::from(monitor.top);
    let zone = i64::from(hot_zone_px);
    match edge {
        DockEdge::Bottom => y >= i64::from(monitor.height) - zone,
        DockEdge::Top => y < zone,
        DockEdge::Left => x < zone,
        DockEdge::Right => x >= i64::from(monitor.width) - zone,
    }
}

pub fn dock_anchor_position(
    edge: DockEdge,
    monitor: MonitorRect,
    dock_size_px: (i32, i32),
) -> ScreenPoint {
    let w = dock_size_px.0.clamp(0, monitor.width.max(0));
    let h = dock_size_px.1.clamp(0, monitor.height.max(0));
    let centered_x = monitor.left.saturating_add((monitor.width - w) / 2);
    let centered_y = monitor.top.saturating_add((monitor.height - h) / 2);
    match edge {
        DockEdge::Bottom => ScreenPoint {
            x: centered_x,
            y: monitor.top.saturating_add(monitor.height - h),
        },
        DockEdge::Top => ScreenPoint {
            x: centered_x,
            y: monitor.top,
        },
        DockEdge::Left => ScreenPoint {
            x: monitor.left,
            y: centered_y,
        },
        DockEdge::Right => ScreenPoint {
            x: monitor.left.saturating_add(monitor.width - w),
            y: centered_y,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_edge_boundary_and_negative_coordinates() {
        for (left, top) in [(0, 0), (-1920, -1080)] {
            let m = MonitorRect {
                left,
                top,
                width: 1920,
                height: 1080,
            };
            for (edge, inside, outside) in [
                (DockEdge::Bottom, (960, 1078), (960, 1077)),
                (DockEdge::Top, (960, 1), (960, 2)),
                (DockEdge::Left, (1, 540), (2, 540)),
                (DockEdge::Right, (1918, 540), (1917, 540)),
            ] {
                let point = |p: (i32, i32)| ScreenPoint {
                    x: left + p.0,
                    y: top + p.1,
                };
                assert!(is_in_hot_zone(edge, point(inside), m, 2));
                assert!(!is_in_hot_zone(edge, point(outside), m, 2));
                for p in [(-1, 0), (0, -1), (1920, 540), (960, 1080)] {
                    assert!(!is_in_hot_zone(edge, point(p), m, 2));
                }
                assert!(!is_in_hot_zone(edge, point(inside), m, 0));
            }
            assert!(is_in_hot_zone(
                DockEdge::Bottom,
                ScreenPoint {
                    x: left + 960,
                    y: top + 1079
                },
                m,
                2
            ));
        }
    }

    #[test]
    fn anchors_center_each_edge_and_clamp_oversized_dock() {
        let m = MonitorRect {
            left: -1000,
            top: -800,
            width: 1000,
            height: 800,
        };
        for (edge, expected) in [
            (DockEdge::Bottom, (-600, -80)),
            (DockEdge::Top, (-600, -800)),
            (DockEdge::Left, (-1000, -440)),
            (DockEdge::Right, (-200, -440)),
        ] {
            assert_eq!(
                dock_anchor_position(edge, m, (200, 80)),
                ScreenPoint {
                    x: expected.0,
                    y: expected.1
                }
            );
            assert_eq!(
                dock_anchor_position(edge, m, (2000, 1600)),
                ScreenPoint { x: -1000, y: -800 }
            );
        }
    }
}
