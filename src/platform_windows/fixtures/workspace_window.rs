//! Standalone controlled GUI receiver, compiled by the ignored native test with rustc.
#![windows_subsystem = "windows"]
use std::{ffi::c_void, ptr::null_mut, time::Duration};
type Hwnd = *mut c_void;
#[repr(C)]
struct Class {
    style: u32,
    proc: Option<unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize>,
    class_extra: i32,
    window_extra: i32,
    instance: Hwnd,
    icon: Hwnd,
    cursor: Hwnd,
    brush: Hwnd,
    menu: *const u16,
    name: *const u16,
}
#[repr(C)]
#[derive(Default)]
struct Message {
    hwnd: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    x: i32,
    y: i32,
    private: u32,
}
#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(class: *const Class) -> u16;
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: Hwnd,
        menu: Hwnd,
        instance: Hwnd,
        param: Hwnd,
    ) -> Hwnd;
    fn DefWindowProcW(hwnd: Hwnd, message: u32, wparam: usize, lparam: isize) -> isize;
    fn GetMessageW(message: *mut Message, hwnd: Hwnd, min: u32, max: u32) -> i32;
    fn TranslateMessage(message: *const Message) -> i32;
    fn DispatchMessageW(message: *const Message) -> isize;
    fn PostQuitMessage(code: i32);
    fn SetProcessDpiAwarenessContext(context: Hwnd) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Hwnd;
}
unsafe extern "system" fn proc(hwnd: Hwnd, msg: u32, wp: usize, lp: isize) -> isize {
    // SAFETY: standard WNDPROC arguments forwarded to DefWindowProc.
    unsafe {
        if msg == 2 {
            PostQuitMessage(0);
            0
        } else {
            DefWindowProcW(hwnd, msg, wp, lp)
        }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    std::fs::write(
        &args[0],
        format!(
            "{}\n{:?}\n{}",
            std::process::id(),
            args,
            std::env::current_dir().unwrap().display()
        ),
    )
    .unwrap();
    // Guaranteed bounded lifetime even if the test panics or is interrupted.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(25));
        std::process::exit(0);
    });
    if args.iter().any(|a| a == "--no-window") {
        std::thread::sleep(Duration::from_secs(24));
        return;
    }
    let class = wide("MaXIMDockWorkspaceTest");
    let title = wide(&args[1]);
    // SAFETY: all FFI structures have C layout, strings outlive the message loop; fixture owns every created window.
    unsafe {
        assert_ne!(SetProcessDpiAwarenessContext(-4isize as Hwnd), 0);
        let instance = GetModuleHandleW(std::ptr::null());
        let c = Class {
            style: 0,
            proc: Some(proc),
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: null_mut(),
            cursor: null_mut(),
            brush: null_mut(),
            menu: std::ptr::null(),
            name: class.as_ptr(),
        };
        assert_ne!(RegisterClassW(&c), 0);
        let count = if args.iter().any(|a| a == "--two-windows") {
            2
        } else {
            1
        };
        for i in 0..count {
            assert!(
                !CreateWindowExW(
                    0,
                    class.as_ptr(),
                    title.as_ptr(),
                    0x10CF0000,
                    140 + i * 30,
                    160,
                    400,
                    300,
                    null_mut(),
                    null_mut(),
                    instance,
                    null_mut()
                )
                .is_null()
            );
        }
        let mut message = Message::default();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
