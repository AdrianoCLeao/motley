use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use bevy_ecs::component::Component;
use bevy_ecs::prelude::{Entity, Resource};
use bevy_ecs::world::World;
use bevy_reflect::{ReflectMut, ReflectRef, TypeInfo};

pub use bevy_reflect::{
    self, GetTypeRegistration, PartialReflect, Reflect, TypePath, TypeRegistry,
};
pub use engine_reflect_derive::RegisterReflect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorHint {
    Range { min: f64, max: f64 },
    Multiline,
    Color,
    HideInInspector,
    ReadOnly,
    Degrees,
}

#[derive(Resource, Default)]
pub struct ReflectMetadataRegistry {
    hidden_fields: HashMap<TypeId, HashSet<&'static str>>,
    field_hints: HashMap<TypeId, HashMap<&'static str, EditorHint>>,
}

impl ReflectMetadataRegistry {
    pub fn hide_field<T>(&mut self, field_path: &'static str)
    where
        T: 'static,
    {
        self.hidden_fields
            .entry(TypeId::of::<T>())
            .or_default()
            .insert(field_path);
    }

    pub fn set_hint<T>(&mut self, field_path: &'static str, hint: EditorHint)
    where
        T: 'static,
    {
        self.field_hints
            .entry(TypeId::of::<T>())
            .or_default()
            .insert(field_path, hint);
    }

    pub fn is_hidden_for<T>(&self, field_path: &str) -> bool
    where
        T: 'static,
    {
        self.is_hidden_by_type_id(TypeId::of::<T>(), field_path)
    }

    pub fn is_hidden_by_type_id(&self, type_id: TypeId, field_path: &str) -> bool {
        self.hidden_fields
            .get(&type_id)
            .is_some_and(|fields| fields.contains(field_path))
    }

    pub fn hint_for<T>(&self, field_path: &str) -> Option<EditorHint>
    where
        T: 'static,
    {
        self.hint_by_type_id(TypeId::of::<T>(), field_path)
    }

    pub fn hint_by_type_id(&self, type_id: TypeId, field_path: &str) -> Option<EditorHint> {
        self.field_hints
            .get(&type_id)
            .and_then(|hints| hints.get(field_path))
            .copied()
    }
}

#[derive(Resource, Default)]
pub struct ReflectTypeRegistry {
    registry: TypeRegistry,
}

impl ReflectTypeRegistry {
    pub fn register<T>(&mut self)
    where
        T: GetTypeRegistration + 'static,
    {
        self.registry.register::<T>();
    }

    pub fn contains<T>(&self) -> bool
    where
        T: 'static,
    {
        self.registry.get(TypeId::of::<T>()).is_some()
    }

    pub fn read(&self) -> &TypeRegistry {
        &self.registry
    }

    pub fn write(&mut self) -> &mut TypeRegistry {
        &mut self.registry
    }
}

pub trait ReflectRegistration {
    fn register_reflect(
        type_registry: &mut ReflectTypeRegistry,
        component_registry: &mut ComponentRegistry,
        metadata_registry: &mut ReflectMetadataRegistry,
    );
}

pub fn register_component_type<C>(
    type_registry: &mut ReflectTypeRegistry,
    component_registry: &mut ComponentRegistry,
) -> bool
where
    C: Component + Reflect + Default + GetTypeRegistration + 'static,
{
    type_registry.register::<C>();
    component_registry.register::<C>()
}

pub fn reflect_field<'a>(
    value: &'a dyn PartialReflect,
    name: &str,
) -> Option<&'a dyn PartialReflect> {
    match value.reflect_ref() {
        ReflectRef::Struct(data) => data.field(name),
        ReflectRef::TupleStruct(data) => {
            let index = name.parse::<usize>().ok()?;
            data.field(index)
        }
        _ => None,
    }
}

pub fn reflect_field_mut<'a>(
    value: &'a mut dyn PartialReflect,
    name: &str,
) -> Option<&'a mut dyn PartialReflect> {
    match value.reflect_mut() {
        ReflectMut::Struct(data) => data.field_mut(name),
        ReflectMut::TupleStruct(data) => {
            let index = name.parse::<usize>().ok()?;
            data.field_mut(index)
        }
        _ => None,
    }
}

pub fn reflect_variant_name(value: &dyn Reflect) -> Option<&str> {
    match value.reflect_ref() {
        ReflectRef::Enum(value) => Some(value.variant_name()),
        _ => None,
    }
}

pub fn reflect_enum_variants<T>(type_registry: &ReflectTypeRegistry) -> Option<Vec<&'static str>>
where
    T: 'static,
{
    let registration = type_registry.read().get(TypeId::of::<T>())?;
    match registration.type_info() {
        TypeInfo::Enum(enum_info) => Some(enum_info.iter().map(|variant| variant.name()).collect()),
        _ => None,
    }
}

pub fn with_reflection_registries<R>(
    world: &mut World,
    f: impl FnOnce(&mut ReflectTypeRegistry, &mut ComponentRegistry, &mut ReflectMetadataRegistry) -> R,
) -> R {
    let mut type_registry = world
        .remove_resource::<ReflectTypeRegistry>()
        .unwrap_or_default();
    let mut component_registry = world
        .remove_resource::<ComponentRegistry>()
        .unwrap_or_default();
    let mut metadata_registry = world
        .remove_resource::<ReflectMetadataRegistry>()
        .unwrap_or_default();

    let result = f(
        &mut type_registry,
        &mut component_registry,
        &mut metadata_registry,
    );

    world.insert_resource(type_registry);
    world.insert_resource(component_registry);
    world.insert_resource(metadata_registry);

    result
}

