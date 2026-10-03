//! Small icon controls: adjacent on horizontal edges, switch above gear on vertical edges.
use crate::ui::{commands::Command, theme::Palette};
use eframe::egui;
pub(super) fn render(
    ui: &mut egui::Ui,
    horizontal: bool,
    workspaces: bool,
    commands: &mut Vec<Command>,
) -> [egui::Rect; 2] {
    let layout = if horizontal {
        egui::Layout::left_to_right(egui::Align::Center)
    } else {
        egui::Layout::top_down(egui::Align::Center)
    };
    ui.with_layout(layout, |ui| {
        let switch = button(ui, false, workspaces).on_hover_text(if workspaces {
            "アプリ一覧へ"
        } else {
            "作業環境一覧へ"
        });
        if switch.clicked() {
            commands.push(Command::ToggleWorkspaces);
        }
        let gear = button(ui, true, false).on_hover_text("設定を開く");
        if gear.clicked() {
            commands.push(Command::OpenSettings);
        }
        [switch.rect, gear.rect]
    })
    .inner
}
fn button(ui: &mut egui::Ui, gear: bool, selected: bool) -> egui::Response {
    let p = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::click());
    if response.hovered() || response.has_focus() || selected {
        ui.painter().rect_filled(rect, 9, p.hover);
    }
    let center = rect.center();
    let color = if selected { p.accent } else { p.text };
    let stroke = egui::Stroke::new(1.8, color);
    if gear {
        ui.painter().circle_stroke(center, 7.0, stroke);
        ui.painter().circle_stroke(center, 2.5, stroke);
        for i in 0..8 {
            let d = egui::Vec2::angled(i as f32 * std::f32::consts::TAU / 8.0);
            ui.painter()
                .line_segment([center + d * 7.0, center + d * 10.0], stroke);
        }
    } else {
        for (x, y) in [(-5.0, -5.0), (5.0, -5.0), (-5.0, 5.0), (5.0, 5.0)] {
            ui.painter().rect_stroke(
                egui::Rect::from_center_size(center + egui::vec2(x, y), egui::vec2(7.0, 7.0)),
                2,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DockEdge;
    #[test]
    fn all_edges_have_small_adjacent_controls_with_switch_above_vertical_gear() {
        for edge in [
            DockEdge::Top,
            DockEdge::Bottom,
            DockEdge::Left,
            DockEdge::Right,
        ] {
            for workspaces in [false, true] {
                for target in 0..2 {
                    let ctx = egui::Context::default();
                    let mut rects = [egui::Rect::NOTHING; 2];
                    let mut commands = vec![];
                    for step in 0..3 {
                        let events = if step == 0 {
                            vec![]
                        } else {
                            let pos = rects[target].center();
                            vec![
                                egui::Event::PointerMoved(pos),
                                egui::Event::PointerButton {
                                    pos,
                                    button: egui::PointerButton::Primary,
                                    pressed: step == 1,
                                    modifiers: egui::Modifiers::NONE,
                                },
                            ]
                        };
                        let mut output = ctx.run_ui(
                            egui::RawInput {
                                events,
                                screen_rect: Some(egui::Rect::from_min_size(
                                    egui::Pos2::ZERO,
                                    egui::vec2(300.0, 200.0),
                                )),
                                ..Default::default()
                            },
                            |ui| {
                                rects = render(ui, edge.is_horizontal(), workspaces, &mut commands);
                            },
                        );
                        output.textures_delta.clear();
                    }
                    assert!(rects.iter().all(|r| r.size() == egui::vec2(36.0, 36.0)));
                    if edge.is_horizontal() {
                        assert!(rects[0].right() <= rects[1].left());
                    } else {
                        assert!(rects[0].bottom() <= rects[1].top());
                    }
                    assert_eq!(commands.len(), 1);
                    assert!(matches!(
                        (&commands[0], target),
                        (Command::ToggleWorkspaces, 0) | (Command::OpenSettings, 1)
                    ));
                }
            }
        }
    }
}
