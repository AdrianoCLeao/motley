use eframe::egui::{self, CentralPanel, Context, SidePanel, Visuals};
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

        SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Controles");

            if ui.button("Alternar Modo Claro/Escuro").clicked() {
                self.dark_mode = !self.dark_mode;
            }

            ui.separator();
            ui.label("Configurações do Visualizador:");
            if self.viewer.is_none() {
                if ui.button("Inicializar Visualizador 3D").clicked() {
                    self.viewer = Some(block_on(ThreeDViewer::new()));
                }
            } else {
                ui.label("Visualizador 3D Iniciado");
                if ui.button("Resetar Visualizador").clicked() {
                    self.viewer = None;
                }
            }

            ui.separator();
            ui.label("Informações:");
            ui.label(format!("Modo Escuro: {}", self.dark_mode));
        });

        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Visualizador 3D");
            if let Some(viewer) = &mut self.viewer {
                viewer.render(ui);
            } else {
                ui.label("O visualizador 3D não está inicializado.");
            }
        });
    }
}
