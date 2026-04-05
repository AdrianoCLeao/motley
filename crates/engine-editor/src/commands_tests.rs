use bevy_ecs::world::World;
use engine_core::{
    register_core_reflection_types, Children, EntityName, Parent, Transform,
};
use engine_reflect::{ComponentRegistry, ReflectMetadataRegistry, ReflectTypeRegistry};

use crate::commands::{
    AddComponentCommand, CommandHistory, DeleteEntityCommand, DuplicateEntityCommand,
    RemoveComponentCommand, RenameEntityCommand,
};

fn setup_world() -> World {
    let mut world = World::new();

    let mut type_registry = ReflectTypeRegistry::default();
    let mut component_registry = ComponentRegistry::default();
    let mut metadata_registry = ReflectMetadataRegistry::default();
    register_core_reflection_types(
        &mut type_registry,
        &mut component_registry,
        &mut metadata_registry,
    );

    world.insert_resource(type_registry);
    world.insert_resource(component_registry);
    world.insert_resource(metadata_registry);

    world
}

fn entity_name(world: &World, entity: bevy_ecs::entity::Entity) -> String {
    world
        .get::<EntityName>(entity)
        .map(|value| value.0.clone())
        .unwrap_or_default()
}

#[test]
fn rename_entity_command_supports_undo_redo() {
    let mut world = setup_world();
    let entity = world.spawn(EntityName::new("Old")).id();

    let mut history = CommandHistory::new(16);

    let execute_hint = history.execute(
        Box::new(RenameEntityCommand::new(
            entity,
            "Old".to_owned(),
            "New".to_owned(),
        )),
        &mut world,
    );
    assert_eq!(execute_hint, Some(entity));
    assert_eq!(entity_name(&world, entity), "New");

    let undo_hint = history.undo(&mut world);
    assert_eq!(undo_hint, Some(entity));
    assert_eq!(entity_name(&world, entity), "Old");

    let redo_hint = history.redo(&mut world);
    assert_eq!(redo_hint, Some(entity));
    assert_eq!(entity_name(&world, entity), "New");
}

#[test]
fn delete_entity_command_restores_subtree_on_undo() {
    let mut world = setup_world();

    let parent = world.spawn((EntityName::new("Parent"), Children::default())).id();
    let child = world
        .spawn((EntityName::new("Child"), Parent(parent), Children::default()))
        .id();
    let grandchild = world.spawn((EntityName::new("Grandchild"), Parent(child))).id();

    {
        let mut parent_ref = world.entity_mut(parent);
        parent_ref.insert(Children(vec![child]));
    }
    {
        let mut child_ref = world.entity_mut(child);
        child_ref.insert(Children(vec![grandchild]));
    }

    let mut history = CommandHistory::new(16);

    let execute_hint = history.execute(Box::new(DeleteEntityCommand::new(child)), &mut world);
    assert_eq!(execute_hint, Some(parent));
    assert!(world.get_entity(child).is_err());
    assert!(world.get_entity(grandchild).is_err());
    assert!(
        world
            .get::<Children>(parent)
            .map(|children| children.0.is_empty())
            .unwrap_or(false)
    );

    let undo_hint = history.undo(&mut world);
    assert_eq!(undo_hint, Some(parent));

    let restored_child = world
        .get::<Children>(parent)
        .and_then(|children| children.0.first().copied())
        .expect("restored child should be attached to parent");
    assert_eq!(entity_name(&world, restored_child), "Child");

    let restored_grandchild = world
        .get::<Children>(restored_child)
        .and_then(|children| children.0.first().copied())
        .expect("restored grandchild should be attached to restored child");
    assert_eq!(entity_name(&world, restored_grandchild), "Grandchild");

    let redo_hint = history.redo(&mut world);
    assert_eq!(redo_hint, Some(parent));
    assert!(world.get_entity(restored_child).is_err());
    assert!(world.get_entity(restored_grandchild).is_err());
}

#[test]
fn duplicate_entity_command_clones_subtree_and_supports_undo_redo() {
    let mut world = setup_world();

    let parent = world.spawn((EntityName::new("Parent"), Children::default())).id();
    let source = world
        .spawn((EntityName::new("Source"), Parent(parent), Children::default()))
        .id();
    let grandchild = world.spawn((EntityName::new("Leaf"), Parent(source))).id();

    {
        let mut parent_ref = world.entity_mut(parent);
        parent_ref.insert(Children(vec![source]));
    }
    {
        let mut source_ref = world.entity_mut(source);
        source_ref.insert(Children(vec![grandchild]));
    }

    let mut history = CommandHistory::new(16);

    let execute_hint = history.execute(Box::new(DuplicateEntityCommand::new(source)), &mut world);
    let duplicate_root = execute_hint.expect("duplicate entity should be selected after execute");

    let parent_children = world
        .get::<Children>(parent)
        .map(|children| children.0.clone())
        .unwrap_or_default();
    assert_eq!(parent_children.len(), 2);
    assert_eq!(parent_children[0], source);
    assert_eq!(parent_children[1], duplicate_root);
    assert_eq!(entity_name(&world, duplicate_root), "Source");

    let duplicate_leaf = world
        .get::<Children>(duplicate_root)
        .and_then(|children| children.0.first().copied())
        .expect("duplicate subtree should include child");
    assert_eq!(entity_name(&world, duplicate_leaf), "Leaf");
    assert_eq!(world.get::<Parent>(duplicate_root).map(|p| p.0), Some(parent));
    assert_eq!(world.get::<Parent>(duplicate_leaf).map(|p| p.0), Some(duplicate_root));

    let undo_hint = history.undo(&mut world);
    assert_eq!(undo_hint, None);
    assert!(world.get_entity(duplicate_root).is_err());

    let children_after_undo = world
        .get::<Children>(parent)
        .map(|children| children.0.clone())
        .unwrap_or_default();
    assert_eq!(children_after_undo, vec![source]);

    let redo_hint = history.redo(&mut world);
    let duplicate_again = redo_hint.expect("duplicate entity should be selected after redo");
    assert_ne!(duplicate_again, source);

    let children_after_redo = world
        .get::<Children>(parent)
        .map(|children| children.0.clone())
        .unwrap_or_default();
    assert_eq!(children_after_redo.len(), 2);
    assert_eq!(children_after_redo[0], source);
    assert_eq!(children_after_redo[1], duplicate_again);
}

#[test]
fn add_remove_component_commands_roundtrip_state() {
    let mut world = setup_world();
    let entity = world.spawn_empty().id();

    let mut history = CommandHistory::new(16);

    let _ = history.execute(Box::new(AddComponentCommand::new(entity, "Transform")), &mut world);
    assert!(world.get::<Transform>(entity).is_some());

    {
        let mut transform = world
            .get_mut::<Transform>(entity)
            .expect("Transform should exist after AddComponentCommand");
        transform.translation.x = 42.0;
    }

    let _ = history.execute(
        Box::new(RemoveComponentCommand::new(entity, "Transform")),
        &mut world,
    );
    assert!(world.get::<Transform>(entity).is_none());

    let _ = history.undo(&mut world);
    let restored = world
        .get::<Transform>(entity)
        .expect("Transform should be restored after undo");
    assert_eq!(restored.translation.x, 42.0);
}
