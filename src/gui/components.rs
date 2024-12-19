use std::sync::{Arc, Mutex};
use glam::*;
use crate::gui::Window;

/*
Creates the menu components and adds them to the given window.
It sets up menu items like "Reset View", "Zoom In", and "Zoom Out"
and associates their functionality with the appropriate actions.
*/
pub fn setup_menu(window: &mut Window, rotation: Arc<Mutex<Vec2>>, zoom: Arc<Mutex<f32>>) {
    {
        let rotation = Arc::clone(&rotation);
        let zoom = Arc::clone(&zoom);
        window.add_menu_item("Reset View", 10, 10, 100, 30, move || {
            *rotation.lock().unwrap() = Vec2::ZERO;
            *zoom.lock().unwrap() = 2.5;
        });
    }

    {
        let zoom = Arc::clone(&zoom);
        window.add_menu_item("Zoom In", 120, 10, 100, 30, move || {
            let mut zoom_guard = zoom.lock().unwrap();
            *zoom_guard -= 0.5;
        });
    }

    {
        let zoom = Arc::clone(&zoom);
        window.add_menu_item("Zoom Out", 230, 10, 100, 30, move || {
            let mut zoom_guard = zoom.lock().unwrap();
            *zoom_guard += 0.5;
        });
    }
}
