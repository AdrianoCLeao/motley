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
