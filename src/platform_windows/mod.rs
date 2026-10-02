use crate::core::{DockEdge, MonitorRect, ScreenPoint, dock_anchor_position};
use raw_window_handle::RawWindowHandle;
use std::ffi::c_void;
use std::os::windows::ffi::OsStringExt;
mod config_store;
pub use config_store::*;
mod tray;
pub use tray::*;
mod icon;
pub use icon::*;
mod file_dialog;
mod file_drop;
pub use file_dialog::{FilePicker, FileSelection, PickKind};
pub use file_drop::open_files_with_app;
use windows::{
    Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY,
            MONITORINFO, MonitorFromPoint, MonitorFromWindow, ScreenToClient,
        },
        UI::{
            HiDpi::GetDpiForWindow,
            Shell::{CSIDL_APPDATA, CSIDL_PROFILE, SHGetFolderPathW, ShellExecuteW},
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, w},
};

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("{operation}: {source}")]
    Win32 {
        operation: &'static str,
        source: windows::core::Error,
    },
    #[error("Shell refused the target (code {0})")]
    Shell(isize),
    #[error("{0}")]
    Invalid(&'static str),
}

fn error(operation: &'static str, source: windows::core::Error) -> PlatformError {
    PlatformError::Win32 { operation, source }
}

pub trait CursorProvider {
    fn screen_position(&self) -> Result<ScreenPoint, PlatformError>;
}
pub struct WindowsCursor;
impl CursorProvider for WindowsCursor {
    fn screen_position(&self) -> Result<ScreenPoint, PlatformError> {
        let mut point = POINT::default();
        // SAFETY: valid writable POINT; the API retains no pointer.
        unsafe { GetCursorPos(&mut point) }.map_err(|e| error("GetCursorPos", e))?;
        Ok(ScreenPoint {
            x: point.x,
            y: point.y,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Monitor {
    pub id: usize,
    pub rect: MonitorRect,
}

pub fn primary_monitor() -> Result<Monitor, PlatformError> {
    // SAFETY: value arguments only; returned monitor handle is borrowed.
    monitor_info(unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) })
}

pub fn cursor_monitor(point: ScreenPoint) -> Result<Option<Monitor>, PlatformError> {
    // SAFETY: physical screen point, no pointers retained. Desktop gaps return null.
    let handle = unsafe {
        MonitorFromPoint(
            POINT {
                x: point.x,
                y: point.y,
            },
            MONITOR_DEFAULTTONULL,
        )
    };
    if handle.0.is_null() {
        return Ok(None);
    }
    monitor_info(handle).map(Some)
}

fn monitor_info(monitor: HMONITOR) -> Result<Monitor, PlatformError> {
    // SAFETY: value arguments and correctly sized writable MONITORINFO.
    unsafe {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info)
            .ok()
            .map_err(|e| error("GetMonitorInfoW", e))?;
        let r = info.rcMonitor;
        Ok(Monitor {
            id: monitor.0 as usize,
            rect: MonitorRect {
                left: r.left,
                top: r.top,
                width: r.right - r.left,
                height: r.bottom - r.top,
            },
        })
    }
}

pub fn home_directory() -> Result<String, PlatformError> {
    let mut buffer = [0u16; 260];
    // SAFETY: fixed MAX_PATH output buffer; Windows owns the profile lookup.
    unsafe { SHGetFolderPathW(None, CSIDL_PROFILE as i32, None, 0, &mut buffer) }
        .map_err(|e| error("SHGetFolderPathW", e))?;
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..end]))
}

pub fn configuration_path() -> Result<std::path::PathBuf, PlatformError> {
    let mut buffer = [0u16; 260];
    // SAFETY: fixed MAX_PATH output buffer, platform-resolved roaming AppData.
    unsafe { SHGetFolderPathW(None, CSIDL_APPDATA as i32, None, 0, &mut buffer) }
        .map_err(|e| error("SHGetFolderPathW(AppData)", e))?;
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Ok(
        std::path::PathBuf::from(std::ffi::OsString::from_wide(&buffer[..end]))
            .join("MaXImDock")
            .join("config.toml"),
    )
}

