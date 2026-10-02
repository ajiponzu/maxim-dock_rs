//! eframe integration: startup, visibility scheduling, and view composition.
mod actions;
mod dock_actions;
mod smoke;
mod smoke_capture;

use super::{commands::Command, dock_view, icons, poll_wake::PollWake, settings::Editor};
use crate::{core::*, platform_windows::*};
use eframe::egui;
use raw_window_handle::HasWindowHandle;
use smoke::Smoke;
use std::{path::PathBuf, time::Instant};

pub struct DockApp {
    window: DockWindow,
    monitor: Monitor,
    tray: Option<WindowsTray>,
    layout_dpi: u32,
    icons: icons::Icons,
    config: Config,
    store: Option<ConfigStore>,
    editor: Editor,
    settings_window: super::settings_window::SettingsWindow,
    config_path: PathBuf,
    state: DockVisibility,
    wake: PollWake,
    last_poll: Instant,
    pending: Vec<Command>,
    inside: bool,
    initial_layout: bool,
    smoke: Option<Smoke>,
    _smoke_directory: Option<tempfile::TempDir>,
    picker: FilePicker,
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
        super::theme::configure(&cc.egui_ctx, &config.appearance);
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
        let repaint = cc.egui_ctx.clone();
        let tray = match WindowsTray::new(move || repaint.request_repaint()) {
            Ok(tray) => Some(tray),
            Err(e) => {
                tracing::warn!(error = %e, "tray unavailable; Dock continues");
                editor.open = true;
                editor.message = Some(
                    "Tray unavailable. Use Dock Settings / Quit; hide/reveal still works.".into(),
                );
                None
            }
        };
        tracing::info!(edge = ?config.dock.edge, items = config.items.len(), "configuration ready");
        let mut icons = icons::Icons::new(cc.egui_ctx.clone());
        icons.sync(&config);
        Ok(Self {
            window,
            monitor,
            tray,
            layout_dpi: 0,
            icons,
            config,
            store,
            editor,
            settings_window: Default::default(),
            config_path: path,
            state: DockVisibility::Revealing {
                started_at: now,
                monitor_id: monitor.id,
            },
            wake: PollWake::new(cc.egui_ctx.clone(), timing),
            last_poll: now,
            pending: Vec::new(),
            inside: true,
            initial_layout: true,
            smoke: smoke.then(|| Smoke::new(now)),
            _smoke_directory: smoke_directory,
            picker: FilePicker::default(),
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
            match self.position() {
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
        self.wake.set_hidden(next.is_hidden());
        if changed {
            if next.is_hidden() {
                self.window.hide();
            }
            tracing::info!(?next, "Dock visibility changed");
            ctx.request_repaint();
        }
    }
}

impl eframe::App for DockApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let settings_frames = self
            .settings_window
            .receive(&mut self.editor, &mut self.pending);
        if let Some(smoke) = &mut self.smoke {
            for _ in 0..settings_frames {
                smoke.record_settings_frame();
            }
        }
        self.receive_tray();
        self.receive_dialog();
        self.save_smoke_captures(ctx);
        self.process_commands(ctx);
        self.icons.receive(ctx);
        let now = Instant::now();
        let timing = self.config.dock.timing();
        if self.initial_layout {
            if let Err(e) = self.position() {
                tracing::error!(%e, "initial Dock positioning failed");
                self.report("Cannot position Dock. Check display settings.".into());
            }
            self.initial_layout = false;
        }
        let dpi = self.window.dpi();
        if !self.state.is_hidden() && dpi != self.layout_dpi {
            if let Err(e) = self.position() {
                tracing::error!(%e, "DPI layout correction failed");
            }
            self.layout_dpi = self.window.dpi();
        }
        let mut hot_monitor = None;
        if self.state.is_hidden()
            && now.duration_since(self.last_poll) >= timing.cursor_poll_interval
        {
            self.last_poll = now;
            match WindowsCursor.screen_position() {
                Ok(cursor) => {
                    if let Some(smoke) = &mut self.smoke {
                        smoke.record_hidden_poll();
                    }
                    tracing::debug!(?cursor, "hidden cursor poll");
                    match cursor_monitor(cursor) {
                        Ok(Some(monitor)) => {
                            if is_in_hot_zone(
                                self.config.dock.edge,
                                cursor,
                                monitor.rect,
                                timing.hot_zone_px,
                            ) && self.smoke.is_none()
                            {
                                self.monitor = monitor;
                                hot_monitor = Some(monitor.id);
                            }
                        }
                        Ok(None) => {}
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
        self.settings_window.sync(
            ctx,
            &self.editor,
            &self.config,
            self.store.as_ref().is_some_and(ConfigStore::is_blocked),
            &self.config_path,
        );
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
            smoke.record_frame();
        }
        let ctx = ui.ctx().clone();
        if self
            .smoke
            .as_ref()
            .is_some_and(|smoke| smoke.preview_settings)
        {
            self.editor.render(
                ui,
                &self.config,
                false,
                &self.config_path,
                &mut self.pending,
            );
            self.queue_smoke_capture(&ctx);
            return;
        }
        self.inside = ctx.input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|p| i.viewport_rect().contains(p))
        }) || egui::Popup::is_any_open(&ctx)
            || super::dock_drag::active(&ctx)
            || ctx.input(|i| !i.raw.hovered_files.is_empty());
        if !self.state.is_hidden() {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.pending.push(Command::Hide);
            }
            let external_point = ctx
                .input(|i| !i.raw.hovered_files.is_empty() || !i.raw.dropped_files.is_empty())
                .then(|| self.window.cursor_client_points().ok())
                .flatten()
                .map(|(x, y)| egui::pos2(x, y));
            dock_view::render(
                ui,
                &self.config,
                &self.icons,
                external_point,
                &mut self.pending,
            );
            self.queue_smoke_capture(&ctx);
        }
        self.settings_window.show(&ctx, self.editor.open);
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

fn apply_theme(ctx: &egui::Context, theme: &str) {
    ctx.set_theme(match theme {
        "dark" => egui::ThemePreference::Dark,
        "light" => egui::ThemePreference::Light,
        _ => egui::ThemePreference::System,
    });
}
