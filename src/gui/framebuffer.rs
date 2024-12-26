use glam::{Mat4, Vec2, Vec3, Vec4};

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

    pub fn render_3d_axes(&mut self, view_projection_matrix: &Mat4) {
        let axis_length = 5.0;

        self.draw_line_3d(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(axis_length, 0.0, 0.0),
            0xFF0000,
            view_projection_matrix,
        );

        self.draw_line_3d(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, axis_length, 0.0),
            0x00FF00,
            view_projection_matrix,
        );

        self.draw_line_3d(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, axis_length),
            0x0000FF,
            view_projection_matrix,
        );
    }

    pub fn draw_line_3d(
        &mut self,
        start: Vec3,
        end: Vec3,
        color: u32,
        view_projection_matrix: &Mat4,
    ) {
        let start_projected = *view_projection_matrix * Vec4::from((start, 1.0));
        let end_projected = *view_projection_matrix * Vec4::from((end, 1.0));

        if start_projected.w <= 0.0 || end_projected.w <= 0.0 {
            return;
        }

        let start_ndc = Vec3::new(
            start_projected.x / start_projected.w,
            start_projected.y / start_projected.w,
            start_projected.z / start_projected.w,
        );

        let end_ndc = Vec3::new(
            end_projected.x / end_projected.w,
            end_projected.y / end_projected.w,
            end_projected.z / end_projected.w,
        );

        let screen_start = Vec2::new(
            (start_ndc.x * 0.5 + 0.5) * self.width as f32,
            (1.0 - (start_ndc.y * 0.5 + 0.5)) * self.height as f32,
        );
        let screen_end = Vec2::new(
            (end_ndc.x * 0.5 + 0.5) * self.width as f32,
            (1.0 - (end_ndc.y * 0.5 + 0.5)) * self.height as f32,
        );

        self.draw_line(screen_start, screen_end, color);
    }

    pub fn draw_line(&mut self, start: Vec2, end: Vec2, color: u32) {
        let delta = end - start;
        let steps = delta.length().ceil() as usize;

        for i in 0..steps {
            let t = i as f32 / steps as f32;
            let x = (start.x + t * delta.x).round() as usize;
            let y = (start.y + t * delta.y).round() as usize;
            self.set_pixel(x, y, color);
        }
    }
}
