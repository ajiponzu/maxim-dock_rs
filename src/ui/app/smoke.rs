//! Native integration driver for --smoke-test; never uses the user's config.
use super::DockApp;
use crate::{
    core::*,
    platform_windows::{ConfigStore, TrayAction},
    ui::commands::Command,
};
use eframe::egui;
use std::time::{Duration, Instant};

pub(super) struct Smoke {
    started: Instant,
    reveals: usize,
    next: Instant,
    hidden_polls: usize,
    shown_frames: usize,
    settings_frames: usize,
    finish_stage: u8,
}

impl Smoke {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            started: now,
            reveals: 0,
            next: now + Duration::from_millis(400),
            hidden_polls: 0,
            shown_frames: 0,
            settings_frames: 0,
            finish_stage: 0,
        }
    }
    pub(super) fn record_hidden_poll(&mut self) {
        self.hidden_polls += 1;
    }
    pub(super) fn record_frame(&mut self) {
        self.shown_frames += 1;
    }
    pub(super) fn record_settings_frame(&mut self) {
        self.settings_frames += 1;
    }
}

impl DockApp {
    pub(super) fn smoke_tick(&mut self, ctx: &egui::Context, now: Instant) {
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
                let next = self.state.advance(
                    now,
                    Some(self.monitor.id),
                    true,
                    false,
                    self.config.dock.timing(),
                );
                smoke.next = now + Duration::from_millis(300);
                self.apply_state(ctx, next);
                assert!(self.window.is_visible(), "HWND must really be shown");
                let bounds = self.window.bounds().expect("GetWindowRect during smoke");
                let expected = dock_anchor_position(
                    self.config.dock.edge,
                    self.monitor.rect,
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
                        assert!(self.tray.is_some(), "native smoke requires tray creation");
                        let directory = self._smoke_directory.as_ref().unwrap().path().to_owned();
                        let file = directory.join("dropped 日本語.txt");
                        std::fs::write(&file, b"smoke").unwrap();
                        let original = self.config.items.len();
                        self.pending
                            .push(Command::DropPaths(vec![file.clone(), directory]));
                        self.process_commands(ctx);
                        assert_eq!(self.config.items.len(), original + 2);
                        self.pending.push(Command::DropPaths(vec![file]));
                        self.process_commands(ctx);
                        assert_eq!(
                            self.config.items.len(),
                            original + 2,
                            "duplicate must not add"
                        );
                        self.editor.message = None;
                        let (_, restored, error) = ConfigStore::load(
                            self.config_path.clone(),
                            Config::defaults("unused".into()),
                        );
                        assert!(error.is_none());
                        assert_eq!(restored, self.config, "Apply/save must survive reload");
                        self.apply_state(ctx, DockVisibility::Hidden);
                        self.editor.open = false;
                        self.tray
                            .as_ref()
                            .unwrap()
                            .inject_for_smoke(TrayAction::Settings);
                        smoke.finish_stage = 1;
                    }
                    1 => {
                        assert!(
                            smoke.settings_frames > 0,
                            "settings must render with hidden root"
                        );
                        self.editor.open = false;
                        ctx.request_repaint();
                        self.tray
                            .as_ref()
                            .unwrap()
                            .inject_for_smoke(TrayAction::Show);
                        smoke.finish_stage = 2;
                    }
                    _ => {
                        assert!(
                            self.window.is_visible(),
                            "tray Show must reveal hidden root"
                        );
                        tracing::info!(
                            polls = smoke.hidden_polls,
                            frames = smoke.shown_frames,
                            "NATIVE_SMOKE_PASS: 30 real hide/show cycles; synthetic hot-zone entry"
                        );
                        tracing::info!(
                            "PHASE2_SMOKE_PASS: four-edge Apply/save/reload and settings open/close with hidden root"
                        );
                        tracing::info!(
                            "PHASE3_SMOKE_PASS: native tray created; synthetic menu Settings/Show/Quit; dropped-path command save/reload and duplicate rejection"
                        );
                        self.tray
                            .as_ref()
                            .unwrap()
                            .inject_for_smoke(TrayAction::Quit);
                        self.receive_tray();
                        self.process_commands(ctx);
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
                    self.monitor.rect,
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
