use super::Editor;
use crate::{core::DockEdge, ui::theme};
use eframe::egui;

pub(super) fn render(editor: &mut Editor, ui: &mut egui::Ui) {
    ui.heading("Dock の設定");
    theme::subtitle(ui, "変更は「適用して保存」で反映されます。");
    ui.add_space(8.0);
    super::colors::render(editor, ui);
    ui.add_space(8.0);
    theme::card(ui, |ui| {
        ui.label(egui::RichText::new("表示と外観").size(20.0).strong());
        ui.label("表示する画面端");
        ui.horizontal_wrapped(|ui| {
            for (edge, name) in [
                (DockEdge::Top, "上"),
                (DockEdge::Bottom, "下"),
                (DockEdge::Left, "左"),
                (DockEdge::Right, "右"),
            ] {
                let p = theme::Palette::of(ui);
                let text = egui::RichText::new(name).color(if editor.draft.dock.edge == edge {
                    p.on_accent
                } else {
                    p.text
                });
                ui.selectable_value(&mut editor.draft.dock.edge, edge, text);
            }
        });
        ui.add(
            egui::Slider::new(&mut editor.draft.dock.icon_size, 24.0..=128.0)
                .text("アイコンサイズ"),
        );
        ui.add(
            egui::Slider::new(&mut editor.draft.dock.spacing, 0.0..=32.0).text("アイテムの間隔"),
        );
        ui.add(
            egui::Slider::new(&mut editor.draft.appearance.background_opacity, 0.2..=1.0)
                .text("背景の不透明度"),
        );
        ui.checkbox(
            &mut editor.draft.dock.always_on_top,
            "ほかのウィンドウより手前に表示",
        );
    });
    ui.add_space(8.0);
    theme::card(ui, |ui| {
        ui.label(egui::RichText::new("自動非表示").size(20.0).strong());
        ui.checkbox(
            &mut editor.draft.dock.auto_hide,
            "使っていないときは Dock を隠す",
        );
        ui.add(
            egui::Slider::new(&mut editor.draft.dock.hide_delay_ms, 100..=10000)
                .suffix(" ms")
                .text("非表示までの時間"),
        );
        ui.add(
            egui::Slider::new(&mut editor.draft.dock.reveal_hold_ms, 100..=2000)
                .suffix(" ms")
                .text("表示直後の保持時間"),
        );
        ui.add(
            egui::Slider::new(&mut editor.draft.dock.hot_zone_px, 1..=32)
                .suffix(" px")
                .text("画面端の検知幅"),
        );
        theme::subtitle(ui, "画面端の検知幅は、ディスプレイの物理ピクセルです。");
    });
}
