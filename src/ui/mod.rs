mod commands;
mod dock_view;
mod settings;

use crate::{core::*, platform_windows::*};
use commands::Command;
use eframe::egui;
use raw_window_handle::HasWindowHandle;
use settings::Editor;
use std::{
    path::PathBuf,
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
    config: Config,
    store: Option<ConfigStore>,
    editor: Editor,
    config_path: PathBuf,
    state: DockVisibility,
    wake: PollWake,
    last_poll: Instant,
    pending: Vec<Command>,
    inside: bool,
    initial_layout: bool,
    smoke: Option<Smoke>,
    _smoke_directory: Option<tempfile::TempDir>,
    dialog_result: Option<std::sync::mpsc::Receiver<Option<Vec<PathBuf>>>>,
}
struct Smoke {
    started: Instant,
    reveals: usize,
    next: Instant,
    hidden_polls: usize,
    shown_frames: usize,
    settings_frames: usize,
    finish_stage: u8,
}

impl DockApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        edge: Option<DockEdge>,
        smoke: bool,
        config_path: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let window = DockWindow::from_handle(cc.window_handle()?.as_raw())?;
        let monitor = primary_monitor()?;
        let defaults = Config::defaults(home_directory()?);
        let smoke_directory = if smoke {
            Some(tempfile::tempdir()?)
        } else {
            None
        };
        let path = if let Some(directory) = &smoke_directory {
            directory.path().join("config.toml")
        } else if let Some(path) = config_path {
            path
        } else {
            configuration_path()?
        };
        // Smoke uses only its own temporary configuration, never the user's file.
        let (store, mut config, message) = {
            let (mut store, config, error) = ConfigStore::load(path.clone(), defaults);
            let mut message = error.map(|e| {
                tracing::warn!(error = %e, "configuration fallback; original preserved");
                e.to_string()
            });
            if store.is_missing()
                && let Err(e) = store.save(&config)
            {
                tracing::error!(error = %e, "initial configuration save failed");
                message = Some(e.to_string());
            }
            (Some(store), config, message)
        };
        if let Some(edge) = edge {
            config.dock.edge = edge;
        }
        let timing = config.dock.timing();
        config.validate()?;
        cc.egui_ctx.set_zoom_factor(1.0);
        cc.egui_ctx
            .options_mut(|options| options.zoom_with_keyboard = false);
        apply_theme(&cc.egui_ctx, &config.appearance.theme);
        match japanese_font() {
            Ok(Some(bytes)) => {
                let mut fonts = egui::FontDefinitions::default();
                fonts.font_data.insert(
                    "windows-japanese".into(),
                    egui::FontData::from_owned(bytes).into(),
                );
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts
                        .families
                        .entry(family)
                        .or_default()
                        .push("windows-japanese".into());
                }
                cc.egui_ctx.set_fonts(fonts);
            }
            Ok(None) => tracing::warn!("Japanese font unavailable; using bundled fonts"),
            Err(e) => tracing::warn!(error = %e, "Cannot load Japanese fallback font"),
        }
        let now = Instant::now();
        let mut editor = Editor::new(config.clone());
        if message.is_some() {
            editor.open = true;
            editor.message = message;
        }
        tracing::info!(edge = ?config.dock.edge, items = config.items.len(), "configuration ready");
        Ok(Self {
            window,
            monitor,
            config,
            store,
            editor,
            config_path: path,
            state: DockVisibility::Revealing {
                started_at: now,
                monitor_id: 0,
            },
            wake: PollWake::new(cc.egui_ctx.clone(), timing),
            last_poll: now,
            pending: Vec::new(),
            inside: true,
            initial_layout: true,
            smoke: smoke.then_some(Smoke {
                started: now,
                reveals: 0,
                next: now + Duration::from_millis(400),
                hidden_polls: 0,
                shown_frames: 0,
                settings_frames: 0,
                finish_stage: 0,
            }),
            _smoke_directory: smoke_directory,
            dialog_result: None,
        })
    }

    fn position(&self) -> Result<(), PlatformError> {
        self.window.position(
            self.config.dock.edge,
            self.monitor,
            dock_view::dock_size(&self.config),
            self.config.dock.always_on_top,
        )
    }

    fn report(&mut self, message: String) {
        self.editor.open = true;
        self.editor.message = Some(message);
    }

    fn apply_state(&mut self, ctx: &egui::Context, next: DockVisibility) {
        let changed = self.state.is_hidden() != next.is_hidden();
        if changed && !next.is_hidden() {
            match primary_monitor().and_then(|monitor| {
                self.monitor = monitor;
                self.position()
            }) {
                Ok(()) => self.window.show_without_activation(),
                Err(e) => {
                    tracing::error!(%e, "Dock positioning failed");
                    self.report(
                        "Cannot position Dock. Check monitor availability and retry.".into(),
                    );
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

    fn process_commands(&mut self, ctx: &egui::Context) {
        for command in std::mem::take(&mut self.pending) {
            match command {
                Command::Launch(id) => {
                    if let Some(item) = self.config.items.iter().find(|i| i.id == id) {
                        tracing::info!(id = %id, target = %masked_target(item), "launch requested");
                        let result = validate_target(&item.target, item.kind)
                            .map_err(str::to_owned)
                            .and_then(|()| {
                                WindowsShell.open_target(&item.target).map_err(|e| {
                                    tracing::error!(%e, item_id = %id, "Shell launch failed");
                                    "Launch failed. Check target, association and permissions."
                                        .into()
                                })
                            });
                        if let Err(e) = result {
                            self.report(e);
                        }
                    }
                }
                Command::PickFiles | Command::PickFolder => {
                    if self.dialog_result.is_some() {
                        continue;
                    }
                    let folder = matches!(command, Command::PickFolder);
                    let (sender, receiver) = std::sync::mpsc::channel();
                    let repaint = ctx.clone();
                    // The worker owns only the native dialog, not configuration or HWND.
                    // Keep the UI thread free so hidden GetCursorPos polling continues.
                    std::thread::spawn(move || {
                        let paths = if folder {
                            rfd::FileDialog::new()
                                .set_title("Add folder to MaXImDock")
                                .pick_folder()
                                .map(|p| vec![p])
                        } else {
                            rfd::FileDialog::new()
                                .set_title("Add files to MaXImDock")
                                .pick_files()
                        };
                        if sender.send(paths).is_ok() {
                            repaint.request_repaint();
                        }
                    });
                    self.dialog_result = Some(receiver);
                    self.editor.message =
                        Some("Choose files or a folder in the native dialog.".into());
                }
                Command::Apply(config) => {
                    let result = config.validate().map_err(|e| e.to_string()).and_then(|()| {
                        self.store
                            .as_mut()
                            .ok_or_else(|| "Saving is disabled in smoke mode.".to_owned())?
                            .save(&config)
                            .map_err(|e| {
                                tracing::error!(error = %e, "configuration save failed");
                                e.to_string()
                            })
                    });
                    match result {
                        Ok(()) => {
                            self.config = config;
                            apply_theme(ctx, &self.config.appearance.theme);
                            self.editor.draft = self.config.clone();
                            self.editor.message = Some("Saved and applied.".into());
                            self.wake = PollWake::new(ctx.clone(), self.config.dock.timing());
                            self.wake
                                .hidden
                                .store(self.state.is_hidden(), Ordering::Release);
                            self.state = if self.state.is_hidden() {
                                DockVisibility::Hidden
                            } else {
                                DockVisibility::Visible { monitor_id: 0 }
                            };
                            if !self.state.is_hidden()
                                && let Err(e) = self.position()
                            {
                                tracing::error!(%e, "configuration applied but positioning failed");
                                self.report("Settings saved. Dock positioning failed; hide and show it again.".into());
                            }
                        }
                        Err(e) => self.report(e),
                    }
                }
                Command::BackUpInvalid => {
                    if let Some(store) = &mut self.store {
                        match store.back_up_invalid() {
                            Ok(path) => {
                                self.editor.message = Some(format!(
                                    "Original backed up to {}. Choose Apply and save to replace it.",
                                    path.display()
                                ))
                            }
                            Err(e) => self.report(e.to_string()),
                        }
                    }
                }
                Command::OpenConfigFolder => {
                    if let Some(parent) = self.config_path.parent() {
                        let result = std::fs::create_dir_all(parent)
                            .map_err(|e| e.to_string())
                            .and_then(|()| {
                                WindowsShell
                                    .open_target(&parent.to_string_lossy())
                                    .map_err(|e| e.to_string())
                            });
                        if let Err(e) = result {
                            tracing::error!(error = %e, "cannot open config folder");
                            self.report(
                                "Cannot open config folder. Check folder permissions.".into(),
                            );
                        }
                    }
                }
                Command::OpenSettings => {
                    self.editor.open = true;
                }
                Command::Show => self.apply_state(
                    ctx,
                    DockVisibility::Revealing {
                        started_at: Instant::now(),
                        monitor_id: 0,
                    },
                ),
                Command::Hide => self.apply_state(ctx, DockVisibility::Hidden),
                Command::Quit => {
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close)
                }
            }
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
            if self.state.is_hidden() && smoke.finish_stage == 0 {
                assert!(!self.window.is_visible(), "HWND must really be hidden");
                smoke.reveals += 1;
                let before = self.window.bounds().unwrap();
                let mut config = self.config.clone();
                config.dock.edge = [
                    DockEdge::Top,
                    DockEdge::Bottom,
                    DockEdge::Left,
                    DockEdge::Right,
                ][(smoke.reveals + 1) % 4];
                self.pending.push(Command::Apply(config.clone()));
                self.process_commands(ctx);
                assert_eq!(self.config, config);
                assert!(
                    self.state.is_hidden() && !self.window.is_visible(),
                    "hidden Apply must not show Dock"
                );
                assert_eq!(
                    self.window.bounds().unwrap(),
                    before,
                    "hidden edge changes wait until next show"
                );
                let next = self
                    .state
                    .advance(now, Some(0), true, false, self.config.dock.timing());
                smoke.next = now + Duration::from_millis(300);
                self.apply_state(ctx, next);
                assert!(self.window.is_visible(), "HWND must really be shown");
                let bounds = self.window.bounds().expect("GetWindowRect during smoke");
                let expected = dock_anchor_position(
                    self.config.dock.edge,
                    self.monitor,
                    (bounds.width, bounds.height),
                );
                assert_eq!(
                    ScreenPoint {
                        x: bounds.left,
                        y: bounds.top
                    },
                    expected
                );
            } else if smoke.reveals >= 30 || smoke.finish_stage > 0 {
                assert!(
                    smoke.hidden_polls >= 30,
                    "hidden GetCursorPos must keep working"
                );
                assert!(smoke.shown_frames >= 30, "eframe must resume drawing");
                match smoke.finish_stage {
                    0 => {
                        let (_, restored, error) = ConfigStore::load(
                            self.config_path.clone(),
                            Config::defaults("unused".into()),
                        );
                        assert!(error.is_none());
                        assert_eq!(restored, self.config, "Apply/save must survive reload");
                        self.editor.open = true;
                        self.apply_state(ctx, DockVisibility::Hidden);
                        smoke.finish_stage = 1;
                    }
                    1 => {
                        assert!(
                            smoke.settings_frames > 0,
                            "settings must render with hidden root"
                        );
                        self.editor.open = false;
                        ctx.request_repaint();
                        self.apply_state(ctx, DockVisibility::Visible { monitor_id: 0 });
                        smoke.finish_stage = 2;
                    }
                    _ => {
                        tracing::info!(
                            polls = smoke.hidden_polls,
                            frames = smoke.shown_frames,
                            "NATIVE_SMOKE_PASS: 30 real hide/show cycles; synthetic hot-zone entry"
                        );
                        tracing::info!(
                            "PHASE2_SMOKE_PASS: four-edge Apply/save/reload and settings open/close with hidden root"
                        );
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                }
                smoke.next = now + Duration::from_millis(350);
            } else {
                let mut config = self.config.clone();
                config.dock.edge = [
                    DockEdge::Top,
                    DockEdge::Bottom,
                    DockEdge::Left,
                    DockEdge::Right,
                ][smoke.reveals % 4];
                config.dock.icon_size = 40.0 + (smoke.reveals % 4) as f32 * 8.0;
                config.dock.auto_hide = smoke.reveals.is_multiple_of(2);
                config.dock.always_on_top = smoke.reveals.is_multiple_of(2);
                if smoke.reveals == 0 {
                    config.items[0].label = "Renamed Explorer".into();
                    config.move_item(0, 2);
                    config
                        .add_item(DockItem::new(
                            "Smoke URL",
                            "https://example.com/",
                            TargetKind::Url,
                        ))
                        .unwrap();
                } else if smoke.reveals == 1 {
                    config.items.pop();
                }
                self.pending.push(Command::Apply(config.clone()));
                self.process_commands(ctx);
                assert_eq!(self.config, config, "Apply must update configuration");
                let bounds = self.window.bounds().unwrap();
                let expected = dock_anchor_position(
                    self.config.dock.edge,
                    self.monitor,
                    (bounds.width, bounds.height),
                );
                assert_eq!(
                    ScreenPoint {
                        x: bounds.left,
                        y: bounds.top
                    },
                    expected,
                    "visible edge changes must reposition immediately"
                );
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
        self.receive_dialog();
        self.process_commands(ctx);
        let now = Instant::now();
        let timing = self.config.dock.timing();
        if self.initial_layout {
            if let Err(e) = self.position() {
                tracing::error!(%e, "initial Dock positioning failed");
                self.report("Cannot position Dock. Check display settings.".into());
            }
            self.initial_layout = false;
        }
        let mut hot_monitor = None;
        if self.state.is_hidden()
            && now.duration_since(self.last_poll) >= timing.cursor_poll_interval
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
                            if is_in_hot_zone(
                                self.config.dock.edge,
                                cursor,
                                monitor,
                                timing.hot_zone_px,
                            ) && self.smoke.is_none()
                            {
                                hot_monitor = Some(0);
                            }
                        }
                        Err(e) => {
                            tracing::error!(%e, "monitor query failed");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(%e, "cursor query failed; retrying");
                }
            }
        }
        let held = self.inside || self.editor.open || !self.config.dock.auto_hide;
        let next = self.state.advance(now, hot_monitor, held, false, timing);
        self.apply_state(ctx, next);
        self.smoke_tick(ctx, now);
        match self.state {
            DockVisibility::Hidden => ctx.request_repaint_after(
                timing
                    .cursor_poll_interval
                    .saturating_sub(now.duration_since(self.last_poll)),
            ),
            DockVisibility::Revealing { started_at, .. } => ctx.request_repaint_after(
                timing
                    .reveal_hold
                    .saturating_sub(now.duration_since(started_at)),
            ),
            DockVisibility::HidePending { since, .. } => ctx
                .request_repaint_after(timing.hide_delay.saturating_sub(now.duration_since(since))),
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
        }) || egui::Popup::is_any_open(&ctx);
        if !self.state.is_hidden() {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.pending.push(Command::Hide);
            }
            dock_view::render(ui, &self.config, &mut self.pending);
        }
        if self.editor.open {
            let blocked = self.store.as_ref().is_some_and(ConfigStore::is_blocked);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("settings"),
                egui::ViewportBuilder::default()
                    .with_title("MaXImDock Settings")
                    .with_inner_size([660.0, 700.0])
                    .with_min_inner_size([520.0, 420.0]),
                |ui, _class| {
                    self.editor
                        .render(ui, blocked, &self.config_path, &mut self.pending);
                    if let Some(smoke) = &mut self.smoke {
                        smoke.settings_frames += 1;
                    }
                },
            );
            if !self.editor.open {
                ctx.request_repaint();
            }
        }
        let held = self.inside || self.editor.open || !self.config.dock.auto_hide;
        if !self.pending.is_empty()
            || matches!(self.state, DockVisibility::Visible { .. }) && !held
            || matches!(self.state, DockVisibility::HidePending { .. }) && held
        {
            ctx.request_repaint();
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

fn apply_theme(ctx: &egui::Context, theme: &str) {
    ctx.set_theme(match theme {
        "dark" => egui::ThemePreference::Dark,
        "light" => egui::ThemePreference::Light,
        _ => egui::ThemePreference::System,
    });
}

impl DockApp {
    fn receive_dialog(&mut self) {
        let Some(receiver) = &self.dialog_result else {
            return;
        };
        let paths = match receiver.try_recv() {
            Ok(paths) => paths,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.dialog_result = None;
                self.report("File dialog failed. Try again or enter a path manually.".into());
                return;
            }
        };
        self.dialog_result = None;
        let Some(paths) = paths else {
            self.editor.message = Some("Selection cancelled.".into());
            return;
        };
        let mut failures = Vec::new();
        for path in paths {
            let target = path.to_string_lossy().into_owned();
            let result = validate_target(&target, TargetKind::Path)
                .map_err(str::to_owned)
                .and_then(|()| {
                    self.editor
                        .draft
                        .add_item(DockItem::new(
                            settings::label_for_path(&path),
                            target,
                            TargetKind::Path,
                        ))
                        .map_err(|e| e.to_string())
                });
            if let Err(e) = result {
                failures.push(e);
            }
        }
        self.editor.message = Some(if failures.is_empty() {
            "Added to draft. Choose Apply and save.".into()
        } else {
            failures.join("\n")
        });
    }
}
