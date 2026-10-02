use super::Editor;
use crate::{
    core::{IconSource, TargetKind},
    ui::{
        commands::Command,
        theme::{self, Palette},
    },
};
use eframe::egui;

pub(super) fn render(editor: &mut Editor, ui: &mut egui::Ui, commands: &mut Vec<Command>) {
    ui.heading("アイテム");
    theme::subtitle(
        ui,
        "表示名や順序を編集できます。Dock へのドロップでも追加できます。",
    );
    ui.add_space(8.0);
    theme::card(ui, |ui| {
        ui.label(egui::RichText::new("新しいアイテム").size(20.0).strong());
        ui.horizontal_wrapped(|ui| {
            if ui.button("ファイルを選択").clicked() {
                commands.push(Command::PickFiles);
            }
            if ui.button("フォルダを選択").clicked() {
                commands.push(Command::PickFolder);
            }
        });
        ui.label("表示名（空欄なら自動）");
        ui.add(
            egui::TextEdit::singleline(&mut editor.new_label)
                .min_size(egui::vec2(0.0, 36.0))
                .desired_width(f32::INFINITY)
                .hint_text("例：仕事用フォルダ"),
        );
        ui.label("URL またはローカルパス");
        ui.add(
            egui::TextEdit::singleline(&mut editor.new_target)
                .min_size(egui::vec2(0.0, 36.0))
                .desired_width(f32::INFINITY)
                .hint_text("https://example.com または C:\\..."),
        );
        ui.horizontal_wrapped(|ui| {
            if ui.button("URL を追加").clicked() {
                editor.add_entered(TargetKind::Url);
            }
            if ui.button("パスを追加").clicked() {
                editor.add_entered(TargetKind::Path);
            }
        });
    });
    ui.add_space(12.0);
    ui.label(egui::RichText::new(format!("登録済み · {} 件", editor.draft.items.len())).strong());
    let mut move_to = None;
    let mut remove = None;
    let len = editor.draft.items.len();
    for (index, item) in editor.draft.items.iter_mut().enumerate() {
        ui.push_id(item.id, |ui| {
            theme::card(ui, |ui| {
                let p = Palette::of(ui);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{:02}", index + 1)).color(p.muted));
                    ui.add(
                        egui::TextEdit::singleline(&mut item.label)
                            .min_size(egui::vec2(0.0, 36.0))
                            .desired_width((ui.available_width() - 5.0).max(80.0)),
                    );
                });
                ui.add(
                    egui::Label::new(egui::RichText::new(&item.target).size(15.0).color(p.muted))
                        .truncate(),
                )
                .on_hover_text(&item.target);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(if item.kind == TargetKind::Url {
                            "URL"
                        } else {
                            "ファイル / フォルダ"
                        })
                        .size(15.0)
                        .color(p.muted),
                    );
                    if ui
                        .add_enabled(index > 0, egui::Button::new("上へ"))
                        .clicked()
                    {
                        move_to = Some((index, index - 1));
                    }
                    if ui
                        .add_enabled(index + 1 < len, egui::Button::new("下へ"))
                        .clicked()
                    {
                        move_to = Some((index, index + 1));
                    }
                    if ui.button("削除").clicked() {
                        remove = Some(index);
                    }
                });
                egui::CollapsingHeader::new("アイコン画像を変更").show(ui, |ui| {
                    let mut path = match &item.icon {
                        IconSource::File { path } => path.to_string_lossy().into_owned(),
                        _ => String::new(),
                    };
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut path)
                                .min_size(egui::vec2(0.0, 36.0))
                                .desired_width(f32::INFINITY)
                                .hint_text("PNG / ICO / JPEG のパス"),
                        )
                        .changed()
                    {
                        item.icon = if path.is_empty() {
                            IconSource::Auto
                        } else {
                            IconSource::File { path: path.into() }
                        };
                    }
                    if ui.button("自動アイコンに戻す").clicked() {
                        item.icon = IconSource::Auto;
                    }
                });
            });
        });
        ui.add_space(6.0);
    }
    if let Some(index) = remove {
        editor.draft.items.remove(index);
    } else if let Some((from, to)) = move_to {
        editor.draft.move_item(from, to);
    }
}
