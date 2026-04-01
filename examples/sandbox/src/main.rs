use bevy_ecs::prelude::{Commands, Query, Res, Resource, With};
use bevy_ecs::world::World;
use engine_assets::{AssetModule, MaterialHandle, MeshHandle, TextureHandle};
use engine_audio::AudioModule;
use engine_core::{
    self, Camera2d, Camera3d, Children, Engine, EngineConfig, EngineModules, Parent, Plugin,
    PrimaryCamera, RenderLayer2D, RenderLayer3D, Result, SpatialBundle, Transform, Visible,
    WindowConfig,
};
use engine_input::InputModule;
use engine_physics::PhysicsModule;
use engine_render::{MeshRenderable3d, RenderModule, SpriteRenderable2d};
use std::sync::{Arc, Once};
use winit::window::Window;

static ECS_UPDATE_LOG_ONCE: Once = Once::new();

#[derive(Resource, Clone, Copy)]
struct SandboxRenderAssets {
    mesh: MeshHandle,
    texture: TextureHandle,
    material: MaterialHandle,
}

fn ecs_startup_smoke() {
    log::info!(target: "engine::sandbox", "ECS Startup schedule executed");
}

fn ecs_spawn_smoke_entities(mut commands: Commands) {
    let parent = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_xyz(0.0, 1.0, 0.0),
                ..SpatialBundle::default()
            },
            Children::default(),
            Visible,
            RenderLayer3D,
        ))
        .id();

    let child = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_xyz(1.0, 0.0, 0.0),
                ..SpatialBundle::default()
            },
            Parent(parent),
            Visible,
            RenderLayer3D,
        ))
        .id();

    commands.entity(parent).insert(Children(vec![child]));

    commands.spawn((
        SpatialBundle::default(),
        Camera3d::default(),
        PrimaryCamera,
        Visible,
    ));

    commands.spawn((
        SpatialBundle::default(),
        Camera2d::default(),
        PrimaryCamera,
        Visible,
    ));

    log::info!(
        target: "engine::sandbox",
        "ECS smoke entities spawned (hierarchy + 3D/2D cameras)"
    );
}

fn ecs_spawn_renderable_entity(
    mut commands: Commands,
    render_assets: Option<Res<SandboxRenderAssets>>,
) {
    let Some(render_assets) = render_assets else {
        log::warn!(
            target: "engine::sandbox",
            "Skipping 3D renderable spawn because demo assets are unavailable"
        );
        return;
    };

    let render_assets = *render_assets;
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(0.0, 0.0, -3.0),
            ..SpatialBundle::default()
        },
        MeshRenderable3d::new(render_assets.mesh, render_assets.texture, render_assets.material),
        Visible,
        RenderLayer3D,
    ));

    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(420.0, -220.0, 0.0),
            ..SpatialBundle::default()
        },
        SpriteRenderable2d::new(render_assets.texture)
            .with_size(256.0, 256.0)
            .with_color([1.0, 1.0, 1.0, 0.92]),
        Visible,
        RenderLayer2D,
    ));

    log::info!(
        target: "engine::sandbox",
        "Spawned 3D mesh and 2D sprite entities for EP-04 render validation"
    );
}

fn ecs_update_smoke(
    primary_cameras: Query<(), With<PrimaryCamera>>,
    parented_entities: Query<(), With<Parent>>,
) {
    ECS_UPDATE_LOG_ONCE.call_once(|| {
        let camera_count = primary_cameras.iter().count();
        let parented_count = parented_entities.iter().count();

        log::info!(
            target: "engine::sandbox",
            "ECS Update schedule executed (primary_cameras={}, parented_entities={})",
            camera_count,
            parented_count
        );
    });
}

struct SandboxModules {
    renderer: RenderModule,
    physics: PhysicsModule,
    audio: AudioModule,
    input: InputModule,
    assets: AssetModule,
}

impl SandboxModules {
    fn new() -> Result<Self> {
        let assets = AssetModule::new("assets");
        let _asset_path = assets.load_stub("textures/placeholder.png")?;

        Ok(Self {
            renderer: RenderModule::new(),
            physics: PhysicsModule::new(),
            audio: AudioModule::new(),
            input: InputModule::new(),
            assets,
        })
    }
}

