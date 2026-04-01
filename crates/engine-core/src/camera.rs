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
mod tests {
    use super::{sync_camera_aspect_from_window, Camera2d, Camera3d, PrimaryCamera, WindowSize};
    use bevy_ecs::{
        prelude::{Entity, With, World},
        schedule::Schedule,
    };
    use engine_math::glam::Vec4;

    fn assert_approx(actual: f32, expected: f32, epsilon: f32) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn camera_3d_projection_matrix_is_finite() {
        let camera = Camera3d::default();
        let projection = camera.projection_matrix();

        for value in projection.to_cols_array() {
            assert!(value.is_finite());
        }
    }

    #[test]
    fn camera_2d_projection_matrix_is_finite() {
        let camera = Camera2d::default();
        let projection = camera.projection_matrix();

        for value in projection.to_cols_array() {
            assert!(value.is_finite());
        }
    }

    #[test]
    fn resize_resource_updates_camera_aspect_ratio() {
        let mut world = World::new();
        world.insert_resource(WindowSize::new(1920, 1080));

        let entity = world.spawn(Camera3d::default()).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(sync_camera_aspect_from_window);
        schedule.run(&mut world);

        let camera = world
            .entity(entity)
            .get::<Camera3d>()
            .expect("camera exists");
        assert!((camera.aspect_ratio - (1920.0 / 1080.0)).abs() < 1e-6);
    }

    #[test]
    fn camera_3d_projection_matrix_matches_expected_scaling_terms() {
        let camera = Camera3d {
            fov_y_radians: std::f32::consts::FRAC_PI_4,
            near: 0.1,
            far: 1000.0,
            aspect_ratio: 16.0 / 9.0,
        };

        let projection = camera.projection_matrix();
        let expected_y = 1.0 / (camera.fov_y_radians * 0.5).tan();
        let expected_x = expected_y / camera.aspect_ratio;

        assert_approx(projection.x_axis.x, expected_x, 1e-5);
        assert_approx(projection.y_axis.y, expected_y, 1e-5);
    }

    #[test]
    fn camera_2d_projection_maps_viewport_bounds_to_ndc() {
        let camera = Camera2d {
            viewport_width: 100.0,
            viewport_height: 50.0,
            ..Camera2d::default()
        };

        let projection = camera.projection_matrix();

        let left_top = projection * Vec4::new(-50.0, 25.0, 0.0, 1.0);
        let right_bottom = projection * Vec4::new(50.0, -25.0, 0.0, 1.0);

        assert_approx(left_top.x, -1.0, 1e-6);
        assert_approx(left_top.y, 1.0, 1e-6);
        assert_approx(right_bottom.x, 1.0, 1e-6);
        assert_approx(right_bottom.y, -1.0, 1e-6);
    }

    #[test]
    fn primary_camera_query_selects_only_marked_camera() {
        let mut world = World::new();

        let primary = world.spawn((Camera3d::default(), PrimaryCamera)).id();
        world.spawn(Camera3d::default());

        let mut query = world.query_filtered::<Entity, (With<Camera3d>, With<PrimaryCamera>)>();
        let cameras: Vec<Entity> = query.iter(&world).collect();

        assert_eq!(cameras, vec![primary]);
    }

    #[test]
    fn zero_height_viewport_falls_back_to_safe_aspect_ratio() {
        let mut camera = Camera3d::default();
        camera.set_aspect_ratio_from_viewport(1920, 0);

        assert_approx(camera.aspect_ratio, 1.0, 1e-6);
    }
}
