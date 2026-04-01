use bevy_ecs::prelude::Component;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Visible;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Hidden;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct RenderLayer3D;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct RenderLayer2D;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PhysicsControlled;

#[cfg(test)]
#[path = "tag_tests.rs"]
mod tests;
