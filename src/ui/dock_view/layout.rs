//! Fixed toolbar and bounded card viewport. Independent of the active list and scroll offset.
use eframe::egui;
pub(super) struct Regions {
    pub cards: egui::Rect,
    pub controls: egui::Rect,
    pub separator: [egui::Pos2; 2],
}
pub(super) fn regions(area: egui::Rect, horizontal: bool, spacing: f32) -> Regions {
    let toolbar = 72.0 + spacing;
    let gap = 6.0 + spacing;
    if horizontal {
        let split = (area.right() - toolbar).max(area.left());
        let end = (split - gap).max(area.left());
        let line = (end + split) / 2.0;
        Regions {
            cards: egui::Rect::from_min_max(area.min, egui::pos2(end, area.bottom())),
            controls: egui::Rect::from_min_max(egui::pos2(split, area.top()), area.max),
            separator: [
                egui::pos2(line, area.top()),
                egui::pos2(line, area.bottom()),
            ],
        }
    } else {
        let split = (area.bottom() - toolbar).max(area.top());
        let end = (split - gap).max(area.top());
        let line = (end + split) / 2.0;
        Regions {
            cards: egui::Rect::from_min_max(area.min, egui::pos2(area.right(), end)),
            controls: egui::Rect::from_min_max(egui::pos2(area.left(), split), area.max),
            separator: [
                egui::pos2(area.left(), line),
                egui::pos2(area.right(), line),
            ],
        }
    }
}
