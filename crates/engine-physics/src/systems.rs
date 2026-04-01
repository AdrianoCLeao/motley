use bevy_ecs::prelude::{
    Added, Changed, Commands, Entity, Query, Res, ResMut, Resource, With, Without,
};
use bevy_ecs::schedule::IntoSystemConfigs;
use engine_core::{PhysicsControlled, Transform, DEFAULT_FIXED_TIMESTEP_SECONDS};
use engine_math::glam::Quat;
use rapier3d::na::{Isometry3, Quaternion, Translation3, UnitQuaternion};
use rapier3d::prelude::{Collider, ColliderBuilder, Point, RigidBodyBuilder};

use crate::components::{
    ColliderHandle3D, ColliderShape3D, PhysicsMaterial, RigidBodyHandle3D, RigidBodyType,
};
use crate::mapping::{ColliderEntityMap3D, PhysicsEntityHandles3D};
use crate::world3d::PhysicsWorld3D;

type NewBodyQueryData = (
    Entity,
    &'static Transform,
    &'static RigidBodyType,
    &'static ColliderShape3D,
    &'static PhysicsMaterial,
);
type NewBodyQueryFilter = (Added<RigidBodyType>, Without<RigidBodyHandle3D>);

type KinematicSyncQueryData = (
    &'static RigidBodyHandle3D,
    &'static RigidBodyType,
    &'static Transform,
);
type KinematicSyncQueryFilter = (With<PhysicsControlled>, Changed<Transform>);

#[derive(Resource, Clone, Copy, Debug)]
pub struct PhysicsStepConfig3D {
    pub fixed_dt_seconds: f32,
}

impl PhysicsStepConfig3D {
    pub fn new(fixed_dt_seconds: f32) -> Self {
        Self {
            fixed_dt_seconds: fixed_dt_seconds.max(0.000_001),
        }
    }
}

impl Default for PhysicsStepConfig3D {
    fn default() -> Self {
        Self::new(DEFAULT_FIXED_TIMESTEP_SECONDS as f32)
    }
}

/// Returns the canonical EP-05 fixed-update order for ECS/Rapier synchronization.
pub fn physics_fixed_update_systems_3d() -> impl IntoSystemConfigs<()> {
    (
        cleanup_orphaned_bodies,
        sync_new_bodies,
        sync_kinematic_bodies_from_transforms,
        step_physics_world,
        write_back_transforms,
    )
        .chain()
}

pub fn sync_new_bodies(
    mut commands: Commands,
    query: Query<NewBodyQueryData, NewBodyQueryFilter>,
    mut physics: Option<ResMut<PhysicsWorld3D>>,
    mut collider_entity_map: Option<ResMut<ColliderEntityMap3D>>,
    mut entity_handles: Option<ResMut<PhysicsEntityHandles3D>>,
) {
    let (Some(physics), Some(collider_entity_map), Some(entity_handles)) = (
        physics.as_deref_mut(),
        collider_entity_map.as_deref_mut(),
        entity_handles.as_deref_mut(),
    ) else {
        return;
    };

    for (entity, transform, body_type, shape, material) in &query {
        let Some(collider) = build_collider(body_type, shape, material) else {
            log::warn!(
                target: "engine::physics",
                "Rejected physics body spawn for entity {:?}: Trimesh colliders must be Static",
                entity
            );
            continue;
        };

        let builder = match body_type {
            RigidBodyType::Dynamic => RigidBodyBuilder::dynamic(),
            RigidBodyType::Kinematic => RigidBodyBuilder::kinematic_position_based(),
            RigidBodyType::Static => RigidBodyBuilder::fixed(),
        };

        let rigid_body = builder.position(transform_to_isometry(transform)).build();
        let rb_handle = physics.rigid_body_set.insert(rigid_body);

        let col_handle = {
            let crate::world3d::PhysicsWorld3D {
                rigid_body_set,
                collider_set,
                ..
            } = &mut *physics;
            collider_set.insert_with_parent(collider, rb_handle, rigid_body_set)
        };

        commands
            .entity(entity)
            .insert(RigidBodyHandle3D(rb_handle))
            .insert(ColliderHandle3D(col_handle));

        collider_entity_map.insert(col_handle, entity);
        entity_handles.insert(entity, rb_handle, col_handle);
    }
}

