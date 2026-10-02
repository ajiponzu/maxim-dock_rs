#![deny(unsafe_code)]

pub mod core;
#[cfg(windows)]
#[allow(unsafe_code)]
pub mod platform_windows;
#[cfg(windows)]
pub mod ui;
