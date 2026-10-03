//! New top-level window enumeration and physical placement. Owned process handles use RAII.
use super::{PlatformError, error, workspace_launch::path_key};
use crate::core::{MonitorRect, Placement, WindowIdentity};
use std::collections::HashSet;
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Dwm::*,
        System::Threading::*,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
    core::{BOOL, PWSTR},
};
pub(super) struct Window {
    pub identity: WindowIdentity,
    pub executable: String,
    pub title: String,
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: exclusively owned OpenProcess handle.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
pub(super) struct PhysicalDpi(DPI_AWARENESS_CONTEXT);
impl PhysicalDpi {
    pub fn enter() -> Result<Self, PlatformError> {
        // SAFETY: thread-only context, restored on the same thread by Drop.
        let old =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        if old.0.is_null() {
            Err(error(
                "SetThreadDpiAwarenessContext",
                windows::core::Error::from_thread(),
            ))
        } else {
            Ok(Self(old))
        }
    }
}
impl Drop for PhysicalDpi {
    fn drop(&mut self) {
        // SAFETY: restores this thread's previous context.
        unsafe {
            SetThreadDpiAwarenessContext(self.0);
        }
    }
}
unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
    // SAFETY: synchronous enumeration uses the caller's Vec. All HWNDs are borrowed.
    unsafe {
        let windows = &mut *(data.0 as *mut Vec<Window>);
        if !IsWindowVisible(hwnd).as_bool()
            || GetWindow(hwnd, GW_OWNER).is_ok_and(|h| !h.0.is_null())
            || GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0
        {
            return BOOL(1);
        }
        let mut cloaked = 0u32;
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        )
        .is_err()
            || cloaked != 0
        {
            return BOOL(1);
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == std::process::id() {
            return BOOL(1);
        }
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return BOOL(1);
        };
        let handle = Handle(handle);
        let mut buffer = vec![0u16; 32768];
        let mut size = buffer.len() as u32;
        if QueryFullProcessImageNameW(
            handle.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
        .is_err()
        {
            return BOOL(1);
        }
        let executable = path_key(&String::from_utf16_lossy(&buffer[..size as usize]));
        let mut title = vec![0u16; (GetWindowTextLengthW(hwnd).max(0) as usize + 1).min(32768)];
        let len = GetWindowTextW(hwnd, &mut title).max(0) as usize;
        windows.push(Window {
            identity: WindowIdentity {
                handle: hwnd.0 as usize,
                pid,
            },
            executable,
            title: String::from_utf16_lossy(&title[..len]),
        });
    }
    BOOL(1)
}
pub(super) fn enumerate() -> Result<Vec<Window>, PlatformError> {
    let mut windows = vec![];
    // SAFETY: callback is synchronous and stack Vec remains alive.
    unsafe {
        EnumWindows(
            Some(collect),
            LPARAM(&mut windows as *mut Vec<Window> as isize),
        )
    }
    .map_err(|e| error("EnumWindows", e))?;
    Ok(windows)
}
unsafe extern "system" fn snapshot(hwnd: HWND, data: LPARAM) -> BOOL {
    // SAFETY: synchronous enumeration. Include hidden/cloaked/owned windows so none can become "new" later.
    unsafe {
        let identities = &mut *(data.0 as *mut HashSet<WindowIdentity>);
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        identities.insert(WindowIdentity {
            handle: hwnd.0 as usize,
            pid,
        });
    }
    BOOL(1)
}
pub(super) fn identities() -> Result<HashSet<WindowIdentity>, PlatformError> {
    let mut identities = HashSet::new();
    // SAFETY: stack-owned set is valid throughout synchronous callback execution.
    unsafe {
        EnumWindows(
            Some(snapshot),
            LPARAM(&mut identities as *mut HashSet<WindowIdentity> as isize),
        )
    }
    .map_err(|e| error("EnumWindows snapshot", e))?;
    Ok(identities)
}
pub(super) fn place(
    identity: WindowIdentity,
    placement: Placement,
    work: MonitorRect,
) -> Result<(), PlatformError> {
    let hwnd = HWND(identity.handle as *mut _);
    // SAFETY: validate identity immediately before using the borrowed HWND; no activation/z-order change.
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != identity.pid || !IsWindow(Some(hwnd)).as_bool() {
            return Err(PlatformError::Invalid("Window disappeared"));
        }
        ShowWindowAsync(hwnd, SW_SHOWNOACTIVATE)
            .ok()
            .map_err(|e| error("ShowWindowAsync workspace", e))?;
        let r = placement.rectangle(work);
        SetWindowPos(
            hwnd,
            None,
            r.left,
            r.top,
            r.width,
            r.height,
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_ASYNCWINDOWPOS,
        )
        .map_err(|e| error("SetWindowPos workspace", e))?;
        // A second pass handles apps which changed their frame at WM_DPICHANGED.
        SetWindowPos(
            hwnd,
            None,
            r.left,
            r.top,
            r.width,
            r.height,
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_ASYNCWINDOWPOS,
        )
        .map_err(|e| error("SetWindowPos workspace DPI", e))?;
        if placement == Placement::Maximize {
            ShowWindowAsync(hwnd, SW_MAXIMIZE)
                .ok()
                .map_err(|e| error("ShowWindowAsync maximize", e))?;
        }
    }
    Ok(())
}
pub(super) fn placed(identity: WindowIdentity, placement: Placement, work: MonitorRect) -> bool {
    let hwnd = HWND(identity.handle as *mut _);
    // SAFETY: borrowed HWND; recheck PID to exclude handle reuse, read-only geometry query.
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != identity.pid || !IsWindow(Some(hwnd)).as_bool() {
            return false;
        }
        if placement == Placement::Maximize {
            return IsZoomed(hwnd).as_bool();
        }
        let mut actual = RECT::default();
        if GetWindowRect(hwnd, &mut actual).is_err() {
            return false;
        }
        let expected = placement.rectangle(work);
        !IsZoomed(hwnd).as_bool()
            && actual.left == expected.left
            && actual.top == expected.top
            && actual.right - actual.left == expected.width
            && actual.bottom - actual.top == expected.height
    }
}
