use crate::core::{DockEdge, MonitorRect, ScreenPoint, dock_anchor_position};
use raw_window_handle::RawWindowHandle;
use std::ffi::c_void;
use windows::{
    Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint},
        UI::{
            HiDpi::GetDpiForWindow,
            Shell::{CSIDL_PROFILE, SHGetFolderPathW, ShellExecuteW},
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

pub fn primary_monitor() -> Result<MonitorRect, PlatformError> {
    // SAFETY: value arguments and correctly sized writable MONITORINFO.
    unsafe {
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info)
            .ok()
            .map_err(|e| error("GetMonitorInfoW", e))?;
        let r = info.rcMonitor;
        Ok(MonitorRect {
            left: r.left,
            top: r.top,
            width: r.right - r.left,
            height: r.bottom - r.top,
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
        monitor: MonitorRect,
        points: (f32, f32),
    ) -> Result<(), PlatformError> {
        // SAFETY: live eframe HWND. DPI conversion is centralized here.
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        if dpi == 0 {
            return Err(PlatformError::Invalid(
                "GetDpiForWindow returned zero for the Dock HWND",
            ));
        }
        let scale = dpi as f32 / 96.0;
        let size = (
            ((points.0 * scale).round() as i32).min(monitor.width),
            ((points.1 * scale).round() as i32).min(monitor.height),
        );
        let anchor = dock_anchor_position(edge, monitor, size);
        // SAFETY: live HWND and physical dimensions. SWP_NOACTIVATE preserves focus.
        unsafe {
            SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
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
