use glam::{Mat4, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub rotation: Vec3,
}

impl Camera {
    pub fn new(
        position: Vec3,
        target: Vec3,
        up: Vec3,
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let rotation = Vec3::ZERO;
        Camera {
            position,
            target,
            up,
            fov,
            aspect_ratio,
            near,
            far,
            rotation,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), self.aspect_ratio, self.near, self.far)
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }

    pub fn target(&self) -> Vec3 {
        self.target
    }

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let radius = (self.target - self.position).length();

        self.rotation.y += delta_x * 0.01; 
        self.rotation.x = (self.rotation.x + delta_y * 0.01).clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

        let new_position = Vec3::new(
            radius * self.rotation.y.sin() * self.rotation.x.cos(),
            radius * self.rotation.x.sin(),
            radius * self.rotation.y.cos() * self.rotation.x.cos(),
        );

        self.position = self.target - new_position;
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let right = self.right();
        let up = self.up();

        let pan_offset = right * delta_x * 0.01 + up * delta_y * 0.01;
        self.position += pan_offset;
        self.target += pan_offset;
    }

    pub fn zoom(&mut self, amount: f32) {
        let direction = (self.target - self.position).normalize();
        let distance = (self.target - self.position).length();

        let new_distance = (distance + amount).clamp(1.0, 50.0);
        self.position = self.target - direction * new_distance;
    }

    pub fn right(&self) -> Vec3 {
        self.view_matrix().x_axis.truncate().normalize()
    }

    pub fn up(&self) -> Vec3 {
        self.view_matrix().y_axis.truncate().normalize()
    }
}
