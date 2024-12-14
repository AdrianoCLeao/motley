pub struct Window {
    window: minifb::Window
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize) -> Self{
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

        Window {
            window
        }
    }

    pub fn should_close(&self) -> bool{
        !self.window.is_open()
    }
}