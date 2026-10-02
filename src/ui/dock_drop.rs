//! External file drop routing only; no registration, mutation or Shell calls.
use crate::{
    core::{Config, TargetKind},
    ui::{commands::Command, theme::Palette},
};
use eframe::egui;

#[cfg(test)]
mod tests;

pub(super) fn accepts(item: &crate::core::DockItem) -> bool {
    item.kind == TargetKind::Path
        && std::path::Path::new(&item.target)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe") || ext.eq_ignore_ascii_case("lnk"))
}

pub(super) fn finish(
    ui: &egui::Ui,
    config: &Config,
    cards: &[(uuid::Uuid, egui::Rect)],
    point: Option<egui::Pos2>,
    commands: &mut Vec<Command>,
) {
    let hovered = ui.input(|i| !i.raw.hovered_files.is_empty());
    let dropped = ui
        .ctx()
        .input_mut(|i| std::mem::take(&mut i.raw.dropped_files));
    if !hovered && dropped.is_empty() {
        return;
    }
    if hovered {
        // OLE drag motion may not generate PointerMoved/repaint events.
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(33));
    }
    let target = point
        .filter(|p| ui.clip_rect().contains(*p))
        .and_then(|p| cards.iter().find(|(_, rect)| rect.contains(p)))
        .filter(|(id, _)| {
            config
                .items
                .iter()
                .any(|item| item.id == *id && accepts(item))
        });
    if let Some((_, rect)) = target {
        ui.painter().rect_stroke(
            *rect,
            12,
            egui::Stroke::new(3.0, Palette::of(ui).accent),
            egui::StrokeKind::Inside,
        );
        egui::Area::new(egui::Id::new("file-drop-hint"))
            .fixed_pos(rect.left_bottom())
            .order(egui::Order::Tooltip)
            .show(ui.ctx(), |ui| {
                ui.label("このアプリで開く");
            });
    }
    if !dropped.is_empty() {
        if let Some((id, _)) = target {
            commands.push(Command::OpenDropped {
                id: *id,
                paths: dropped.iter().map(|f| f.path().to_owned()).collect(),
            });
        } else {
            commands.push(Command::RejectDrop);
        }
    }
}
