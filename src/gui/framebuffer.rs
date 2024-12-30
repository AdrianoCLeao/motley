use glam::{Mat4, Vec2, Vec3, Vec4};
use rayon::prelude::*;

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
        let screen_bounds = Vec2::new(self.width as f32, self.height as f32);
        let axis_length = 15.0;
    
        self.draw_line_3d_clipped(
            Vec3::new(-axis_length, 0.0, 0.0),
            Vec3::new(axis_length, 0.0, 0.0),
            0xA63737,
            view_projection_matrix,
            screen_bounds,
        );
    
        self.draw_line_3d_clipped(
            Vec3::new(0.0, 0.0, -axis_length),
            Vec3::new(0.0, 0.0, axis_length),
            0x468E2C,
            view_projection_matrix,
            screen_bounds,
        );
    
        self.render_grid(view_projection_matrix, 15.0);
    }
    
    pub fn render_grid(&mut self, view_projection_matrix: &Mat4, size: f32) {
        let screen_bounds = Vec2::new(self.width as f32, self.height as f32);
    
        let lines: Vec<(Vec3, Vec3, u32)> = (-size as i32..=size as i32)
            .filter(|&x| x != 0)
            .flat_map(|x| {
                let x_f32 = x as f32;
                let start_x = Vec3::new(x_f32, 0.0, -size);
                let end_x = Vec3::new(x_f32, 0.0, size);
    
                let start_z = Vec3::new(-size, 0.0, x_f32);
                let end_z = Vec3::new(size, 0.0, x_f32);
    
                vec![
                    (start_x, end_x, 0x505050),
                    (start_z, end_z, 0x505050),
                ]
            })
            .collect();

        let projected_lines: Vec<(Vec2, Vec2, u32)> = lines
            .into_par_iter()
            .filter_map(|(start, end, color)| {
                let start_projected = *view_projection_matrix * Vec4::from((start, 1.0));
                let end_projected = *view_projection_matrix * Vec4::from((end, 1.0));
    
                if start_projected.w <= 0.0 && end_projected.w <= 0.0 {
                    return None;
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
                    (start_ndc.x * 0.5 + 0.5) * screen_bounds.x,
                    (1.0 - (start_ndc.y * 0.5 + 0.5)) * screen_bounds.y,
                );
    
                let screen_end = Vec2::new(
                    (end_ndc.x * 0.5 + 0.5) * screen_bounds.x,
                    (1.0 - (end_ndc.y * 0.5 + 0.5)) * screen_bounds.y,
                );
    
                Some((screen_start, screen_end, color))
            })
            .collect();

        for (start, end, color) in projected_lines {
            self.draw_line(start, end, color);
        }
    }  

    pub fn draw_line_3d_clipped(
        &mut self,
        start: Vec3,
        end: Vec3,
        color: u32,
        view_projection_matrix: &Mat4,
        screen_bounds: Vec2,
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
            (start_ndc.x * 0.5 + 0.5) * screen_bounds.x,
            (1.0 - (start_ndc.y * 0.5 + 0.5)) * screen_bounds.y,
        );
    
        let screen_end = Vec2::new(
            (end_ndc.x * 0.5 + 0.5) * screen_bounds.x,
            (1.0 - (end_ndc.y * 0.5 + 0.5)) * screen_bounds.y,
        );
    
        if screen_start.x < 0.0 && screen_end.x < 0.0 || screen_start.x > screen_bounds.x && screen_end.x > screen_bounds.x {
            return;
        }
        if screen_start.y < 0.0 && screen_end.y < 0.0 || screen_start.y > screen_bounds.y && screen_end.y > screen_bounds.y {
            return;
        }
    
        self.draw_line(screen_start, screen_end, color);
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
        let directions = [
            (Vec3::new(axis_length, 0.0, 0.0), 0xFF0000),
            (Vec3::new(0.0, -axis_length, 0.0), 0x00FF00),
            (Vec3::new(axis_length * 0.7, axis_length * 0.7, 0.0), 0x0000FF),
        ];

        for (axis, color) in directions {
            let rotated = camera_rotation.transform_point3(axis);
            let end = center + Vec2::new(rotated.x, -rotated.y);
            self.draw_line_2d(center, end, color);
        }
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
