use eframe::egui::{self, CentralPanel, Context, SidePanel, Visuals};
use crate::renderer::ThreeDViewer;
use crate::utilities::{modular_distance, positive_mod, erf_approximation, clamp};
use pollster::block_on;

pub struct MyApp {
    dark_mode: bool,
    viewer: Option<ThreeDViewer>,
    a0: f64,
    b0: f64,
    modulus: f64,
    x: f64,
    clamp_min: f64,
    clamp_max: f64,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            dark_mode: false,
            viewer: None,
            a0: 0.0,
            b0: 0.0,
            modulus: 1.0,
            x: 0.0,
            clamp_min: 0.0,
            clamp_max: 1.0,
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
            ui.label("Funções Utilitárias");

            ui.label("Modular Distance");
            ui.add(egui::Slider::new(&mut self.a0, 0.0..=10.0).text("a0"));
            ui.add(egui::Slider::new(&mut self.b0, 0.0..=10.0).text("b0"));
            ui.add(egui::Slider::new(&mut self.modulus, 1.0..=10.0).text("Modulus"));
            let distance = modular_distance(self.a0, self.b0, self.modulus);
            ui.label(format!("Resultado: {:.3}", distance));

            ui.separator();
            ui.label("Positive Mod");
            ui.add(egui::Slider::new(&mut self.x, -10.0..=10.0).text("x"));
            ui.add(egui::Slider::new(&mut self.modulus, 1.0..=10.0).text("Modulus"));
            let pos_mod = positive_mod(self.x, self.modulus);
            ui.label(format!("Resultado: {:.3}", pos_mod));

            ui.separator();
            ui.label("ERF Approximation");
            ui.add(egui::Slider::new(&mut self.x, -3.0..=3.0).text("x"));
            let erf_result = erf_approximation(self.x);
            ui.label(format!("Resultado: {:.3}", erf_result));

            ui.separator();
            ui.label("Clamp");
            ui.add(egui::Slider::new(&mut self.x, -10.0..=10.0).text("x"));
            ui.add(egui::Slider::new(&mut self.clamp_min, -10.0..=10.0).text("Min"));
            ui.add(egui::Slider::new(&mut self.clamp_max, -10.0..=10.0).text("Max"));
            let clamped = clamp(self.x, self.clamp_min, self.clamp_max);
            ui.label(format!("Resultado: {:.3}", clamped));
        });

        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Visualizador 3D");
            if let Some(viewer) = &mut self.viewer {
                viewer.render(ui);
            } else {
                ui.label("O visualizador 3D não está inicializado.");
                if ui.button("Inicializar Visualizador 3D").clicked() {
                    self.viewer = Some(block_on(ThreeDViewer::new()));
                }
            }
        });
    }
}