impl EngineModules for SandboxModules {
    fn window_created(&mut self, window: Arc<Window>, window_config: &WindowConfig) -> Result<()> {
        self.renderer
            .initialize_with_window(window, window_config.vsync)
    }

    fn flush_input(&mut self) -> Result<()> {
        self.input.pump()
    }

    fn fixed_update(&mut self, fixed_dt_seconds: f32) -> Result<()> {
        self.physics.step(fixed_dt_seconds)
    }

    fn update(&mut self, _delta_seconds: f32) -> Result<()> {
        let reload_count = self.assets.poll_texture_hot_reload();
        if reload_count > 0 {
            log::info!(
                target: "engine::sandbox",
                "Hot-reloaded {} texture asset(s)",
                reload_count
            );
        }

        self.audio.update()
    }

    fn render(&mut self, world: &mut World, _alpha: f32) -> Result<()> {
        self.renderer.tick(world, self.assets.asset_server())
    }

    fn resized(&mut self, width: u32, height: u32) -> Result<()> {
        self.renderer.resize(width, height);
        Ok(())
    }
}

struct SandboxBootstrapPlugin;

impl Plugin<SandboxModules> for SandboxBootstrapPlugin {
    fn build(&self, engine: &mut Engine<SandboxModules>) {
        let _identity = engine_math::identity();

        let render_assets = (|| -> Result<SandboxRenderAssets> {
            let texture = engine
                .modules
                .assets
                .load_texture_handle("textures/placeholder.png")?;
            let mesh = engine.modules.assets.load_mesh_handle("meshes/cube.glb")?;
            let material = engine
                .modules
                .assets
                .load_material_handle("materials/default.ron")?;
            Ok(SandboxRenderAssets {
                mesh,
                texture,
                material,
            })
        })();

        match render_assets {
            Ok(render_assets) => {
                engine.insert_resource(render_assets);
                log::info!(
                    target: "engine::sandbox",
                    "Sandbox render assets loaded (mesh/texture/material handles)"
                );
            }
            Err(error) => {
                log::warn!(
                    target: "engine::sandbox",
                    "Sandbox render assets unavailable: {}",
                    error
                );
            }
        }

        engine
            .add_startup_systems(ecs_startup_smoke)
            .add_startup_systems(ecs_spawn_smoke_entities)
            .add_startup_systems(ecs_spawn_renderable_entity)
            .add_update_systems(ecs_update_smoke);

        log::info!(target: "engine::sandbox", "Engine: {}", engine_core::engine_name());
        log::info!(
            target: "engine::sandbox",
            "Modules: {}, {}, {}, {}, {}, {}",
            engine_math::module_name(),
            engine_render::module_name(),
            engine_physics::module_name(),
            engine_audio::module_name(),
            engine_input::module_name(),
            engine_assets::module_name()
        );
        log::info!(
            target: "engine::sandbox",
            "Backends: render={}, physics2d/3d={:?}, audio={:?}, input={:?}",
            engine.modules.renderer.backend_type_name(),
            engine_physics::dimensions_supported(),
            engine.modules.audio.backend_type_names(),
            engine.modules.input.backend_type_names(),
        );
        log::info!(target: "engine::sandbox", "Sandbox bootstrap complete");
    }
}

fn run() -> Result<()> {
    let modules = SandboxModules::new()?;

    let config = EngineConfig::with_app_name("Motley Sandbox").with_window_config(
        WindowConfig::default()
            .with_title("Motley Sandbox")
            .with_size(1280, 720)
            .with_resizable(true)
            .with_vsync(true),
    );

    let mut engine = Engine::new(config, modules)?;
    engine.add_plugin(SandboxBootstrapPlugin);

    engine.run()
}

fn main() {
    engine_core::init_logging();

    if let Err(error) = run() {
        log::error!(target: "engine::sandbox", "Startup failed: {}", error);
        std::process::exit(1);
    }
}
