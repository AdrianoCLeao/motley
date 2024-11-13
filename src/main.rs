mod gui;
mod renderer;
mod utilities;

use eframe::NativeOptions;
use gui::MyApp;

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions::default();
    eframe::run_native("Visualizador 3D em Rust", options, Box::new(|_cc| Box::new(MyApp::default())))
}
