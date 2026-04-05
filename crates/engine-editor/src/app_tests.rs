use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use engine_core::{Children, EntityName, Parent};

use crate::app::{
    build_scene_tree_visibility_map, collect_scene_tree_roots, is_scene_tree_descendant,
};

fn attach_child(world: &mut World, parent: Entity, child: Entity) {
    if let Ok(mut child_ref) = world.get_entity_mut(child) {
        child_ref.insert(Parent(parent));
    }

    if let Ok(mut parent_ref) = world.get_entity_mut(parent) {
        if let Some(mut children) = parent_ref.get_mut::<Children>() {
            if !children.0.contains(&child) {
                children.0.push(child);
            }
        } else {
            parent_ref.insert(Children(vec![child]));
        }
    }
}

#[test]
fn scene_tree_visibility_keeps_matching_ancestors_visible() {
    let mut world = World::new();

    let root = world
        .spawn((EntityName::new("Level Root"), Children::default()))
        .id();
    let branch = world
        .spawn((EntityName::new("Gameplay"), Children::default()))
        .id();
    let camera = world.spawn((EntityName::new("Main Camera"),)).id();
    let unrelated = world.spawn((EntityName::new("Audio Bus"),)).id();

    attach_child(&mut world, root, branch);
    attach_child(&mut world, branch, camera);

    let roots = collect_scene_tree_roots(&world);
    let visibility = build_scene_tree_visibility_map(&world, &roots, "camera");

    assert!(visibility.get(&root).copied().unwrap_or(false));
    assert!(visibility.get(&branch).copied().unwrap_or(false));
    assert!(visibility.get(&camera).copied().unwrap_or(false));
    assert!(!visibility.get(&unrelated).copied().unwrap_or(false));
}

#[test]
fn collect_scene_tree_roots_treats_entities_with_missing_parent_as_roots() {
    let mut world = World::new();

    let parent = world
        .spawn((EntityName::new("Parent"), Children::default()))
        .id();
    let child = world.spawn((EntityName::new("Child"),)).id();
    attach_child(&mut world, parent, child);

    let removed_parent = world.spawn((EntityName::new("Removed Parent"),)).id();
    let orphan = world
        .spawn((EntityName::new("Orphan"), Parent(removed_parent)))
        .id();
    let independent_root = world.spawn((EntityName::new("Independent"),)).id();

    let _ = world.despawn(removed_parent);

    let roots = collect_scene_tree_roots(&world);

    assert!(roots.contains(&parent));
    assert!(roots.contains(&orphan));
    assert!(roots.contains(&independent_root));
    assert!(!roots.contains(&child));
}

#[test]
fn scene_tree_descendant_check_uses_parent_chain() {
    let mut world = World::new();

    let root = world.spawn((EntityName::new("Root"),)).id();
    let child = world.spawn((EntityName::new("Child"), Parent(root))).id();
    let grandchild = world
        .spawn((EntityName::new("Grandchild"), Parent(child)))
        .id();
    let unrelated = world.spawn((EntityName::new("Unrelated"),)).id();

    assert!(is_scene_tree_descendant(&world, grandchild, root));
    assert!(is_scene_tree_descendant(&world, child, root));
    assert!(!is_scene_tree_descendant(&world, root, grandchild));
    assert!(!is_scene_tree_descendant(&world, unrelated, root));
}

#[test]
fn scene_tree_visibility_map_handles_child_cycles_without_recursing_forever() {
    let mut world = World::new();

    let node_a = world
        .spawn((EntityName::new("Node A"), Children::default()))
        .id();
    let node_b = world
        .spawn((EntityName::new("Node B"), Children::default()))
        .id();

    attach_child(&mut world, node_a, node_b);
    attach_child(&mut world, node_b, node_a);

    let visibility = build_scene_tree_visibility_map(&world, &[node_a], "not-found");

    assert!(!visibility.get(&node_a).copied().unwrap_or(false));
    assert!(!visibility.get(&node_b).copied().unwrap_or(false));
}
