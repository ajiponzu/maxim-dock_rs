//! Asynchronous native picker. Only owned paths cross the UI-thread boundary.
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
};

#[derive(Clone, Copy)]
pub enum PickKind {
    Files,
    Folder,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FileSelection {
    Selected(Vec<PathBuf>),
    Cancelled,
    Failed,
}

#[derive(Default)]
pub struct FilePicker {
    result: Option<Receiver<Option<Vec<PathBuf>>>>,
}
impl FilePicker {
    /// One outstanding dialog at a time. The worker owns no HWND or UI state.
    pub fn start(&mut self, kind: PickKind, wake: impl FnOnce() + Send + 'static) -> bool {
        if self.result.is_some() {
            return false;
        }
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let paths = match kind {
                PickKind::Folder => rfd::FileDialog::new()
                    .set_title("Add folder to MaXImDock")
                    .pick_folder()
                    .map(|p| vec![p]),
                PickKind::Files => rfd::FileDialog::new()
                    .set_title("Add files to MaXImDock")
                    .pick_files(),
            };
            if sender.send(paths).is_ok() {
                wake();
            }
        });
        self.result = Some(receiver);
        true
    }

    /// None means idle or still waiting, without blocking hidden cursor polling.
    pub fn poll(&mut self) -> Option<FileSelection> {
        let receiver = self.result.as_ref()?;
        let selection = match receiver.try_recv() {
            Ok(Some(paths)) => FileSelection::Selected(paths),
            Ok(None) => FileSelection::Cancelled,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => FileSelection::Failed,
        };
        self.result = None;
        Some(selection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_picker_rejects_second_dialog_and_preserves_paths() {
        let (sender, receiver) = mpsc::channel();
        let mut picker = FilePicker {
            result: Some(receiver),
        };
        assert_eq!(picker.poll(), None);
        assert!(!picker.start(PickKind::Files, || panic!("must not open second dialog")));
        let paths = vec![PathBuf::from("日本語.txt"), PathBuf::from("folder")];
        sender.send(Some(paths.clone())).unwrap();
        assert_eq!(picker.poll(), Some(FileSelection::Selected(paths)));
        assert!(picker.result.is_none());
        assert_eq!(picker.poll(), None);
    }
    #[test]
    fn cancellation_and_disconnection_release_pending_picker() {
        let (sender, receiver) = mpsc::channel();
        let mut picker = FilePicker {
            result: Some(receiver),
        };
        sender.send(None).unwrap();
        assert_eq!(picker.poll(), Some(FileSelection::Cancelled));
        assert!(picker.result.is_none());
        let (sender, receiver) = mpsc::channel();
        picker.result = Some(receiver);
        drop(sender);
        assert_eq!(picker.poll(), Some(FileSelection::Failed));
        assert!(picker.result.is_none());
    }
}
