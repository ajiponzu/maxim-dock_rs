use super::{
    commands::Command,
    theme::{DOCK_LABEL_SIZE, Palette},
};
use crate::core::*;
use eframe::egui;

/// Code-drawn fallback icons require no Shell/GDI resource ownership.
pub(super) fn render(
    ui: &mut egui::Ui,
    config: &Config,
    icons: &super::icons::Icons,
    commands: &mut Vec<Command>,
) {
    let p = Palette::of(ui);
    let alpha = (config.appearance.background_opacity * 255.0).round() as u8;
    let response = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(
            p.panel.r(),
            p.panel.g(),
            p.panel.b(),
            alpha,
        ))
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(20)
        .outer_margin(5)
        .inner_margin(12)
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
                            ui.label("空の Dock");
                        }
                        for item in &config.items {
                            let size = config.dock.icon_size;
                            let (rect, response) =
                                ui.allocate_exact_size(item_size(size), egui::Sense::click());
                            let fill = if response.hovered() {
                                p.hover
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter().rect_filled(rect, 12, fill);
                            if response.hovered() || response.has_focus() {
                                ui.painter().rect_stroke(
                                    rect,
                                    12,
                                    egui::Stroke::new(1.0, p.accent),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            let icon = egui::Rect::from_center_size(
                                egui::pos2(rect.center().x, rect.top() + size / 2.0 + 6.0),
                                egui::vec2(size * 0.82, size * 0.82),
                            );
                            let color = p.accent;
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
                            paint_label(ui, rect, &item.label, p.text);
                            if response
                                .on_hover_text(format!("{}\n{}", item.label, item.target))
                                .clicked()
                            {
                                commands.push(Command::Launch(item.id));
                            }
                        }
                        ui.separator();
                        if settings_button(ui, p).on_hover_text("設定を開く").clicked() {
                            commands.push(Command::OpenSettings);
                        }
                    });
                });
        })
        .response;
    response.context_menu(|ui| {
        if ui.button("Dock を表示").clicked() {
            commands.push(Command::Show);
            ui.close();
        }
        if ui.button("設定").clicked() {
            commands.push(Command::OpenSettings);
            ui.close();
        }
        if ui.button("隠す（Esc）").clicked() {
            commands.push(Command::Hide);
            ui.close();
        }
        if ui.button("終了").clicked() {
            commands.push(Command::Quit);
        }
    });
}

pub(super) fn dock_size(config: &Config) -> (f32, f32) {
    let n = config.items.len() as f32;
    let tile = item_size(config.dock.icon_size);
    let padding = 40.0; // frame margins, border and room for the scroll bar
    let controls = 52.0 + 6.0 + config.dock.spacing * (n + 1.0);
    if config.dock.edge.is_horizontal() {
        (
            (tile.x * n + controls + padding).max(200.0),
            tile.y.max(52.0) + padding,
        )
    } else {
        (
            tile.x.max(52.0) + padding,
            (tile.y * n + controls + padding).max(200.0),
        )
    }
}

fn item_size(icon_size: f32) -> egui::Vec2 {
    egui::vec2((icon_size + 24.0).max(88.0), icon_size + 38.0)
}
fn paint_label(ui: &egui::Ui, rect: egui::Rect, label: &str, color: egui::Color32) {
    let mut job = egui::text::LayoutJob::simple_singleline(
        label.to_owned(),
        egui::FontId::proportional(DOCK_LABEL_SIZE),
        color,
    );
    job.wrap.max_width = rect.width() - 12.0;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.painter().layout_job(job);
    let position = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.bottom() - 8.0 - galley.size().y,
    );
    ui.painter()
        .with_clip_rect(rect)
        .galley(position, galley, color);
}
fn settings_button(ui: &mut egui::Ui, p: Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(52.0, 52.0), egui::Sense::click());
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 12, p.hover);
        ui.painter().rect_stroke(
            rect,
            12,
            egui::Stroke::new(1.0, p.accent),
            egui::StrokeKind::Inside,
        );
    }
    let center = egui::pos2(rect.center().x, rect.top() + 16.0);
    ui.painter()
        .circle_stroke(center, 7.0, egui::Stroke::new(2.0, p.muted));
    ui.painter()
        .circle_stroke(center, 2.5, egui::Stroke::new(1.5, p.muted));
    for i in 0..8 {
        let direction = egui::Vec2::angled(i as f32 * std::f32::consts::TAU / 8.0);
        ui.painter().line_segment(
            [center + direction * 7.0, center + direction * 10.0],
            egui::Stroke::new(2.0, p.muted),
        );
    }
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 12.0),
        egui::Align2::CENTER_CENTER,
        "設定",
        egui::FontId::proportional(15.0),
        p.text,
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readable_tiles_and_frame_budget_cover_all_edges_and_icon_sizes() {
        let mut config = Config::defaults("home".into());
        for size in [24.0, 56.0, 128.0] {
            config.dock.icon_size = size;
            let tile = item_size(size);
            assert!(tile.x >= 88.0 && tile.y >= size + DOCK_LABEL_SIZE + 16.0);
            for edge in [
                DockEdge::Top,
                DockEdge::Bottom,
                DockEdge::Left,
                DockEdge::Right,
            ] {
                config.dock.edge = edge;
                let (w, h) = dock_size(&config);
                if edge.is_horizontal() {
                    assert!(w >= tile.x * 3.0 + 52.0 && h >= tile.y + 36.0);
                } else {
                    assert!(h >= tile.y * 3.0 + 52.0 && w >= tile.x + 36.0);
                }
            }
        }
        config.items.clear();
        assert!(dock_size(&config).0 > 0.0 && dock_size(&config).1 > 0.0);
    }
}
