//! Dock-local drag geometry and feedback. No config mutation, I/O or Shell calls.
use crate::{
    core::DockEdge,
    ui::{commands::Command, theme::Palette},
};
use eframe::egui;

pub(super) struct CardDrag {
    pub id: uuid::Uuid,
}
pub(super) fn active(ctx: &egui::Context) -> bool {
    egui::DragAndDrop::has_payload_of_type::<CardDrag>(ctx)
}

fn insertion_slot(edge: DockEdge, cards: &[(uuid::Uuid, egui::Rect)], point: egui::Pos2) -> usize {
    let axis = |p: egui::Pos2| if edge.is_horizontal() { p.x } else { p.y };
    cards
        .iter()
        .position(|(_, rect)| axis(point) < axis(rect.center()))
        .unwrap_or(cards.len())
}

pub(super) fn finish(
    ui: &egui::Ui,
    edge: DockEdge,
    cards: &[(uuid::Uuid, egui::Rect)],
    commands: &mut Vec<Command>,
) {
    let Some(payload) = egui::DragAndDrop::payload::<CardDrag>(ui.ctx()) else {
        return;
    };
    if ui.input(|i| i.pointer.any_released()) {
        ui.ctx().request_repaint();
    }
    let Some(point) = ui.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let area = ui.min_rect().intersect(ui.clip_rect());
    if !area.contains(point) || cards.is_empty() {
        return;
    }
    let slot = insertion_slot(edge, cards, point);
    let rect = if slot < cards.len() {
        cards[slot].1
    } else {
        cards[cards.len() - 1].1
    };
    let horizontal = edge.is_horizontal();
    let (start, end) = if horizontal {
        let x = if slot < cards.len() {
            rect.left()
        } else {
            rect.right()
        };
        (egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom()))
    } else {
        let y = if slot < cards.len() {
            rect.top()
        } else {
            rect.bottom()
        };
        (egui::pos2(rect.left(), y), egui::pos2(rect.right(), y))
    };
    ui.painter()
        .line_segment([start, end], egui::Stroke::new(3.0, Palette::of(ui).accent));
    if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
        commands.push(Command::Reorder {
            id: payload.id,
            before: cards.get(slot).map(|(id, _)| *id),
        });
        egui::DragAndDrop::clear_payload(ui.ctx());
    }
}
