use super::commands::Command;
use crate::core::*;
use eframe::egui;

pub(super) struct Editor {
    pub open: bool,
    pub draft: Config,
    pub message: Option<String>,
    new_label: String,
    new_target: String,
}

impl Editor {
    pub fn new(config: Config) -> Self {
        Self {
            open: false,
            draft: config,
            message: None,
            new_label: String::new(),
            new_target: String::new(),
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        blocked: bool,
        config_path: &std::path::Path,
        commands: &mut Vec<Command>,
    ) {
        if ui.ctx().input(|i| i.viewport().close_requested()) {
            self.open = false;
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("MaXImDock Settings");
            ui.label("Changes take effect when you choose Apply and save.");
            if let Some(message) = &self.message {
                ui.colored_label(egui::Color32::from_rgb(210, 120, 40), message);
            }
            if blocked {
                ui.label("Original config is preserved. Saving is disabled until it is backed up.");
                if ui
                    .button("Back up invalid config and enable saving")
                    .clicked()
                {
                    commands.push(Command::BackUpInvalid);
                }
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!blocked, egui::Button::new("Apply and save"))
                    .clicked()
                {
                    commands.push(Command::Apply(self.draft.clone()));
                }
                if ui.button("Discard edits").clicked() {
                    commands.push(Command::DiscardSettings);
                }
                if ui.button("Show Dock").clicked() {
                    commands.push(Command::Show);
                }
                if ui.button("Open config folder").clicked() {
                    commands.push(Command::OpenConfigFolder);
                }
                if ui.button("Quit app").clicked() {
                    commands.push(Command::Quit);
                }
            });
            ui.label(config_path.display().to_string());
            if ui.button("Reload icons").clicked() {
                commands.push(Command::ReloadIcons);
            }
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Dock");
                ui.horizontal(|ui| {
                    ui.label("Edge");
                    for (edge, label) in [
                        (DockEdge::Top, "Top"),
                        (DockEdge::Bottom, "Bottom"),
                        (DockEdge::Left, "Left"),
                        (DockEdge::Right, "Right"),
                    ] {
                        ui.selectable_value(&mut self.draft.dock.edge, edge, label);
                    }
                });
                ui.add(
                    egui::Slider::new(&mut self.draft.dock.icon_size, 24.0..=128.0)
                        .text("Icon size (points)"),
                );
                ui.add(egui::Slider::new(&mut self.draft.dock.spacing, 0.0..=32.0).text("Spacing"));
                ui.checkbox(&mut self.draft.dock.auto_hide, "Auto hide");
                ui.checkbox(&mut self.draft.dock.always_on_top, "Always on top");
                ui.add(
                    egui::Slider::new(&mut self.draft.dock.hide_delay_ms, 100..=10000)
                        .text("Hide delay (ms)"),
                );
                ui.add(
                    egui::Slider::new(&mut self.draft.dock.reveal_hold_ms, 100..=2000)
                        .text("Reveal hold (ms)"),
                );
                ui.add(
                    egui::Slider::new(&mut self.draft.dock.hot_zone_px, 1..=32)
                        .text("Hot zone (physical px)"),
                );
                ui.add(
                    egui::Slider::new(&mut self.draft.dock.cursor_poll_interval_ms, 50..=100)
                        .text("Cursor polling (ms)"),
                );
                ui.add(
                    egui::Slider::new(&mut self.draft.appearance.background_opacity, 0.2..=1.0)
                        .text("Background opacity"),
                );
                ui.horizontal(|ui| {
                    ui.label("Theme");
                    for name in ["system", "dark", "light"] {
                        ui.selectable_value(
                            &mut self.draft.appearance.theme,
                            name.to_owned(),
                            name,
                        );
                    }
                });
                ui.separator();
                ui.heading("Items (ordered)");
                let mut move_to = None;
                let mut remove = None;
                let len = self.draft.items.len();
                for (index, item) in self.draft.items.iter_mut().enumerate() {
                    ui.push_id(item.id, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut item.label).desired_width(180.0),
                            );
                            if ui.add_enabled(index > 0, egui::Button::new("Up")).clicked() {
                                move_to = Some((index, index - 1));
                            }
                            if ui
                                .add_enabled(index + 1 < len, egui::Button::new("Down"))
                                .clicked()
                            {
                                move_to = Some((index, index + 1));
                            }
                            if ui.button("Delete").clicked() {
                                remove = Some(index);
                            }
                        });
                        ui.label(&item.target);
                        ui.horizontal(|ui| {
                            ui.label("Icon image (optional)");
                            let mut path = match &item.icon {
                                IconSource::File { path } => path.to_string_lossy().into_owned(),
                                _ => String::new(),
                            };
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut path)
                                        .desired_width(260.0)
                                        .hint_text("PNG / ICO / JPEG path"),
                                )
                                .changed()
                            {
                                item.icon = if path.is_empty() {
                                    IconSource::Auto
                                } else {
                                    IconSource::File { path: path.into() }
                                };
                            }
                            if ui.button("Auto").clicked() {
                                item.icon = IconSource::Auto;
                            }
                        });
                    });
                }
                if let Some(index) = remove {
                    self.draft.items.remove(index);
                } else if let Some((from, to)) = move_to {
                    self.draft.move_item(from, to);
                }
                ui.separator();
                ui.heading("Add item");
                ui.horizontal(|ui| {
                    if ui.button("Choose files...").clicked() {
                        commands.push(Command::PickFiles);
                    }
                    if ui.button("Choose folder...").clicked() {
                        commands.push(Command::PickFolder);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Label");
                    ui.text_edit_singleline(&mut self.new_label);
                });
                ui.horizontal(|ui| {
                    ui.label("URL / path");
                    ui.text_edit_singleline(&mut self.new_target);
                });
                ui.horizontal(|ui| {
                    if ui.button("Add URL").clicked() {
                        self.add_entered(TargetKind::Url);
                    }
                    if ui.button("Add path").clicked() {
                        self.add_entered(TargetKind::Path);
                    }
                });
            });
        });
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
