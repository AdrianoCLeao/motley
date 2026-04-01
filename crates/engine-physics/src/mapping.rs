use bevy_ecs::prelude::{Entity, Resource};
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct ColliderEntityMap3D {
    by_collider: HashMap<ColliderHandle, Entity>,
}

impl ColliderEntityMap3D {
    pub fn insert(&mut self, collider: ColliderHandle, entity: Entity) {
        self.by_collider.insert(collider, entity);
    }

    pub fn get(&self, collider: &ColliderHandle) -> Option<Entity> {
        self.by_collider.get(collider).copied()
    }

    pub fn remove(&mut self, collider: ColliderHandle) {
        self.by_collider.remove(&collider);
    }

    pub fn as_map(&self) -> &HashMap<ColliderHandle, Entity> {
        &self.by_collider
    }
}

#[derive(Resource, Default)]
pub struct PhysicsEntityHandles3D {
    by_entity: HashMap<Entity, (RigidBodyHandle, ColliderHandle)>,
}

impl PhysicsEntityHandles3D {
    pub fn insert(
        &mut self,
        entity: Entity,
        rigid_body: RigidBodyHandle,
        collider: ColliderHandle,
    ) {
        self.by_entity.insert(entity, (rigid_body, collider));
    }

    pub fn remove(&mut self, entity: Entity) -> Option<(RigidBodyHandle, ColliderHandle)> {
        self.by_entity.remove(&entity)
    }

    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.by_entity.keys().copied()
    }
}
