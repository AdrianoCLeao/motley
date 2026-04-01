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
