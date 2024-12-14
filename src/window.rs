/*
The `Window` struct encapsulates a graphical window and an associated framebuffer,
providing functionality for rendering and managing window interactions.
*/
pub struct Window {
    window: minifb::Window,
    framebuffer: Framebuffer
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

        Window {
            window,
            framebuffer
        }
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