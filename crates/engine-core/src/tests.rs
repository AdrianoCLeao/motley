use super::{Engine, EngineConfig, EngineModules, Plugin, TimeConfig, WindowConfig};
use bevy_ecs::prelude::{Changed, Commands, Component, Query, ResMut, Resource, World};

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
    fn flush_input(&mut self, _world: &mut World) -> super::Result<()> {
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

    fn render(&mut self, _world: &mut World, alpha: f32) -> super::Result<()> {
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
    let mut engine = Engine::new(EngineConfig::default(), MockModules::default()).expect("engine");

    engine.add_plugin(AppNamePlugin);

    assert_eq!(engine.config.app_name, "PluginName");
}

#[test]
fn engine_resize_forwards_to_modules() {
    let mut engine = Engine::new(EngineConfig::default(), MockModules::default()).expect("engine");

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

#[test]
fn engine_bootstraps_reflection_registries_with_core_components() {
    let mut engine = Engine::new(EngineConfig::default(), MockModules::default()).expect("engine");

    let type_registry = engine
        .world
        .resource::<engine_reflect::ReflectTypeRegistry>();
    assert!(type_registry.contains::<super::Transform>());
    assert!(type_registry.contains::<super::Camera3d>());

    let component_registry = engine.world.resource::<engine_reflect::ComponentRegistry>();
    assert!(component_registry
        .by_name(std::any::type_name::<super::Transform>())
        .is_some());
    assert!(component_registry
        .by_name(std::any::type_name::<super::Camera3d>())
        .is_some());

    let metadata_registry = engine
        .world
        .resource::<engine_reflect::ReflectMetadataRegistry>();
    assert_eq!(
        metadata_registry.hint_for::<super::Transform>("scale"),
        Some(engine_reflect::EditorHint::Range {
            min: 0.001,
            max: 100.0,
        })
    );

    let entity = engine.world.spawn(super::Transform::default()).id();
    let component_registry = engine.world.resource::<engine_reflect::ComponentRegistry>();
    let descriptor = component_registry
        .by_name(std::any::type_name::<super::Transform>())
        .expect("transform descriptor should exist");
    let reflected = descriptor
        .get_reflect(entity, &engine.world)
        .expect("entity should expose reflected transform");
    assert!(engine_reflect::reflect_field(reflected, "translation").is_some());

    let mut transform = super::Transform::default();
    let Some(field) = engine_reflect::reflect_field_mut(&mut transform, "translation") else {
        panic!("translation field should be mutable through reflection");
    };
    let translation = field
        .try_downcast_mut::<engine_math::Vec3>()
        .expect("translation field should be Vec3");
    translation.x = 5.0;
    assert!((transform.translation.x - 5.0).abs() < f32::EPSILON);
}