pub struct ComponentDescriptor {
    pub name: &'static str,
    pub type_id: TypeId,
    has_fn: for<'w> fn(Entity, &'w World) -> bool,
    get_reflect_fn: for<'w> fn(Entity, &'w World) -> Option<&'w dyn Reflect>,
    get_reflect_mut_fn: for<'w> fn(Entity, &'w mut World) -> Option<&'w mut dyn Reflect>,
    insert_default_fn: fn(Entity, &mut World) -> bool,
    remove_fn: fn(Entity, &mut World) -> bool,
}

impl ComponentDescriptor {
    pub fn has(&self, entity: Entity, world: &World) -> bool {
        (self.has_fn)(entity, world)
    }

    pub fn get_reflect<'w>(&self, entity: Entity, world: &'w World) -> Option<&'w dyn Reflect> {
        (self.get_reflect_fn)(entity, world)
    }

    pub fn get_reflect_mut<'w>(
        &self,
        entity: Entity,
        world: &'w mut World,
    ) -> Option<&'w mut dyn Reflect> {
        (self.get_reflect_mut_fn)(entity, world)
    }

    pub fn insert_default(&self, entity: Entity, world: &mut World) -> bool {
        (self.insert_default_fn)(entity, world)
    }

    pub fn remove(&self, entity: Entity, world: &mut World) -> bool {
        (self.remove_fn)(entity, world)
    }
}

#[derive(Resource, Default)]
pub struct ComponentRegistry {
    descriptors: Vec<ComponentDescriptor>,
    by_name: HashMap<&'static str, usize>,
    by_type: HashMap<TypeId, usize>,
}

impl ComponentRegistry {
    pub fn register<C>(&mut self) -> bool
    where
        C: Component + Reflect + Default + 'static,
    {
        let name = std::any::type_name::<C>();
        let type_id = TypeId::of::<C>();

        if self.by_type.contains_key(&type_id) {
            return false;
        }

        let descriptor = ComponentDescriptor {
            name,
            type_id,
            has_fn: has_component::<C>,
            get_reflect_fn: get_component_reflect::<C>,
            get_reflect_mut_fn: get_component_reflect_mut::<C>,
            insert_default_fn: insert_default_component::<C>,
            remove_fn: remove_component::<C>,
        };

        let index = self.descriptors.len();
        self.by_name.insert(name, index);
        self.by_type.insert(type_id, index);
        self.descriptors.push(descriptor);

        true
    }

    pub fn register_with_type_registry<C>(
        &mut self,
        type_registry: &mut ReflectTypeRegistry,
    ) -> bool
    where
        C: Component + Reflect + Default + GetTypeRegistration + 'static,
    {
        type_registry.register::<C>();
        self.register::<C>()
    }

    pub fn register_with_reflection<C>(
        &mut self,
        type_registry: &mut ReflectTypeRegistry,
        metadata_registry: &mut ReflectMetadataRegistry,
    ) -> bool
    where
        C: Component + Reflect + Default + GetTypeRegistration + ReflectRegistration + 'static,
    {
        let was_registered = self.by_type.contains_key(&TypeId::of::<C>());
        C::register_reflect(type_registry, self, metadata_registry);
        !was_registered && self.by_type.contains_key(&TypeId::of::<C>())
    }

    pub fn all(&self) -> &[ComponentDescriptor] {
        &self.descriptors
    }

    pub fn by_name(&self, name: &str) -> Option<&ComponentDescriptor> {
        self.by_name
            .get(name)
            .and_then(|index| self.descriptors.get(*index))
    }

    pub fn by_type_id(&self, type_id: TypeId) -> Option<&ComponentDescriptor> {
        self.by_type
            .get(&type_id)
            .and_then(|index| self.descriptors.get(*index))
    }

    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

fn has_component<C>(entity: Entity, world: &World) -> bool
where
    C: Component + 'static,
{
    world.get::<C>(entity).is_some()
}

fn get_component_reflect<C>(entity: Entity, world: &World) -> Option<&dyn Reflect>
where
    C: Component + Reflect + 'static,
{
    world
        .get::<C>(entity)
        .map(|component| component as &dyn Reflect)
}

fn get_component_reflect_mut<C>(entity: Entity, world: &mut World) -> Option<&mut dyn Reflect>
where
    C: Component + Reflect + 'static,
{
    let component = world.get_mut::<C>(entity)?;
    Some(component.into_inner() as &mut dyn Reflect)
}

fn insert_default_component<C>(entity: Entity, world: &mut World) -> bool
where
    C: Component + Default + 'static,
{
    let Ok(mut entity_ref) = world.get_entity_mut(entity) else {
        return false;
    };

    entity_ref.insert(C::default());
    true
}

fn remove_component<C>(entity: Entity, world: &mut World) -> bool
where
    C: Component + 'static,
{
    let Ok(mut entity_ref) = world.get_entity_mut(entity) else {
        return false;
    };

    if !entity_ref.contains::<C>() {
        return false;
    }

    entity_ref.remove::<C>();
    true
}
