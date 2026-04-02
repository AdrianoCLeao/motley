use bevy_ecs::component::Component;
use bevy_reflect::{GetTypeRegistration, Reflect};
use engine_reflect::{
    ComponentRegistry, ReflectMetadataRegistry, ReflectRegistration, ReflectTypeRegistry,
};

use crate::{
    Camera2d, Camera3d, Hidden, PhysicsControlled, PrimaryCamera, RenderLayer2D, RenderLayer3D,
    Transform, Visible, WindowSize,
};

pub fn register_core_reflection_types(
    type_registry: &mut ReflectTypeRegistry,
    component_registry: &mut ComponentRegistry,
    metadata_registry: &mut ReflectMetadataRegistry,
) {
    type_registry.register::<WindowSize>();

    register_component::<Transform>(type_registry, component_registry, metadata_registry);
    register_component::<Camera3d>(type_registry, component_registry, metadata_registry);
    register_component::<Camera2d>(type_registry, component_registry, metadata_registry);
    register_component::<PrimaryCamera>(type_registry, component_registry, metadata_registry);
    register_component::<Visible>(type_registry, component_registry, metadata_registry);
    register_component::<Hidden>(type_registry, component_registry, metadata_registry);
    register_component::<RenderLayer3D>(type_registry, component_registry, metadata_registry);
    register_component::<RenderLayer2D>(type_registry, component_registry, metadata_registry);
    register_component::<PhysicsControlled>(type_registry, component_registry, metadata_registry);
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
