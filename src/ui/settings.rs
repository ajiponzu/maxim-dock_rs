mod advanced;
mod appearance;
mod colors;
mod items;
mod view;

use super::commands::Command;
use crate::core::*;
use eframe::egui;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Dock,
    Items,
    Advanced,
}

#[derive(Clone, PartialEq)]
pub(super) struct Editor {
    pub open: bool,
    pub draft: Config,
    pub message: Option<String>,
    page: Page,
    new_label: String,
    new_target: String,
}

impl Editor {
    pub fn new(config: Config) -> Self {
        Self {
            open: false,
            page: Page::Dock,
            draft: config,
            message: None,
            new_label: String::new(),
            new_target: String::new(),
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        live: &Config,
        blocked: bool,
        config_path: &std::path::Path,
        commands: &mut Vec<Command>,
    ) {
        view::render(self, ui, live, blocked, config_path, commands);
    }

    fn add_entered(&mut self, kind: TargetKind) {
        let target = self.new_target.trim().to_owned();
        if let Err(e) = validate_target(&target, kind) {
            self.message = Some(e.into());
            return;
        }
        let label = if self.new_label.trim().is_empty() {
            match kind {
                TargetKind::Url => url::Url::parse(&target)
                    .ok()
                    .and_then(|u| u.host_str().map(str::to_owned))
                    .unwrap_or_else(|| "Website".into()),
                TargetKind::Path => label_for_path(std::path::Path::new(&target)),
            }
        } else {
            self.new_label.trim().to_owned()
        };
        match self.draft.add_item(DockItem::new(label, target, kind)) {
            Ok(()) => {
                self.new_label.clear();
                self.new_target.clear();
                self.message = Some("Item added to draft. Choose Apply and save.".into());
            }
            Err(e) => self.message = Some(e.to_string()),
        }
    }

    // Native smoke capture visits the actual settings views without clicking or saving.
    pub(super) fn capture_tag(&self) -> &'static str {
        match self.page {
            Page::Dock => "settings-dock",
            Page::Items => "settings-items",
            Page::Advanced => "settings-advanced",
        }
    }
    pub(super) fn capture_page(&mut self, index: usize) {
        self.page = match index {
            1 => Page::Items,
            2 => Page::Advanced,
            _ => Page::Dock,
        };
    }
}

pub(super) fn label_for_path(path: &std::path::Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_pages_render_at_minimum_size_without_mutating_config_or_emitting_commands() {
        for theme in [egui::ThemePreference::Light, egui::ThemePreference::Dark] {
            for page in [Page::Dock, Page::Items, Page::Advanced] {
                for palette in ["default", "ocean", "forest", "rose", "custom"] {
                    let ctx = egui::Context::default();
                    ctx.set_theme(theme);
                    let mut config = Config::defaults("missing-home".into());
                    config.appearance.palette = palette.into();
                    crate::ui::theme::configure(&ctx, &config.appearance);
                    let mut editor = Editor::new(config.clone());
                    editor.page = page;
                    let mut commands = Vec::new();
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(680.0, 520.0),
                            )),
                            ..Default::default()
                        },
                        |ui| {
                            editor.render(
                                ui,
                                &config,
                                false,
                                std::path::Path::new("config.toml"),
                                &mut commands,
                            )
                        },
                    );
                    assert!(!output.shapes.is_empty());
                    output.textures_delta.clear(); // Headless test has no GPU texture consumer.
                    assert_eq!(editor.draft, config);
                    assert!(commands.is_empty());
                }
            }
        }
    }
    #[test]
    fn editor_adds_urls_paths_and_rejects_duplicates_without_losing_draft() {
        let mut editor = Editor::new(Config::defaults("missing-home".into()));
        editor.new_target = "https://example.com/".into();
        editor.add_entered(TargetKind::Url);
        assert_eq!(editor.draft.items.last().unwrap().label, "example.com");
        editor.new_target = "https://EXAMPLE.com:443/".into();
        editor.add_entered(TargetKind::Url);
        assert_eq!(editor.draft.items.len(), 4);
        editor.new_target = "file://bad".into();
        editor.add_entered(TargetKind::Url);
        assert_eq!(editor.draft.items.len(), 4);
        let folder = tempfile::tempdir().unwrap();
        editor.new_target = folder.path().to_string_lossy().into_owned();
        editor.add_entered(TargetKind::Path);
        assert_eq!(editor.draft.items.len(), 5);
        editor.draft.validate().unwrap();
    }
}
