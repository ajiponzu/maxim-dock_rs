//! App-side workspace wiring. Recipe editing and native execution remain separate.
use super::DockApp;
use crate::{platform_windows::*, ui::settings::Page};
use eframe::egui;
#[derive(Default)]
pub(super) struct Workspaces {
    pub visible: bool,
    runner: WorkspaceRunner,
}
impl DockApp {
    pub(super) fn toggle_workspaces(&mut self, ctx: &egui::Context) {
        self.workspaces.visible = !self.workspaces.visible;
        ctx.request_repaint();
    }
    pub(super) fn launch_workspace(&mut self, id: uuid::Uuid, ctx: &egui::Context) {
        let Some(workspace) = self.config.workspaces.iter().find(|w| w.id == id).cloned() else {
            self.report("保存済みの作業環境がありません。".into());
            return;
        };
        if !saved_workspace_matches(&self.editor.draft, &workspace) {
            self.report("この作業環境には未保存の変更があります。「適用して保存」するか変更を破棄してから起動してください。古い設定での起動は行いません。".into());
            return;
        }
        if workspace.entries.is_empty() {
            self.report("作業環境にアプリを追加して保存してください。".into());
            return;
        }
        let repaint = ctx.clone();
        if let Err(e) = self
            .workspaces
            .runner
            .start(workspace, move || repaint.request_repaint())
        {
            self.report(e);
        }
        self.editor.page = Page::Workspaces;
    }
    pub(super) fn receive_workspace(&mut self) {
        self.editor.restore = self.workspaces.runner.poll();
    }
    pub(super) fn cancel_workspace(&mut self) {
        self.workspaces.runner.cancel();
    }
    pub(super) fn refresh_displays(&mut self) {
        match display_catalog() {
            Ok(d) => self.editor.displays = d,
            Err(e) => self.report(e.to_string()),
        }
    }
}
fn saved_workspace_matches(draft: &crate::core::Config, saved: &crate::core::Workspace) -> bool {
    draft.workspaces.iter().find(|w| w.id == saved.id) == Some(saved)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::*;
    #[test]
    fn changed_execution_settings_never_launch_the_old_saved_recipe() {
        let mut draft = Config::defaults("home".into());
        let mut saved = Workspace::new("WSL");
        saved.entries.push(WorkspaceEntry {
            executable: "code".into(),
            wsl: Some(WslSettings::default()),
            ..Default::default()
        });
        draft.workspaces.push(saved.clone());
        assert!(saved_workspace_matches(&draft, &saved));
        draft.workspaces[0].entries[0]
            .wsl
            .as_mut()
            .unwrap()
            .login_shell = true;
        assert!(!saved_workspace_matches(&draft, &saved));
        draft.workspaces[0] = saved.clone();
        draft.workspaces[0].entries[0]
            .arguments
            .push("--new-window".into());
        assert!(!saved_workspace_matches(&draft, &saved));
        saved.entries[0].wsl = None;
        saved.entries[0].terminal = Some(TerminalSettings::default());
        draft.workspaces[0] = saved.clone();
        assert!(saved_workspace_matches(&draft, &saved));
        draft.workspaces[0].entries[0]
            .terminal
            .as_mut()
            .unwrap()
            .script = "python".into();
        assert!(!saved_workspace_matches(&draft, &saved));
        draft.workspaces.clear();
        assert!(!saved_workspace_matches(&draft, &saved));
    }
}
