mod gui;

use eframe::NativeOptions;
use gui::MyApp;

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions::default();
    eframe::run_native("Minha GUI em Rust", options, Box::new(|_cc| Box::new(MyApp::default())))
}
