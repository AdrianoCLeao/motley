use engine_core::Result;

pub struct RenderModule;

impl RenderModule {
    pub fn new() -> Self {
        Self
    }

    pub fn tick(&self) -> Result<()> {
        log::trace!(target: "engine::render", "Render tick completed");
        Ok(())
    }

    pub fn backend_type_name(&self) -> &'static str {
        std::any::type_name::<wgpu::Backends>()
    }
}

pub fn module_name() -> &'static str {
    "engine-render"
}
