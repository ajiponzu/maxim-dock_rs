//! Hidden-only repaint wake and thread shutdown; no native window operations.
use crate::core::Timing;
use eframe::egui;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

pub(super) struct PollWake {
    hidden: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl PollWake {
    pub(super) fn new(ctx: egui::Context, timing: Timing) -> Self {
        let hidden = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let h = hidden.clone();
        let s = stop.clone();
        let worker = std::thread::spawn(move || {
            while !s.load(Ordering::Acquire) {
                std::thread::park_timeout(timing.cursor_poll_interval);
                if h.load(Ordering::Acquire) && !s.load(Ordering::Acquire) {
                    ctx.request_repaint();
                }
            }
        });
        Self {
            hidden,
            stop,
            worker: Some(worker),
        }
    }
    pub(super) fn set_hidden(&self, hidden: bool) {
        self.hidden.store(hidden, Ordering::Release);
    }
}
impl Drop for PollWake {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            if worker.join().is_err() {
                tracing::error!("poll wake worker panicked");
            }
        }
    }
}
