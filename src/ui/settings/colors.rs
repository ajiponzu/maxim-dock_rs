//! Draft-only color editing and preview; Apply performs validation and persistence.
use super::Editor;
use crate::{
    core::parse_hex_color,
    ui::theme::{self, Palette},
};
use eframe::egui;

pub(super) fn render(editor: &mut Editor, ui: &mut egui::Ui) {
    let live = Palette::of(ui);
    theme::card(ui, |ui| {
        ui.label(egui::RichText::new("カラーテーマ").size(20.0).strong());
        ui.horizontal_wrapped(|ui| {
            for (value, name) in [
                ("system", "システム"),
                ("light", "ライト"),
                ("dark", "ダーク"),
            ] {
                let text =
                    egui::RichText::new(name).color(if editor.draft.appearance.theme == value {
                        live.on_accent
                    } else {
                        live.text
                    });
                ui.selectable_value(&mut editor.draft.appearance.theme, value.to_owned(), text);
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (value, name) in [
                ("default", "標準"),
                ("ocean", "Ocean"),
                ("forest", "Forest"),
                ("rose", "Rose"),
                ("custom", "カスタム"),
            ] {
                let text =
                    egui::RichText::new(name).color(if editor.draft.appearance.palette == value {
                        live.on_accent
                    } else {
                        live.text
                    });
                ui.selectable_value(&mut editor.draft.appearance.palette, value.to_owned(), text);
            }
        });
        let dark = match editor.draft.appearance.theme.as_str() {
            "dark" => true,
            "light" => false,
            _ => ui.visuals().dark_mode,
        };
        let p = Palette::resolve(dark, &editor.draft.appearance);
        egui::Frame::new()
            .fill(p.panel)
            .stroke(egui::Stroke::new(1.0, p.border))
            .corner_radius(10)
            .inner_margin(14)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("プレビュー").color(p.text));
                    for color in [p.background, p.panel, p.field, p.accent, p.hover] {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 6, color);
                        ui.painter().rect_stroke(
                            rect,
                            6,
                            egui::Stroke::new(1.0, p.border),
                            egui::StrokeKind::Inside,
                        );
                    }
                });
                ui.label(
                    egui::RichText::new("Dock と設定画面に共通で適用されます。")
                        .size(15.0)
                        .color(p.muted),
                );
                ui.add(
                    egui::Button::new(egui::RichText::new("アクセント").color(p.on_accent))
                        .sense(egui::Sense::hover())
                        .fill(p.accent),
                );
            });
        if editor.draft.appearance.palette != "custom" {
            if ui.button("この配色をコピーしてカスタマイズ").clicked() {
                let c = &mut editor.draft.appearance.colors;
                for (target, color) in [
                    &mut c.background,
                    &mut c.panel,
                    &mut c.field,
                    &mut c.border,
                    &mut c.text,
                    &mut c.muted,
                    &mut c.accent,
                    &mut c.on_accent,
                    &mut c.hover,
                ]
                .into_iter()
                .zip([
                    p.background,
                    p.panel,
                    p.field,
                    p.border,
                    p.text,
                    p.muted,
                    p.accent,
                    p.on_accent,
                    p.hover,
                ]) {
                    *target = hex(color.to_array());
                }
                editor.draft.appearance.palette = "custom".into();
            }
        } else {
            theme::subtitle(
                ui,
                "色・カラーコードをクリックして編集。カスタム色は明暗モードに関係なく固定です。",
            );
            let c = &mut editor.draft.appearance.colors;
            for (name, value) in [
                ("画面の背景", &mut c.background),
                ("Dock・カード", &mut c.panel),
                ("入力欄", &mut c.field),
                ("境界線", &mut c.border),
                ("文字", &mut c.text),
                ("補足文字", &mut c.muted),
                ("アクセント", &mut c.accent),
                ("アクセント上の文字", &mut c.on_accent),
                ("ホバー", &mut c.hover),
            ] {
                ui.horizontal_wrapped(|ui| {
                    let mut rgb = parse_hex_color(value).unwrap_or([128, 128, 128]);
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        *value = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
                    }
                    code_picker(ui, value);
                    ui.label(name);
                });
                if parse_hex_color(value).is_none() {
                    ui.label("# と6桁の16進数を入力してください。保存はできません。");
                }
            }
            theme::subtitle(
                ui,
                "文字と背景のコントラストはプレビューで確認してください。",
            );
        }
        theme::subtitle(
            ui,
            "プレビューのみ即時更新。「適用して保存」で Dock 全体へ反映します。",
        );
    });
}

fn hex(color: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
}

fn code_picker(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    let response = ui
        .add_sized([110.0, 36.0], egui::Button::new(value.as_str()))
        .on_hover_text("クリックしてカラーピッカーを開く");
    egui::Popup::menu(&response)
        .id(response.id.with("code-picker"))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.spacing_mut().slider_width = 275.0;
            let [r, g, b] = parse_hex_color(value).unwrap_or([128, 128, 128]);
            let mut color = egui::Color32::from_rgb(r, g, b);
            if egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::Opaque,
            ) {
                *value = hex(color.to_array());
            }
            ui.label("カラーコード（直接入力できます）");
            ui.add(
                egui::TextEdit::singleline(value)
                    .desired_width(275.0)
                    .min_size(egui::vec2(0.0, 36.0)),
            );
            if parse_hex_color(value).is_none() {
                ui.label("# と6桁の16進数を入力してください。");
            }
        });
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clicking_color_code_opens_picker_without_changing_color() {
        let ctx = egui::Context::default();
        let mut value = "#123456".to_owned();
        let mut rect = egui::Rect::NOTHING;
        let mut popup_id = egui::Id::NULL;
        for step in 0..3 {
            let mut events = Vec::new();
            if step > 0 {
                events.push(egui::Event::PointerMoved(rect.center()));
                events.push(egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed: step == 1,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    let response = code_picker(ui, &mut value);
                    rect = response.rect;
                    popup_id = response.id.with("code-picker");
                },
            );
            output.textures_delta.clear();
        }
        assert!(egui::Popup::is_id_open(&ctx, popup_id));
        assert_eq!(value, "#123456");
    }
}
