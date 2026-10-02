#![deny(unsafe_code)]

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use eframe::egui;
    use maxim_dock_rs::{core::DockEdge, ui::DockApp};
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "maxim_dock_rs=info".into()),
        )
        .init();
    let mut edge = DockEdge::Bottom;
    let mut smoke = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--edge" => edge = args.next().ok_or("--edge requires a value")?.parse()?,
            "--smoke-test" => smoke = true,
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    tracing::info!(?edge, "starting MaXImDock Phase 1");
    let size = if edge.is_horizontal() {
        [310.0, 100.0]
    } else {
        [100.0, 310.0]
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(false)
            .with_resizable(false)
            .with_active(false),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "MaXImDock v2",
        options,
        Box::new(move |cc| Ok(Box::new(DockApp::new(cc, edge, smoke)?))),
    )?;
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("MaXImDock v2 requires Windows 10 or later.");
}
