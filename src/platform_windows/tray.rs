use std::sync::{
    Arc, Mutex, Once, Weak,
    mpsc::{self, Receiver},
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Show,
    Settings,
    Quit,
}

type Dispatch = dyn Fn(MenuEvent) + Send + Sync;
static REGISTER: Once = Once::new();
static CURRENT: Mutex<Option<Weak<Dispatch>>> = Mutex::new(None);
fn route_menu_event(event: MenuEvent) {
    let dispatch = match CURRENT.lock() {
        Ok(current) => current.as_ref().and_then(Weak::upgrade),
        Err(_) => {
            tracing::error!("tray dispatcher lock poisoned");
            None
        }
    };
    if let Some(dispatch) = dispatch {
        dispatch(event);
    }
}

/// Created and dropped on the eframe UI thread. Its native hidden window uses
/// winit's existing Win32 message pump; callbacks only send owned commands.
pub struct WindowsTray {
    _icon: TrayIcon,
    events: Receiver<TrayAction>,
    ids: [MenuId; 3],
    _dispatch: Arc<Dispatch>,
}
impl WindowsTray {
    pub fn new(
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        let show = MenuItem::new("Show Dock", true, None);
        let settings = MenuItem::new("Settings", true, None);
        let quit = MenuItem::new("Quit", true, None);
        menu.append_items(&[&show, &settings, &quit])?;
        let mut rgba = vec![0; 32 * 32 * 4];
        for y in 4..28 {
            for x in 4..28 {
                let i = (y * 32 + x) * 4;
                rgba[i..i + 4].copy_from_slice(if (10..22).contains(&x) && (10..22).contains(&y) {
                    &[230, 240, 255, 255]
                } else {
                    &[45, 100, 180, 255]
                });
            }
        }
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("MaXImDock")
            .with_icon(Icon::from_rgba(rgba, 32, 32)?)
            .build()?;
        let ids = [show.id().clone(), settings.id().clone(), quit.id().clone()];
        let dispatch_ids = ids.clone();
        let (sender, events) = mpsc::channel();
        let dispatch: Arc<Dispatch> = Arc::new(move |event: MenuEvent| {
            let action = if event.id == dispatch_ids[0] {
                Some(TrayAction::Show)
            } else if event.id == dispatch_ids[1] {
                Some(TrayAction::Settings)
            } else if event.id == dispatch_ids[2] {
                Some(TrayAction::Quit)
            } else {
                None
            };
            if let Some(action) = action
                && sender.send(action).is_ok()
            {
                wake();
            }
        });
        // muda 0.19 uses OnceCell: set_event_handler(None) cannot unregister.
        // Keep only a weak relay globally, so App/egui are not retained at exit.
        *CURRENT
            .lock()
            .map_err(|_| "tray dispatcher lock poisoned")? = Some(Arc::downgrade(&dispatch));
        REGISTER.call_once(|| MenuEvent::set_event_handler(Some(route_menu_event)));
        Ok(Self {
            _icon: icon,
            events,
            ids,
            _dispatch: dispatch,
        })
    }
    pub fn actions(&self) -> Vec<TrayAction> {
        self.events.try_iter().collect()
    }
    /// Synthetic MenuEvent, not a native menu click. Only called by --smoke-test.
    pub fn inject_for_smoke(&self, action: TrayAction) {
        let index = match action {
            TrayAction::Show => 0,
            TrayAction::Settings => 1,
            TrayAction::Quit => 2,
        };
        route_menu_event(MenuEvent {
            id: self.ids[index].clone(),
        });
    }
}
impl Drop for WindowsTray {
    fn drop(&mut self) {
        if let Ok(mut current) = CURRENT.lock() {
            *current = None;
        }
    }
}
