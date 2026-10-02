use crate::{
    core::*,
    platform_windows::{IconPixels, load_icon},
};
use eframe::egui;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[derive(Clone, PartialEq)]
struct Key {
    target: String,
    kind: TargetKind,
    source: IconSource,
}
impl From<&DockItem> for Key {
    fn from(item: &DockItem) -> Self {
        Self {
            target: item.target.clone(),
            kind: item.kind,
            source: item.icon.clone(),
        }
    }
}
struct Entry {
    key: Key,
    texture: Option<egui::TextureHandle>,
}
struct Loaded {
    id: uuid::Uuid,
    key: Key,
    pixels: Result<Option<IconPixels>, String>,
}
pub(super) struct Icons {
    entries: HashMap<uuid::Uuid, Entry>,
    requests: Option<mpsc::Sender<DockItem>>,
    results: mpsc::Receiver<Loaded>,
    stop: Arc<AtomicBool>,
}
impl Icons {
    pub fn new(ctx: egui::Context) -> Self {
        let (requests, jobs) = mpsc::channel::<DockItem>();
        let (completed, results) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        std::thread::spawn(move || {
            while let Ok(item) = jobs.recv() {
                if worker_stop.load(Ordering::Acquire) {
                    break;
                }
                let loaded = Loaded {
                    id: item.id,
                    key: Key::from(&item),
                    pixels: load_icon(&item),
                };
                if completed.send(loaded).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            entries: HashMap::new(),
            requests: Some(requests),
            results,
            stop,
        }
    }
    pub fn sync(&mut self, config: &Config) {
        self.entries
            .retain(|id, _| config.items.iter().any(|i| i.id == *id));
        for item in &config.items {
            let key = Key::from(item);
            if self
                .entries
                .get(&item.id)
                .is_some_and(|entry| entry.key == key)
            {
                continue;
            }
            self.entries.insert(item.id, Entry { key, texture: None });
            if let Some(sender) = &self.requests
                && sender.send(item.clone()).is_err()
            {
                tracing::warn!("icon worker unavailable; using fallback");
                self.requests = None;
            }
        }
    }
    pub fn reload(&mut self, config: &Config) {
        self.entries.clear();
        self.sync(config);
    }
    pub fn receive(&mut self, ctx: &egui::Context) {
        for loaded in self.results.try_iter() {
            let Some(entry) = self.entries.get_mut(&loaded.id) else {
                continue;
            };
            if entry.key != loaded.key {
                continue;
            } // ignore stale edits/deletions
            match loaded.pixels {
                Ok(Some(image)) => {
                    entry.texture = Some(ctx.load_texture(
                        format!("icon-{}", loaded.id),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [image.width, image.height],
                            &image.rgba,
                        ),
                        egui::TextureOptions::LINEAR,
                    ));
                }
                Ok(None) => {}
                Err(e) => tracing::warn!(item_id = %loaded.id, error = %e, "Shell icon fallback"),
            }
        }
    }
    pub fn texture(&self, id: uuid::Uuid) -> Option<egui::TextureId> {
        self.entries
            .get(&id)?
            .texture
            .as_ref()
            .map(egui::TextureHandle::id)
    }
}
impl Drop for Icons {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.requests.take();
        // No join on potentially blocking third-party Shell extensions. Worker owns
        // no UI/native window handles and releases COM/GDI when its current job ends.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_does_not_reload_labels_order_or_size_but_reloads_source() {
        let (sender, jobs) = mpsc::channel();
        let (_completed, results) = mpsc::channel();
        let mut icons = Icons {
            entries: HashMap::new(),
            requests: Some(sender),
            results,
            stop: Arc::new(AtomicBool::new(false)),
        };
        let mut config = Config::defaults("home".into());
        icons.sync(&config);
        assert_eq!(jobs.try_iter().count(), 3);
        config.items[0].label = "renamed".into();
        config.move_item(0, 2);
        config.dock.icon_size = 96.0;
        icons.sync(&config);
        assert_eq!(jobs.try_iter().count(), 0);
        config.items[0].icon = IconSource::Builtin {
            name: "folder".into(),
        };
        icons.sync(&config);
        assert_eq!(jobs.try_iter().count(), 1);
        config.items.clear();
        icons.sync(&config);
        assert!(icons.entries.is_empty());
    }
}
