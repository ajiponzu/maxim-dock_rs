use super::commands::Command;
use crate::core::*;
use eframe::egui;

/// Code-drawn fallback icons require no Shell/GDI resource ownership.
pub(super) fn render(
    ui: &mut egui::Ui,
    config: &Config,
    icons: &super::icons::Icons,
    commands: &mut Vec<Command>,
) {
    let alpha = (config.appearance.background_opacity * 255.0).round() as u8;
    let response = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(25, 30, 40, alpha))
        .corner_radius(16)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(config.dock.spacing, config.dock.spacing);
            let horizontal = config.dock.edge.is_horizontal();
            let layout = if horizontal {
                egui::Layout::left_to_right(egui::Align::Center)
            } else {
                egui::Layout::top_down(egui::Align::Center)
            };
            egui::ScrollArea::new([horizontal, !horizontal])
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.with_layout(layout, |ui| {
                        if config.items.is_empty() {
                            ui.label("Empty Dock");
                        }
                        for item in &config.items {
                            let size = config.dock.icon_size;
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(size + 12.0, size + 26.0),
                                egui::Sense::click(),
                            );
                            let fill = if response.hovered() {
                                egui::Color32::from_rgb(65, 90, 125)
                            } else {
                                egui::Color32::from_rgb(45, 55, 75)
                            };
                            ui.painter().rect_filled(rect, 8, fill);
                            let icon = egui::Rect::from_center_size(
                                egui::pos2(rect.center().x, rect.top() + size / 2.0 + 4.0),
                                egui::vec2(size * 0.65, size * 0.65),
                            );
                            let color = egui::Color32::from_rgb(175, 205, 245);
                            if let Some(texture) = icons.texture(item.id) {
                                ui.painter().image(
                                    texture,
                                    icon,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    egui::Color32::WHITE,
                                );
                            } else if item.kind == TargetKind::Url {
                                ui.painter().circle_stroke(
                                    icon.center(),
                                    icon.width() / 2.0,
                                    egui::Stroke::new(2.0, color),
                                );
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(icon.left(), icon.center().y),
                                        egui::pos2(icon.right(), icon.center().y),
                                    ],
                                    egui::Stroke::new(2.0, color),
                                );
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(icon.center().x, icon.top()),
                                        egui::pos2(icon.center().x, icon.bottom()),
                                    ],
                                    egui::Stroke::new(2.0, color),
                                );
                            } else {
                                ui.painter().rect_stroke(
                                    icon,
                                    4,
                                    egui::Stroke::new(2.0, color),
                                    egui::StrokeKind::Inside,
                                );
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(icon.left() + 4.0, icon.top() + 8.0),
                                        egui::pos2(icon.right() - 4.0, icon.top() + 8.0),
                                    ],
                                    egui::Stroke::new(2.0, color),
                                );
                            }
                            let label: String = item.label.chars().take(9).collect();
                            ui.painter().with_clip_rect(rect).text(
                                egui::pos2(rect.center().x, rect.bottom() - 12.0),
                                egui::Align2::CENTER_CENTER,
                                label,
                                egui::FontId::proportional(12.0),
                                egui::Color32::WHITE,
                            );
                            if response
                                .on_hover_text(format!("{}\n{}", item.label, item.target))
                                .clicked()
                            {
                                commands.push(Command::Launch(item.id));
                            }
                        }
                        if ui
                            .add_sized([32.0, 32.0], egui::Button::new("..."))
                            .on_hover_text("Settings")
                            .clicked()
                        {
                            commands.push(Command::OpenSettings);
                        }
                    });
                });
        })
        .response;
    response.context_menu(|ui| {
        if ui.button("Show Dock").clicked() {
            commands.push(Command::Show);
            ui.close();
        }
        if ui.button("Settings").clicked() {
            commands.push(Command::OpenSettings);
            ui.close();
        }
        if ui.button("Hide (Esc)").clicked() {
            commands.push(Command::Hide);
            ui.close();
        }
        if ui.button("Quit").clicked() {
            commands.push(Command::Quit);
        }
    });
}

pub(super) fn dock_size(config: &Config) -> (f32, f32) {
    let n = config.items.len() as f32;
    let along = (config.dock.icon_size + 12.0) * n + config.dock.spacing * n + 64.0;
    let cross = config.dock.icon_size + 58.0;
    if config.dock.edge.is_horizontal() {
        (along.max(160.0), cross)
    } else {
        (
            cross,
            ((config.dock.icon_size + 26.0) * n + config.dock.spacing * n + 64.0).max(160.0),
        )
    }
}
