use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;

#[derive(Component, Debug, Clone, Copy, Default, Reflect, engine_reflect::RegisterReflect)]
pub struct Visible;

#[derive(Component, Debug, Clone, Copy, Default, Reflect, engine_reflect::RegisterReflect)]
pub struct Hidden;

#[derive(Component, Debug, Clone, Copy, Default, Reflect, engine_reflect::RegisterReflect)]
pub struct RenderLayer3D;

#[derive(Component, Debug, Clone, Copy, Default, Reflect, engine_reflect::RegisterReflect)]
pub struct RenderLayer2D;

#[derive(Component, Debug, Clone, Copy, Default, Reflect, engine_reflect::RegisterReflect)]
pub struct PhysicsControlled;

#[cfg(test)]
#[path = "tag_tests.rs"]
mod tests;
