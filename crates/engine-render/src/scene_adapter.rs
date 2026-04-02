use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use engine_assets::{AssetServer, SceneExternalComponents, SceneValue};
use engine_core::Result;

use crate::{MeshRenderable3d, SpriteRenderable2d};

pub struct RenderSceneAdapter;

impl SceneExternalComponents for RenderSceneAdapter {
    fn serialize_entity_components(
        &self,
        world: &World,
        entity: Entity,
        asset_server: &AssetServer,
        out: &mut std::collections::HashMap<String, SceneValue>,
    ) -> Result<()> {
        if let Some(mesh_renderer) = world.get::<MeshRenderable3d>(entity) {
            let Some(mesh_path) = asset_server.mesh_relative_path(mesh_renderer.mesh) else {
                log::warn!(
                    target: "engine::assets",
                    "Skipping MeshRenderer serialization for entity {:?}: unresolved mesh handle",
                    entity
                );
                return Ok(());
            };

            let Some(texture_path) = asset_server.texture_relative_path(mesh_renderer.texture)
            else {
                log::warn!(
                    target: "engine::assets",
                    "Skipping MeshRenderer serialization for entity {:?}: unresolved texture handle",
                    entity
                );
                return Ok(());
            };

            let Some(material_path) = asset_server.material_relative_path(mesh_renderer.material)
            else {
                log::warn!(
                    target: "engine::assets",
                    "Skipping MeshRenderer serialization for entity {:?}: unresolved material handle",
                    entity
                );
                return Ok(());
            };

            let value = map_value(vec![
                ("mesh", SceneValue::String(mesh_path)),
                ("texture", SceneValue::String(texture_path)),
                ("material", SceneValue::String(material_path)),
            ]);
            out.insert("MeshRenderer".to_owned(), value);
        }

        if let Some(sprite) = world.get::<SpriteRenderable2d>(entity) {
            let Some(texture_path) = asset_server.texture_relative_path(sprite.texture) else {
                log::warn!(
                    target: "engine::assets",
                    "Skipping Sprite serialization for entity {:?}: unresolved texture handle",
                    entity
                );
                return Ok(());
            };

            let value = map_value(vec![
                ("texture", SceneValue::String(texture_path)),
                ("size", vec2_value(sprite.size)),
                ("color", vec4_value(sprite.color)),
                ("uv_min", vec2_value(sprite.uv_min)),
                ("uv_max", vec2_value(sprite.uv_max)),
            ]);
            out.insert("Sprite".to_owned(), value);
        }

        Ok(())
    }

