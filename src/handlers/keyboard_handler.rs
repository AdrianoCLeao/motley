use crate::gui::Window;

pub struct KeyboardHandler;

impl KeyboardHandler {
    pub fn new() -> Self {
        KeyboardHandler
    }

    pub fn handle(&self, _window: &mut Window) {
        // Just a mock for the keyboard handlers
    }
}
