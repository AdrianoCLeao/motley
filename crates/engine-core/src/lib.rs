pub mod camera;
pub mod error;
pub mod schedule;
pub mod tag;
pub mod time;
pub mod transform;
pub mod window;

pub use camera::{sync_camera_aspect_from_window, Camera2d, Camera3d, PrimaryCamera, WindowSize};
pub use error::{EngineError, Result};
pub use schedule::{EngineSchedules, FixedUpdate, PreRender, Startup, Update};
pub use tag::{Hidden, PhysicsControlled, RenderLayer2D, RenderLayer3D, Visible};
pub use time::{
    FixedStepIterator, Time, TimeConfig, DEFAULT_FIXED_TIMESTEP_SECONDS,
    DEFAULT_FPS_AVERAGE_WINDOW_SAMPLES, DEFAULT_MAX_FRAME_TIME_SECONDS,
};
pub use transform::{
    propagate_transforms, Children, GlobalTransform, Parent, SpatialBundle, Transform,
};
pub use window::{run_windowed, WindowConfig, WindowLoop};

use bevy_ecs::{schedule::IntoSystemConfigs, system::Resource, world::World};
use std::sync::Once;

static LOGGER_INIT: Once = Once::new();

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub app_name: String,
    pub window: WindowConfig,
    pub time: TimeConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        let app_name = engine_name().to_owned();

        Self {
            app_name: app_name.clone(),
            window: WindowConfig::default().with_title(app_name),
            time: TimeConfig::default(),
        }
    }
}

impl EngineConfig {
    pub fn with_app_name(app_name: impl Into<String>) -> Self {
        let app_name = app_name.into();

        Self {
            app_name: app_name.clone(),
            window: WindowConfig::default().with_title(app_name),
            time: TimeConfig::default(),
        }
    }

    pub fn with_window_config(mut self, window: WindowConfig) -> Self {
        self.window = window;
        self
    }

    pub fn with_time_config(mut self, time: TimeConfig) -> Self {
        self.time = time;
        self
    }

