//! Terminal draft editing only. Scripts run in the platform worker, never in draw.
use crate::{core::*, ui::theme};
use eframe::egui;
pub(super) fn render(ui: &mut egui::Ui, entry: &mut WorkspaceEntry) {
    let terminal = entry.terminal.as_mut().expect("terminal editor");
    egui::ComboBox::from_id_salt((entry.id, "terminal-shell"))
        .selected_text(match terminal.shell {
            TerminalShell::PowerShell => "Windows PowerShell",
            TerminalShell::CommandPrompt => "コマンドプロンプト",
            TerminalShell::Wsl => "WSL (bash)",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut terminal.shell,
                TerminalShell::PowerShell,
                "Windows PowerShell",
            );
            ui.selectable_value(
                &mut terminal.shell,
                TerminalShell::CommandPrompt,
                "コマンドプロンプト",
            );
            ui.selectable_value(&mut terminal.shell, TerminalShell::Wsl, "WSL (bash)");
        });
    if terminal.shell == TerminalShell::Wsl {
        super::field(
            ui,
            "ディストリビューション（空欄なら既定）",
            &mut terminal.distribution,
        );
        super::field(ui, "Linux ユーザー（空欄なら既定）", &mut terminal.user);
        super::field(
            ui,
            "Linux 作業ディレクトリ（例: /home/user/project）",
            &mut entry.working_directory,
        );
    } else {
        super::field(
            ui,
            "Windows 作業ディレクトリ（任意・絶対パス）",
            &mut entry.working_directory,
        );
    }
    ui.label("起動時に実行するコマンド／スクリプト（空欄ならシェルのみ）");
    ui.add(
        egui::TextEdit::multiline(&mut terminal.script)
            .desired_rows(5)
            .desired_width(f32::INFINITY),
    );
    ui.checkbox(
        &mut terminal.keep_open,
        "コマンド終了後も端末を開いたままにする",
    );
    theme::subtitle(
        ui,
        "例: python ／ WSL なら作業ディレクトリを指定して codex。複数行・シェル構文をそのまま実行します。信頼できるコマンドのみ登録し、秘密情報は保存しないでください。",
    );
    egui::CollapsingHeader::new("Terminal の詳細")
        .id_salt((entry.id, "terminal-launcher"))
        .show(ui, |ui| {
            super::field(
                ui,
                "Terminal 実行ファイル（空欄なら wt.exe を自動検出）",
                &mut terminal.launcher,
            );
        });
    theme::subtitle(
        ui,
        "必ず新規ウィンドウを開いて配置します。内部コマンドの終了や準備完了は待ちません。失敗内容はTerminal画面で確認してください。",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_editor_draw_keeps_all_shell_recipes_unchanged() {
        for shell in [
            TerminalShell::PowerShell,
            TerminalShell::CommandPrompt,
            TerminalShell::Wsl,
        ] {
            let ctx = egui::Context::default();
            let mut entry = WorkspaceEntry {
                terminal: Some(TerminalSettings {
                    shell,
                    script: "python\n# 日本語".into(),
                    ..Default::default()
                }),
                working_directory: if shell == TerminalShell::Wsl {
                    "/home/user/project"
                } else {
                    "C:\\project"
                }
                .into(),
                ..Default::default()
            };
            let original = entry.clone();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                super::super::execution::render(ui, &mut entry)
            });
            output.textures_delta.clear();
            assert_eq!(entry, original);
        }
    }
}
