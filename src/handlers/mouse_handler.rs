use std::sync::{Arc, Mutex};
use glam::{Vec2, Vec3};
use crate::camera::Camera;
use crate::gui::Window;

pub struct MouseHandler {
    last_mouse_pos: Arc<Mutex<Option<(f32, f32)>>>,
}

impl MouseHandler {
    pub fn new() -> Self {
        MouseHandler {
            last_mouse_pos: Arc::new(Mutex::new(None)),
        }
    }

    pub fn handle(
        &self,
        window: &mut Window,
        camera: Arc<Mutex<Camera>>,
    ) {
        if let Some(mouse_pos) = window.get_mouse_pos() {
            let mouse_left_down = window.is_mouse_down(minifb::MouseButton::Left);
            let mouse_middle_down = window.is_mouse_down(minifb::MouseButton::Middle);
            let shift_pressed = window.is_key_down(minifb::Key::LeftShift) || window.is_key_down(minifb::Key::RightShift);
            let scroll_delta = window.get_scroll_wheel();

            let mut last_mouse_pos = self.last_mouse_pos.lock().unwrap();
            if let Some(last_pos) = *last_mouse_pos {
                let delta = Vec2::new(mouse_pos.0 - last_pos.0, mouse_pos.1 - last_pos.1);

                if mouse_middle_down {
                    let mut cam = camera.lock().unwrap();
                    if shift_pressed {

                        let pan_speed = 0.01;
                        let right = cam.right();
                        let up = cam.up();
                        cam.pan(-delta.x * pan_speed, delta.y * pan_speed, right, up);
                    } else {
                        let rotation_speed = 0.01;
                        cam.orbit(delta.x * rotation_speed, delta.y * rotation_speed);
                    }
                }
            }

            if scroll_delta != 0.0 {
                let mut cam = camera.lock().unwrap();
                cam.zoom(-scroll_delta * 0.1); 
            }

            *last_mouse_pos = Some(mouse_pos);
        } else {
            *self.last_mouse_pos.lock().unwrap() = None;
        }
    }
}
