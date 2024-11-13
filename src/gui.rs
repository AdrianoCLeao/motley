use eframe::egui::{ CentralPanel, Context, Visuals};
use crate::renderer::ThreeDViewer;
use pollster::block_on;

pub struct MyApp {
    dark_mode: bool,
    viewer: Option<ThreeDViewer>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            dark_mode: false,
            viewer: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if self.dark_mode {
            ctx.set_visuals(Visuals::dark());
        } else {
            ctx.set_visuals(Visuals::light());
        }

        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Visualizador 3D");
            if self.viewer.is_none() {
                self.viewer = Some(block_on(ThreeDViewer::new()));
            }

            if let Some(viewer) = &mut self.viewer {
                viewer.render(ui);
            }
        });
    }
}