pub fn sync_kinematic_bodies_from_transforms(
    query: Query<KinematicSyncQueryData, KinematicSyncQueryFilter>,
    mut physics: Option<ResMut<PhysicsWorld3D>>,
) {
    let Some(physics) = physics.as_deref_mut() else {
        return;
    };

    for (handle, body_type, transform) in &query {
        if !matches!(body_type, RigidBodyType::Kinematic) {
            continue;
        }

        let Some(body) = physics.rigid_body_set.get_mut(handle.0) else {
            continue;
        };

        body.set_next_kinematic_position(transform_to_isometry(transform));
    }
}

pub fn step_physics_world(
    mut physics: Option<ResMut<PhysicsWorld3D>>,
    step_config: Option<Res<PhysicsStepConfig3D>>,
) {
    let Some(physics) = physics.as_deref_mut() else {
        return;
    };

    if let Some(step_config) = step_config {
        physics.set_timestep(step_config.fixed_dt_seconds);
    }

    physics.step();
}

pub fn write_back_transforms(
    mut query: Query<(&RigidBodyHandle3D, &mut Transform), With<PhysicsControlled>>,
    physics: Option<Res<PhysicsWorld3D>>,
) {
    let Some(physics) = physics.as_deref() else {
        return;
    };

    for (handle, mut transform) in &mut query {
        let Some(body) = physics.rigid_body_set.get(handle.0) else {
            continue;
        };

        if body.is_sleeping() {
            continue;
        }

        let position = body.position();
        transform.translation = engine_math::Vec3::new(
            position.translation.x,
            position.translation.y,
            position.translation.z,
        );

        let quaternion = position.rotation.quaternion();
        transform.rotation =
            Quat::from_xyzw(quaternion.i, quaternion.j, quaternion.k, quaternion.w);
    }
}

pub fn cleanup_orphaned_bodies(
    active_bodies: Query<(), With<RigidBodyHandle3D>>,
    mut physics: Option<ResMut<PhysicsWorld3D>>,
    mut collider_entity_map: Option<ResMut<ColliderEntityMap3D>>,
    mut entity_handles: Option<ResMut<PhysicsEntityHandles3D>>,
) {
    let (Some(physics), Some(collider_entity_map), Some(entity_handles)) = (
        physics.as_deref_mut(),
        collider_entity_map.as_deref_mut(),
        entity_handles.as_deref_mut(),
    ) else {
        return;
    };

    let stale_entities: Vec<Entity> = entity_handles
        .entities()
        .filter(|entity| active_bodies.get(*entity).is_err())
        .collect();

    for entity in stale_entities {
        let Some((rb_handle, collider_handle)) = entity_handles.remove(entity) else {
            continue;
        };

        {
            let crate::world3d::PhysicsWorld3D {
                rigid_body_set,
                island_manager,
                collider_set,
                impulse_joint_set,
                multibody_joint_set,
                ..
            } = physics;
            let _ = rigid_body_set.remove(
                rb_handle,
                island_manager,
                collider_set,
                impulse_joint_set,
                multibody_joint_set,
                true,
            );
        }

        collider_entity_map.remove(collider_handle);
    }
}

