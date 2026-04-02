use std::collections::HashMap;

use bevy_ecs::world::World;
use engine_assets::{AssetServer, SceneExternalComponents, SceneValue};

use crate::{MeshRenderable3d, RenderSceneAdapter, SpriteRenderable2d};

#[test]
fn render_scene_adapter_serializes_and_deserializes_mesh_and_sprite_components() {
    let assets_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets");
    let mut asset_server = AssetServer::new(assets_root.to_string_lossy().to_string());
    let texture = asset_server
        .load_texture_handle("textures/placeholder.png")
        .expect("texture should load");
    let mesh = asset_server
        .load_mesh_handle("meshes/cube.glb")
        .expect("mesh should load");
    let material = asset_server
        .load_material_handle("materials/default.ron")
        .expect("material should load");

    let mut world = World::new();
    let entity = world
        .spawn((
            MeshRenderable3d::new(mesh, texture, material),
            SpriteRenderable2d::new(texture)
                .with_size(2.0, 3.0)
                .with_color([0.25, 0.5, 0.75, 1.0]),
        ))
        .id();

    let adapter = RenderSceneAdapter;
    let mut serialized_components = HashMap::new();
    adapter
        .serialize_entity_components(&world, entity, &asset_server, &mut serialized_components)
        .expect("render components should serialize");

    assert!(serialized_components.contains_key("MeshRenderer"));
    assert!(serialized_components.contains_key("Sprite"));

    let mut loaded_world = World::new();
    let loaded_entity = loaded_world.spawn_empty().id();
    for (component_name, component_value) in &serialized_components {
        let handled = adapter
            .deserialize_entity_component(
                &mut loaded_world,
                loaded_entity,
                component_name,
                component_value,
                &mut asset_server,
            )
            .expect("component should deserialize");
        assert!(handled);
    }

    let loaded_mesh_renderer = loaded_world
        .get::<MeshRenderable3d>(loaded_entity)
        .expect("mesh renderer should be present");
    assert_eq!(loaded_mesh_renderer.mesh.id(), mesh.id());
    assert_eq!(loaded_mesh_renderer.texture.id(), texture.id());
    assert_eq!(loaded_mesh_renderer.material.id(), material.id());

    let loaded_sprite = loaded_world
        .get::<SpriteRenderable2d>(loaded_entity)
        .expect("sprite should be present");
    assert_eq!(loaded_sprite.texture.id(), texture.id());
    assert_eq!(loaded_sprite.size, [2.0, 3.0]);
    assert_eq!(loaded_sprite.color, [0.25, 0.5, 0.75, 1.0]);
}

#[test]
fn render_scene_adapter_skips_mesh_renderer_with_missing_required_field() {
    let mut world = World::new();
    let entity = world.spawn_empty().id();
    let mut asset_server = AssetServer::new("assets");
    let adapter = RenderSceneAdapter;

    let payload = map_value(vec![
        (
            "texture",
            SceneValue::String("textures/placeholder.png".to_owned()),
        ),
        (
            "material",
            SceneValue::String("materials/default.ron".to_owned()),
        ),
    ]);

    let handled = adapter
        .deserialize_entity_component(
            &mut world,
            entity,
            "MeshRenderer",
            &payload,
            &mut asset_server,
        )
        .expect("payload should be handled without failing");

    assert!(handled);
    assert!(world.get::<MeshRenderable3d>(entity).is_none());
}

#[test]
fn render_scene_adapter_skips_sprite_when_texture_asset_is_missing() {
    let mut world = World::new();
    let entity = world.spawn_empty().id();
    let mut asset_server = AssetServer::new("assets");
    let adapter = RenderSceneAdapter;

    let payload = map_value(vec![
        (
            "texture",
            SceneValue::String("textures/does-not-exist.png".to_owned()),
        ),
        (
            "size",
            SceneValue::Seq(vec![
                SceneValue::Number(ron::Number::new(64.0)),
                SceneValue::Number(ron::Number::new(64.0)),
            ]),
        ),
    ]);

    let handled = adapter
        .deserialize_entity_component(&mut world, entity, "Sprite", &payload, &mut asset_server)
        .expect("missing texture should not fail scene loading");

    assert!(handled);
    assert!(world.get::<SpriteRenderable2d>(entity).is_none());
}

fn map_value(entries: Vec<(&str, SceneValue)>) -> SceneValue {
    let mut map = ron::Map::new();
    for (key, value) in entries {
        map.insert(SceneValue::String(key.to_owned()), value);
    }
    SceneValue::Map(map)
}
