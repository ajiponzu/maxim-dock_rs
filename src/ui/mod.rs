use crate::{core::*, platform_windows::*};
use eframe::egui;
use raw_window_handle::HasWindowHandle;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

struct PollWake {
    hidden: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl PollWake {
    fn new(ctx: egui::Context, timing: Timing) -> Self {
        let hidden = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let h = hidden.clone();
        let s = stop.clone();
        let worker = std::thread::spawn(move || {
            while !s.load(Ordering::Acquire) {
                std::thread::park_timeout(timing.cursor_poll_interval);
                if h.load(Ordering::Acquire) && !s.load(Ordering::Acquire) {
                    ctx.request_repaint();
                }
            }
        });
        Self {
            hidden,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for PollWake {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            if worker.join().is_err() {
                tracing::error!("poll wake worker panicked");
            }
        }
    }
}

pub struct DockApp {
    window: DockWindow,
    monitor: MonitorRect,
    edge: DockEdge,
    timing: Timing,
    state: DockVisibility,
    items: Vec<DockItem>,
    wake: PollWake,
    last_poll: Instant,
    error_message: Option<String>,
    escape_pending: bool,
    inside: bool,
    initial_layout: bool,
    smoke: Option<Smoke>,
}
struct Smoke {
    started: Instant,
    reveals: usize,
    next: Instant,
    hidden_polls: usize,
    shown_frames: usize,
}

impl DockApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        edge: DockEdge,
        smoke: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let window = DockWindow::from_handle(cc.window_handle()?.as_raw())?;
        let monitor = primary_monitor()?;
        let timing = Timing::default();
        timing.validate()?;
        cc.egui_ctx.set_zoom_factor(1.0);
        let home = home_directory()?;
        let now = Instant::now();
        Ok(Self {
            window,
            monitor,
            edge,
            timing,
            state: DockVisibility::Revealing {
                started_at: now,
                monitor_id: 0,
            },
            items: vec![
                DockItem {
                    label: "Explorer",
                    target: "C:\\Windows\\explorer.exe".into(),
                    kind: TargetKind::Path,
                },
                DockItem {
                    label: "Home",
                    target: home,
                    kind: TargetKind::Path,
                },
                DockItem {
                    label: "GitHub",
                    target: "https://github.com/".into(),
                    kind: TargetKind::Url,
                },
            ],
            wake: PollWake::new(cc.egui_ctx.clone(), timing),
            last_poll: now,
            error_message: None,
            escape_pending: false,
            inside: true,
            initial_layout: true,
            smoke: smoke.then_some(Smoke {
                started: now,
                reveals: 0,
                next: now + Duration::from_millis(400),
                hidden_polls: 0,
                shown_frames: 0,
            }),
        })
    }
    fn size(&self) -> (f32, f32) {
        if self.edge.is_horizontal() {
            (310.0, 100.0)
        } else {
            (100.0, 310.0)
        }
    }
    fn report(&mut self, error: PlatformError) {
        tracing::error!(%error, "Windows operation failed");
        self.error_message = Some(
            "Operation failed. Check target, association and permissions; see log for details."
                .into(),
        );
    }
    fn apply_state(&mut self, ctx: &egui::Context, next: DockVisibility) {
        let changed = self.state.is_hidden() != next.is_hidden();
        if changed && !next.is_hidden() {
            match primary_monitor().and_then(|monitor| {
                self.monitor = monitor;
                self.window.position(self.edge, monitor, self.size())
            }) {
                Ok(()) => self.window.show_without_activation(),
                Err(e) => {
                    self.report(e);
                    return;
                }
            }
        }
        self.state = next;
        self.wake.hidden.store(next.is_hidden(), Ordering::Release);
        if changed {
            if next.is_hidden() {
                self.window.hide();
            }
            tracing::info!(?next, "Dock visibility changed");
            ctx.request_repaint();
        }
    }
    fn smoke_tick(&mut self, ctx: &egui::Context, now: Instant) {
        let Some(mut smoke) = self.smoke.take() else {
            return;
        };
        assert!(
            now.duration_since(smoke.started) < Duration::from_secs(30),
            "native smoke timed out"
        );
        if now >= smoke.next {
            if self.state.is_hidden() {
                assert!(!self.window.is_visible(), "HWND must really be hidden");
                smoke.reveals += 1;
                // Synthetic hot-zone entry avoids moving the user's cursor.
                let next = self.state.advance(now, Some(0), true, false, self.timing);
                smoke.next = now + Duration::from_millis(300);
                self.apply_state(ctx, next);
                assert!(self.window.is_visible(), "HWND must really be shown");
                let bounds = self.window.bounds().expect("GetWindowRect during smoke");
                let expected =
                    dock_anchor_position(self.edge, self.monitor, (bounds.width, bounds.height));
                assert_eq!(
                    ScreenPoint {
                        x: bounds.left,
                        y: bounds.top
                    },
                    expected
                );
            } else if smoke.reveals >= 30 {
                assert!(
                    smoke.hidden_polls >= 30,
                    "hidden GetCursorPos must keep working"
                );
                assert!(smoke.shown_frames >= 30, "eframe must resume drawing");
                tracing::info!(
                    polls = smoke.hidden_polls,
                    frames = smoke.shown_frames,
                    "NATIVE_SMOKE_PASS: 30 real hide/show cycles; synthetic hot-zone entry"
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            } else {
                smoke.next = now + Duration::from_millis(180);
                self.apply_state(ctx, DockVisibility::Hidden);
            }
        }
        self.smoke = Some(smoke);
        ctx.request_repaint_after(Duration::from_millis(75));
    }
}

impl eframe::App for DockApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        if self.initial_layout {
            if let Err(e) = self.window.position(self.edge, self.monitor, self.size()) {
                self.report(e);
            }
            self.initial_layout = false;
        }
        let mut hot_monitor = None;
        if self.state.is_hidden()
            && now.duration_since(self.last_poll) >= self.timing.cursor_poll_interval
        {
            self.last_poll = now;
            match WindowsCursor.screen_position() {
                Ok(cursor) => {
                    if let Some(smoke) = &mut self.smoke {
                        smoke.hidden_polls += 1;
                    }
                    tracing::debug!(?cursor, "hidden cursor poll");
                    match primary_monitor() {
                        Ok(monitor) => {
                            self.monitor = monitor;
                            if is_in_hot_zone(self.edge, cursor, monitor, self.timing.hot_zone_px)
                                && self.smoke.is_none()
                            {
                                hot_monitor = Some(0);
                            }
                        }
                        Err(e) => self.report(e),
                    }
                }
                Err(e) => self.report(e),
            }
        }
        let escape = std::mem::take(&mut self.escape_pending);
        let next = self
            .state
            .advance(now, hot_monitor, self.inside, escape, self.timing);
        self.apply_state(ctx, next);
        self.smoke_tick(ctx, now);
        match self.state {
            DockVisibility::Hidden => ctx.request_repaint_after(
                self.timing
                    .cursor_poll_interval
                    .saturating_sub(now.duration_since(self.last_poll)),
            ),
            DockVisibility::Revealing { started_at, .. } => ctx.request_repaint_after(
                self.timing
                    .reveal_hold
                    .saturating_sub(now.duration_since(started_at)),
            ),
            DockVisibility::HidePending { since, .. } => ctx.request_repaint_after(
                self.timing
                    .hide_delay
                    .saturating_sub(now.duration_since(since)),
            ),
            _ => {}
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(smoke) = &mut self.smoke {
            smoke.shown_frames += 1;
        }
        let ctx = ui.ctx().clone();
        self.inside = ctx.input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|p| i.viewport_rect().contains(p))
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.escape_pending = true;
            ctx.request_repaint();
        }
        if matches!(self.state, DockVisibility::Visible { .. }) && !self.inside
            || matches!(self.state, DockVisibility::HidePending { .. }) && self.inside
        {
            ctx.request_repaint();
        }
        egui::Frame::new()
            .fill(egui::Color32::from_rgba_unmultiplied(25, 30, 40, 220))
            .corner_radius(16)
            .inner_margin(8)
            .show(ui, |ui| {
                let layout = if self.edge.is_horizontal() {
                    egui::Layout::left_to_right(egui::Align::Center)
                } else {
                    egui::Layout::top_down(egui::Align::Center)
                };
                ui.with_layout(layout, |ui| {
                    for item in &self.items {
                        let button = egui::Button::new(egui::RichText::new(item.label).size(16.0))
                            .min_size(egui::vec2(
                                if self.edge.is_horizontal() {
                                    90.0
                                } else {
                                    84.0
                                },
                                80.0,
                            ));
                        if ui.add(button).on_hover_text(&item.target).clicked() {
                            tracing::info!(
                                label = item.label,
                                target = %masked_target(item), "launch requested"
                            );
                            let result = validate_target(&item.target, item.kind)
                                .map_err(str::to_owned)
                                .and_then(|()| {
                                    WindowsShell.open_target(&item.target).map_err(|e| {
                                tracing::error!(%e, label = item.label, "Shell launch failed");
                                "Launch failed. Check target, association and permissions.".into()
                            })
                                });
                            if let Err(message) = result {
                                self.error_message = Some(message);
                            }
                        }
                    }
                });
            })
            .response
            .context_menu(|ui| {
                if ui.button("Hide (Esc)").clicked() {
                    self.escape_pending = true;
                    ui.close();
                }
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        if let Some(message) = &self.error_message {
            let message = message.clone();
            egui::Window::new("Launch error")
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.label(message);
                    if ui.button("Dismiss").clicked() {
                        self.error_message = None;
                    }
                });
        }
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
}

fn masked_target(item: &DockItem) -> String {
    match item.kind {
        TargetKind::Path => std::path::Path::new(&item.target)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "<path>".into()),
        TargetKind::Url => item
            .target
            .split(['?', '#'])
            .next()
            .unwrap_or("<url>")
            .to_owned(),
    }
}