fn build_collider(
    body_type: &RigidBodyType,
    shape: &ColliderShape3D,
    material: &PhysicsMaterial,
) -> Option<Collider> {
    let friction = material.friction.clamp(0.0, 1.0);
    let restitution = material.restitution.clamp(0.0, 1.0);
    let density = material.density.max(0.000_001);

    let builder = match shape {
        ColliderShape3D::Box { half_extents } => ColliderBuilder::cuboid(
            half_extents.x.max(0.001),
            half_extents.y.max(0.001),
            half_extents.z.max(0.001),
        ),
        ColliderShape3D::Sphere { radius } => ColliderBuilder::ball(radius.max(0.001)),
        ColliderShape3D::Capsule {
            half_height,
            radius,
        } => ColliderBuilder::capsule_y(half_height.max(0.001), radius.max(0.001)),
        ColliderShape3D::Trimesh => {
            if !matches!(body_type, RigidBodyType::Static) {
                return None;
            }

            // Placeholder trimesh primitive while mesh-derived colliders are not wired yet.
            ColliderBuilder::trimesh(
                vec![
                    Point::new(-0.5, 0.0, -0.5),
                    Point::new(0.5, 0.0, -0.5),
                    Point::new(0.5, 0.0, 0.5),
                    Point::new(-0.5, 0.0, 0.5),
                ],
                vec![[0, 1, 2], [0, 2, 3]],
            )
        }
    };

    Some(
        builder
            .friction(friction)
            .restitution(restitution)
            .density(density)
            .build(),
    )
}

fn transform_to_isometry(transform: &Transform) -> Isometry3<f32> {
    // Physics scale is currently authored via collider dimensions, not Transform.scale.
    Isometry3::from_parts(
        Translation3::new(
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        ),
        quat_to_unit_quaternion(transform.rotation),
    )
}

fn quat_to_unit_quaternion(quat: Quat) -> UnitQuaternion<f32> {
    UnitQuaternion::from_quaternion(Quaternion::new(quat.w, quat.x, quat.y, quat.z))
}

#[cfg(test)]
mod tests {
    use super::{physics_fixed_update_systems_3d, PhysicsStepConfig3D};
    use crate::components::{
        ColliderShape3D, PhysicsMaterial, RigidBody3DBundle, RigidBodyHandle3D, RigidBodyType,
    };
    use crate::mapping::{ColliderEntityMap3D, PhysicsEntityHandles3D};
    use crate::query::raycast;
    use crate::world3d::PhysicsWorld3D;
    use bevy_ecs::prelude::{Entity, Schedule, World};
    use engine_math::Vec3;

    fn build_test_world() -> World {
        let mut world = World::new();
        let fixed_dt = 1.0 / 60.0;

        world.insert_resource(PhysicsWorld3D::with_timestep(fixed_dt));
        world.insert_resource(PhysicsStepConfig3D::new(fixed_dt));
        world.insert_resource(ColliderEntityMap3D::default());
        world.insert_resource(PhysicsEntityHandles3D::default());

        world
    }

    fn build_schedule() -> Schedule {
        let mut schedule = Schedule::default();
        schedule.add_systems(physics_fixed_update_systems_3d());
        schedule
    }

    fn spawn_floor(world: &mut World) -> Entity {
        world
            .spawn(RigidBody3DBundle {
                body_type: RigidBodyType::Static,
                shape: ColliderShape3D::Box {
                    half_extents: Vec3::new(20.0, 0.1, 20.0),
                },
                material: PhysicsMaterial {
                    restitution: 0.0,
                    friction: 0.9,
                    density: 1.0,
                },
                transform: engine_core::Transform::from_xyz(0.0, -0.1, 0.0),
                ..RigidBody3DBundle::default()
            })
            .id()
    }

    #[test]
    fn systems_are_noop_without_required_resources() {
        let mut world = World::new();
        world.spawn(RigidBody3DBundle::default());

        let mut schedule = build_schedule();
        schedule.run(&mut world);
    }

    #[test]
    fn dynamic_trimesh_is_rejected_without_creating_rigid_body() {
        let mut world = build_test_world();
        let entity = world
            .spawn(RigidBody3DBundle {
                body_type: RigidBodyType::Dynamic,
                shape: ColliderShape3D::Trimesh,
                ..RigidBody3DBundle::default()
            })
            .id();

        let mut schedule = build_schedule();
        schedule.run(&mut world);

        assert!(world.get::<RigidBodyHandle3D>(entity).is_none());
        let physics = world.resource::<PhysicsWorld3D>();
        assert_eq!(physics.rigid_body_set.len(), 0);
    }

