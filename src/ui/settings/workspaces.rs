//! Recipe editor; mutates draft only, emits commands for all external operations.
mod execution;
mod placement;
mod terminal;
use super::Editor;
use crate::{
    core::*,
    ui::{commands::Command, theme},
};
use eframe::egui;
pub(super) fn render(
    editor: &mut Editor,
    ui: &mut egui::Ui,
    live: &Config,
    commands: &mut Vec<Command>,
) {
    ui.heading("作業環境");
    theme::subtitle(
        ui,
        "アプリの引数でファイル・URL・フォルダーを指定し、新規ウィンドウを配置します。",
    );
    ui.horizontal_wrapped(|ui| {
        if ui.button("＋ 作業環境").clicked() && editor.draft.workspaces.len() < 64 {
            let w = Workspace::new("新しい作業環境");
            editor.selected_workspace = Some(w.id);
            editor.draft.workspaces.push(w);
        }
        if ui.button("モニターを再取得").clicked() {
            commands.push(Command::RefreshDisplays);
        }
    });
    if editor.selected_workspace.is_none() {
        editor.selected_workspace = editor.draft.workspaces.first().map(|w| w.id);
    }
    ui.horizontal_wrapped(|ui| {
        for w in &editor.draft.workspaces {
            if ui
                .selectable_label(editor.selected_workspace == Some(w.id), &w.name)
                .clicked()
            {
                editor.selected_workspace = Some(w.id);
            }
        }
    });
    let mut remove = None;
    if let Some(w) = editor
        .draft
        .workspaces
        .iter_mut()
        .find(|w| Some(w.id) == editor.selected_workspace)
    {
        theme::card(ui, |ui| {
            ui.label("作業環境名");
            ui.text_edit_singleline(&mut w.name);
            ui.horizontal_wrapped(|ui| {
                let saved = live.workspaces.iter().any(|saved| saved == w);
                if ui
                    .add_enabled(
                        saved && !editor.restore.running && !w.entries.is_empty(),
                        egui::Button::new("保存済みの環境を起動"),
                    )
                    .clicked()
                {
                    commands.push(Command::LaunchWorkspace(w.id));
                }
                if ui.button("削除").clicked() {
                    remove = Some(w.id);
                }
            });
            if !live.workspaces.iter().any(|saved| saved == w) {
                theme::subtitle(ui, "先に「適用して保存」してください。");
            }
        });
        ui.add_space(8.0);
        let mut delete = None;
        let mut move_to = None;
        let count = w.entries.len();
        for (index, entry) in w.entries.iter_mut().enumerate() {
            egui::CollapsingHeader::new(format!("{} · {}", index + 1, entry.label))
                .id_salt(entry.id)
                .default_open(index == 0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() {
                            move_to = Some((index, index - 1));
                        }
                        if ui
                            .add_enabled(index + 1 < count, egui::Button::new("↓"))
                            .clicked()
                        {
                            move_to = Some((index, index + 1));
                        }
                        if ui.button("エントリーを削除").clicked() {
                            delete = Some(index);
                        }
                    });
                    field(ui, "表示名", &mut entry.label);
                    execution::render(ui, entry);
                    execution::arguments(ui, entry);
                    if entry.wsl.as_ref().is_none_or(|wsl| wsl.place_window) {
                        egui::CollapsingHeader::new("ウィンドウの識別（起動ブローカー等）")
                            .id_salt((entry.id, "matching"))
                            .default_open(entry.wsl.is_some())
                            .show(ui, |ui| {
                                field(
                                    ui,
                                    if entry.terminal.is_some() { "配置対象 .exe（通常は空欄・Windows Terminal を自動識別）" } else if entry.wsl.is_some() { "配置対象の Windows .exe（必須・例: Code.exe の絶対パス）" } else { "配置対象 .exe（空欄なら起動 .exe）" },
                                    &mut entry.window_executable,
                                );
                                field(
                                    ui,
                                    "タイトルに含まれる文字（任意・大文字小文字を区別）",
                                    &mut entry.title_contains,
                                );
                            });
                        if entry.wsl.is_some() {
                            theme::subtitle(ui, "ウィンドウの識別を開いて対象 .exe を指定してください。VS Code は --new-window、Terminal は新規ウィンドウの引数を使います。");
                        }
                        placement::render(ui, entry, &editor.displays);
                    }
                });
        }
        if let Some(i) = delete {
            w.entries.remove(i);
        } else if let Some((a, b)) = move_to {
            w.entries.swap(a, b);
        }
        if ui.button("＋ アプリを追加").clicked() && w.entries.len() < 64 {
            w.entries.push(WorkspaceEntry::default());
        }
    }
    if let Some(id) = remove {
        editor.draft.workspaces.retain(|w| w.id != id);
        editor.selected_workspace = None;
    }
    if editor.restore.running || !editor.restore.results.is_empty() {
        ui.separator();
        ui.heading(format!("実行結果 · {}", editor.restore.workspace_name));
        if editor.restore.running {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("実行中（Windows 配置: 15秒／WSL: 設定した待機時間）");
                if ui.button("中止").clicked() {
                    commands.push(Command::CancelWorkspace);
                }
            });
        }
        for r in &editor.restore.results {
            ui.label(format!("{}: {}", r.label, r.message));
        }
    }
    theme::subtitle(
        ui,
        "既存ウィンドウ・未保存内容は変更しません。単一インスタンスアプリには新規ウィンドウ用の引数が必要です。",
    );
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
}
