use super::{Hidden, PhysicsControlled, RenderLayer2D, RenderLayer3D, Visible};
use bevy_ecs::prelude::{Entity, With, Without, World};

#[test]
fn tags_support_query_inclusion_and_exclusion() {
    let mut world = World::new();

    let visible: Entity = world.spawn(Visible).id();
    let _hidden: Entity = world.spawn((Visible, Hidden)).id();

    let mut query = world.query_filtered::<Entity, (With<Visible>, Without<Hidden>)>();
    let entities: Vec<Entity> = query.iter(&world).collect();

    assert_eq!(entities, vec![visible]);
}

#[test]
fn physics_controlled_filter_includes_only_marked_entities() {
    let mut world = World::new();

    let physics_entity = world.spawn(PhysicsControlled).id();
    world.spawn(Visible);

    let mut query = world.query_filtered::<Entity, With<PhysicsControlled>>();
    let entities: Vec<Entity> = query.iter(&world).collect();

    assert_eq!(entities, vec![physics_entity]);
}

#[test]
fn render_layer_filters_do_not_overlap() {
    let mut world = World::new();

    let layer_3d = world.spawn(RenderLayer3D).id();
    let layer_2d = world.spawn(RenderLayer2D).id();

    let mut query_3d = world.query_filtered::<Entity, With<RenderLayer3D>>();
    let mut query_2d = world.query_filtered::<Entity, With<RenderLayer2D>>();

    let entities_3d: Vec<Entity> = query_3d.iter(&world).collect();
    let entities_2d: Vec<Entity> = query_2d.iter(&world).collect();

    assert_eq!(entities_3d, vec![layer_3d]);
    assert_eq!(entities_2d, vec![layer_2d]);
}
