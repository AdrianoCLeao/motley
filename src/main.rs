mod window;
use window::Window;

fn main() {
    let window: Window = Window::new("Motley project", 512, 512);

    while !window.should_close() {
        
    }
}