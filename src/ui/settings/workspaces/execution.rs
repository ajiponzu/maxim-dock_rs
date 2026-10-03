//! Execution mode editor; no native launches or filesystem checks while drawing.
use crate::{core::*, ui::theme};
use eframe::egui;
pub(super) fn render(ui: &mut egui::Ui, entry: &mut WorkspaceEntry) {
    let mut enabled = entry.wsl.is_some();
    if ui
        .checkbox(&mut enabled, "WSL モード（Linux 内で実行）")
        .changed()
    {
        entry.wsl = enabled.then(WslSettings::default);
    }
    if let Some(wsl) = &mut entry.wsl {
        super::field(
            ui,
            "ディストリビューション（空欄なら既定）",
            &mut wsl.distribution,
        );
        super::field(ui, "Linux ユーザー（空欄なら既定）", &mut wsl.user);
        let mut mode = if wsl.login_shell {
            1
        } else if wsl.direct_exec {
            2
        } else {
            0
        };
        let previous = mode;
        egui::ComboBox::from_id_salt((entry.id, "wsl-execution-mode"))
            .selected_text(wsl.execution_mode())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, 0, "WSL 標準シェル（通常の wsl code と同じ）");
                ui.selectable_value(&mut mode, 1, "bash ログインシェル");
                ui.selectable_value(&mut mode, 2, "直接実行（--exec、シェルを使わない）");
            });
        if mode != previous {
            wsl.login_shell = mode == 1;
            wsl.direct_exec = mode == 2;
        }
        theme::subtitle(
            ui,
            "通常は WSL 標準シェルを使用。各引数は個別に引用して渡し、シェル式として評価しません。変更後は「適用して保存」してください。",
        );
        ui.checkbox(
            &mut wsl.place_window,
            "起動した Windows ウィンドウを配置する",
        );
        ui.checkbox(
            &mut wsl.wait_for_exit,
            "コマンドの終了・成功を待って次へ進む",
        );
        ui.add_enabled_ui(wsl.wait_for_exit, |ui| {
            ui.horizontal(|ui| {
                ui.label("終了待ちの上限");
                ui.add(
                    egui::DragValue::new(&mut wsl.timeout_secs)
                        .range(1..=600)
                        .suffix(" 秒"),
                );
            });
        });
        theme::subtitle(
            ui,
            "コンソールは表示しません。失敗時は診断出力を表示します。配置を選ぶ場合は対象の Windows .exe を指定してください。",
        );
        super::field(
            ui,
            "Linux のコマンド／実行ファイル（例: ls、/bin/true）",
            &mut entry.executable,
        );
        super::field(
            ui,
            "Linux 作業ディレクトリ（空欄／~／絶対パス）",
            &mut entry.working_directory,
        );
    } else {
        super::field(ui, "起動する .exe の絶対パス", &mut entry.executable);
        super::field(ui, "作業ディレクトリ（任意）", &mut entry.working_directory);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_modes_render_without_changing_linux_command_or_emitting_launches() {
        for wsl in [
            None,
            Some(WslSettings::default()),
            Some(WslSettings {
                direct_exec: true,
                ..Default::default()
            }),
            Some(WslSettings {
                login_shell: true,
                place_window: true,
                ..Default::default()
            }),
        ] {
            let ctx = egui::Context::default();
            let mut entry = WorkspaceEntry {
                executable: if wsl.is_some() {
                    "ls"
                } else {
                    "C:\\apps\\app.exe"
                }
                .into(),
                wsl,
                ..Default::default()
            };
            let original = entry.clone();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500.0, 500.0),
                    )),
                    ..Default::default()
                },
                |ui| render(ui, &mut entry),
            );
            output.textures_delta.clear();
            assert_eq!(entry, original);
        }
    }
}
