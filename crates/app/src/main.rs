mod app;
mod panels;
mod theme;
mod widgets;
mod pipeline;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Glide")
            .with_inner_size([600.0, 96.0])
            .with_min_inner_size([600.0, 96.0])
            // Real OS decorations — no custom chrome, no transparency.
            // This design has no transparency requirement at all, so we
            // avoid the whole class of compositor/alpha bugs from the
            // earlier floating-pill attempt.
            .with_decorations(true)
            .with_transparent(false)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "glide",
        native_options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            widgets::icons::install_fonts(&cc.egui_ctx);
            Ok(Box::new(app::GlideApp::default()))
        }),
    )
}
