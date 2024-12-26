use glam::{Mat4, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(position: Vec3, target: Vec3, up: Vec3, fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        Camera {
            position,
            target,
            up,
            fov,
            aspect_ratio,
            near,
            far,
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

    pub fn set_up(&mut self, up: Vec3) {
        self.up = up;
    }

    pub fn move_forward(&mut self, distance: f32) {
        let direction = (self.target - self.position).normalize();
        self.position += direction * distance;
        self.target += direction * distance;
    }

    pub fn move_backward(&mut self, distance: f32) {
        let direction = (self.target - self.position).normalize();
        self.position -= direction * distance;
        self.target -= direction * distance;
    }

    pub fn move_right(&mut self, distance: f32) {
        let direction = (self.target - self.position).normalize();
        let right = direction.cross(self.up).normalize();
        self.position += right * distance;
        self.target += right * distance;
    }

    pub fn move_left(&mut self, distance: f32) {
        let direction = (self.target - self.position).normalize();
        let right = direction.cross(self.up).normalize();
        self.position -= right * distance;
        self.target -= right * distance;
    }

    // Gira a câmera ao redor do ponto de destino
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let direction = (self.target - self.position).normalize();
        let radius = (self.target - self.position).length();

        let yaw = delta_x; // Rotação ao redor do eixo Y
        let pitch = delta_y; // Rotação ao redor do eixo X

        let rotation_matrix = Mat4::from_rotation_y(yaw)
            * Mat4::from_rotation_x(pitch);

        let new_direction = rotation_matrix.transform_vector3(direction);
        self.position = self.target - new_direction * radius;
    }

    // Move a câmera lateralmente e verticalmente
    pub fn pan(&mut self, delta_x: f32, delta_y: f32, right: Vec3, up: Vec3) {
        let pan_offset = right * delta_x + up * delta_y;
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
