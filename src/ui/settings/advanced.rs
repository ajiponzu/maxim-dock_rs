use super::Editor;
use crate::ui::{commands::Command, theme};
use eframe::egui;

pub(super) fn render(
    editor: &mut Editor,
    ui: &mut egui::Ui,
    path: &std::path::Path,
    commands: &mut Vec<Command>,
) {
    ui.heading("詳細設定");
    theme::subtitle(ui, "通常は既定値のままで使用できます。");
    ui.add_space(8.0);
    theme::card(ui, |ui| {
        ui.label(egui::RichText::new("画面端の検知").size(20.0).strong());
        ui.add(
            egui::Slider::new(&mut editor.draft.dock.cursor_poll_interval_ms, 50..=100)
                .suffix(" ms")
                .text("確認間隔"),
        );
        theme::subtitle(ui, "Dock が隠れている間だけカーソル位置を確認します。");
    });
    ui.add_space(8.0);
    theme::card(ui, |ui| {
        ui.label(egui::RichText::new("設定ファイル").size(20.0).strong());
        ui.add(
            egui::Label::new(egui::RichText::new(path.display().to_string()).monospace()).wrap(),
        );
        if ui.button("保存先フォルダを開く").clicked() {
            commands.push(Command::OpenConfigFolder);
        }
    });
    ui.add_space(8.0);
    theme::card(ui, |ui| {
        ui.label(egui::RichText::new("アプリの操作").size(20.0).strong());
        ui.horizontal_wrapped(|ui| {
            if ui.button("アイコンを再読み込み").clicked() {
                commands.push(Command::ReloadIcons);
            }
            if ui.button("Dock を表示").clicked() {
                commands.push(Command::Show);
            }
        });
        ui.separator();
        theme::subtitle(ui, "設定画面を閉じても Dock は動作を続けます。");
        if ui.button("MaXImDock を終了").clicked() {
            commands.push(Command::Quit);
        }
    });
}
