use crate::gui::{framebuffer::Framebuffer, menu::Menu};

pub struct Window {
    window: minifb::Window,
    framebuffer: Framebuffer,
    menu: Menu,
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize) -> Self {
        let options = minifb::WindowOptions {
            resize: true,
            ..Default::default()
        };

        let window = minifb::Window::new(name, width, height, options).expect("Failed to create window.");
        let framebuffer = Framebuffer::new(width, height);
        let menu = Menu::new();

        Window {
            window,
            framebuffer,
            menu,
        }
    }

    pub fn add_menu_item<F: 'static + FnMut()>(&mut self, label: &str, x: usize, y: usize, width: usize, height: usize, action: F) {
        self.menu.add_item(label, x, y, width, height, action);
    }

    pub fn process_menu_click(&mut self, mouse_x: f32, mouse_y: f32) {
        self.menu.handle_click(mouse_x, mouse_y);
    }

    pub fn render_menu(&mut self) {
        self.menu.render(&mut self.framebuffer);
    }

    pub fn should_close(&self) -> bool {
        !self.window.is_open()
    }

    pub fn display(&mut self) {
        self.window
            .update_with_buffer(&self.framebuffer.data, self.framebuffer.width(), self.framebuffer.height())
            .expect("Failed to update window buffer.");

        let (width, height) = self.window.get_size();
        if width != self.framebuffer.width() || height != self.framebuffer.height() {
            self.framebuffer = Framebuffer::new(width, height);
        }
    }

    pub fn framebuffer(&mut self) -> &mut Framebuffer {
        &mut self.framebuffer
    }

    pub fn get_mouse_pos(&self) -> Option<(f32, f32)> {
        self.window.get_mouse_pos(minifb::MouseMode::Clamp)
    }

    pub fn is_mouse_down(&self, button: minifb::MouseButton) -> bool {
        self.window.get_mouse_down(button)
    }
}
