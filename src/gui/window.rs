use minifb::WindowOptions;
use crate::gui::{framebuffer::Framebuffer, menu::Menu};

pub struct Window {
    window: minifb::Window,
    framebuffer: Framebuffer,
    menu: Menu,
    sidebar_width: usize,
    bottom_bar_height: usize,
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize) -> Self {
        let options = WindowOptions {
            resize: true,
            ..Default::default()
        };

        let total_width = width;
        let total_height = height;

        let window = minifb::Window::new(name, total_width, total_height, options)
            .expect("Failed to create window.");

        let framebuffer = Framebuffer::new(width - 200, height);
        let menu = Menu::new();

        Window {
            window,
            framebuffer,
            menu,
            sidebar_width: 200,
            bottom_bar_height: 0,                 
        }
    }

    pub fn framebuffer_area(&self) -> (usize, usize) {
        (self.framebuffer.width(), self.framebuffer.height())
    }

    pub fn sidebar_width(&self) -> usize {
        self.sidebar_width
    }

    pub fn bottom_bar_height(&self) -> usize {
        self.bottom_bar_height
    }

    pub fn render_bottom_bar(&mut self) {
        let bottom_color = 0x333333;
        for y in self.framebuffer.height()..(self.framebuffer.height() + self.bottom_bar_height) {
            for x in 0..self.framebuffer.width() {
                self.framebuffer.set_pixel(x, y, bottom_color);
            }
        }
    }

    pub fn add_menu_item<F: 'static + FnMut()>(
        &mut self,
        label: &str,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        action: F,
    ) {
        self.menu.add_item(label, x, y, width, height, action);
    }

    pub fn process_menu_click(&mut self, mouse_x: f32, mouse_y: f32) {
        if mouse_x < self.sidebar_width as f32 {
            self.menu.handle_click_in_sidebar(mouse_x, mouse_y, self.sidebar_width);
        }
    }

    pub fn should_close(&self) -> bool {
        !self.window.is_open()
    }

    pub fn display(&mut self) {

        let total_width = self.framebuffer.width() + self.sidebar_width;
        let total_height = self.framebuffer.height() + self.bottom_bar_height;
    
        let mut full_data = vec![0x000000; total_width * total_height];
    
        for y in 0..self.framebuffer.height() {
            for x in 0..self.sidebar_width {
                full_data[x + y * total_width] = 0x141414; 
            }
        }
    
        // Renders the bottom bar, as it is unused i commented it
        /* for y in self.framebuffer.height()..total_height {
            for x in self.sidebar_width..total_width {
                full_data[x + y * total_width] = 0x141414;
            }
        } */
    
        for y in self.framebuffer.height()..total_height {
            for x in 0..self.sidebar_width {
                full_data[x + y * total_width] = 0x141414;
            }
        }

        for y in 0..self.framebuffer.height() {
            for x in 0..self.framebuffer.width() {
                let framebuffer_x = self.sidebar_width + x; 
                full_data[framebuffer_x + y * total_width] =
                    self.framebuffer.data[x + y * self.framebuffer.width()];
            }
        }

        let font_data = include_bytes!("../../fonts/Times-New-Roman/times-new-roman.ttf");
        self.menu.render_in_sidebar(&mut full_data, self.sidebar_width, total_width, total_height, font_data);

        if let Some((mouse_x, mouse_y)) = self.get_mouse_pos() {
            if self.is_mouse_down(minifb::MouseButton::Left) {
                self.process_menu_click(mouse_x, mouse_y);
            }
        }
    
        self.window
            .update_with_buffer(&full_data, total_width, total_height)
            .expect("Failed to update window buffer.");
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

    pub fn is_key_down(&self, key: minifb::Key) -> bool {
        self.window.is_key_down(key)
    }

    pub fn get_scroll_wheel(&self) -> f32 {
        if let Some((_x, y)) = self.window.get_scroll_wheel() {
            y 
        } else {
            0.0
        }
    }
}
