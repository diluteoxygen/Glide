mod app;
mod config;
mod icons;
mod panels;
mod theme;
mod tray;
mod ui;
mod widgets;
mod pipeline;

use eframe::egui;

fn main() -> eframe::Result<()> {
    // We should initialize tracing for the backend pipeline to work nicely
    tracing_subscriber::fmt::init();

    // Load persisted config (or defaults) before the window exists.
    let cfg = config::AppConfig::load();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Glide")
            .with_inner_size([660.0, 138.0])
            .with_min_inner_size([540.0, 138.0])
            // Real OS decorations: the system title bar provides the close
            // button, which is exactly what `minimize-to-tray` intercepts.
            .with_decorations(true)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "glide",
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);       // fonts + visuals, once
            let tray = tray::init(&cc.egui_ctx); // keep alive for app lifetime
            Box::new(app::GlideApp::new(cfg, tray))
        }),
    )
}
