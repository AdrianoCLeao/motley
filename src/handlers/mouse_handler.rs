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
    ) {
        if let Some(mouse_pos) = window.get_mouse_pos() {
            let mouse_left_down = window.is_mouse_down(minifb::MouseButton::Left);
            let within_framebuffer = mouse_pos.0 >= sidebar_width as f32
                && mouse_pos.0 < (sidebar_width + fb_width) as f32
                && mouse_pos.1 >= 0.0
                && mouse_pos.1 < fb_height as f32;

            if mouse_left_down {
                if within_framebuffer {
                    let mut last_mouse_pos = self.last_mouse_pos.lock().unwrap();
                    if let Some(last_pos) = *last_mouse_pos {
                        let delta = Vec2::new(last_pos.0 - mouse_pos.0, mouse_pos.1 - last_pos.1);
                        *rotation.lock().unwrap() += Vec2::new(delta.x, delta.y) * 0.01;
                    }
                    *last_mouse_pos = Some(mouse_pos);
                } else if mouse_pos.0 < sidebar_width as f32 {
                    window.process_menu_click(mouse_pos.0, mouse_pos.1);
                }
            } else {
                *self.last_mouse_pos.lock().unwrap() = None;
            }
        }
    }
}
