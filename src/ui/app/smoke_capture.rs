//! Optional renderer screenshots for visual QA, enabled only during native smoke.
use super::DockApp;
use eframe::egui;
use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};

pub(super) struct Capture {
    directory: PathBuf,
    requested: HashSet<String>,
    received: HashSet<String>,
    saved: HashSet<String>,
    pending: Vec<(String, Arc<egui::ColorImage>)>,
}
impl Capture {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            requested: HashSet::new(),
            received: HashSet::new(),
            saved: HashSet::new(),
            pending: Vec::new(),
        }
    }
    pub fn complete(&self) -> bool {
        self.saved.len() == 6
    }
    fn queue(&mut self, ctx: &egui::Context, tag: &str) {
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot {
                user_data, image, ..
            } = event
                && let Some(name) = user_data
                    .data
                    .as_ref()
                    .and_then(|data| data.downcast_ref::<String>())
                && self.requested.contains(name)
                && self.received.insert(name.clone())
            {
                self.pending.push((name.clone(), image));
            }
        }
        if self.requested.insert(tag.to_owned()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                tag.to_owned(),
            )));
        }
        if !self.received.contains(tag) {
            // Screenshot replies arrive on the next pass of this viewport.
            ctx.request_repaint_after(Duration::from_millis(75));
        }
    }
    fn save_pending(&mut self) -> Vec<String> {
        let mut saved = Vec::new();
        for (name, image) in self.pending.drain(..) {
            std::fs::create_dir_all(&self.directory).expect("create smoke capture directory");
            let bytes: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(egui::Color32::to_srgba_unmultiplied)
                .collect();
            image::save_buffer(
                self.directory.join(format!("{name}.png")),
                &bytes,
                image.width() as u32,
                image.height() as u32,
                image::ColorType::Rgba8,
            )
            .expect("save smoke renderer screenshot");
            self.saved.insert(name.clone());
            saved.push(name);
        }
        saved
    }
}
impl DockApp {
    pub(super) fn queue_smoke_capture(&mut self, ctx: &egui::Context) {
        let Some(smoke) = &mut self.smoke else {
            return;
        };
        if ctx.viewport_id() != egui::ViewportId::ROOT {
            return;
        }
        let tag = if smoke.preview_settings {
            smoke.preview_frames += 1;
            if smoke.preview_frames < 3 {
                ctx.request_repaint();
                return;
            }
            self.editor.capture_tag()
        } else {
            if smoke.shown_frames < 3 || smoke.started.elapsed() < Duration::from_millis(150) {
                return;
            }
            if self.workspaces.visible {
                "dock-workspaces"
            } else {
                "dock"
            }
        };
        if let Some(capture) = &mut smoke.capture {
            capture.queue(ctx, tag);
        }
    }
    pub(super) fn save_smoke_captures(&mut self, ctx: &egui::Context) {
        let Some(capture) = self.smoke.as_mut().and_then(|smoke| smoke.capture.as_mut()) else {
            return;
        };
        for name in capture.save_pending() {
            match name.as_str() {
                "settings-dock" => self.editor.capture_page(1),
                "settings-items" => self.editor.capture_page(2),
                "settings-advanced" => self.editor.capture_page(3),
                _ => {}
            }
            ctx.request_repaint();
        }
    }
}
