use super::{Editor, Page, advanced, appearance, items, workspaces};
use crate::{
    core::Config,
    ui::{
        commands::Command,
        theme::{self, Palette},
    },
};
use eframe::egui;

pub(super) fn render(
    editor: &mut Editor,
    ui: &mut egui::Ui,
    live: &Config,
    blocked: bool,
    path: &std::path::Path,
    commands: &mut Vec<Command>,
) {
    if ui.ctx().input(|i| i.viewport().close_requested()) {
        editor.open = false;
    }
    let p = Palette::of(ui);
    egui::Panel::top("settings_header")
        .frame(egui::Frame::new().fill(p.panel).inner_margin(20))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("MaXImDock").size(26.0).strong());
            theme::subtitle(ui, "自分に合った Dock にカスタマイズ");
        });
    egui::Panel::bottom("settings_actions")
        .frame(egui::Frame::new().fill(p.panel).inner_margin(16))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if theme::primary(ui, "適用して保存", !blocked).clicked() {
                    commands.push(Command::Apply(editor.draft.clone()));
                }
                if ui.button("変更を破棄").clicked() {
                    commands.push(Command::DiscardSettings);
                }
                ui.label(
                    egui::RichText::new(if editor.draft != *live {
                        "未保存の変更があります"
                    } else {
                        "現在の設定と一致しています"
                    })
                    .size(15.0)
                    .color(p.muted),
                );
            });
        });
    egui::Panel::left("settings_navigation")
        .exact_size(152.0)
        .resizable(false)
        .frame(egui::Frame::new().fill(p.panel).inner_margin(14))
        .show(ui, |ui| {
            ui.add_space(8.0);
            for (page, name, description) in [
                (Page::Dock, "Dock", "表示・外観・動作"),
                (Page::Items, "アイテム", "追加・名前・順序"),
                (Page::Workspaces, "作業環境", "起動・引数・配置"),
                (Page::Advanced, "詳細", "設定ファイルなど"),
            ] {
                let selected = editor.page == page;
                let text = egui::RichText::new(name).strong().color(if selected {
                    p.on_accent
                } else {
                    p.text
                });
                if ui
                    .add_sized(
                        [ui.available_width(), 42.0],
                        egui::Button::new(text).fill(if selected {
                            p.accent
                        } else {
                            egui::Color32::TRANSPARENT
                        }),
                    )
                    .clicked()
                {
                    editor.page = page;
                }
                theme::subtitle(ui, description);
                ui.add_space(12.0);
            }
            ui.separator();
            if ui.button("Dock を表示").clicked() {
                commands.push(Command::Show);
            }
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(p.background).inner_margin(22))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("settings_content")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(message) = &editor.message {
                        theme::card(ui, |ui| {
                            ui.label(message);
                        });
                        ui.add_space(8.0);
                    }
                    if blocked {
                        theme::card(ui, |ui| {
                            ui.label(
                                egui::RichText::new("元の設定ファイルは保護されています").strong(),
                            );
                            ui.label("バックアップを作成するまで、設定の保存はできません。");
                            if ui.button("元ファイルをバックアップ").clicked() {
                                commands.push(Command::BackUpInvalid);
                            }
                        });
                        ui.add_space(8.0);
                    }
                    match editor.page {
                        Page::Dock => appearance::render(editor, ui),
                        Page::Items => items::render(editor, ui, commands),
                        Page::Advanced => advanced::render(editor, ui, path, commands),
                        Page::Workspaces => workspaces::render(editor, ui, live, commands),
                    }
                });
        });
}
