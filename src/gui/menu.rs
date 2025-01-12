use rusttype::{Font, Scale, point};

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

    pub fn render_in_sidebar(
        &self,
        full_data: &mut Vec<u32>,
        sidebar_width: usize,
        total_width: usize,
        total_height: usize,
        font_data: &[u8],
    ) {
        let button_height = 40;
        let mut y_offset = 10;
    
        // Carregar fonte
        let font = Font::try_from_bytes(font_data).expect("Failed to load font");
        let scale = Scale::uniform(20.0); // Tamanho do texto
    
        for item in &self.items {
            if y_offset >= total_height {
                break;
            }
    
            let button_y_end = (y_offset + button_height).min(total_height);
            let button_x_end = (sidebar_width - 10).min(total_width);
    
            // Desenhar o fundo do botão
            for y in y_offset..button_y_end {
                for x in 10..button_x_end {
                    full_data[x + y * total_width] = 0xAAAAAA;
                }
            }
    
            // Renderizar o texto
            let label_color = 0x000000;
            let label_x = 15;
            let label_y = y_offset + 25;
    
            if label_y < total_height {
                let text_position = point(label_x as f32, label_y as f32);
    
                for glyph in font.layout(&item.label, scale, text_position) {
                    if let Some(bb) = glyph.pixel_bounding_box() {
                        glyph.draw(|gx, gy, v| {
                            let x = bb.min.x as usize + gx as usize;
                            let y = bb.min.y as usize + gy as usize;
                            if x < button_x_end && y < total_height {
                                let index = x + y * total_width;
                                full_data[index] = Self::blend_color(full_data[index], label_color, v);
                            }
                        });
                    }
                }
            }
    
            y_offset += button_height + 10;
        }
    }
    
    fn blend_color(existing: u32, new: u32, alpha: f32) -> u32 {
        let existing_r = (existing >> 16) & 0xFF;
        let existing_g = (existing >> 8) & 0xFF;
        let existing_b = existing & 0xFF;
    
        let new_r = (new >> 16) & 0xFF;
        let new_g = (new >> 8) & 0xFF;
        let new_b = new & 0xFF;
    
        let blended_r = (existing_r as f32 * (1.0 - alpha) + new_r as f32 * alpha) as u32;
        let blended_g = (existing_g as f32 * (1.0 - alpha) + new_g as f32 * alpha) as u32;
        let blended_b = (existing_b as f32 * (1.0 - alpha) + new_b as f32 * alpha) as u32;
    
        (blended_r << 16) | (blended_g << 8) | blended_b
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
