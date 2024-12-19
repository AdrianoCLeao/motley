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

    pub fn render(&self, framebuffer: &mut Framebuffer) {
        for item in &self.items {
            // Draw menu item background
            for y in item.y..item.y + item.height {
                for x in item.x..item.x + item.width {
                    framebuffer.set_pixel(x, y, 0xAAAAAA); // Light gray
                }
            }

            // Draw menu item label (simple placeholder for text rendering)
            let label_color = 0x000000; // Black
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

pub struct Window {
    window: minifb::Window,
    framebuffer: Framebuffer,
    menu: Menu,
}

/*
The `Framebuffer` struct stores pixel data and dimensions, serving as an off-screen
buffer for rendering graphics before displaying them in a window.
*/
pub struct Framebuffer {
    data: Vec<u32>,
    width: usize,
    height: usize
}

/*
The `Window` implementation provides methods to create, render, and manage the
window, including interaction with the framebuffer for graphical updates.
*/
impl Window {
    /*
    Creates a new `Window` with a specified title, width, and height. Initializes
    the framebuffer with matching dimensions for rendering pixel data.
    */
    pub fn new(name: &str, width: usize, height: usize) -> Self {
        let options = minifb::WindowOptions {
            resize: true,
            ..Default::default()
        };

        let window = minifb::Window::new(
            name,
            width,
            height,
            options
        ).expect("Failed to create window.");

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

    /*
    Renders the framebuffer content to the window and handles resizing by reinitializing
    the framebuffer if the window dimensions have changed.
    */
    pub fn display(&mut self) {
        self.window.update_with_buffer(
            &self.framebuffer.data,
            self.framebuffer.width(),
            self.framebuffer.height()
        ).expect("Failed to update window buffer.");

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

/*
The `Framebuffer` implementation provides methods for managing pixel data, including
initialization, pixel manipulation, and clearing the buffer.
*/
impl Framebuffer {
    /*
    Creates a new `Framebuffer` with specified dimensions and initializes all pixels
    to zero, representing a blank buffer.
    */
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            data: vec![0; width * height],
            width,
            height
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /*
    Sets the value of a specific pixel in the buffer, identified by its (x, y)
    coordinates. This allows direct pixel-level updates.
    */
    pub fn set_pixel(&mut self, x: usize, y: usize, value: u32) {
        self.data[x + y * self.width] = value;
    }

    pub fn set_pixel_f32(&mut self, x: usize, y: usize, value: f32) {
        self.data[y * self.width + x] = (value * u32::MAX as f32) as u32;
    }

    pub fn get_pixel_f32(&mut self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x] as f32 / u32::MAX as f32
    }

    /*
    Clears the framebuffer by filling it with a single color value, effectively
    resetting its content for new rendering operations.
    */
    pub fn clear(&mut self, value: u32) {
        for i in 0..self.data.len() {
            self.data[i] = value;
        }
    }
}