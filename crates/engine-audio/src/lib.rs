use engine_core::Result;

#[derive(Default)]
pub struct AudioModule;

impl AudioModule {
    pub fn new() -> Self {
        Self
    }

    pub fn update(&self) -> Result<()> {
        log::trace!(target: "engine::audio", "Audio update completed");
        Ok(())
    }

    pub fn backend_type_names(&self) -> (&'static str, &'static str) {
        (
            std::any::type_name::<cpal::SampleRate>(),
            std::any::type_name::<kira::manager::AudioManagerSettings<kira::manager::DefaultBackend>>(
            ),
        )
    }
}

pub fn module_name() -> &'static str {
    "engine-audio"
}
