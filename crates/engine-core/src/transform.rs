use bevy_ecs::prelude::{Bundle, Component, Entity, Query};
use engine_math::glam::{Affine3A, Quat, Vec3};

#[derive(Component, Clone, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vec3::new(x, y, z),
            ..Self::IDENTITY
        }
    }

    pub fn to_affine(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Component, Clone, Debug)]
pub struct GlobalTransform(pub Affine3A);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Affine3A::IDENTITY)
    }
}

impl GlobalTransform {
    pub fn translation(&self) -> Vec3 {
        self.0.translation.into()
    }
}

#[derive(Component, Clone, Debug)]
pub struct Parent(pub Entity);

#[derive(Component, Clone, Debug, Default)]
pub struct Children(pub Vec<Entity>);

#[derive(Bundle, Default)]
pub struct SpatialBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

pub fn propagate_transforms(
    locals: Query<(Entity, &Transform, Option<&Parent>)>,
    mut globals: Query<&mut GlobalTransform>,
) {
    use std::collections::{HashMap, HashSet};

    #[derive(Clone, Copy)]
    struct Node {
        parent: Option<Entity>,
        local: Affine3A,
    }

    fn resolve_global(
        entity: Entity,
        nodes: &HashMap<Entity, Node>,
        cache: &mut HashMap<Entity, Affine3A>,
        visiting: &mut HashSet<Entity>,
    ) -> Option<Affine3A> {
        if let Some(cached) = cache.get(&entity) {
            return Some(*cached);
        }

        let node = nodes.get(&entity)?;

        if !visiting.insert(entity) {
            log::warn!(
                target: "engine::ecs",
                "Cycle detected in transform hierarchy at entity {:?}; treating as root",
                entity
            );
            return Some(node.local);
        }

        let global = if let Some(parent) = node.parent {
            let parent_global =
                resolve_global(parent, nodes, cache, visiting).unwrap_or(Affine3A::IDENTITY);
            parent_global * node.local
        } else {
            node.local
        };

        visiting.remove(&entity);
        cache.insert(entity, global);

        Some(global)
    }

    let mut nodes = HashMap::new();
    for (entity, local, parent) in &locals {
        nodes.insert(
            entity,
            Node {
                parent: parent.map(|value| value.0),
                local: local.to_affine(),
            },
        );
    }

    let mut cache = HashMap::with_capacity(nodes.len());
    let mut visiting = HashSet::with_capacity(nodes.len());
    for entity in nodes.keys().copied() {
        visiting.clear();
        let _ = resolve_global(entity, &nodes, &mut cache, &mut visiting);
    }

    for (entity, global) in cache {
        if let Ok(mut current) = globals.get_mut(entity) {
            *current = GlobalTransform(global);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        propagate_transforms, Children, GlobalTransform, Parent, SpatialBundle, Transform,
    };
    use bevy_ecs::{prelude::World, schedule::Schedule};
    use engine_math::Vec3;
    use std::time::Instant;

    #[test]
    fn parent_and_child_global_transforms_are_propagated() {
        let mut world = World::new();

        let parent = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(1.0, 2.0, 3.0),
                    ..SpatialBundle::default()
                },
                Children::default(),
            ))
            .id();

        let child = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(2.0, 0.0, 0.0),
                    ..SpatialBundle::default()
                },
                Parent(parent),
            ))
            .id();

        world
            .entity_mut(parent)
            .get_mut::<Children>()
            .expect("children exist")
            .0
            .push(child);

        let mut schedule = Schedule::default();
        schedule.add_systems(propagate_transforms);
        schedule.run(&mut world);

        let parent_global = world
            .entity(parent)
            .get::<GlobalTransform>()
            .expect("parent global")
            .translation();
        let child_global = world
            .entity(child)
            .get::<GlobalTransform>()
            .expect("child global")
            .translation();

        assert_eq!(parent_global, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(child_global, Vec3::new(3.0, 2.0, 3.0));
    }

    #[test]
    fn three_level_hierarchy_is_propagated() {
        let mut world = World::new();

        let root = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(5.0, 0.0, 0.0),
                    ..SpatialBundle::default()
                },
                Children::default(),
            ))
            .id();

        let mid = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(0.0, 3.0, 0.0),
                    ..SpatialBundle::default()
                },
                Parent(root),
                Children::default(),
            ))
            .id();

        let leaf = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(0.0, 0.0, 2.0),
                    ..SpatialBundle::default()
                },
                Parent(mid),
            ))
            .id();

        world
            .entity_mut(root)
            .get_mut::<Children>()
            .expect("root children exist")
            .0
            .push(mid);
        world
            .entity_mut(mid)
            .get_mut::<Children>()
            .expect("mid children exist")
            .0
            .push(leaf);

        let mut schedule = Schedule::default();
        schedule.add_systems(propagate_transforms);
        schedule.run(&mut world);

        let leaf_global = world
            .entity(leaf)
            .get::<GlobalTransform>()
            .expect("leaf global")
            .translation();

        assert_eq!(leaf_global, Vec3::new(5.0, 3.0, 2.0));
    }

    #[test]
    fn changed_parent_transform_updates_child_global_same_frame() {
        let mut world = World::new();

        let parent = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(1.0, 0.0, 0.0),
                    ..SpatialBundle::default()
                },
                Children::default(),
            ))
            .id();

        let child = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(1.0, 0.0, 0.0),
                    ..SpatialBundle::default()
                },
                Parent(parent),
            ))
            .id();

        world
            .entity_mut(parent)
            .get_mut::<Children>()
            .expect("children exist")
            .0
            .push(child);

        let mut schedule = Schedule::default();
        schedule.add_systems(propagate_transforms);
        schedule.run(&mut world);

        world
            .entity_mut(parent)
            .get_mut::<Transform>()
            .expect("parent transform")
            .translation = Vec3::new(4.0, 0.0, 0.0);

        schedule.run(&mut world);

        let child_global = world
            .entity(child)
            .get::<GlobalTransform>()
            .expect("child global")
            .translation();

        assert_eq!(child_global, Vec3::new(5.0, 0.0, 0.0));
    }

    #[test]
    fn reparenting_child_updates_global_transform_to_new_parent() {
        let mut world = World::new();

        let parent_a = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(2.0, 0.0, 0.0),
                    ..SpatialBundle::default()
                },
                Children::default(),
            ))
            .id();

        let parent_b = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(10.0, 0.0, 0.0),
                    ..SpatialBundle::default()
                },
                Children::default(),
            ))
            .id();

        let child = world
            .spawn((
                SpatialBundle {
                    transform: Transform::from_xyz(1.0, 0.0, 0.0),
                    ..SpatialBundle::default()
                },
                Parent(parent_a),
            ))
            .id();

        world
            .entity_mut(parent_a)
            .get_mut::<Children>()
            .expect("parent_a children")
            .0
            .push(child);

        let mut schedule = Schedule::default();
        schedule.add_systems(propagate_transforms);
        schedule.run(&mut world);

        let initial_child_global = world
            .entity(child)
            .get::<GlobalTransform>()
            .expect("child global")
            .translation();
        assert_eq!(initial_child_global, Vec3::new(3.0, 0.0, 0.0));

        world
            .entity_mut(parent_a)
            .get_mut::<Children>()
            .expect("parent_a children")
            .0
            .retain(|entity| *entity != child);
        world
            .entity_mut(parent_b)
            .get_mut::<Children>()
            .expect("parent_b children")
            .0
            .push(child);
        world
            .entity_mut(child)
            .get_mut::<Parent>()
            .expect("child parent")
            .0 = parent_b;

        schedule.run(&mut world);

        let reparented_child_global = world
            .entity(child)
            .get::<GlobalTransform>()
            .expect("child global")
            .translation();

        assert_eq!(reparented_child_global, Vec3::new(11.0, 0.0, 0.0));
    }

    #[test]
    #[ignore = "manual EP-03 performance evidence"]
    fn hierarchy_propagation_10k_entities_soft_benchmark() {
        const ROOTS: usize = 100;
        const CHILDREN_PER_ROOT: usize = 99;
        const TOTAL_ENTITIES: usize = ROOTS * (1 + CHILDREN_PER_ROOT);

        let mut world = World::new();
        let mut roots = Vec::with_capacity(ROOTS);

        for root_index in 0..ROOTS {
            let root = world
                .spawn((
                    SpatialBundle {
                        transform: Transform::from_xyz(root_index as f32, 0.0, 0.0),
                        ..SpatialBundle::default()
                    },
                    Children::default(),
                ))
                .id();
            roots.push(root);
        }

        for &root in &roots {
            let mut children = Vec::with_capacity(CHILDREN_PER_ROOT);

            for child_index in 0..CHILDREN_PER_ROOT {
                let child = world
                    .spawn((
                        SpatialBundle {
                            transform: Transform::from_xyz(child_index as f32 * 0.01, 1.0, 0.0),
                            ..SpatialBundle::default()
                        },
                        Parent(root),
                    ))
                    .id();
                children.push(child);
            }

            world
                .entity_mut(root)
                .get_mut::<Children>()
                .expect("root children")
                .0 = children;
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(propagate_transforms);

        let started_at = Instant::now();
        schedule.run(&mut world);
        let elapsed = started_at.elapsed();

        let mut query = world.query::<&GlobalTransform>();
        assert_eq!(query.iter(&world).count(), TOTAL_ENTITIES);

        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        println!(
            "EP-03 benchmark: propagated {} entities in {:.3} ms",
            TOTAL_ENTITIES, elapsed_ms
        );
    }
}
