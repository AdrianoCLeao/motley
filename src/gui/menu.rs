pub struct MenuItem {
    label: String,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    action: Box<dyn FnMut()>,
}

impl MenuItem {
    pub fn new<F: 'static + FnMut()>(label: &str, x: usize, y: usize, width: usize, height: usize, action: F) -> Self {
        MenuItem {
            label: label.to_string(),
            x,
            y,
            width,
            height,
            action: Box::new(action),
        }
    }

    pub fn is_hovered(&self, mouse_x: f32, mouse_y: f32) -> bool {
        let mouse_x = mouse_x as usize;
        let mouse_y = mouse_y as usize;
        mouse_x >= self.x && mouse_x <= self.x + self.width && mouse_y >= self.y && mouse_y <= self.y + self.height
    }

    pub fn execute(&mut self) {
        (self.action)();
    }
}

pub struct Menu {
    items: Vec<MenuItem>,
}

impl Menu {
    pub fn new() -> Self {
        Menu { items: Vec::new() }
    }

    pub fn add_item<F: 'static + FnMut()>(&mut self, label: &str, x: usize, y: usize, width: usize, height: usize, action: F) {
        self.items.push(MenuItem::new(label, x, y, width, height, action));
    }

    pub fn render(&self, framebuffer: &mut crate::gui::framebuffer::Framebuffer) {
        for item in &self.items {
            for y in item.y..item.y + item.height {
                for x in item.x..item.x + item.width {
                    framebuffer.set_pixel(x, y, 0xAAAAAA);
                }
            }

            let label_color = 0x000000;
            let label_x = item.x + 5;
            let label_y = item.y + 10;

            for (i, _) in item.label.chars().enumerate() {
                let offset_x = label_x + i * 6;
                framebuffer.set_pixel(offset_x, label_y, label_color);
            }
        }
    }

    pub fn handle_click(&mut self, mouse_x: f32, mouse_y: f32) {
        for item in &mut self.items {
            if item.is_hovered(mouse_x, mouse_y) {
                item.execute();
            }
        }
    }

    pub fn render_in_sidebar(&self, framebuffer: &mut crate::gui::framebuffer::Framebuffer, sidebar_width: usize) {
        let button_height = 40;
        let mut y_offset = 10;

        for item in &self.items {
            for y in y_offset..(y_offset + button_height) {
                for x in 10..(sidebar_width - 10) {
                    framebuffer.set_pixel(x, y, 0xAAAAAA);
                }
            }

            let label_color = 0x000000;
            let label_x = 15;
            let label_y = y_offset + 25;

            for (i, _) in item.label.chars().enumerate() {
                let offset_x = label_x + i * 6;
                framebuffer.set_pixel(offset_x, label_y, label_color);
            }

            y_offset += button_height + 10; 
        }
    }

    pub fn handle_click_in_sidebar(&mut self, mouse_x: f32, mouse_y: f32, sidebar_width: usize) {
        let button_height = 40;
        let mut y_offset = 10;

        if mouse_x < 10.0 || mouse_x > sidebar_width as f32 - 10.0 {
            return;
        }

        for item in &mut self.items {
            if mouse_y >= y_offset as f32 && mouse_y <= (y_offset + button_height) as f32 {
                item.execute();
                return;
            }
            y_offset += button_height + 10;
        }
    }
}
