//! One Dock card: rendering, click and drag-source interaction only.
#[cfg(test)]
mod drag_tests;
use crate::{
    core::*,
    ui::{
        commands::Command,
        theme::{DOCK_LABEL_SIZE, Palette},
    },
};
use eframe::egui;
pub(super) fn render(
    ui: &mut egui::Ui,
    size: f32,
    texture: Option<egui::TextureId>,
    item: &DockItem,
    commands: &mut Vec<Command>,
) -> egui::Rect {
    let p = Palette::of(ui);
    let (rect, response) =
        ui.allocate_exact_size(super::item_size(size), egui::Sense::click_and_drag());
    response.dnd_set_drag_payload(super::super::dock_drag::CardDrag { id: item.id });
    let fill = if response.hovered() || response.dragged() {
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
    if let Some(texture) = texture {
        ui.painter().image(
            texture,
            icon,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
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

    rect
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
