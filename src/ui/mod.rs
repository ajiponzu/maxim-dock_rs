//! Dock UI modules; the public entry point remains `ui::DockApp`.
mod app;
mod commands;
mod dock_view;
mod icons;
mod item_import;
mod poll_wake;
mod settings;

pub use app::DockApp;
