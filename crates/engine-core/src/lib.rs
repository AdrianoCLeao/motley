pub mod error;

pub use error::{EngineError, Result};

use bevy_ecs::world::World;
use std::sync::Once;

static LOGGER_INIT: Once = Once::new();

pub fn init_logging() {
    LOGGER_INIT.call_once(|| {
        let mut builder = env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("info"),
        );
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
