//! Workspace cards are launch buttons, not application drag/drop targets.
use crate::{
    core::Workspace,
    ui::{commands::Command, theme::Palette},
};
use eframe::egui;
pub(super) fn render(
    ui: &mut egui::Ui,
    size: f32,
    workspace: &Workspace,
    commands: &mut Vec<Command>,
) -> egui::Rect {
    let p = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(super::item_size(size), egui::Sense::click());
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 12, p.hover);
    }
    let icon = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + size / 2.0 + 6.0),
        egui::vec2(size * 0.8, size * 0.7),
    );
    let stroke = egui::Stroke::new(2.0, p.accent);
    ui.painter()
        .rect_stroke(icon, 4, stroke, egui::StrokeKind::Inside);
    ui.painter().line_segment(
        [
            icon.left_top() + egui::vec2(icon.width() * 0.5, 0.0),
            icon.left_bottom() + egui::vec2(icon.width() * 0.5, 0.0),
        ],
        stroke,
    );
    ui.painter()
        .line_segment([icon.center(), icon.right_center()], stroke);
    super::card::paint_label(ui, rect, &workspace.name, p.text);
    if response
        .on_hover_text(format!(
            "{} · {} アプリ",
            workspace.name,
            workspace.entries.len()
        ))
        .clicked()
    {
        commands.push(Command::LaunchWorkspace(workspace.id));
    }
    rect
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn click_emits_workspace_id_once_without_application_launch_or_reorder() {
        let ctx = egui::Context::default();
        let workspace = Workspace::new("開発環境");
        let mut rect = egui::Rect::NOTHING;
        let mut commands = vec![];
        for step in 0..3 {
            let events = if step == 0 {
                vec![]
            } else {
                let pos = rect.center();
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
                    rect = render(ui, 56.0, &workspace, &mut commands);
                },
            );
            output.textures_delta.clear();
        }
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0],Command::LaunchWorkspace(id) if id==workspace.id));
    }
}
