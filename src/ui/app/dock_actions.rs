//! Dock gesture commands: stage changes, then reuse validated atomic Apply.
use super::DockApp;
use crate::ui::commands::Command;
use eframe::egui;
use std::path::PathBuf;

impl DockApp {
    pub(super) fn open_dropped(&mut self, id: uuid::Uuid, paths: Vec<PathBuf>) {
        let Some(item) = self.config.items.iter().find(|item| item.id == id) else {
            return;
        };
        if let Err(e) = crate::platform_windows::open_files_with_app(item, &paths) {
            tracing::warn!(item_id = %id, %e, "file drop launch failed");
            self.report(format!(
                "{}: アプリで開けませんでした。対象・関連付け・権限を確認してください。",
                item.label
            ));
        }
    }
    pub(super) fn reorder_dock(
        &mut self,
        ctx: &egui::Context,
        id: uuid::Uuid,
        before: Option<uuid::Uuid>,
    ) {
        if self.editor.draft != self.config {
            self.report("並べ替えの前に、設定の変更を適用または破棄してください。".into());
            return;
        }
        let mut staged = self.config.clone();
        match staged.reorder_before(id, before) {
            Ok(true) => {
                self.pending.push(Command::Apply(staged));
                self.process_commands(ctx);
            }
            Ok(false) => {}
            Err(e) => self.report(e.to_string()),
        }
    }
}
