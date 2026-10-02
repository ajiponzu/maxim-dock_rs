use std::time::{Duration, Instant};

pub type MonitorId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockVisibility {
    Hidden,
    Revealing {
        started_at: Instant,
        monitor_id: MonitorId,
    },
    Visible {
        monitor_id: MonitorId,
    },
    HidePending {
        monitor_id: MonitorId,
        since: Instant,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub cursor_poll_interval: Duration,
    pub hide_delay: Duration,
    pub reveal_hold: Duration,
    pub hot_zone_px: i32,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            cursor_poll_interval: Duration::from_millis(75),
            hide_delay: Duration::from_millis(500),
            reveal_hold: Duration::from_millis(200),
            hot_zone_px: 2,
        }
    }
}

impl Timing {
    pub fn validate(self) -> Result<(), &'static str> {
        if !(Duration::from_millis(50)..=Duration::from_millis(100))
            .contains(&self.cursor_poll_interval)
            || !(Duration::from_millis(100)..=Duration::from_secs(10)).contains(&self.hide_delay)
            || !(Duration::from_millis(100)..=Duration::from_secs(2)).contains(&self.reveal_hold)
            || !(1..=32).contains(&self.hot_zone_px)
        {
            Err("Dock timing is outside safe bounds")
        } else {
            Ok(())
        }
    }
}

impl DockVisibility {
    pub fn is_hidden(self) -> bool {
        matches!(self, Self::Hidden)
    }

    pub fn advance(
        self,
        now: Instant,
        hot_monitor: Option<MonitorId>,
        inside_dock: bool,
        escape: bool,
        timing: Timing,
    ) -> Self {
        if escape {
            return Self::Hidden;
        }
        match self {
            Self::Hidden => hot_monitor.map_or(Self::Hidden, |monitor_id| Self::Revealing {
                started_at: now,
                monitor_id,
            }),
            Self::Revealing {
                started_at,
                monitor_id,
            } if now.saturating_duration_since(started_at) >= timing.reveal_hold => {
                Self::Visible { monitor_id }
            }
            Self::Visible { monitor_id } if !inside_dock => Self::HidePending {
                monitor_id,
                since: now,
            },
            Self::HidePending { monitor_id, .. } if inside_dock => Self::Visible { monitor_id },
            Self::HidePending { since, .. }
                if now.saturating_duration_since(since) >= timing.hide_delay =>
            {
                Self::Hidden
            }
            state => state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_cycle_hold_delay_return_and_escape() {
        let t = Timing::default();
        let now = Instant::now();
        let state = DockVisibility::Hidden.advance(now, Some(7), false, false, t);
        assert!(matches!(
            state,
            DockVisibility::Revealing { monitor_id: 7, .. }
        ));
        assert_eq!(
            state.advance(now + Duration::from_millis(199), None, false, false, t),
            state
        );
        let visible = state.advance(now + t.reveal_hold, None, false, false, t);
        assert_eq!(visible, DockVisibility::Visible { monitor_id: 7 });
        let pending = visible.advance(now + t.reveal_hold, Some(8), false, false, t);
        assert_eq!(
            pending.advance(now + Duration::from_millis(699), None, false, false, t),
            pending
        );
        assert_eq!(
            pending.advance(now + Duration::from_millis(700), None, false, false, t),
            DockVisibility::Hidden
        );
        assert_eq!(
            pending.advance(now + Duration::from_millis(700), None, true, false, t),
            visible
        );
        for s in [state, visible, pending, DockVisibility::Hidden] {
            assert_eq!(
                s.advance(now, Some(8), true, true, t),
                DockVisibility::Hidden
            );
        }
    }

    #[test]
    fn repeated_cycles_do_not_depend_on_gui_events() {
        let t = Timing::default();
        let mut now = Instant::now();
        let mut s = DockVisibility::Hidden;
        for _ in 0..30 {
            s = s.advance(now, Some(0), true, false, t);
            now += t.reveal_hold;
            s = s.advance(now, None, true, false, t);
            assert!(matches!(s, DockVisibility::Visible { .. }));
            s = s.advance(now, None, false, false, t);
            now += t.hide_delay;
            s = s.advance(now, None, false, false, t);
            assert!(s.is_hidden());
        }
    }

    #[test]
    fn timing_validation() {
        assert!(Timing::default().validate().is_ok());
        assert!(
            Timing {
                hot_zone_px: 0,
                ..Timing::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            Timing {
                cursor_poll_interval: Duration::ZERO,
                ..Timing::default()
            }
            .validate()
            .is_err()
        );
    }
}
