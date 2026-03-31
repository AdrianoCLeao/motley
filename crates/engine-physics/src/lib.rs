use engine_core::Result;

#[derive(Default)]
pub struct PhysicsModule;

impl PhysicsModule {
    pub fn new() -> Self {
        Self
    }

    pub fn step(&self, dt_seconds: f32) -> Result<()> {
        log::trace!(
            target: "engine::physics",
            "Physics step executed in fixed delta: {:.4}",
            dt_seconds
        );
        Ok(())
    }
}

pub fn module_name() -> &'static str {
    "engine-physics"
}

pub fn dimensions_supported() -> (&'static str, &'static str) {
    (
        std::any::type_name::<rapier2d::prelude::RigidBodySet>(),
        std::any::type_name::<rapier3d::prelude::RigidBodySet>(),
    )
}
