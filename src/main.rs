mod window;
use window::Window;

fn from_u8_rgb(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);
    (r << 16) | (g << 8) | b
}

fn main() {
    let mut window: Window = Window::new("Motley project", 512, 512);

    while !window.should_close() {
        let framebuffer = window.framebuffer();

        for x in 0..framebuffer.width(){
            for y in 0..framebuffer.height(){
                framebuffer.set_pixel(x, y, from_u8_rgb(20, 20, 20));
            }
        }
        
        window.display();
    }


}