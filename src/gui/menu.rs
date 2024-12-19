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
}
