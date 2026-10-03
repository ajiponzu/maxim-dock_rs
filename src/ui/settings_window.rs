//! Deferred settings viewport. Owns view snapshots, never a reference to DockApp.
use super::{commands::Command, settings::Editor};
use crate::core::Config;
use eframe::egui;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};

struct Session {
    editor: Editor,
    live: Config,
    blocked: bool,
    path: PathBuf,
}
pub(super) struct SettingsWindow {
    session: Arc<Mutex<Option<Session>>>,
    sender: mpsc::Sender<(Editor, Vec<Command>)>,
    receiver: mpsc::Receiver<(Editor, Vec<Command>)>,
    frames: Arc<AtomicUsize>,
    visible: bool,
}
fn id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("settings")
}
impl Default for SettingsWindow {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            session: Arc::new(Mutex::new(None)),
            sender,
            receiver,
            frames: Arc::new(AtomicUsize::new(0)),
            visible: false,
        }
    }
}
impl SettingsWindow {
    pub fn receive(&mut self, editor: &mut Editor, commands: &mut Vec<Command>) -> usize {
        for (updated, pending) in self.receiver.try_iter() {
            *editor = updated;
            commands.extend(pending);
        }
        self.frames.swap(0, Ordering::Relaxed)
    }
    pub fn sync(
        &mut self,
        ctx: &egui::Context,
        editor: &Editor,
        live: &Config,
        blocked: bool,
        path: &Path,
    ) {
        *self.session.lock().expect("settings session") = Some(Session {
            editor: editor.clone(),
            live: live.clone(),
            blocked,
            path: path.into(),
        });
        if self.visible != editor.open {
            self.visible = editor.open;
            ctx.send_viewport_cmd_to(id(), egui::ViewportCommand::Visible(editor.open));
        }
    }
    pub fn show(&self, ctx: &egui::Context, open: bool) {
        let session = self.session.clone();
        let sender = self.sender.clone();
        let frames = self.frames.clone();
        // Keep one hidden child ready: it can be shown by logic even with a hidden root.
        ctx.show_viewport_deferred(
            id(),
            egui::ViewportBuilder::default()
                .with_title("MaXImDock — 設定")
                .with_visible(open)
                .with_inner_size([880.0, 760.0])
                .with_min_inner_size([680.0, 520.0]),
            move |ui, _| {
                let mut locked = session.lock().expect("settings session");
                let Some(state) = locked.as_mut() else {
                    return;
                };
                if !state.editor.open {
                    return;
                }
                let previous = state.editor.clone();
                let mut commands = Vec::new();
                if ui.input(|i| i.viewport().close_requested()) {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
                state
                    .editor
                    .render(ui, &state.live, state.blocked, &state.path, &mut commands);
                frames.fetch_add(1, Ordering::Relaxed);
                if state.editor != previous || !commands.is_empty() {
                    let _ = sender.send((state.editor.clone(), commands));
                    ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deferred_edits_return_to_app_before_commands_without_changing_live_config() {
        let ctx = egui::Context::default();
        let config = Config::defaults("home".into());
        let mut editor = Editor::new(config.clone());
        editor.open = true;
        let mut bridge = SettingsWindow::default();
        bridge.sync(&ctx, &editor, &config, false, Path::new("config.toml"));
        let mut edited = editor.clone();
        edited.draft.items[0].label = "edited in settings".into();
        let mut workspace = crate::core::Workspace::new("draft workspace");
        workspace.entries.push(crate::core::WorkspaceEntry {
            executable: "C:\\missing\\app.exe".into(),
            ..Default::default()
        });
        edited.draft.workspaces.push(workspace);
        bridge
            .sender
            .send((edited.clone(), vec![Command::Apply(edited.draft.clone())]))
            .unwrap();
        let mut commands = Vec::new();
        bridge.receive(&mut editor, &mut commands);
        assert_eq!(editor.draft, edited.draft);
        assert!(matches!(&commands[0], Command::Apply(staged) if *staged == edited.draft));
        assert_ne!(config, editor.draft);
        bridge.sync(&ctx, &editor, &config, false, Path::new("config.toml"));
        let state = bridge.session.lock().unwrap();
        assert_eq!(state.as_ref().unwrap().live, config);
        assert_eq!(state.as_ref().unwrap().editor.draft, edited.draft);
    }
    #[test]
    fn hidden_settings_callback_does_not_render_or_emit_commands() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        let config = Config::defaults("home".into());
        let mut editor = Editor::new(config.clone());
        let mut bridge = SettingsWindow::default();
        bridge.sync(&ctx, &editor, &config, false, Path::new("config.toml"));
        let mut output = ctx.run_ui(Default::default(), |ui| bridge.show(ui.ctx(), false));
        output.textures_delta.clear();
        let mut commands = Vec::new();
        assert_eq!(bridge.receive(&mut editor, &mut commands), 0);
        assert!(!editor.open && commands.is_empty());
        assert_eq!(editor.draft, config);
    }
}