    fn validate(&self) -> Result<()> {
        if self.app_name.trim().is_empty() {
            return Err(EngineError::Config("app_name cannot be empty".to_owned()));
        }

        if self.window.width == 0 || self.window.height == 0 {
            return Err(EngineError::Config(
                "window width and height must be greater than zero".to_owned(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameStats {
    pub fps_rolling: f32,
    pub fps_instant: f32,
    pub jitter_seconds: f32,
    pub delta_seconds: f32,
    pub elapsed_seconds: f64,
    pub frame_count: u64,
}

pub trait EngineModules {
    /// Called once per rendered frame before fixed-step simulation.
    fn flush_input(&mut self) -> Result<()>;

    /// Called zero or more times per rendered frame using a fixed timestep.
    fn fixed_update(&mut self, _fixed_dt_seconds: f32) -> Result<()> {
        Ok(())
    }

    /// Called once per rendered frame with variable delta time.
    fn update(&mut self, _delta_seconds: f32) -> Result<()> {
        Ok(())
    }

    /// Called once per rendered frame after update with interpolation alpha [0, 1).
    fn render(&mut self, _alpha: f32) -> Result<()> {
        Ok(())
    }

    /// Called when the OS reports a window resize event for the active window.
    fn resized(&mut self, _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }
}

pub trait Plugin<M: EngineModules> {
    fn build(&self, engine: &mut Engine<M>);
}

pub struct Engine<M: EngineModules> {
    pub world: World,
    pub time: Time,
    pub modules: M,
    pub config: EngineConfig,
    schedules: EngineSchedules,
    startup_completed: bool,
}

impl<M: EngineModules> Engine<M> {
    pub fn new(config: EngineConfig, modules: M) -> Result<Self> {
        config.validate()?;

        let window_size = WindowSize::new(config.window.width, config.window.height);

        let mut engine = Self {
            world: create_world(),
            time: Time::with_config(config.time),
            modules,
            config,
            schedules: EngineSchedules::new(),
            startup_completed: false,
        };

        engine
            .insert_resource(window_size)
            .add_pre_render_systems(propagate_transforms)
            .add_pre_render_systems(sync_camera_aspect_from_window);

        Ok(engine)
    }

    pub fn add_plugin<P: Plugin<M>>(&mut self, plugin: P) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn add_startup_systems<Marker>(
        &mut self,
        systems: impl IntoSystemConfigs<Marker>,
    ) -> &mut Self {
        self.schedules.startup.add_systems(systems);
        self
    }

    pub fn add_fixed_update_systems<Marker>(
        &mut self,
        systems: impl IntoSystemConfigs<Marker>,
    ) -> &mut Self {
        self.schedules.fixed_update.add_systems(systems);
        self
    }

    pub fn add_update_systems<Marker>(
        &mut self,
        systems: impl IntoSystemConfigs<Marker>,
    ) -> &mut Self {
        self.schedules.update.add_systems(systems);
        self
    }

    pub fn add_pre_render_systems<Marker>(
        &mut self,
        systems: impl IntoSystemConfigs<Marker>,
    ) -> &mut Self {
        self.schedules.pre_render.add_systems(systems);
        self
    }

    pub fn tick(&mut self) -> Result<FrameStats> {
        self.time.advance();
        self.run_frame()
    }

    pub fn tick_with_frame_time(&mut self, frame_time_seconds: f64) -> Result<FrameStats> {
        self.time.advance_by(frame_time_seconds);
        self.run_frame()
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.insert_resource(WindowSize::new(width, height));
        self.modules.resized(width, height)
    }

    pub fn frame_stats(&self) -> FrameStats {
        FrameStats {
            fps_rolling: self.time.fps(),
            fps_instant: self.time.instant_fps(),
            jitter_seconds: self.time.jitter_seconds(),
            delta_seconds: self.time.delta_seconds(),
            elapsed_seconds: self.time.elapsed_seconds(),
            frame_count: self.time.frame_count(),
        }
    }

    pub fn window_title(&self) -> String {
        let vsync_status = if self.config.window.vsync {
            "VSync On"
        } else {
            "VSync Off"
        };

        format!(
            "{} | {:.1} FPS | {}",
            self.config.app_name,
            self.time.fps(),
            vsync_status
        )
    }

    pub fn run(self) -> Result<()>
    where
        M: 'static,
    {
        let window_config = self.config.window.clone();
        run_windowed(window_config, self)
    }

    fn run_frame(&mut self) -> Result<FrameStats> {
        if !self.startup_completed {
            self.schedules.startup.run(&mut self.world);
            self.startup_completed = true;
        }

        self.modules.flush_input()?;

        for fixed_dt in self.time.fixed_steps() {
            self.schedules.fixed_update.run(&mut self.world);
            self.modules.fixed_update(fixed_dt)?;
        }

        self.schedules.update.run(&mut self.world);
        self.modules.update(self.time.delta_seconds())?;

        self.schedules.pre_render.run(&mut self.world);
        self.modules.render(self.time.alpha())?;

        Ok(self.frame_stats())
    }
}

impl<M: EngineModules> WindowLoop for Engine<M> {
    fn tick(&mut self) -> Result<()> {
        Engine::tick(self).map(|_| ())
    }

    fn resized(&mut self, width: u32, height: u32) -> Result<()> {
        self.resize(width, height)
    }

    fn title(&self) -> String {
        self.window_title()
    }
}

pub fn init_logging() {
    LOGGER_INIT.call_once(|| {
        let mut builder =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
        builder.format_timestamp_millis();
        let _ = builder.try_init();
    });
}

pub fn engine_name() -> &'static str {
    "Motley"
}

pub fn create_world() -> World {
    World::new()
}

#[cfg(test)]
mod tests {
    use super::{Engine, EngineConfig, EngineModules, Plugin, TimeConfig, WindowConfig};
    use bevy_ecs::prelude::{Changed, Commands, Component, Query, ResMut, Resource};

    #[derive(Resource, Default)]
    struct EcsCounters {
        startup_runs: u32,
        fixed_runs: u32,
        update_runs: u32,
        pre_render_runs: u32,
        changed_runs: u32,
    }

    #[derive(Resource, Default)]
    struct EcsTrace {
        events: Vec<&'static str>,
    }

    #[derive(Component)]
    struct ProbeTransform {
        value: f32,
    }

    fn count_startup(mut counters: ResMut<EcsCounters>) {
        counters.startup_runs += 1;
    }

    fn count_fixed(mut counters: ResMut<EcsCounters>) {
        counters.fixed_runs += 1;
    }

    fn count_update(mut counters: ResMut<EcsCounters>) {
        counters.update_runs += 1;
    }

    fn count_pre_render(mut counters: ResMut<EcsCounters>) {
        counters.pre_render_runs += 1;
    }

    fn trace_startup(mut trace: ResMut<EcsTrace>) {
        trace.events.push("startup");
    }

    fn trace_fixed(mut trace: ResMut<EcsTrace>) {
        trace.events.push("fixed");
    }

    fn trace_update(mut trace: ResMut<EcsTrace>) {
        trace.events.push("update");
    }

    fn trace_pre_render(mut trace: ResMut<EcsTrace>) {
        trace.events.push("pre_render");
    }

    fn spawn_probe(mut commands: Commands) {
        commands.spawn(ProbeTransform { value: 0.0 });
    }

    fn mutate_probe(mut probes: Query<&mut ProbeTransform>) {
        for mut probe in &mut probes {
            probe.value += 1.0;
        }
    }

    fn count_changed_probe(
        probes: Query<&ProbeTransform, Changed<ProbeTransform>>,
        mut counters: ResMut<EcsCounters>,
    ) {
        counters.changed_runs += probes.iter().count() as u32;
    }

    #[derive(Default)]
    struct MockModules {
        flush_calls: u32,
        fixed_calls: u32,
        update_calls: u32,
        render_calls: u32,
        last_delta_seconds: f32,
        last_alpha: f32,
        resized_to: Option<(u32, u32)>,
    }

    impl EngineModules for MockModules {
        fn flush_input(&mut self) -> super::Result<()> {
            self.flush_calls += 1;
            Ok(())
        }

        fn fixed_update(&mut self, _fixed_dt_seconds: f32) -> super::Result<()> {
            self.fixed_calls += 1;
            Ok(())
        }

        fn update(&mut self, delta_seconds: f32) -> super::Result<()> {
            self.update_calls += 1;
            self.last_delta_seconds = delta_seconds;
            Ok(())
        }

        fn render(&mut self, alpha: f32) -> super::Result<()> {
            self.render_calls += 1;
            self.last_alpha = alpha;
            Ok(())
        }

        fn resized(&mut self, width: u32, height: u32) -> super::Result<()> {
            self.resized_to = Some((width, height));
            Ok(())
        }
    }

    struct AppNamePlugin;

    impl Plugin<MockModules> for AppNamePlugin {
        fn build(&self, engine: &mut Engine<MockModules>) {
            engine.config.app_name = "PluginName".to_owned();
        }
    }

    #[test]
    fn engine_tick_executes_stage_order() {
        let mut engine = Engine::new(
            EngineConfig {
                app_name: "Test".to_owned(),
                window: WindowConfig::default(),
                time: TimeConfig {
                    fixed_timestep_seconds: 0.1,
                    max_frame_time_seconds: 1.0,
                    fps_average_window_samples: 8,
                },
            },
            MockModules::default(),
        )
        .expect("engine should be created");

        let stats = engine
            .tick_with_frame_time(0.25)
            .expect("tick should succeed");

        assert_eq!(engine.modules.flush_calls, 1);
        assert_eq!(engine.modules.fixed_calls, 2);
        assert_eq!(engine.modules.update_calls, 1);
        assert_eq!(engine.modules.render_calls, 1);
        assert!((engine.modules.last_delta_seconds - 0.25).abs() < 1e-6);
        assert!((engine.modules.last_alpha - 0.5).abs() < 1e-6);
        assert_eq!(stats.frame_count, 1);
        assert!((stats.elapsed_seconds - 0.25).abs() < 1e-6);
        assert!((stats.fps_rolling - 4.0).abs() < 1e-6);
        assert!((stats.fps_instant - 4.0).abs() < 1e-6);
        assert!(stats.jitter_seconds >= 0.0);
    }

    #[test]
    fn plugin_build_updates_engine_configuration() {
        let mut engine =
            Engine::new(EngineConfig::default(), MockModules::default()).expect("engine");

        engine.add_plugin(AppNamePlugin);

        assert_eq!(engine.config.app_name, "PluginName");
    }

    #[test]
    fn engine_resize_forwards_to_modules() {
        let mut engine =
            Engine::new(EngineConfig::default(), MockModules::default()).expect("engine");

        engine.resize(1920, 1080).expect("resize should succeed");

        assert_eq!(engine.modules.resized_to, Some((1920, 1080)));
    }

    #[test]
    fn engine_rejects_invalid_window_configuration() {
        let config = EngineConfig::with_app_name("Test")
            .with_window_config(WindowConfig::default().with_title("Test").with_size(0, 720));

        let result = Engine::new(config, MockModules::default());
        assert!(result.is_err());
    }

    #[test]
    fn window_title_exposes_rolling_fps_and_vsync_mode() {
        let mut engine = Engine::new(
            EngineConfig::with_app_name("Title Test").with_window_config(
                WindowConfig::default()
                    .with_title("Title Test")
                    .with_vsync(false),
            ),
            MockModules::default(),
        )
        .expect("engine");

        engine
            .tick_with_frame_time(0.5)
            .expect("tick should succeed");

        let title = engine.window_title();
        assert!(title.contains("Title Test"));
        assert!(title.contains("FPS"));
        assert!(title.contains("VSync Off"));
    }

    #[test]
    fn long_pause_uses_frame_cap_and_recovers_next_tick() {
        let mut engine = Engine::new(
            EngineConfig {
                app_name: "Long Pause".to_owned(),
                window: WindowConfig::default().with_vsync(true),
                time: TimeConfig {
                    fixed_timestep_seconds: 0.05,
                    max_frame_time_seconds: 0.25,
                    fps_average_window_samples: 8,
                },
            },
            MockModules::default(),
        )
        .expect("engine");

        let paused_stats = engine
            .tick_with_frame_time(5.0)
            .expect("paused tick should succeed");
        assert_eq!(engine.modules.fixed_calls, 5);
        assert!((paused_stats.delta_seconds - 0.25).abs() < 1e-6);

        let resumed_stats = engine
            .tick_with_frame_time(0.05)
            .expect("resumed tick should succeed");
        assert_eq!(engine.modules.fixed_calls, 6);
        assert!((resumed_stats.delta_seconds - 0.05).abs() < 1e-6);
        assert!((resumed_stats.elapsed_seconds - 0.30).abs() < 1e-6);
        assert_eq!(resumed_stats.frame_count, 2);
    }

    #[test]
    fn ecs_schedules_run_with_expected_frequency_and_startup_runs_once() {
        let mut engine = Engine::new(
            EngineConfig {
                app_name: "Schedules".to_owned(),
                window: WindowConfig::default(),
                time: TimeConfig {
                    fixed_timestep_seconds: 0.1,
                    max_frame_time_seconds: 1.0,
                    fps_average_window_samples: 8,
                },
            },
            MockModules::default(),
        )
        .expect("engine should be created");

        engine
            .insert_resource(EcsCounters::default())
            .add_startup_systems(count_startup)
            .add_fixed_update_systems(count_fixed)
            .add_update_systems(count_update)
            .add_pre_render_systems(count_pre_render);

        engine
            .tick_with_frame_time(0.25)
            .expect("first tick should succeed");

        {
            let counters = engine.world.resource::<EcsCounters>();
            assert_eq!(counters.startup_runs, 1);
            assert_eq!(counters.fixed_runs, 2);
            assert_eq!(counters.update_runs, 1);
            assert_eq!(counters.pre_render_runs, 1);
        }

        engine
            .tick_with_frame_time(0.25)
            .expect("second tick should succeed");

        let counters = engine.world.resource::<EcsCounters>();
        assert_eq!(counters.startup_runs, 1);
        assert_eq!(counters.fixed_runs, 4);
        assert_eq!(counters.update_runs, 2);
        assert_eq!(counters.pre_render_runs, 2);
    }

    #[test]
    fn ecs_schedule_order_is_startup_then_fixed_then_update_then_pre_render() {
        let mut engine = Engine::new(
            EngineConfig {
                app_name: "Order".to_owned(),
                window: WindowConfig::default(),
                time: TimeConfig {
                    fixed_timestep_seconds: 0.1,
                    max_frame_time_seconds: 1.0,
                    fps_average_window_samples: 8,
                },
            },
            MockModules::default(),
        )
        .expect("engine should be created");

        engine
            .insert_resource(EcsTrace::default())
            .add_startup_systems(trace_startup)
            .add_fixed_update_systems(trace_fixed)
            .add_update_systems(trace_update)
            .add_pre_render_systems(trace_pre_render);

        engine
            .tick_with_frame_time(0.1)
            .expect("tick should succeed");

        let trace = engine.world.resource::<EcsTrace>();
        assert_eq!(
            trace.events,
            vec!["startup", "fixed", "update", "pre_render"]
        );
    }

    #[test]
    fn ecs_changed_detection_observes_transform_mutations_across_frames() {
        let mut engine = Engine::new(
            EngineConfig {
                app_name: "Changed".to_owned(),
                window: WindowConfig::default(),
                time: TimeConfig {
                    fixed_timestep_seconds: 0.1,
                    max_frame_time_seconds: 1.0,
                    fps_average_window_samples: 8,
                },
            },
            MockModules::default(),
        )
        .expect("engine should be created");

        engine
            .insert_resource(EcsCounters::default())
            .add_startup_systems(spawn_probe)
            .add_update_systems(mutate_probe)
            .add_pre_render_systems(count_changed_probe);

        engine
            .tick_with_frame_time(0.016)
            .expect("first tick should succeed");
        engine
            .tick_with_frame_time(0.016)
            .expect("second tick should succeed");

        let counters = engine.world.resource::<EcsCounters>();
        assert!(
            counters.changed_runs >= 2,
            "expected changed runs >= 2, got {}",
            counters.changed_runs
        );
    }
}
