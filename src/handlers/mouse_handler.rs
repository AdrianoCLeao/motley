use std::sync::{Arc, Mutex};
use crate::gui::{camera::Camera, Window};

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
        fb_width: usize,
        fb_height: usize,
        sidebar_width: usize,
        camera: Arc<Mutex<Camera>>,
    ) {
        if let Some(mouse_pos) = window.get_mouse_pos() {
            let mouse_middle_down = window.is_mouse_down(minifb::MouseButton::Middle);
            let shift_pressed = window.is_key_down(minifb::Key::LeftShift) || window.is_key_down(minifb::Key::RightShift);
            let scroll_delta = window.get_scroll_wheel();

            let within_framebuffer = mouse_pos.0 >= sidebar_width as f32
                && mouse_pos.0 < (sidebar_width + fb_width) as f32
                && mouse_pos.1 >= 0.0
                && mouse_pos.1 < fb_height as f32;

            if mouse_middle_down && within_framebuffer {
                let mut last_mouse_pos = self.last_mouse_pos.lock().unwrap();
                if let Some(last_pos) = *last_mouse_pos {
                    let delta_x = mouse_pos.0 - last_pos.0;
                    let delta_y = -(mouse_pos.1 - last_pos.1);

                    let mut cam = camera.lock().unwrap();
                    if shift_pressed {
                        cam.position.x += delta_x * 0.01;
                        cam.position.y += delta_y * 0.01;
                        cam.target.x += delta_x * 0.01;
                        cam.target.y += delta_y * 0.01;
                    } else {
                        let radius = (cam.position - cam.target).length();
                        cam.position.x = cam.target.x + radius * (delta_x * 0.01).cos();
                        cam.position.z = cam.target.z + radius * (delta_y * 0.01).sin();
                    }
                }
                *last_mouse_pos = Some(mouse_pos);
            } else {
                *self.last_mouse_pos.lock().unwrap() = None;
            }

            if scroll_delta != 0.0 {
                let mut cam = camera.lock().unwrap();
                let direction = (cam.target - cam.position).normalize();
                cam.position += direction * scroll_delta * 0.5;
            }
        }
    }
}