pub fn japanese_font() -> Result<Option<Vec<u8>>, std::io::Error> {
    let Some(windows_directory) = std::env::var_os("WINDIR") else {
        return Ok(None);
    };
    let directory = std::path::PathBuf::from(windows_directory).join("Fonts");
    for name in ["meiryo.ttc", "YuGothM.ttc"] {
        match std::fs::read(directory.join(name)) {
            Ok(bytes) => return Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(None)
}

pub trait ShellLauncher {
    fn open_target(&self, target: &str) -> Result<(), PlatformError>;
}
pub struct WindowsShell;
impl ShellLauncher for WindowsShell {
    fn open_target(&self, target: &str) -> Result<(), PlatformError> {
        if target.is_empty() || target.contains('\0') {
            return Err(PlatformError::Invalid("empty target or embedded NUL"));
        }
        let target: Vec<u16> = target.encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL terminated strings live throughout the call; no retained pointers.
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(target.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        let code = result.0 as isize;
        if code <= 32 {
            Err(PlatformError::Shell(code))
        } else {
            Ok(())
        }
    }
}

/// UI-thread-only adapter. The HWND is owned by eframe and outlives the App.
pub struct DockWindow {
    hwnd: HWND,
}
impl DockWindow {
    pub fn from_handle(handle: RawWindowHandle) -> Result<Self, PlatformError> {
        match handle {
            RawWindowHandle::Win32(handle) => Ok(Self {
                hwnd: HWND(handle.hwnd.get() as *mut c_void),
            }),
            _ => Err(PlatformError::Invalid("expected Windows HWND")),
        }
    }
    pub fn position(
        &self,
        edge: DockEdge,
        target: Monitor,
        points: (f32, f32),
        always_on_top: bool,
    ) -> Result<(), PlatformError> {
        // Refresh the pinned handle to detect disconnected/reconfigured displays.
        let monitor = monitor_info(HMONITOR(target.id as *mut c_void))?.rect;
        // SAFETY: live HWND, borrowed HMONITOR. Move while hidden before querying
        // the window DPI; querying the old monitor's DPI would size incorrectly.
        let current = unsafe { MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONULL) };
        if current.0 as usize != target.id {
            unsafe {
                SetWindowPos(
                    self.hwnd,
                    None,
                    monitor.left,
                    monitor.top,
                    1,
                    1,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )
            }
            .map_err(|e| error("SetWindowPos(target monitor)", e))?;
        }
        // SAFETY: live eframe HWND. DPI conversion is centralized here.
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        if dpi == 0 {
            return Err(PlatformError::Invalid(
                "GetDpiForWindow returned zero for the Dock HWND",
            ));
        }
        let size = physical_size(points, dpi, monitor);
        let anchor = dock_anchor_position(edge, monitor, size);
        // SAFETY: live HWND and physical dimensions. SWP_NOACTIVATE preserves focus.
        unsafe {
            SetWindowPos(
                self.hwnd,
                Some(if always_on_top {
                    HWND_TOPMOST
                } else {
                    HWND_NOTOPMOST
                }),
                anchor.x,
                anchor.y,
                size.0,
                size.1,
                SWP_NOACTIVATE,
            )
        }
        .map_err(|e| error("SetWindowPos", e))?;
        tracing::debug!(?monitor, ?anchor, ?size, dpi, "Dock layout");
        Ok(())
    }
    pub fn dpi(&self) -> u32 {
        // SAFETY: live eframe HWND, no pointers.
        unsafe { GetDpiForWindow(self.hwnd) }
    }
    /// External OLE drags need not deliver egui PointerMoved events.
    pub fn cursor_client_points(&self) -> Result<(f32, f32), PlatformError> {
        let mut point = POINT::default();
        // SAFETY: live HWND and writable POINT. Conversion stays in the adapter.
        unsafe {
            GetCursorPos(&mut point).map_err(|e| error("GetCursorPos", e))?;
            ScreenToClient(self.hwnd, &mut point)
                .ok()
                .map_err(|e| error("ScreenToClient", e))?;
        }
        let scale = self.dpi() as f32 / 96.0;
        if scale <= 0.0 {
            return Err(PlatformError::Invalid("invalid window DPI"));
        }
        Ok((point.x as f32 / scale, point.y as f32 / scale))
    }
    pub fn show_without_activation(&self) {
        // SAFETY: live HWND. Return value is previous visibility, not an error code.
        unsafe {
            let _previous_visibility = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
    }
    pub fn is_visible(&self) -> bool {
        // SAFETY: HWND is alive throughout the App lifetime.
        unsafe { IsWindowVisible(self.hwnd).as_bool() }
    }
    pub fn hide(&self) {
        // SAFETY: live HWND; result describes previous visibility, not failure.
        unsafe {
            let _previous_visibility = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
    pub fn bounds(&self) -> Result<MonitorRect, PlatformError> {
        let mut r = RECT::default();
        // SAFETY: live eframe HWND and valid writable RECT.
        unsafe { GetWindowRect(self.hwnd, &mut r) }.map_err(|e| error("GetWindowRect", e))?;
        Ok(MonitorRect {
            left: r.left,
            top: r.top,
            width: r.right - r.left,
            height: r.bottom - r.top,
        })
    }
}

fn physical_size(points: (f32, f32), dpi: u32, monitor: MonitorRect) -> (i32, i32) {
    let scale = dpi as f32 / 96.0;
    (
        ((points.0 * scale).round() as i32).clamp(1, monitor.width),
        ((points.1 * scale).round() as i32).clamp(1, monitor.height),
    )
}

#[cfg(test)]
mod monitor_tests {
    use super::*;
    #[test]
    fn dpi_sizes_and_negative_monitor_anchors_use_physical_pixels() {
        let monitor = MonitorRect {
            left: -1920,
            top: -1080,
            width: 1920,
            height: 1080,
        };
        assert_eq!(physical_size((300.0, 100.0), 96, monitor), (300, 100));
        let size = physical_size((300.0, 100.0), 144, monitor);
        assert_eq!(size, (450, 150));
        assert_eq!(
            dock_anchor_position(DockEdge::Top, monitor, size),
            ScreenPoint { x: -1185, y: -1080 }
        );
        assert_eq!(
            physical_size((10000.0, 10000.0), 144, monitor),
            (1920, 1080)
        );
    }
}