    #[test]
    fn dynamic_cube_falls_hits_floor_and_settles() {
        let mut world = build_test_world();
        let _floor = spawn_floor(&mut world);

        let cube = world
            .spawn(RigidBody3DBundle {
                body_type: RigidBodyType::Dynamic,
                shape: ColliderShape3D::Box {
                    half_extents: Vec3::splat(0.5),
                },
                material: PhysicsMaterial {
                    restitution: 0.0,
                    friction: 0.8,
                    density: 1.0,
                },
                transform: engine_core::Transform::from_xyz(0.0, 10.0, 0.0),
                ..RigidBody3DBundle::default()
            })
            .id();

        let mut schedule = build_schedule();
        for _ in 0..300 {
            schedule.run(&mut world);
        }

        let transform = world
            .get::<engine_core::Transform>(cube)
            .expect("cube transform should exist");
        assert!(
            transform.translation.y > 0.40 && transform.translation.y < 0.70,
            "cube should settle near floor top, got y={} ",
            transform.translation.y
        );

        let physics = world.resource::<PhysicsWorld3D>();
        let handle = world
            .get::<RigidBodyHandle3D>(cube)
            .expect("cube should have rigid body handle")
            .0;
        let body = physics
            .rigid_body_set
            .get(handle)
            .expect("rigid body should still exist");

        assert!(body.is_sleeping() || body.linvel().norm() < 0.05);
    }

    #[test]
    fn kinematic_transform_change_updates_rapier_position() {
        let mut world = build_test_world();
        let entity = world
            .spawn(RigidBody3DBundle {
                body_type: RigidBodyType::Kinematic,
                shape: ColliderShape3D::Box {
                    half_extents: Vec3::splat(0.5),
                },
                transform: engine_core::Transform::from_xyz(0.0, 2.0, 0.0),
                ..RigidBody3DBundle::default()
            })
            .id();

        let mut schedule = build_schedule();
        schedule.run(&mut world);

        {
            let mut transform = world
                .get_mut::<engine_core::Transform>(entity)
                .expect("entity transform should exist");
            transform.translation.x = 3.0;
        }

        schedule.run(&mut world);

        let physics = world.resource::<PhysicsWorld3D>();
        let handle = world
            .get::<RigidBodyHandle3D>(entity)
            .expect("entity should have rigid body handle")
            .0;
        let body = physics
            .rigid_body_set
            .get(handle)
            .expect("kinematic body should exist");

        assert!((body.position().translation.x - 3.0).abs() < 0.05);
    }

    #[test]
    fn raycast_returns_hit_and_none_for_miss() {
        let mut world = build_test_world();

        let target = world
            .spawn(RigidBody3DBundle {
                body_type: RigidBodyType::Static,
                shape: ColliderShape3D::Box {
                    half_extents: Vec3::splat(0.5),
                },
                transform: engine_core::Transform::from_xyz(0.0, 0.5, 0.0),
                ..RigidBody3DBundle::default()
            })
            .id();

        let mut schedule = build_schedule();
        schedule.run(&mut world);

        let hit = {
            let physics = world.resource::<PhysicsWorld3D>();
            let map = world.resource::<ColliderEntityMap3D>();
            raycast(
                Vec3::new(0.0, 5.0, 0.0),
                Vec3::new(0.0, -1.0, 0.0),
                20.0,
                physics,
                map.as_map(),
            )
        }
        .expect("raycast should hit target cube");

        assert_eq!(hit.entity, target);
        assert!(hit.distance > 0.0);

        let miss = {
            let physics = world.resource::<PhysicsWorld3D>();
            let map = world.resource::<ColliderEntityMap3D>();
            raycast(
                Vec3::new(50.0, 5.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                5.0,
                physics,
                map.as_map(),
            )
        };

        assert!(miss.is_none());
    }
}
