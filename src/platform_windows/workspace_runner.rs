//! One cancellable background restore job. Cancellation never terminates applications.
#[cfg(test)]
mod native_tests;
mod placement;
use super::{display_catalog, workspace_launch, workspace_windows, workspace_wsl};
use crate::core::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
enum Event {
    Result(EntryResult),
    Done,
}
#[derive(Default)]
pub struct WorkspaceRunner {
    receiver: Option<mpsc::Receiver<Event>>,
    cancel: Arc<AtomicBool>,
    status: RestoreStatus,
}
impl WorkspaceRunner {
    pub fn start(
        &mut self,
        workspace: Workspace,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(), String> {
        if self.status.running {
            return Err("作業環境の起動処理は既に実行中です。".into());
        }
        validate_workspaces(std::slice::from_ref(&workspace)).map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = cancel.clone();
        let name = workspace.name.clone();
        thread::Builder::new()
            .name("workspace-restore".into())
            .spawn(move || {
                let dpi = workspace_windows::PhysicalDpi::enter();
                for entry in workspace.entries {
                    if cancel.load(Ordering::Acquire) {
                        let _ = tx.send(Event::Result(EntryResult {
                            label: "中止".into(),
                            message: "残りのエントリーは起動しません。".into(),
                        }));
                        break;
                    }
                    let result = if entry.wsl.as_ref().is_some_and(|wsl| !wsl.place_window) {
                        workspace_wsl::run(&entry, &cancel)
                    } else {
                        match &dpi {
                            Ok(_) => restore_entry(&entry, &cancel),
                            Err(e) => Err(e.to_string()),
                        }
                    };
                    let stop = entry.wsl.is_some() && result.is_err();
                    let message = result.unwrap_or_else(|e| e);
                    if tx
                        .send(Event::Result(EntryResult {
                            label: entry.label,
                            message,
                        }))
                        .is_err()
                    {
                        return;
                    }
                    wake();
                    if stop {
                        let _ = tx.send(Event::Result(EntryResult {
                            label: "停止".into(),
                            message: "WSL 処理が成功しなかったため、後続エントリーを起動しません。"
                                .into(),
                        }));
                        break;
                    }
                }
                let _ = tx.send(Event::Done);
                wake();
            })
            .map_err(|e| e.to_string())?;
        self.receiver = Some(rx);
        self.status = RestoreStatus {
            running: true,
            workspace_name: name,
            results: vec![],
        };
        Ok(())
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn poll(&mut self) -> RestoreStatus {
        if let Some(rx) = &self.receiver {
            loop {
                match rx.try_recv() {
                    Ok(Event::Result(r)) => self.status.results.push(r),
                    Ok(Event::Done) => {
                        self.status.running = false;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if self.status.running {
                            self.status.results.push(EntryResult {
                                label: "復元処理".into(),
                                message: "worker が終了しました。".into(),
                            });
                            self.status.running = false;
                        }
                        break;
                    }
                }
            }
        }
        if !self.status.running {
            self.receiver = None;
        }
        self.status.clone()
    }
}
impl Drop for WorkspaceRunner {
    fn drop(&mut self) {
        self.cancel();
    }
}
fn restore_entry(entry: &WorkspaceEntry, cancel: &AtomicBool) -> Result<String, String> {
    let target = if entry.window_executable.is_empty() {
        &entry.executable
    } else {
        &entry.window_executable
    };
    let target = workspace_launch::executable(target)?;
    let target = workspace_launch::path_key(&target.to_string_lossy());
    let displays = display_catalog().map_err(|e| e.to_string())?;
    let (display, fallback) =
        select_display(&entry.monitor_id, &displays).ok_or("モニターがありません。")?;
    let display_id = display.id.clone();
    let before = workspace_windows::identities().map_err(|e| e.to_string())?;
    if cancel.load(Ordering::Acquire) {
        return Err("中止しました（起動していません）。".into());
    }
    let launch_message = if entry.wsl.is_some() {
        workspace_wsl::run(entry, cancel)?
    } else {
        workspace_launch::launch(entry)?;
        String::new()
    };
    placement::wait(entry, cancel, target, display_id, before, fallback).map(|message| {
        if launch_message.is_empty() {
            message
        } else {
            format!("{launch_message} {message}")
        }
    })
}
