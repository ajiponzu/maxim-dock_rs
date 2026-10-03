//! Monitor topology and normalized presets use core geometry, not native APIs.
use crate::core::*;
use eframe::egui;
pub(super) fn render(ui: &mut egui::Ui, entry: &mut WorkspaceEntry, displays: &[DisplayInfo]) {
    ui.label("配置先モニター");
    if ui
        .selectable_label(entry.monitor_id.is_empty(), "プライマリ（自動）")
        .clicked()
    {
        entry.monitor_id.clear();
    }
    if !displays.is_empty() {
        let left = displays.iter().map(|d| d.bounds.left).min().unwrap() as f32;
        let top = displays.iter().map(|d| d.bounds.top).min().unwrap() as f32;
        let right = displays
            .iter()
            .map(|d| d.bounds.left.saturating_add(d.bounds.width))
            .max()
            .unwrap() as f32;
        let bottom = displays
            .iter()
            .map(|d| d.bounds.top.saturating_add(d.bounds.height))
            .max()
            .unwrap() as f32;
        let size = egui::vec2(ui.available_width().min(460.0), 140.0);
        let (area, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let scale = (size.x / (right - left).max(1.0)).min(size.y / (bottom - top).max(1.0)) * 0.88;
        for (i, d) in displays.iter().enumerate() {
            let min = area.min
                + egui::vec2(
                    (d.bounds.left as f32 - left) * scale,
                    (d.bounds.top as f32 - top) * scale,
                )
                + egui::vec2(4.0, 4.0);
            let rect = egui::Rect::from_min_size(
                min,
                egui::vec2(
                    d.bounds.width as f32 * scale,
                    d.bounds.height as f32 * scale,
                ),
            )
            .shrink(2.0);
            let selected = !d.id.is_empty() && entry.monitor_id == d.id;
            let response = ui
                .interact(
                    rect,
                    ui.id().with((entry.id, "display", i)),
                    egui::Sense::click(),
                )
                .on_hover_text(&d.name);
            if response.clicked() && !d.id.is_empty() {
                entry.monitor_id = d.id.clone();
            }
            let v = ui.visuals();
            ui.painter().rect_filled(
                rect,
                5,
                if selected {
                    v.selection.bg_fill
                } else {
                    v.widgets.inactive.bg_fill
                },
            );
            ui.painter().rect_stroke(
                rect,
                5,
                egui::Stroke::new(1.0, v.text_color()),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{}{}", i + 1, if d.primary { " ★" } else { "" }),
                egui::FontId::proportional(18.0),
                v.text_color(),
            );
        }
    }
    if let Some((d, fallback)) = select_display(&entry.monitor_id, displays) {
        ui.label(if fallback {
            format!("指定モニター未接続 → {}", d.name)
        } else {
            d.name.clone()
        });
    }
    ui.label("配置（モニターの作業領域に対する割合）");
    ui.horizontal_wrapped(|ui| {
        if ui
            .selectable_label(entry.placement == Placement::Maximize, "最大化")
            .clicked()
        {
            entry.placement = Placement::Maximize;
        }
        for (name, x, y, width, height) in [
            ("左半分", 0.0, 0.0, 0.5, 1.0),
            ("右半分", 0.5, 0.0, 0.5, 1.0),
            ("上半分", 0.0, 0.0, 1.0, 0.5),
            ("下半分", 0.0, 0.5, 1.0, 0.5),
            ("左上", 0.0, 0.0, 0.5, 0.5),
            ("右上", 0.5, 0.0, 0.5, 0.5),
            ("左下", 0.0, 0.5, 0.5, 0.5),
            ("右下", 0.5, 0.5, 0.5, 0.5),
        ] {
            let p = Placement::Rect {
                x,
                y,
                width,
                height,
            };
            if ui.selectable_label(entry.placement == p, name).clicked() {
                entry.placement = p;
            }
        }
        if ui.button("カスタム").clicked() {
            entry.placement = Placement::Rect {
                x: 0.1,
                y: 0.1,
                width: 0.8,
                height: 0.8,
            };
        }
    });
    if let Placement::Rect {
        x,
        y,
        width,
        height,
    } = &mut entry.placement
    {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [("X", x), ("Y", y), ("幅", width), ("高さ", height)] {
                ui.label(label);
                let mut percent = *value * 100.0;
                ui.add(
                    egui::DragValue::new(&mut percent)
                        .range(0.0..=100.0)
                        .suffix("%"),
                );
                *value = percent / 100.0;
            }
        });
    }
}
