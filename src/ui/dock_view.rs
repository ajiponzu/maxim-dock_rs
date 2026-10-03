mod card;
mod controls;
#[cfg(test)]
mod fixed_controls_tests;
mod layout;
mod workspace_card;
use super::{commands::Command, theme::Palette};
use crate::core::*;
use eframe::egui;

/// Code-drawn fallback icons require no Shell/GDI resource ownership.
pub(super) fn render(
    ui: &mut egui::Ui,
    config: &Config,
    icons: &super::icons::Icons,
    workspaces: bool,
    external_point: Option<egui::Pos2>,
    commands: &mut Vec<Command>,
) -> [egui::Rect; 2] {
    let p = Palette::of(ui);
    let external_drop = ui.input(|i| !i.raw.hovered_files.is_empty());
    let alpha = (config.appearance.background_opacity * 255.0).round() as u8;
    let output = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(
            p.panel.r(),
            p.panel.g(),
            p.panel.b(),
            alpha,
        ))
        .stroke(egui::Stroke::new(
            if external_drop { 3.0 } else { 1.0 },
            if external_drop { p.accent } else { p.border },
        ))
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
            let regions = layout::regions(ui.max_rect(), horizontal, config.dock.spacing);
            ui.scope_builder(egui::UiBuilder::new().max_rect(regions.cards), |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(regions.cards));
                egui::ScrollArea::new([horizontal, !horizontal])
                    .id_salt(("dock-cards", workspaces))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.with_layout(layout, |ui| {
                            if (workspaces && config.workspaces.is_empty())
                                || (!workspaces && config.items.is_empty())
                            {
                                ui.label(if workspaces {
                                    "作業環境は設定から追加"
                                } else {
                                    "空の Dock"
                                });
                            }
                            let mut cards = Vec::new();
                            if workspaces {
                                for workspace in &config.workspaces {
                                    workspace_card::render(
                                        ui,
                                        config.dock.icon_size,
                                        workspace,
                                        commands,
                                    );
                                }
                            } else {
                                for item in &config.items {
                                    cards.push((
                                        item.id,
                                        card::render(
                                            ui,
                                            config.dock.icon_size,
                                            icons.texture(item.id),
                                            item,
                                            commands,
                                        ),
                                    ));
                                }
                            }
                            if !workspaces {
                                super::dock_drag::finish(ui, config.dock.edge, &cards, commands);
                            }
                            super::dock_drop::finish(ui, config, &cards, external_point, commands);
                        });
                    });
            });
            ui.painter()
                .line_segment(regions.separator, egui::Stroke::new(1.0, p.border));
            ui.scope_builder(egui::UiBuilder::new().max_rect(regions.controls), |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(regions.controls));
                controls::render(ui, horizontal, workspaces, commands)
            })
            .inner
        });
    output.response.context_menu(|ui| {
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
    output.inner
}

pub(super) fn dock_size(config: &Config) -> (f32, f32) {
    // The application list owns the window budget; other views scroll inside it.
    let n = config.items.len().max(1) as f32;
    let tile = item_size(config.dock.icon_size);
    let padding = 40.0; // frame margins, border and room for the scroll bar
    let controls = 72.0 + 6.0 + config.dock.spacing * (n + 2.0);
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_count_never_changes_application_sized_window_on_any_edge() {
        let mut config = Config::defaults("home".into());
        for edge in [
            DockEdge::Top,
            DockEdge::Bottom,
            DockEdge::Left,
            DockEdge::Right,
        ] {
            config.dock.edge = edge;
            for app_count in [0, 1, 3] {
                config.items = Config::defaults("home".into())
                    .items
                    .into_iter()
                    .take(app_count)
                    .collect();
                let expected = dock_size(&config);
                for count in [0, 1, 16, 64] {
                    config.workspaces = (0..count).map(|_| Workspace::new("環境")).collect();
                    assert_eq!(dock_size(&config), expected);
                }
            }
        }
    }
    #[test]
    fn readable_tiles_and_frame_budget_cover_all_edges_and_icon_sizes() {
        let mut config = Config::defaults("home".into());
        for size in [24.0, 56.0, 128.0] {
            config.dock.icon_size = size;
            let tile = item_size(size);
            assert!(tile.x >= 88.0 && tile.y >= size + crate::ui::theme::DOCK_LABEL_SIZE + 16.0);
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
