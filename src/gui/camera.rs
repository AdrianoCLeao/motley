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

    pub fn up(&self) -> Vec3 {
        self.up
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

    pub fn zoom(&mut self, amount: f32) {
        let direction = (self.target - self.position).normalize();
        self.position += direction * amount;
    }

    pub fn rotate(&mut self, delta_pitch: f32, delta_yaw: f32) {
        let direction = (self.target - self.position).normalize();
        let right = direction.cross(self.up).normalize();
        let pitch = delta_pitch.to_radians();
        let yaw = delta_yaw.to_radians();

        let rotation_yaw = Mat4::from_axis_angle(self.up, yaw);
        let rotation_pitch = Mat4::from_axis_angle(right, pitch);

        let rotation_matrix = rotation_yaw * rotation_pitch;

        let new_direction = rotation_matrix.transform_vector3(direction);
        self.target = self.position + new_direction;
    }
}
