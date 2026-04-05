use std::any::TypeId;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use eframe::egui;
use engine_core::EntityName;
use engine_reflect::bevy_reflect::{
    std_traits::ReflectDefault, DynamicArray, DynamicEnum, DynamicList, DynamicMap, DynamicSet,
    DynamicStruct, DynamicTuple, DynamicTupleStruct, PartialReflect, ReflectRef, TypeInfo,
    VariantInfo, VariantType,
};
use engine_reflect::{
    ComponentDescriptor, ComponentRegistry, EditorHint, ReflectMetadataRegistry,
    ReflectTypeRegistry,
};

use crate::commands::{
    short_type_name, AddComponentCommand, CommandHistory, EditorCommand, RemoveComponentCommand,
    SetFieldCommand,
};
use crate::selection::Selection;

pub struct InspectorPanel;

impl InspectorPanel {
    pub fn show(
        ui: &mut egui::Ui,
        world: &mut World,
        selection: &Selection,
        history: &mut CommandHistory,
    ) -> bool {
        let mut pending_commands: Vec<Box<dyn EditorCommand>> = Vec::new();

        with_inspector_registries(
            world,
            |world, type_registry, component_registry, metadata_registry| {
            let Some(entity) = selection.primary() else {
                ui.centered_and_justified(|ui| {
                    ui.label("No entity selected");
                });
                return;
            };

            if world.get_entity(entity).is_err() {
                ui.centered_and_justified(|ui| {
                    ui.label("Selection no longer exists");
                });
                return;
            }

            Self::draw_entity_name_editor(
                ui,
                world,
                entity,
                component_registry,
                &mut pending_commands,
            );

            ui.separator();

            for descriptor in component_registry.all() {
                if !descriptor.has(entity, world) {
                    continue;
                }

                let component_name = descriptor.name;
                let header = short_type_name(component_name);

                egui::CollapsingHeader::new(header)
                    .id_salt(("inspector_component", component_name))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .small_button("✕")
                                .on_hover_text("Remove component")
                                .clicked()
                            {
                                pending_commands.push(Box::new(RemoveComponentCommand::new(
                                    entity,
                                    component_name,
                                )));
                            }
                        });

                        if let Some(component) = descriptor.get_reflect(entity, world) {
                            Self::draw_reflect(
                                ui,
                                entity,
                                descriptor,
                                type_registry,
                                metadata_registry,
                                component.as_partial_reflect(),
                                "",
                                &mut pending_commands,
                            );
                        }
                    });
            }

            ui.separator();
            ui.menu_button("+ Add Component", |ui| {
                for descriptor in component_registry.all() {
                    if descriptor.has(entity, world) {
                        continue;
                    }

                    let display = short_type_name(descriptor.name);
                    if ui.button(display).clicked() {
                        pending_commands.push(Box::new(AddComponentCommand::new(entity, descriptor.name)));
                        ui.close_menu();
                    }
                }
            });
            },
        );

        if pending_commands.is_empty() {
            return false;
        }

        for command in pending_commands {
            let _ = history.execute(command, world);
        }

        true
    }

    fn draw_entity_name_editor(
        ui: &mut egui::Ui,
        world: &World,
        entity: Entity,
        component_registry: &ComponentRegistry,
        pending_commands: &mut Vec<Box<dyn EditorCommand>>,
    ) {
        let Some(descriptor) = Self::find_component(world, entity, component_registry, "EntityName") else {
            return;
        };

        let Some(entity_name) = world.get::<EntityName>(entity) else {
            return;
        };

        let mut edited_name = entity_name.0.clone();
        ui.horizontal(|ui| {
            ui.label("EntityName");
            if ui.text_edit_singleline(&mut edited_name).changed() {
                pending_commands.push(Box::new(SetFieldCommand {
                    entity,
                    component_name: descriptor.name.to_owned(),
                    field_path: "0".to_owned(),
                    old_value: Box::new(entity_name.0.clone()),
                    new_value: Box::new(edited_name.clone()),
                    desc: "Rename entity".to_owned(),
                }));
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_reflect(
        ui: &mut egui::Ui,
        entity: Entity,
        descriptor: &ComponentDescriptor,
        type_registry: &ReflectTypeRegistry,
        metadata_registry: &ReflectMetadataRegistry,
        value: &dyn PartialReflect,
        field_path: &str,
        pending_commands: &mut Vec<Box<dyn EditorCommand>>,
    ) {
        match value.reflect_ref() {
            ReflectRef::Struct(struct_reflect) => {
                for index in 0..struct_reflect.field_len() {
                    let Some(field_name) = struct_reflect.name_at(index) else {
                        continue;
                    };
                    let Some(field_value) = struct_reflect.field_at(index) else {
                        continue;
                    };

                    let full_path = if field_path.is_empty() {
                        field_name.to_owned()
                    } else {
                        format!("{}.{}", field_path, field_name)
                    };

                    if metadata_registry.is_hidden_by_type_id(descriptor.type_id, &full_path) {
                        continue;
                    }

                    Self::draw_field_row(
                        ui,
                        entity,
                        descriptor,
                        type_registry,
                        metadata_registry,
                        field_name,
                        field_value,
                        &full_path,
                        pending_commands,
                    );
                }
            }
            ReflectRef::TupleStruct(tuple_struct) => {
                for index in 0..tuple_struct.field_len() {
                    let Some(field_value) = tuple_struct.field(index) else {
                        continue;
                    };

                    let full_path = if field_path.is_empty() {
                        index.to_string()
                    } else {
                        format!("{}.{}", field_path, index)
                    };

                    Self::draw_field_row(
                        ui,
                        entity,
                        descriptor,
                        type_registry,
                        metadata_registry,
                        &index.to_string(),
                        field_value,
                        &full_path,
                        pending_commands,
                    );
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_field_row(
        ui: &mut egui::Ui,
        entity: Entity,
        descriptor: &ComponentDescriptor,
        type_registry: &ReflectTypeRegistry,
        metadata_registry: &ReflectMetadataRegistry,
        field_label: &str,
        field_value: &dyn PartialReflect,
        full_path: &str,
        pending_commands: &mut Vec<Box<dyn EditorCommand>>,
    ) {
        let hint = metadata_registry.hint_by_type_id(descriptor.type_id, full_path);
        let read_only = matches!(hint, Some(EditorHint::ReadOnly));

        let mut committed = false;

        ui.horizontal(|ui| {
            ui.label(field_label);
            ui.add_enabled_ui(!read_only, |ui| {
                if let Some(current) = field_value.try_downcast_ref::<bool>() {
                    let mut edited = *current;
                    if ui.checkbox(&mut edited, "").changed() {
                        committed = true;
                        pending_commands.push(Box::new(SetFieldCommand {
                            entity,
                            component_name: descriptor.name.to_owned(),
                            field_path: full_path.to_owned(),
                            old_value: field_value.clone_value(),
                            new_value: Box::new(edited),
                            desc: format!("Set {}.{}", short_type_name(descriptor.name), full_path),
                        }));
                    }
                    return;
                }

                if let Some(current) = field_value.try_downcast_ref::<f32>() {
                    let mut edited = *current;
                    let changed = match hint {
                        Some(EditorHint::Range { min, max }) => {
                            ui.add(egui::Slider::new(&mut edited, min as f32..=max as f32)).changed()
                        }
                        Some(EditorHint::Degrees) => {
                            let mut degrees = edited.to_degrees();
                            let changed = ui.add(egui::DragValue::new(&mut degrees).suffix("°")).changed();
                            edited = degrees.to_radians();
                            changed
                        }
                        _ => ui.add(egui::DragValue::new(&mut edited).speed(0.01)).changed(),
                    };

                    if changed {
                        committed = true;
                        pending_commands.push(Box::new(SetFieldCommand {
                            entity,
                            component_name: descriptor.name.to_owned(),
                            field_path: full_path.to_owned(),
                            old_value: field_value.clone_value(),
                            new_value: Box::new(edited),
                            desc: format!("Set {}.{}", short_type_name(descriptor.name), full_path),
                        }));
                    }
                    return;
                }

                if let Some(current) = field_value.try_downcast_ref::<f64>() {
                    let mut edited = *current;
                    if ui.add(egui::DragValue::new(&mut edited).speed(0.01)).changed() {
                        committed = true;
                        pending_commands.push(Box::new(SetFieldCommand {
                            entity,
                            component_name: descriptor.name.to_owned(),
                            field_path: full_path.to_owned(),
                            old_value: field_value.clone_value(),
                            new_value: Box::new(edited),
                            desc: format!("Set {}.{}", short_type_name(descriptor.name), full_path),
                        }));
                    }
                    return;
                }

                if let Some(current) = field_value.try_downcast_ref::<i32>() {
                    let mut edited = *current;
                    if ui.add(egui::DragValue::new(&mut edited).speed(1.0)).changed() {
                        committed = true;
                        pending_commands.push(Box::new(SetFieldCommand {
                            entity,
                            component_name: descriptor.name.to_owned(),
                            field_path: full_path.to_owned(),
                            old_value: field_value.clone_value(),
                            new_value: Box::new(edited),
                            desc: format!("Set {}.{}", short_type_name(descriptor.name), full_path),
                        }));
                    }
                    return;
                }

                if let Some(current) = field_value.try_downcast_ref::<u32>() {
                    let mut edited = *current;
                    if ui.add(egui::DragValue::new(&mut edited).speed(1.0)).changed() {
                        committed = true;
                        pending_commands.push(Box::new(SetFieldCommand {
                            entity,
                            component_name: descriptor.name.to_owned(),
                            field_path: full_path.to_owned(),
                            old_value: field_value.clone_value(),
                            new_value: Box::new(edited),
                            desc: format!("Set {}.{}", short_type_name(descriptor.name), full_path),
                        }));
                    }
                    return;
                }

                if let Some(current) = field_value.try_downcast_ref::<String>() {
                    let multiline = matches!(hint, Some(EditorHint::Multiline));
                    let mut edited = current.clone();
                    let response = if multiline {
                        ui.text_edit_multiline(&mut edited)
                    } else {
                        ui.text_edit_singleline(&mut edited)
                    };

                    if response.changed() {
                        committed = true;
                        pending_commands.push(Box::new(SetFieldCommand {
                            entity,
                            component_name: descriptor.name.to_owned(),
                            field_path: full_path.to_owned(),
                            old_value: field_value.clone_value(),
                            new_value: Box::new(edited),
                            desc: format!("Set {}.{}", short_type_name(descriptor.name), full_path),
                        }));
                    }
                    return;
                }

                if let ReflectRef::Enum(enum_reflect) = field_value.reflect_ref() {
                    let mut selected_variant = enum_reflect.variant_name().to_owned();
                    egui::ComboBox::from_id_salt(("enum_variant", descriptor.name, full_path))
                        .selected_text(&selected_variant)
                        .show_ui(ui, |ui| {
                            if let Some(TypeInfo::Enum(enum_info)) = field_value.get_represented_type_info() {
                                for variant in enum_info.variant_names() {
                                    ui.selectable_value(&mut selected_variant, (*variant).to_owned(), *variant);
                                }
                            }
                        });

                    if selected_variant != enum_reflect.variant_name() {
                        if let Some(new_enum) =
                            build_enum_variant_update(field_value, type_registry, &selected_variant)
                        {
                            committed = true;
                            pending_commands.push(Box::new(SetFieldCommand {
                                entity,
                                component_name: descriptor.name.to_owned(),
                                field_path: full_path.to_owned(),
                                old_value: field_value.clone_value(),
                                new_value: new_enum,
                                desc: format!("Set {}.{}", short_type_name(descriptor.name), full_path),
                            }));
                        }
                    }
                }
            });
        });

        if committed {
            return;
        }

        match field_value.reflect_ref() {
            ReflectRef::Struct(_) | ReflectRef::TupleStruct(_) => {
                ui.indent(("nested", descriptor.name, full_path), |ui| {
                    Self::draw_reflect(
                        ui,
                        entity,
                        descriptor,
                        type_registry,
                        metadata_registry,
                        field_value,
                        full_path,
                        pending_commands,
                    );
                });
            }
            ReflectRef::Enum(enum_reflect) => {
                match enum_reflect.variant_type() {
                    VariantType::Struct | VariantType::Tuple => {
                        ui.indent(("enum_fields", descriptor.name, full_path), |ui| {
                            for index in 0..enum_reflect.field_len() {
                                let Some(inner) = enum_reflect.field_at(index) else {
                                    continue;
                                };
                                let inner_label = enum_reflect
                                    .name_at(index)
                                    .map(ToOwned::to_owned)
                                    .unwrap_or_else(|| index.to_string());
                                let inner_path = format!("{}.{}", full_path, inner_label);

                                Self::draw_field_row(
                                    ui,
                                    entity,
                                    descriptor,
                                    type_registry,
                                    metadata_registry,
                                    &inner_label,
                                    inner,
                                    &inner_path,
                                    pending_commands,
                                );
                            }
                        });
                    }
                    VariantType::Unit => {}
                }
            }
            _ => {}
        }
    }

    fn find_component<'a>(
        world: &'a World,
        entity: Entity,
        component_registry: &'a ComponentRegistry,
        display_name: &str,
    ) -> Option<&'a ComponentDescriptor> {
        component_registry.all().iter().find_map(|descriptor| {
            if short_type_name(descriptor.name) != display_name {
                return None;
            }

            descriptor.get_reflect(entity, world).map(|_| descriptor)
        })
    }
}

const MAX_DEFAULT_BUILD_DEPTH: usize = 16;

fn build_enum_variant_update(
    current: &dyn PartialReflect,
    type_registry: &ReflectTypeRegistry,
    variant_name: &str,
) -> Option<Box<dyn PartialReflect>> {
    let Some(TypeInfo::Enum(enum_info)) = current.get_represented_type_info() else {
        return None;
    };

    build_dynamic_enum_variant(enum_info, variant_name, type_registry, 0)
}

fn build_dynamic_enum_variant(
    enum_info: &engine_reflect::bevy_reflect::EnumInfo,
    variant_name: &str,
    type_registry: &ReflectTypeRegistry,
    depth: usize,
) -> Option<Box<dyn PartialReflect>> {
    if depth > MAX_DEFAULT_BUILD_DEPTH {
        return None;
    }

    let variant = enum_info.variant(variant_name)?;

    match variant {
        VariantInfo::Unit(_) => Some(Box::new(DynamicEnum::new(variant_name.to_owned(), ()))),
        VariantInfo::Tuple(tuple_variant) => {
            let mut payload = DynamicTuple::default();
            for field in tuple_variant.iter() {
                let value = build_default_value(
                    field.type_info(),
                    field.type_id(),
                    field.type_path(),
                    type_registry,
                    depth + 1,
                )?;
                payload.insert_boxed(value);
            }

            Some(Box::new(DynamicEnum::new(variant_name.to_owned(), payload)))
        }
        VariantInfo::Struct(struct_variant) => {
            let mut payload = DynamicStruct::default();
            for field in struct_variant.iter() {
                let value = build_default_value(
                    field.type_info(),
                    field.type_id(),
                    field.type_path(),
                    type_registry,
                    depth + 1,
                )?;
                payload.insert_boxed(field.name(), value);
            }

            Some(Box::new(DynamicEnum::new(variant_name.to_owned(), payload)))
        }
    }
}

fn build_default_value(
    type_info: Option<&'static TypeInfo>,
    type_id: TypeId,
    type_path: &str,
    type_registry: &ReflectTypeRegistry,
    depth: usize,
) -> Option<Box<dyn PartialReflect>> {
    build_default_from_registry(type_id, type_registry, depth)
        .or_else(|| {
            type_info.and_then(|info| build_dynamic_default_from_type_info(info, type_registry, depth))
        })
        .or_else(|| primitive_default_for_type_path(type_path))
}

fn build_default_from_registry(
    type_id: TypeId,
    type_registry: &ReflectTypeRegistry,
    depth: usize,
) -> Option<Box<dyn PartialReflect>> {
    if depth > MAX_DEFAULT_BUILD_DEPTH {
        return None;
    }

    let registration = type_registry.read().get(type_id)?;

    if let Some(reflect_default) = registration.data::<ReflectDefault>() {
        let default_reflect = reflect_default.default();
        return Some(default_reflect.as_partial_reflect().clone_value());
    }

    build_dynamic_default_from_type_info(registration.type_info(), type_registry, depth + 1)
}

fn build_dynamic_default_from_type_info(
    type_info: &TypeInfo,
    type_registry: &ReflectTypeRegistry,
    depth: usize,
) -> Option<Box<dyn PartialReflect>> {
    if depth > MAX_DEFAULT_BUILD_DEPTH {
        return None;
    }

    match type_info {
        TypeInfo::Struct(struct_info) => {
            let mut dynamic = DynamicStruct::default();
            for field in struct_info.iter() {
                let value = build_default_value(
                    field.type_info(),
                    field.type_id(),
                    field.type_path(),
                    type_registry,
                    depth + 1,
                )?;
                dynamic.insert_boxed(field.name(), value);
            }
            Some(Box::new(dynamic))
        }
        TypeInfo::TupleStruct(tuple_struct_info) => {
            let mut dynamic = DynamicTupleStruct::default();
            for field in tuple_struct_info.iter() {
                let value = build_default_value(
                    field.type_info(),
                    field.type_id(),
                    field.type_path(),
                    type_registry,
                    depth + 1,
                )?;
                dynamic.insert_boxed(value);
            }
            Some(Box::new(dynamic))
        }
        TypeInfo::Tuple(tuple_info) => {
            let mut dynamic = DynamicTuple::default();
            for field in tuple_info.iter() {
                let value = build_default_value(
                    field.type_info(),
                    field.type_id(),
                    field.type_path(),
                    type_registry,
                    depth + 1,
                )?;
                dynamic.insert_boxed(value);
            }
            Some(Box::new(dynamic))
        }
        TypeInfo::Enum(enum_info) => {
            let first_variant = enum_info.iter().next()?;
            build_dynamic_enum_variant(enum_info, first_variant.name(), type_registry, depth + 1)
        }
        TypeInfo::List(_) => Some(Box::new(DynamicList::default())),
        TypeInfo::Array(array_info) => {
            let mut values = Vec::with_capacity(array_info.capacity());
            for _ in 0..array_info.capacity() {
                let value = build_default_value(
                    array_info.item_info(),
                    array_info.item_ty().id(),
                    array_info.item_ty().path(),
                    type_registry,
                    depth + 1,
                )?;
                values.push(value);
            }

            Some(Box::new(DynamicArray::new(values.into_boxed_slice())))
        }
        TypeInfo::Map(_) => Some(Box::new(DynamicMap::default())),
        TypeInfo::Set(_) => Some(Box::new(DynamicSet::default())),
        TypeInfo::Opaque(_) => primitive_default_for_type_path(type_info.type_path()),
    }
}

fn primitive_default_for_type_path(type_path: &str) -> Option<Box<dyn PartialReflect>> {
    match type_path {
        "bool" => Some(Box::new(false)),
        "char" => Some(Box::new('\0')),
        "f32" => Some(Box::new(0.0f32)),
        "f64" => Some(Box::new(0.0f64)),
        "i8" => Some(Box::new(0_i8)),
        "i16" => Some(Box::new(0_i16)),
        "i32" => Some(Box::new(0_i32)),
        "i64" => Some(Box::new(0_i64)),
        "i128" => Some(Box::new(0_i128)),
        "isize" => Some(Box::new(0_isize)),
        "u8" => Some(Box::new(0_u8)),
        "u16" => Some(Box::new(0_u16)),
        "u32" => Some(Box::new(0_u32)),
        "u64" => Some(Box::new(0_u64)),
        "u128" => Some(Box::new(0_u128)),
        "usize" => Some(Box::new(0_usize)),
        "alloc::string::String" | "std::string::String" | "String" => {
            Some(Box::new(String::new()))
        }
        _ if type_path.starts_with("core::option::Option<")
            || type_path.starts_with("std::option::Option<") =>
        {
            Some(Box::new(DynamicEnum::new("None".to_owned(), ())))
        }
        _ if type_path.starts_with("alloc::vec::Vec<")
            || type_path.starts_with("std::vec::Vec<") =>
        {
            Some(Box::new(DynamicList::default()))
        }
        _ => None,
    }
}

fn with_inspector_registries<R>(
    world: &mut World,
    f: impl FnOnce(
        &mut World,
        &ReflectTypeRegistry,
        &ComponentRegistry,
        &ReflectMetadataRegistry,
    ) -> R,
) -> R {
    let type_registry = world
        .remove_resource::<ReflectTypeRegistry>()
        .unwrap_or_default();
    let component_registry = world
        .remove_resource::<ComponentRegistry>()
        .unwrap_or_default();
    let metadata_registry = world
        .remove_resource::<ReflectMetadataRegistry>()
        .unwrap_or_default();

    let result = f(world, &type_registry, &component_registry, &metadata_registry);

    world.insert_resource(type_registry);
    world.insert_resource(component_registry);
    world.insert_resource(metadata_registry);

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_reflect::bevy_reflect;

    #[derive(engine_reflect::bevy_reflect::Reflect, Clone, Debug)]
    enum EnumWithPayload {
        Unit,
        Tuple(f32, bool),
        Struct { label: String, enabled: bool },
    }

    #[derive(engine_reflect::bevy_reflect::Reflect, Clone, Debug)]
    struct NestedPayload {
        x: f32,
        y: f32,
    }

    #[derive(engine_reflect::bevy_reflect::Reflect, Clone, Debug)]
    enum EnumWithNestedPayload {
        Unit,
        Struct { nested: NestedPayload },
    }

    #[test]
    fn build_enum_variant_update_supports_tuple_payload_variants() {
        let registry = ReflectTypeRegistry::default();
        let current = EnumWithPayload::Unit;

        let updated = build_enum_variant_update(&current, &registry, "Tuple")
            .expect("expected tuple payload variant to be built");

        let ReflectRef::Enum(enum_reflect) = updated.reflect_ref() else {
            panic!("updated payload should be an enum");
        };

        assert_eq!(enum_reflect.variant_name(), "Tuple");
        assert_eq!(enum_reflect.field_len(), 2);

        let first = enum_reflect
            .field_at(0)
            .and_then(|value| value.try_downcast_ref::<f32>())
            .copied();
        let second = enum_reflect
            .field_at(1)
            .and_then(|value| value.try_downcast_ref::<bool>())
            .copied();

        assert_eq!(first, Some(0.0));
        assert_eq!(second, Some(false));
    }

    #[test]
    fn build_enum_variant_update_supports_struct_payload_variants() {
        let registry = ReflectTypeRegistry::default();
        let current = EnumWithPayload::Unit;

        let updated = build_enum_variant_update(&current, &registry, "Struct")
            .expect("expected struct payload variant to be built");

        let ReflectRef::Enum(enum_reflect) = updated.reflect_ref() else {
            panic!("updated payload should be an enum");
        };

        assert_eq!(enum_reflect.variant_name(), "Struct");

        let label = enum_reflect
            .field("label")
            .and_then(|value| value.try_downcast_ref::<String>());
        let enabled = enum_reflect
            .field("enabled")
            .and_then(|value| value.try_downcast_ref::<bool>());

        assert_eq!(label.map(String::as_str), Some(""));
        assert_eq!(enabled.copied(), Some(false));
    }

    #[test]
    fn build_enum_variant_update_builds_nested_struct_defaults() {
        let registry = ReflectTypeRegistry::default();
        let current = EnumWithNestedPayload::Unit;

        let updated = build_enum_variant_update(&current, &registry, "Struct")
            .expect("expected nested struct payload variant to be built");

        let ReflectRef::Enum(enum_reflect) = updated.reflect_ref() else {
            panic!("updated payload should be an enum");
        };

        let nested = enum_reflect.field("nested").expect("nested field should exist");
        let ReflectRef::Struct(struct_reflect) = nested.reflect_ref() else {
            panic!("nested payload should be a struct");
        };

        let x = struct_reflect
            .field("x")
            .and_then(|value| value.try_downcast_ref::<f32>())
            .copied();
        let y = struct_reflect
            .field("y")
            .and_then(|value| value.try_downcast_ref::<f32>())
            .copied();

        assert_eq!(x, Some(0.0));
        assert_eq!(y, Some(0.0));
    }

    #[test]
    fn build_enum_variant_update_returns_none_for_unknown_variant() {
        let registry = ReflectTypeRegistry::default();
        let current = EnumWithPayload::Unit;

        let updated = build_enum_variant_update(&current, &registry, "Missing");
        assert!(updated.is_none());
    }
}
