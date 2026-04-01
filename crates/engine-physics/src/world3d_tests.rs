use super::PhysicsWorld3D;
use rapier3d::prelude::*;

#[test]
fn defaults_are_initialized_for_fixed_step_simulation() {
    let world = PhysicsWorld3D::new();

    assert!(world.timestep_seconds() > 0.0);
    assert_eq!(world.gravity, vector![0.0, -9.81, 0.0]);
}

#[test]
fn repeated_steps_do_not_panic() {
    let mut world = PhysicsWorld3D::new();

    for _ in 0..120 {
        world.step();
    }

    assert!(world.timestep_seconds() > 0.0);
}

#[test]
fn gravity_moves_dynamic_body_downward() {
    let mut world = PhysicsWorld3D::new();

    let body = RigidBodyBuilder::dynamic()
        .translation(vector![0.0, 10.0, 0.0])
        .build();
    let handle = world.rigid_body_set.insert(body);
    let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.5).build();
    world
        .collider_set
        .insert_with_parent(collider, handle, &mut world.rigid_body_set);

    for _ in 0..60 {
        world.step();
    }

    let y = world
        .rigid_body_set
        .get(handle)
        .expect("body should exist")
        .position()
        .translation
        .y;

    assert!(y < 10.0);
}
