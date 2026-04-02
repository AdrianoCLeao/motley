use bevy_ecs::prelude::{Component, World};
use bevy_reflect::Reflect;
use engine_math::Vec3;
use engine_reflect::{
    reflect_enum_variants, reflect_field, reflect_field_mut, reflect_variant_name,
    ComponentRegistry, EditorHint, ReflectMetadataRegistry, ReflectRegistration,
    ReflectTypeRegistry,
};

#[derive(Component, Reflect, Default, Debug)]
struct TestComponent {
    value: f32,
    enabled: bool,
}

#[derive(Component, Reflect, Default, Debug, PartialEq, engine_reflect::RegisterReflect)]
struct DerivedWithMetadata {
    value: f32,
    #[engine_reflect(range(min = 0.0, max = 1.0))]
    normalized: f32,
    #[reflect(ignore)]
    #[engine_reflect(skip)]
    internal_flag: bool,
}

#[derive(Component, Reflect, Default, Debug, PartialEq, Eq, engine_reflect::RegisterReflect)]
enum TestBodyType {
    #[default]
    Dynamic,
    Kinematic,
    Static,
}

#[derive(Reflect, Default, Debug)]
struct Vec3Container {
    value: Vec3,
}

#[test]
fn reflect_type_registry_registers_type() {
    let mut registry = ReflectTypeRegistry::default();

    registry.register::<TestComponent>();

    assert!(registry.contains::<TestComponent>());
}

#[test]
fn component_registry_registers_and_exposes_descriptor() {
    let mut registry = ComponentRegistry::default();

    assert!(registry.register::<TestComponent>());
    assert!(!registry.register::<TestComponent>());
    assert_eq!(registry.len(), 1);

    let descriptor = registry
        .by_name(std::any::type_name::<TestComponent>())
        .expect("descriptor should exist");

    let mut world = World::new();
    let entity = world
        .spawn(TestComponent {
            value: 1.5,
            enabled: true,
        })
        .id();

    assert!(descriptor.has(entity, &world));

    let reflect = descriptor
        .get_reflect(entity, &world)
        .expect("reflect value should exist");
    let typed = reflect
        .as_any()
        .downcast_ref::<TestComponent>()
        .expect("downcast should succeed");
    assert!((typed.value - 1.5).abs() < f32::EPSILON);
    assert!(typed.enabled);

    {
        let reflect = descriptor
            .get_reflect_mut(entity, &mut world)
            .expect("mutable reflect value should exist");
        let typed = reflect
            .as_any_mut()
            .downcast_mut::<TestComponent>()
            .expect("mutable downcast should succeed");
        typed.value = 7.25;
        typed.enabled = false;
    }

    let updated = world
        .get::<TestComponent>(entity)
        .expect("component should still exist");
    assert!((updated.value - 7.25).abs() < f32::EPSILON);
    assert!(!updated.enabled);
}

#[test]
fn component_descriptor_inserts_default_and_removes_component() {
    let mut registry = ComponentRegistry::default();
    registry.register::<TestComponent>();

    let descriptor = registry
        .by_name(std::any::type_name::<TestComponent>())
        .expect("descriptor should exist");

    let mut world = World::new();
    let entity = world.spawn_empty().id();

    assert!(descriptor.insert_default(entity, &mut world));
    assert!(world.get::<TestComponent>(entity).is_some());

    assert!(descriptor.remove(entity, &mut world));
    assert!(world.get::<TestComponent>(entity).is_none());
}

#[test]
fn register_reflect_extracts_range_and_skip_metadata() {
    let mut type_registry = ReflectTypeRegistry::default();
    let mut component_registry = ComponentRegistry::default();
    let mut metadata_registry = ReflectMetadataRegistry::default();

    <DerivedWithMetadata as ReflectRegistration>::register_reflect(
        &mut type_registry,
        &mut component_registry,
        &mut metadata_registry,
    );

    assert!(type_registry.contains::<DerivedWithMetadata>());
    assert!(component_registry
        .by_name(std::any::type_name::<DerivedWithMetadata>())
        .is_some());
    assert_eq!(
        metadata_registry.hint_for::<DerivedWithMetadata>("normalized"),
        Some(EditorHint::Range { min: 0.0, max: 1.0 })
    );
    assert!(metadata_registry.is_hidden_for::<DerivedWithMetadata>("internal_flag"));
}

#[test]
fn reflection_helpers_access_and_mutate_glam_fields() {
    let mut value = Vec3Container {
        value: Vec3::new(1.0, 2.0, 3.0),
    };

    let value_field = reflect_field(&value, "value").expect("container should expose value field");
    let x_field = reflect_field(value_field, "x").expect("Vec3 should expose x field");
    let x = x_field
        .try_downcast_ref::<f32>()
        .copied()
        .expect("x field should be f32");
    assert!((x - 1.0).abs() < f32::EPSILON);

    let value_field_mut =
        reflect_field_mut(&mut value, "value").expect("value field should be mutable");
    let y_field_mut = reflect_field_mut(value_field_mut, "y").expect("Vec3 should expose y field");
    let y = y_field_mut
        .try_downcast_mut::<f32>()
        .expect("y field should be mutable f32");
    *y = 9.5;

    assert!((value.value.y - 9.5).abs() < f32::EPSILON);
}

#[test]
fn enum_reflection_helpers_expose_variant_information() {
    let mut type_registry = ReflectTypeRegistry::default();
    let mut component_registry = ComponentRegistry::default();
    let mut metadata_registry = ReflectMetadataRegistry::default();
    <TestBodyType as ReflectRegistration>::register_reflect(
        &mut type_registry,
        &mut component_registry,
        &mut metadata_registry,
    );

    let variant_name = reflect_variant_name(&TestBodyType::Static)
        .expect("enum variant should be available through reflection");
    assert_eq!(variant_name, "Static");

    let variants = reflect_enum_variants::<TestBodyType>(&type_registry)
        .expect("enum variant list should be available from type registry");
    assert_eq!(variants, vec!["Dynamic", "Kinematic", "Static"]);
}
