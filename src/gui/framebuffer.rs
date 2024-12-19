pub struct Framebuffer {
    pub data: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            data: vec![0; width * height],
            width,
            height,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, value: u32) {
        self.data[x + y * self.width] = value;
    }

    pub fn set_pixel_f32(&mut self, x: usize, y: usize, value: f32) {
        self.data[y * self.width + x] = (value * u32::MAX as f32) as u32;
    }

    pub fn get_pixel_f32(&mut self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x] as f32 / u32::MAX as f32
    }

    pub fn clear(&mut self, value: u32) {
        self.data.fill(value);
    }
}
