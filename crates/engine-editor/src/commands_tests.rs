use bevy_ecs::world::World;
use engine_core::{register_core_reflection_types, Children, EntityName, Parent, Transform};
use engine_reflect::{ComponentRegistry, ReflectMetadataRegistry, ReflectTypeRegistry};

use crate::commands::{
    AddComponentCommand, CommandHistory, DeleteEntityCommand, DuplicateEntityCommand,
    RemoveComponentCommand, RenameEntityCommand, ReparentEntityCommand, SpawnEntityCommand,
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

    let parent = world
        .spawn((EntityName::new("Parent"), Children::default()))
        .id();
    let child = world
        .spawn((
            EntityName::new("Child"),
            Parent(parent),
            Children::default(),
        ))
        .id();
    let grandchild = world
        .spawn((EntityName::new("Grandchild"), Parent(child)))
        .id();

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
    assert!(world
        .get::<Children>(parent)
        .map(|children| children.0.is_empty())
        .unwrap_or(false));

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

    let parent = world
        .spawn((EntityName::new("Parent"), Children::default()))
        .id();
    let source = world
        .spawn((
            EntityName::new("Source"),
            Parent(parent),
            Children::default(),
        ))
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
    assert_eq!(
        world.get::<Parent>(duplicate_root).map(|p| p.0),
        Some(parent)
    );
    assert_eq!(
        world.get::<Parent>(duplicate_leaf).map(|p| p.0),
        Some(duplicate_root)
    );

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

    let _ = history.execute(
        Box::new(AddComponentCommand::new(entity, "Transform")),
        &mut world,
    );
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

#[test]
fn spawn_entity_command_supports_undo_redo_for_root_entities() {
    let mut world = setup_world();
    let mut history = CommandHistory::new(16);

    let execute_hint = history.execute(Box::new(SpawnEntityCommand::new_root()), &mut world);
    let first_entity = execute_hint.expect("spawned root entity should be selected after execute");

    assert!(world.get::<EntityName>(first_entity).is_some());
    assert!(world.get::<Transform>(first_entity).is_some());
    assert!(world.get::<Parent>(first_entity).is_none());

    let undo_hint = history.undo(&mut world);
    assert_eq!(undo_hint, None);
    assert!(world.get_entity(first_entity).is_err());

    let redo_hint = history.redo(&mut world);
    let second_entity = redo_hint.expect("spawned root entity should be selected after redo");
    assert!(world.get_entity(second_entity).is_ok());
    assert!(world.get::<Parent>(second_entity).is_none());
}

#[test]
fn spawn_entity_command_links_child_and_restores_on_undo() {
    let mut world = setup_world();
    let parent = world
        .spawn((EntityName::new("Parent"), Children::default()))
        .id();
    let mut history = CommandHistory::new(16);

    let execute_hint = history.execute(Box::new(SpawnEntityCommand::new_child(parent)), &mut world);
    let first_child = execute_hint.expect("spawned child should be selected after execute");

    assert_eq!(
        world.get::<Parent>(first_child).map(|value| value.0),
        Some(parent)
    );
    assert_eq!(
        world
            .get::<Children>(parent)
            .map(|children| children.0.clone())
            .unwrap_or_default(),
        vec![first_child]
    );

    let undo_hint = history.undo(&mut world);
    assert_eq!(undo_hint, Some(parent));
    assert!(world.get_entity(first_child).is_err());
    assert!(world
        .get::<Children>(parent)
        .map(|children| children.0.is_empty())
        .unwrap_or(true));

    let redo_hint = history.redo(&mut world);
    let second_child = redo_hint.expect("spawned child should be selected after redo");
    assert_eq!(
        world.get::<Parent>(second_child).map(|value| value.0),
        Some(parent)
    );
    assert_eq!(
        world
            .get::<Children>(parent)
            .map(|children| children.0.clone())
            .unwrap_or_default(),
        vec![second_child]
    );
}

#[test]
fn reparent_entity_command_moves_between_parents_and_supports_undo_redo() {
    let mut world = setup_world();

    let parent_a = world
        .spawn((EntityName::new("Parent A"), Children::default()))
        .id();
    let parent_b = world
        .spawn((EntityName::new("Parent B"), Children::default()))
        .id();
    let child = world
        .spawn((EntityName::new("Child"), Parent(parent_a)))
        .id();

    {
        let mut parent_ref = world.entity_mut(parent_a);
        parent_ref.insert(Children(vec![child]));
    }

    let mut history = CommandHistory::new(16);

    let execute_hint = history.execute(
        Box::new(ReparentEntityCommand::new(child, Some(parent_b))),
        &mut world,
    );
    assert_eq!(execute_hint, Some(child));
    assert_eq!(
        world.get::<Parent>(child).map(|value| value.0),
        Some(parent_b)
    );
    assert!(world
        .get::<Children>(parent_a)
        .map(|children| children.0.is_empty())
        .unwrap_or(true));
    assert_eq!(
        world
            .get::<Children>(parent_b)
            .map(|children| children.0.clone())
            .unwrap_or_default(),
        vec![child]
    );

    let undo_hint = history.undo(&mut world);
    assert_eq!(undo_hint, Some(child));
    assert_eq!(
        world.get::<Parent>(child).map(|value| value.0),
        Some(parent_a)
    );
    assert_eq!(
        world
            .get::<Children>(parent_a)
            .map(|children| children.0.clone())
            .unwrap_or_default(),
        vec![child]
    );
    assert!(world
        .get::<Children>(parent_b)
        .map(|children| children.0.is_empty())
        .unwrap_or(true));

    let redo_hint = history.redo(&mut world);
    assert_eq!(redo_hint, Some(child));
    assert_eq!(
        world.get::<Parent>(child).map(|value| value.0),
        Some(parent_b)
    );
}

#[test]
fn reparent_entity_command_moves_to_root_and_undo_restores_parent() {
    let mut world = setup_world();

    let parent = world
        .spawn((EntityName::new("Parent"), Children::default()))
        .id();
    let child = world.spawn((EntityName::new("Child"), Parent(parent))).id();

    {
        let mut parent_ref = world.entity_mut(parent);
        parent_ref.insert(Children(vec![child]));
    }

    let mut history = CommandHistory::new(16);

    let execute_hint = history.execute(
        Box::new(ReparentEntityCommand::new(child, None)),
        &mut world,
    );
    assert_eq!(execute_hint, Some(child));
    assert!(world.get::<Parent>(child).is_none());
    assert!(world
        .get::<Children>(parent)
        .map(|children| children.0.is_empty())
        .unwrap_or(true));

    let undo_hint = history.undo(&mut world);
    assert_eq!(undo_hint, Some(child));
    assert_eq!(
        world.get::<Parent>(child).map(|value| value.0),
        Some(parent)
    );
    assert_eq!(
        world
            .get::<Children>(parent)
            .map(|children| children.0.clone())
            .unwrap_or_default(),
        vec![child]
    );
}

#[test]
fn reparent_entity_command_rejects_cycles_without_mutating_hierarchy() {
    let mut world = setup_world();

    let root = world
        .spawn((EntityName::new("Root"), Children::default()))
        .id();
    let child = world
        .spawn((EntityName::new("Child"), Parent(root), Children::default()))
        .id();
    let grandchild = world
        .spawn((EntityName::new("Grandchild"), Parent(child)))
        .id();

    {
        let mut root_ref = world.entity_mut(root);
        root_ref.insert(Children(vec![child]));
    }
    {
        let mut child_ref = world.entity_mut(child);
        child_ref.insert(Children(vec![grandchild]));
    }

    let mut history = CommandHistory::new(16);

    let execute_hint = history.execute(
        Box::new(ReparentEntityCommand::new(root, Some(grandchild))),
        &mut world,
    );

    assert_eq!(execute_hint, Some(root));
    assert!(world.get::<Parent>(root).is_none());
    assert_eq!(world.get::<Parent>(child).map(|value| value.0), Some(root));
    assert_eq!(
        world.get::<Parent>(grandchild).map(|value| value.0),
        Some(child)
    );
}
