use bevy_ecs::prelude::Entity;
use engine_math::Vec3;
use rapier3d::prelude::{ColliderHandle, Point, QueryFilter, Ray, Vector};
use std::collections::HashMap;

use crate::world3d::PhysicsWorld3D;

#[derive(Clone, Copy, Debug)]
pub struct RaycastHit {
    pub entity: Entity,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

pub fn raycast(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    physics: &PhysicsWorld3D,
    entity_map: &HashMap<ColliderHandle, Entity>,
) -> Option<RaycastHit> {
    let max_distance = max_distance.max(0.0);
    if max_distance <= 0.0 {
        return None;
    }

    let dir = direction.normalize_or_zero();
    if dir.length_squared() <= f32::EPSILON {
        return None;
    }

    let ray = Ray::new(
        Point::new(origin.x, origin.y, origin.z),
        Vector::new(dir.x, dir.y, dir.z),
    );

    let filter = QueryFilter::default();
    let (collider_handle, intersection) = physics.query_pipeline.cast_ray_and_get_normal(
        &physics.rigid_body_set,
        &physics.collider_set,
        &ray,
        max_distance,
        true,
        filter,
    )?;

    let entity = *entity_map.get(&collider_handle)?;
    let hit_point = ray.point_at(intersection.time_of_impact);

    Some(RaycastHit {
        entity,
        point: Vec3::new(hit_point.x, hit_point.y, hit_point.z),
        normal: Vec3::new(
            intersection.normal.x,
            intersection.normal.y,
            intersection.normal.z,
        ),
        distance: intersection.time_of_impact,
    })
}
