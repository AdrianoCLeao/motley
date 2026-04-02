use engine_reflect::{ComponentRegistry, EditorHint, ReflectMetadataRegistry, ReflectTypeRegistry};

use crate::{register_physics_reflection_types, ColliderShape3D, PhysicsMaterial, RigidBodyType};

#[test]
fn register_physics_reflection_types_registers_expected_components() {
    let mut type_registry = ReflectTypeRegistry::default();
    let mut component_registry = ComponentRegistry::default();
    let mut metadata_registry = ReflectMetadataRegistry::default();

    register_physics_reflection_types(
        &mut type_registry,
        &mut component_registry,
        &mut metadata_registry,
    );

    assert!(type_registry.contains::<RigidBodyType>());
    assert!(type_registry.contains::<ColliderShape3D>());
    assert!(type_registry.contains::<PhysicsMaterial>());

    assert!(component_registry
        .by_name(std::any::type_name::<RigidBodyType>())
        .is_some());
    assert!(component_registry
        .by_name(std::any::type_name::<ColliderShape3D>())
        .is_some());
    assert!(component_registry
        .by_name(std::any::type_name::<PhysicsMaterial>())
        .is_some());

    assert_eq!(
        metadata_registry.hint_for::<PhysicsMaterial>("restitution"),
        Some(EditorHint::Range { min: 0.0, max: 1.0 })
    );
}
