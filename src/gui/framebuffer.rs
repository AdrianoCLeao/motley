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
        if x < self.width && y < self.height {
            self.data[x + y * self.width] = value;
        }
    }

    pub fn set_pixel_f32(&mut self, x: usize, y: usize, value: f32) {
        if x < self.width && y < self.height {
            self.data[x + y * self.width] = (value * u32::MAX as f32) as u32;
        }
    }

    pub fn get_pixel_f32(&mut self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x] as f32 / u32::MAX as f32
    }

    pub fn clear(&mut self, value: u32) {
        self.data.fill(value);
    }

    pub fn render_orientation_cube(&mut self, cube_size: usize) {
        let offset_x = self.width - cube_size - 10; 
        let offset_y = self.height - cube_size - 10;

        for y in offset_y..(offset_y + cube_size) {
            for x in offset_x..(offset_x + cube_size) {
                self.set_pixel(x, y, 0x333333); 
            }
        }

        let center_x = offset_x + cube_size / 2;
        let center_y = offset_y + cube_size / 2;

        for x in center_x..(center_x + cube_size / 4).min(self.width) {
            self.set_pixel(x, center_y, 0xFF0000);
        }

        for y in (center_y - cube_size / 4).max(0)..center_y {
            self.set_pixel(center_x, y, 0x00FF00);
        }

        let z_length = cube_size / 4;
        for i in 0..z_length {
            let z_x = (center_x + i).min(self.width - 1);
            let z_y = (center_y + i).min(self.height - 1);
            self.set_pixel(z_x, z_y, 0x0000FF);
        }
    }
}