    fn deserialize_entity_component(
        &self,
        world: &mut World,
        entity: Entity,
        component_name: &str,
        component_value: &SceneValue,
        asset_server: &mut AssetServer,
    ) -> Result<bool> {
        match component_name {
            "MeshRenderer" | "MeshRenderable3d" => {
                let Some(mesh_path) = map_get_string(component_value, "mesh") else {
                    log::warn!(
                        target: "engine::assets",
                        "MeshRenderer payload on entity {:?} is missing `mesh`; skipping",
                        entity
                    );
                    return Ok(true);
                };
                let Some(texture_path) = map_get_string(component_value, "texture") else {
                    log::warn!(
                        target: "engine::assets",
                        "MeshRenderer payload on entity {:?} is missing `texture`; skipping",
                        entity
                    );
                    return Ok(true);
                };
                let Some(material_path) = map_get_string(component_value, "material") else {
                    log::warn!(
                        target: "engine::assets",
                        "MeshRenderer payload on entity {:?} is missing `material`; skipping",
                        entity
                    );
                    return Ok(true);
                };

                let mesh = match asset_server.load_mesh_handle(mesh_path) {
                    Ok(handle) => handle,
                    Err(error) => {
                        log::warn!(
                            target: "engine::assets",
                            "Failed to load mesh '{}' for entity {:?}: {}",
                            mesh_path,
                            entity,
                            error
                        );
                        return Ok(true);
                    }
                };
                let texture = match asset_server.load_texture_handle(texture_path) {
                    Ok(handle) => handle,
                    Err(error) => {
                        log::warn!(
                            target: "engine::assets",
                            "Failed to load texture '{}' for entity {:?}: {}",
                            texture_path,
                            entity,
                            error
                        );
                        return Ok(true);
                    }
                };
                let material = match asset_server.load_material_handle(material_path) {
                    Ok(handle) => handle,
                    Err(error) => {
                        log::warn!(
                            target: "engine::assets",
                            "Failed to load material '{}' for entity {:?}: {}",
                            material_path,
                            entity,
                            error
                        );
                        return Ok(true);
                    }
                };

                if let Ok(mut entity_ref) = world.get_entity_mut(entity) {
                    entity_ref.insert(MeshRenderable3d::new(mesh, texture, material));
                }

                Ok(true)
            }
            "Sprite" | "SpriteRenderable2d" => {
                let Some(texture_path) = map_get_string(component_value, "texture") else {
                    log::warn!(
                        target: "engine::assets",
                        "Sprite payload on entity {:?} is missing `texture`; skipping",
                        entity
                    );
                    return Ok(true);
                };

                let texture = match asset_server.load_texture_handle(texture_path) {
                    Ok(handle) => handle,
                    Err(error) => {
                        log::warn!(
                            target: "engine::assets",
                            "Failed to load sprite texture '{}' for entity {:?}: {}",
                            texture_path,
                            entity,
                            error
                        );
                        return Ok(true);
                    }
                };

                let mut sprite = SpriteRenderable2d::new(texture);

                if let Some(size) = map_get_vec2(component_value, "size") {
                    sprite = sprite.with_size(size[0], size[1]);
                }
                if let Some(color) = map_get_vec4(component_value, "color") {
                    sprite = sprite.with_color(color);
                }
                if let (Some(uv_min), Some(uv_max)) = (
                    map_get_vec2(component_value, "uv_min"),
                    map_get_vec2(component_value, "uv_max"),
                ) {
                    sprite = sprite.with_uv_rect(uv_min, uv_max);
                }

                if let Ok(mut entity_ref) = world.get_entity_mut(entity) {
                    entity_ref.insert(sprite);
                }

                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

fn map_value(entries: Vec<(&str, SceneValue)>) -> SceneValue {
    let mut map = ron::Map::new();
    for (key, value) in entries {
        map.insert(SceneValue::String(key.to_owned()), value);
    }
    SceneValue::Map(map)
}

fn vec2_value(value: [f32; 2]) -> SceneValue {
    SceneValue::Seq(vec![
        SceneValue::Number(ron::Number::new(value[0] as f64)),
        SceneValue::Number(ron::Number::new(value[1] as f64)),
    ])
}

fn vec4_value(value: [f32; 4]) -> SceneValue {
    SceneValue::Seq(vec![
        SceneValue::Number(ron::Number::new(value[0] as f64)),
        SceneValue::Number(ron::Number::new(value[1] as f64)),
        SceneValue::Number(ron::Number::new(value[2] as f64)),
        SceneValue::Number(ron::Number::new(value[3] as f64)),
    ])
}

fn map_get<'a>(value: &'a SceneValue, key: &str) -> Option<&'a SceneValue> {
    let SceneValue::Map(map) = value else {
        return None;
    };

    map.iter().find_map(|(map_key, map_value)| {
        let SceneValue::String(current) = map_key else {
            return None;
        };

        if current == key {
            return Some(map_value);
        }

        None
    })
}

fn map_get_string<'a>(value: &'a SceneValue, key: &str) -> Option<&'a str> {
    let SceneValue::String(value) = map_get(value, key)? else {
        return None;
    };

    Some(value.as_str())
}

fn map_get_vec2(value: &SceneValue, key: &str) -> Option<[f32; 2]> {
    let SceneValue::Seq(values) = map_get(value, key)? else {
        return None;
    };

    if values.len() != 2 {
        return None;
    }

    Some([
        scene_number_to_f32(&values[0])?,
        scene_number_to_f32(&values[1])?,
    ])
}

fn map_get_vec4(value: &SceneValue, key: &str) -> Option<[f32; 4]> {
    let SceneValue::Seq(values) = map_get(value, key)? else {
        return None;
    };

    if values.len() != 4 {
        return None;
    }

    Some([
        scene_number_to_f32(&values[0])?,
        scene_number_to_f32(&values[1])?,
        scene_number_to_f32(&values[2])?,
        scene_number_to_f32(&values[3])?,
    ])
}

fn scene_number_to_f32(value: &SceneValue) -> Option<f32> {
    let SceneValue::Number(value) = value else {
        return None;
    };

    Some((*value).into_f64() as f32)
}
