use crate::gui::Window;
use rfd::FileDialog;
use std::sync::{Arc, Mutex};

use super::camera::Camera;

/*
Creates the menu components and adds them to the given window.
It sets up menu items like "Reset View", "Zoom In", "Zoom Out",
and a new "Upload" button for selecting .glb files.
*/
pub fn setup_menu(window: &mut Window, camera: Arc<Mutex<Camera>>) {
    window.add_menu_item("Upload", 10, 130, 100, 30, move || {
        if let Some(path) = FileDialog::new()
            .add_filter("GLB Files", &["glb"])
            .pick_file()
        {
            println!("Selected file: {}", path.display());
        } else {
            println!("No file selected.");
        }
    });
    {
        window.add_menu_item("Print 1", 10, 10, 100, 30, move || {
            println!("1");
        });
    }

    {
        window.add_menu_item("Print 2", 10, 50, 100, 30, move || {
            println!("2");
        });
    }

    {
        window.add_menu_item("Print 3", 10, 90, 100, 30, move || {
            println!("3");
        });
    }
}
