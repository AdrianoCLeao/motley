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
        let axis_length = 15.0;
    
        self.draw_line_3d(
            Vec3::new(-axis_length, 0.0, 0.0),
            Vec3::new(axis_length, 0.0, 0.0),
            0xA63737,
            view_projection_matrix,
            false, 
        );
    
        self.draw_line_3d(
            Vec3::new(0.0, 0.0, -axis_length),
            Vec3::new(0.0, 0.0, axis_length),
            0x468E2C,
            view_projection_matrix,
            false, 
        );
    
        self.render_grid(view_projection_matrix, 15.0);
    }
    
    pub fn render_grid(&mut self, view_projection_matrix: &Mat4, size: f32) {
        for x in (-size as i32)..=(size as i32) {
            if x == 0 {
                continue;
            }
            let start = Vec3::new(x as f32, 0.0, -size);
            let end = Vec3::new(x as f32, 0.0, size);
            self.draw_line_3d(start, end, 0x444444, view_projection_matrix, true);
        }
    
        for z in (-size as i32)..=(size as i32) {
            if z == 0 {
                continue;
            }
            let start = Vec3::new(-size, 0.0, z as f32);
            let end = Vec3::new(size, 0.0, z as f32);
            self.draw_line_3d(start, end, 0x444444, view_projection_matrix, true);
        }
    }   

    pub fn draw_line_3d(
        &mut self,
        start: Vec3,
        end: Vec3,
        color: u32,
        view_projection_matrix: &Mat4,
        apply_gradient: bool,
    ) {
        let start_projected = *view_projection_matrix * Vec4::from((start, 1.0));
        let end_projected = *view_projection_matrix * Vec4::from((end, 1.0));
    
        if start_projected.w <= 0.0 && end_projected.w <= 0.0 {
            return;
        }
    
        let (start_projected, end_projected) =
            Self::clip_line_to_frustum(start_projected, end_projected);
    
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
    
        if apply_gradient {
            self.draw_line_with_gradient(screen_start, screen_end, start, end, color);
        } else {
            self.draw_line(screen_start, screen_end, color);
        }
    }
    
    pub fn draw_line_with_gradient(
        &mut self,
        start: Vec2,
        end: Vec2,
        start_world: Vec3,
        end_world: Vec3,
        color: u32,
    ) {
        let delta = end - start;
        let delta_world = end_world - start_world;
        let steps = delta.length().ceil() as usize;
    
        for i in 0..steps {
            let t = i as f32 / steps as f32;
    
            let x = (start.x + t * delta.x).round() as usize;
            let y = (start.y + t * delta.y).round() as usize;
    
            let current_world = start_world + t * delta_world;
            let distance_to_origin = current_world.length();
    
            let gradient_factor = (distance_to_origin / 20.0).min(1.0); 

            let adjusted_color = Self::interpolate_color(color, 0x333333, gradient_factor);
    
            self.set_pixel(x, y, adjusted_color);
        }
    }
    
    fn interpolate_color(color1: u32, color2: u32, factor: f32) -> u32 {
        let r1 = (color1 >> 16) & 0xFF;
        let g1 = (color1 >> 8) & 0xFF;
        let b1 = color1 & 0xFF;
    
        let r2 = (color2 >> 16) & 0xFF;
        let g2 = (color2 >> 8) & 0xFF;
        let b2 = color2 & 0xFF;
    
        let r = ((1.0 - factor) * r1 as f32 + factor * r2 as f32) as u32;
        let g = ((1.0 - factor) * g1 as f32 + factor * g2 as f32) as u32;
        let b = ((1.0 - factor) * b1 as f32 + factor * b2 as f32) as u32;
    
        (r << 16) | (g << 8) | b
    }
    
    pub fn clip_line_to_frustum(mut start: Vec4, mut end: Vec4) -> (Vec4, Vec4) {
        if start.w <= 0.0 {
            let t = (0.01 - start.w) / (end.w - start.w);
            start = start + t * (end - start);
        }
    
        if end.w <= 0.0 {
            let t = (0.01 - end.w) / (start.w - end.w);
            end = end + t * (start - end);
        }
    
        (start, end)
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

    pub fn render_compass(&mut self, camera_rotation: &Mat4, size: usize) {
        let offset_x = self.width - size - 10;
        let offset_y = self.height - size - 10;
        let center = Vec2::new(offset_x as f32 + size as f32 / 2.0, offset_y as f32 + size as f32 / 2.0);
    
        let axis_length = size as f32 / 2.0;
    
        let x_axis = Vec3::new(axis_length, 0.0, 0.0);
        let y_axis = Vec3::new(0.0, -axis_length, 0.0);
        let z_axis = Vec3::new(axis_length * 0.7, axis_length * 0.7, 0.0);

        let rotated_x = camera_rotation.transform_point3(x_axis);
        let rotated_y = camera_rotation.transform_point3(y_axis);
        let rotated_z = camera_rotation.transform_point3(z_axis);
    
        let x_end = center + Vec2::new(rotated_x.x, -rotated_x.y);
        let y_end = center + Vec2::new(rotated_y.x, -rotated_y.y);
        let z_end = center + Vec2::new(rotated_z.x, -rotated_z.y);
    
        self.draw_line_2d(center, x_end, 0xFF0000);
        self.draw_line_2d(center, y_end, 0x00FF00);
        self.draw_line_2d(center, z_end, 0x0000FF);
    }

    pub fn draw_line_2d(&mut self, start: Vec2, end: Vec2, color: u32) {
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
