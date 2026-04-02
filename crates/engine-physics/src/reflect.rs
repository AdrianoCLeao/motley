use bevy_ecs::component::Component;
use bevy_reflect::{GetTypeRegistration, Reflect};
use engine_reflect::{
    ComponentRegistry, ReflectMetadataRegistry, ReflectRegistration, ReflectTypeRegistry,
};

use crate::{ColliderShape3D, PhysicsMaterial, RigidBodyType};

pub fn register_physics_reflection_types(
    type_registry: &mut ReflectTypeRegistry,
    component_registry: &mut ComponentRegistry,
    metadata_registry: &mut ReflectMetadataRegistry,
) {
    register_component::<RigidBodyType>(type_registry, component_registry, metadata_registry);
    register_component::<ColliderShape3D>(type_registry, component_registry, metadata_registry);
    register_component::<PhysicsMaterial>(type_registry, component_registry, metadata_registry);
}

fn register_component<T>(
    type_registry: &mut ReflectTypeRegistry,
    component_registry: &mut ComponentRegistry,
    metadata_registry: &mut ReflectMetadataRegistry,
) where
    T: Component + Reflect + Default + GetTypeRegistration + ReflectRegistration + 'static,
{
    T::register_reflect(type_registry, component_registry, metadata_registry);
}

#[cfg(test)]
#[path = "reflect_tests.rs"]
mod tests;
