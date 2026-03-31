use engine_core::Result;

#[derive(Default)]
pub struct InputModule;

impl InputModule {
    pub fn new() -> Self {
        Self
    }

    pub fn pump(&self) -> Result<()> {
        log::trace!(target: "engine::input", "Input events pumped");
        Ok(())
    }

    pub fn backend_type_names(&self) -> (&'static str, &'static str) {
        (
            std::any::type_name::<winit::event::ElementState>(),
            std::any::type_name::<gilrs::GamepadId>(),
        )
    }
}

pub fn module_name() -> &'static str {
    "engine-input"
}
