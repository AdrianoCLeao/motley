use std::sync::{Arc, Mutex};
use glam::Vec2;
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
        fb_width: usize,
        fb_height: usize,
        sidebar_width: usize,
        rotation: Arc<Mutex<Vec2>>,
        zoom: Arc<Mutex<f32>>,
        pan_offset: Arc<Mutex<Vec2>>,
    ) {
        if let Some(mouse_pos) = window.get_mouse_pos() {
            let mouse_left_down = window.is_mouse_down(minifb::MouseButton::Left);
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
                    let delta = Vec2::new(mouse_pos.0 - last_pos.0, last_pos.1 - mouse_pos.1);

                    if shift_pressed {
                        let mut pan = pan_offset.lock().unwrap();
                        *pan += delta * 0.01; 
                    } else {
                        let mut rot = rotation.lock().unwrap();
                        *rot += Vec2::new(delta.x, delta.y) * 0.01; 
                    }
                }
                *last_mouse_pos = Some(mouse_pos);
            } else if mouse_left_down && within_framebuffer {
                window.process_menu_click(mouse_pos.0, mouse_pos.1);
            } else {
                *self.last_mouse_pos.lock().unwrap() = None;
            }
            if scroll_delta != 0.0 {
                let mut current_zoom = zoom.lock().unwrap();
                *current_zoom -= scroll_delta * 0.1; 
                *current_zoom = current_zoom.clamp(1.0, 10.0); 
            }
        }
    }
}
