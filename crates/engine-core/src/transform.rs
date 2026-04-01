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
#[path = "transform_tests.rs"]
mod tests;
