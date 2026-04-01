use bevy_ecs::{prelude::World, query::With};
use engine_core::{Camera2d, Camera3d, GlobalTransform, PrimaryCamera};
use engine_math::Mat4;

use crate::{Camera2dUniform, Camera3dUniform};

pub(crate) fn extract_camera_uniform_3d(world: &mut World) -> Option<Camera3dUniform> {
    let mut query = world.query_filtered::<(&Camera3d, &GlobalTransform), With<PrimaryCamera>>();
    let (camera, global_transform) = query.iter(world).next()?;

    let view = Mat4::from(global_transform.0.inverse());
    let view_proj = camera.projection_matrix() * view;
    let translation = global_transform.translation();

    Some(Camera3dUniform {
        view_proj: view_proj.to_cols_array_2d(),
        camera_position: [translation.x, translation.y, translation.z, 1.0],
        light_direction: [0.6, -1.0, 0.2, 0.0],
    })
}

pub(crate) fn extract_camera_uniform_2d(
    world: &mut World,
    viewport_width: u32,
    viewport_height: u32,
) -> Camera2dUniform {
    let mut query = world.query_filtered::<(&Camera2d, &GlobalTransform), With<PrimaryCamera>>();

    let view_proj = if let Some((camera, global_transform)) = query.iter(world).next() {
        let view = Mat4::from(global_transform.0.inverse());
        camera.projection_matrix() * view
    } else {
        let width = viewport_width.max(1) as f32;
        let height = viewport_height.max(1) as f32;
        Mat4::orthographic_rh(
            -width * 0.5,
            width * 0.5,
            -height * 0.5,
            height * 0.5,
            -1.0,
            1.0,
        )
    };

    Camera2dUniform {
        view_proj: view_proj.to_cols_array_2d(),
    }
}
