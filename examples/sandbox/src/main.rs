use engine_assets::AssetModule;
use engine_audio::AudioModule;
use engine_core::{self, Result};
use engine_input::InputModule;
use engine_physics::PhysicsModule;
use engine_render::RenderModule;

fn run() -> Result<()> {
    let _world = engine_core::create_world();
    let _identity = engine_math::identity();

    let renderer = RenderModule::new();
    let physics = PhysicsModule::new();
    let audio = AudioModule::new();
    let input = InputModule::new();
    let assets = AssetModule::new("assets");

    let _asset_path = assets.load_stub("textures/placeholder.png")?;

    renderer.tick()?;
    physics.step(1.0 / 60.0)?;
    audio.update()?;
    input.pump()?;

    log::info!(target: "engine::sandbox", "Engine: {}", engine_core::engine_name());
    log::info!(target: "engine::sandbox", "Modules: {}, {}, {}, {}, {}, {}", engine_math::module_name(), engine_render::module_name(), engine_physics::module_name(), engine_audio::module_name(), engine_input::module_name(), engine_assets::module_name());
    log::info!(target: "engine::sandbox", "Sandbox bootstrap complete");

    Ok(())
}

fn main() {
    engine_core::init_logging();

    if let Err(error) = run() {
        log::error!(target: "engine::sandbox", "Startup failed: {}", error);
        std::process::exit(1);
    }
}
