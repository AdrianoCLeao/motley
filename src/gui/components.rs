use crate::gui::Window;
use glam::*;
use std::sync::{Arc, Mutex};

use super::camera::Camera;

/*
Creates the menu components and adds them to the given window.
It sets up menu items like "Reset View", "Zoom In", and "Zoom Out"
and associates their functionality with the appropriate actions.
*/
pub fn setup_menu(window: &mut Window, camera: Arc<Mutex<Camera>>) {
    {
        let camera = Arc::clone(&camera);
        window.add_menu_item("Reset View", 10, 10, 100, 30, move || {
            let mut cam = camera.lock().unwrap();
            cam.set_position(Vec3::new(0.0, 0.0, 5.5));
            cam.set_target(Vec3::ZERO);
            cam.set_up(Vec3::Y);
        });
    }

    {
        let camera = Arc::clone(&camera);
        window.add_menu_item("Zoom In", 10, 50, 100, 30, move || {
            let mut cam = camera.lock().unwrap();
            let zoom_factor = 0.5;

            let position = cam.position();
            let target = cam.target();
            let direction = (target - position).normalize();

            cam.set_position(position + direction * zoom_factor);
        });
    }

    {
        let camera = Arc::clone(&camera);
        window.add_menu_item("Zoom Out", 10, 90, 100, 30, move || {
            let mut cam = camera.lock().unwrap();
            let zoom_factor = 0.5;

            let position = cam.position();
            let target = cam.target();
            let direction = (target - position).normalize();

            cam.set_position(position - direction * zoom_factor);
        });
    }
}
