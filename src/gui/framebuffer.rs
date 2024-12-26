use glam::{Mat4, Vec3};

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

    pub fn render_axes(&mut self, camera_view_proj: &Mat4) {
        // Cores dos eixos
        let x_axis_color = 0xFF0000; // Vermelho para X
        let y_axis_color = 0x00FF00; // Verde para Y
        let z_axis_color = 0x0000FF; // Azul para Z

        // Defina os pontos extremos dos eixos em 3D
        let origin = Vec3::ZERO;
        let x_axis = Vec3::new(1.0, 0.0, 0.0);
        let y_axis = Vec3::new(0.0, 1.0, 0.0);
        let z_axis = Vec3::new(0.0, 0.0, 1.0);

        // Projete os pontos no espaço da tela
        let origin_screen = self.project_to_screen(&origin, camera_view_proj);
        let x_screen = self.project_to_screen(&x_axis, camera_view_proj);
        let y_screen = self.project_to_screen(&y_axis, camera_view_proj);
        let z_screen = self.project_to_screen(&z_axis, camera_view_proj);

        // Renderize as linhas dos eixos
        self.draw_line(origin_screen, x_screen, x_axis_color);
        self.draw_line(origin_screen, y_screen, y_axis_color);
        self.draw_line(origin_screen, z_screen, z_axis_color);
    }

    fn project_to_screen(&self, point: &Vec3, camera_view_proj: &Mat4) -> (usize, usize) {
        let clip_space = *camera_view_proj * point.extend(1.0);
        let ndc_space = clip_space.truncate() / clip_space.w;

        let x_screen = ((ndc_space.x + 1.0) * 0.5 * self.width as f32).round() as usize;
        let y_screen = ((1.0 - (ndc_space.y + 1.0) * 0.5) * self.height as f32).round() as usize;

        (x_screen.min(self.width - 1), y_screen.min(self.height - 1))
    }

    fn draw_line(&mut self, start: (usize, usize), end: (usize, usize), color: u32) {
        let (x0, y0) = start;
        let (x1, y1) = end;

        let dx = (x1 as isize - x0 as isize).abs();
        let dy = (y1 as isize - y0 as isize).abs();

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx - dy;

        let mut x = x0 as isize;
        let mut y = y0 as isize;

        while x != x1 as isize || y != y1 as isize {
            if x >= 0 && y >= 0 && x < self.width as isize && y < self.height as isize {
                self.set_pixel(x as usize, y as usize, color);
            }

            let e2 = err * 2;

            if e2 > -dy {
                err -= dy;
                x += sx;
            }

            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
}
