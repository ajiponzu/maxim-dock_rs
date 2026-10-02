//! UI command execution and external-event integration. No drawing here.
use super::{DockApp, apply_theme};
use crate::{
    core::*,
    platform_windows::*,
    ui::{commands::Command, poll_wake::PollWake},
};
use eframe::egui;
use std::time::Instant;

impl DockApp {
    pub(super) fn process_commands(&mut self, ctx: &egui::Context) {
        for command in std::mem::take(&mut self.pending) {
            match command {
                Command::Launch(id) => {
                    if let Some(item) = self.config.items.iter().find(|i| i.id == id) {
                        tracing::info!(id = %id, target = %masked_target(item), "launch requested");
                        let result = validate_target(&item.target, item.kind)
                            .map_err(str::to_owned)
                            .and_then(|()| {
                                WindowsShell.open_target(&item.target).map_err(|e| {
                                    tracing::error!(%e, item_id = %id, "Shell launch failed");
                                    "Launch failed. Check target, association and permissions."
                                        .into()
                                })
                            });
                        if let Err(e) = result {
                            self.report(format!("{}: {e}", item.label));
                        }
                    }
                }
                Command::OpenDropped { id, paths } => self.open_dropped(id, paths),
                Command::RejectDrop => self.report("ファイルはアプリ（.exe／.lnk）のカードにドロップしてください。登録は設定から行えます。".into()),
                Command::Reorder { id, before } => self.reorder_dock(ctx, id, before),
                Command::DiscardSettings => {
                    self.editor.draft = self.config.clone();
                    self.editor.message = Some("Unsaved edits discarded.".into());
                }
                Command::ReloadIcons => self.icons.reload(&self.config),
                Command::PickFiles | Command::PickFolder => {
                    let kind = if matches!(command, Command::PickFolder) {
                        PickKind::Folder
                    } else {
                        PickKind::Files
                    };
                    let repaint = ctx.clone();
                    if self.picker.start(kind, move || repaint.request_repaint()) {
                        self.editor.message =
                            Some("Choose files or a folder in the native dialog.".into());
                    }
                }
                Command::Apply(config) => {
                    let result = config.validate().map_err(|e| e.to_string()).and_then(|()| {
                        self.store
                            .as_mut()
                            .ok_or_else(|| "Saving is disabled in smoke mode.".to_owned())?
                            .save(&config)
                            .map_err(|e| {
                                tracing::error!(error = %e, "configuration save failed");
                                e.to_string()
                            })
                    });
                    match result {
                        Ok(()) => {
                            self.config = config;
                            self.icons.sync(&self.config);
                            apply_theme(ctx, &self.config.appearance.theme);
                            crate::ui::theme::configure(ctx, &self.config.appearance);
                            self.editor.draft = self.config.clone();
                            self.editor.message = Some("Saved and applied.".into());
                            self.wake = PollWake::new(ctx.clone(), self.config.dock.timing());
                            self.wake.set_hidden(self.state.is_hidden());
                            self.state = if self.state.is_hidden() {
                                DockVisibility::Hidden
                            } else {
                                DockVisibility::Visible {
                                    monitor_id: self.monitor.id,
                                }
                            };
                            if !self.state.is_hidden()
                                && let Err(e) = self.position()
                            {
                                tracing::error!(%e, "configuration applied but positioning failed");
                                self.report("Settings saved. Dock positioning failed; hide and show it again.".into());
                            }
                        }
                        Err(e) => self.report(e),
                    }
                }
                Command::BackUpInvalid => {
                    if let Some(store) = &mut self.store {
                        match store.back_up_invalid() {
                            Ok(path) => {
                                self.editor.message = Some(format!(
                                    "Original backed up to {}. Choose Apply and save to replace it.",
                                    path.display()
                                ))
                            }
                            Err(e) => self.report(e.to_string()),
                        }
                    }
                }
                Command::OpenConfigFolder => {
                    if let Some(parent) = self.config_path.parent() {
                        let result = std::fs::create_dir_all(parent)
                            .map_err(|e| e.to_string())
                            .and_then(|()| {
                                WindowsShell
                                    .open_target(&parent.to_string_lossy())
                                    .map_err(|e| e.to_string())
                            });
                        if let Err(e) = result {
                            tracing::error!(error = %e, "cannot open config folder");
                            self.report(
                                "Cannot open config folder. Check folder permissions.".into(),
                            );
                        }
                    }
                }
                Command::OpenSettings => {
                    self.editor.open = true;
                }
                Command::Show => {
                    if self.state.is_hidden() && self.smoke.is_none() {
                        match WindowsCursor.screen_position().and_then(cursor_monitor) {
                            Ok(Some(monitor)) => self.monitor = monitor,
                            Ok(None) => {}
                            Err(e) => tracing::warn!(%e, "cannot select monitor for explicit show"),
                        }
                    }
                    self.apply_state(
                        ctx,
                        DockVisibility::Revealing {
                            started_at: Instant::now(),
                            monitor_id: self.monitor.id,
                        },
                    );
                }
                Command::Hide => self.apply_state(ctx, DockVisibility::Hidden),
                Command::Quit => {
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close)
                }
            }
        }
    }
}

impl DockApp {
    pub(super) fn receive_tray(&mut self) {
        if let Some(tray) = &self.tray {
            self.pending
                .extend(tray.actions().into_iter().map(|action| match action {
                    TrayAction::Show => Command::Show,
                    TrayAction::Settings => Command::OpenSettings,
                    TrayAction::Quit => Command::Quit,
                }));
        }
    }

    pub(super) fn receive_dialog(&mut self) {
        let paths = match self.picker.poll() {
            None => return,
            Some(FileSelection::Selected(paths)) => paths,
            Some(FileSelection::Cancelled) => {
                self.editor.message = Some("Selection cancelled.".into());
                return;
            }
            Some(FileSelection::Failed) => {
                self.report("File dialog failed. Try again or enter a path manually.".into());
                return;
            }
        };
        let (draft, failures) = crate::ui::item_import::stage_paths(&self.editor.draft, paths);
        self.editor.draft = draft;
        self.editor.message = Some(if failures.is_empty() {
            "Added to draft. Choose Apply and save.".into()
        } else {
            failures.join("\n")
        });
    }
}

fn masked_target(item: &DockItem) -> String {
    match item.kind {
        TargetKind::Path => std::path::Path::new(&item.target)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "<path>".into()),
        TargetKind::Url => item
            .target
            .split(['?', '#'])
            .next()
            .unwrap_or("<url>")
            .to_owned(),
    }
}
