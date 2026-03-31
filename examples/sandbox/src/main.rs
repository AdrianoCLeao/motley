use engine_assets::AssetModule;
use engine_audio::AudioModule;
use engine_core::{self, Engine, EngineConfig, EngineModules, Plugin, Result, WindowConfig};
use engine_input::InputModule;
use engine_physics::PhysicsModule;
use engine_render::RenderModule;

struct SandboxModules {
    renderer: RenderModule,
    physics: PhysicsModule,
    audio: AudioModule,
    input: InputModule,
    _assets: AssetModule,
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
            _assets: assets,
        })
    }
}

impl EngineModules for SandboxModules {
    fn flush_input(&mut self) -> Result<()> {
        self.input.pump()
    }

    fn fixed_update(&mut self, fixed_dt_seconds: f32) -> Result<()> {
        self.physics.step(fixed_dt_seconds)
    }

    fn update(&mut self, _delta_seconds: f32) -> Result<()> {
        self.audio.update()
    }

    fn render(&mut self, _alpha: f32) -> Result<()> {
        self.renderer.tick()
    }
}

struct SandboxBootstrapPlugin;

impl Plugin<SandboxModules> for SandboxBootstrapPlugin {
    fn build(&self, engine: &mut Engine<SandboxModules>) {
        let _identity = engine_math::identity();

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
