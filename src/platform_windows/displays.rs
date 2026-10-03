//! Monitor catalog uses persistent device interfaces and physical work-area rectangles.
use super::{PlatformError, error};
use crate::core::{DisplayInfo, MonitorRect};
use windows::{
    Win32::{
        Foundation::{LPARAM, RECT},
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::{EDD_GET_DEVICE_INTERFACE_NAME, MONITORINFOF_PRIMARY},
    },
    core::{BOOL, PCWSTR},
};

fn text(s: &[u16]) -> String {
    String::from_utf16_lossy(&s[..s.iter().position(|v| *v == 0).unwrap_or(s.len())])
}
fn rect(r: RECT) -> MonitorRect {
    MonitorRect {
        left: r.left,
        top: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
    }
}
struct Catalog {
    displays: Vec<DisplayInfo>,
    failure: Option<PlatformError>,
}
unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
    // SAFETY: EnumDisplayMonitors synchronously supplies our stack-owned Catalog pointer.
    let catalog = unsafe { &mut *(data.0 as *mut Catalog) };
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: MONITORINFOEXW starts with MONITORINFO and cbSize includes the extension.
    if !unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.as_bool() {
        catalog.failure = Some(error(
            "GetMonitorInfoW catalog",
            windows::core::Error::from_thread(),
        ));
        return BOOL(0);
    }
    let mut device = DISPLAY_DEVICEW {
        cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
        ..Default::default()
    };
    // SAFETY: device name is NUL terminated, device is sized and writable.
    let found = unsafe {
        EnumDisplayDevicesW(
            PCWSTR(info.szDevice.as_ptr()),
            0,
            &mut device,
            EDD_GET_DEVICE_INTERFACE_NAME,
        )
    }
    .as_bool();
    // Never persist volatile DISPLAY1 as an identity if a driver doesn't supply a device interface.
    let id = if found {
        text(&device.DeviceID)
    } else {
        String::new()
    };
    catalog.displays.push(DisplayInfo {
        id,
        name: format!("{} · {}", text(&info.szDevice), text(&device.DeviceString)),
        bounds: rect(info.monitorInfo.rcMonitor),
        work: rect(info.monitorInfo.rcWork),
        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
    });
    BOOL(1)
}
pub fn display_catalog() -> Result<Vec<DisplayInfo>, PlatformError> {
    let mut catalog = Catalog {
        displays: vec![],
        failure: None,
    };
    // SAFETY: synchronous callback; stack pointer remains valid until enumeration finishes.
    let ok = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut catalog as *mut Catalog as isize),
        )
    };
    if let Some(e) = catalog.failure {
        return Err(e);
    }
    ok.ok().map_err(|e| error("EnumDisplayMonitors", e))?;
    catalog.displays.sort_by_key(|d| !d.primary);
    if catalog.displays.is_empty() {
        return Err(PlatformError::Invalid("No connected monitors"));
    }
    Ok(catalog.displays)
}
