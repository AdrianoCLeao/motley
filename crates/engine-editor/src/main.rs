mod app;
mod config;
mod layout;
mod viewport;

use app::EditorApp;

fn main() {
    engine_core::init_logging();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Motley Editor")
            .with_inner_size([1600.0, 900.0])
            .with_min_inner_size([800.0, 600.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    if let Err(error) = eframe::run_native(
        "Motley Editor",
        native_options,
        Box::new(|cc| Ok(Box::new(EditorApp::new(cc)))),
    ) {
        log::error!(target: "engine::editor", "Failed to start editor: {}", error);
        std::process::exit(1);
    }
}
