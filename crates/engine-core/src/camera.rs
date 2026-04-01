use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{Component, Query, Res, Resource};
use engine_math::Mat4;

#[derive(Component, Clone, Debug)]
pub struct Camera3d {
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
    pub aspect_ratio: f32,
}

impl Default for Camera3d {
    fn default() -> Self {
        Self {
            fov_y_radians: std::f32::consts::FRAC_PI_4,
            near: 0.1,
            far: 1000.0,
            aspect_ratio: 16.0 / 9.0,
        }
    }
}

impl Camera3d {
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y_radians, self.aspect_ratio, self.near, self.far)
    }

    pub fn set_aspect_ratio_from_viewport(&mut self, width: u32, height: u32) {
        if height == 0 {
            self.aspect_ratio = 1.0;
            return;
        }

        self.aspect_ratio = width as f32 / height as f32;
    }
}

#[derive(Component, Clone, Debug)]
pub struct Camera2d {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera2d {
    fn default() -> Self {
        Self {
            viewport_width: 1280.0,
            viewport_height: 720.0,
            near: -1.0,
            far: 1.0,
        }
    }
}

impl Camera2d {
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            -self.viewport_width / 2.0,
            self.viewport_width / 2.0,
            -self.viewport_height / 2.0,
            self.viewport_height / 2.0,
            self.near,
            self.far,
        )
    }
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PrimaryCamera;

#[derive(Resource, Debug, Clone, Copy)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl WindowSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
        }
    }
}

pub fn sync_camera_aspect_from_window(
    window_size: Res<WindowSize>,
    mut cameras: Query<&mut Camera3d>,
) {
    if !window_size.is_changed() {
        return;
    }

    for mut camera in &mut cameras {
        camera.set_aspect_ratio_from_viewport(window_size.width, window_size.height);
    }
}

#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;
