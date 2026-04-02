use bevy_ecs::prelude::{Bundle, Component};
use bevy_reflect::Reflect;
use engine_core::{GlobalTransform, PhysicsControlled, Transform};
use engine_math::Vec3;
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};

#[derive(Component, Clone, Copy, Debug)]
pub struct RigidBodyHandle3D(pub RigidBodyHandle);

#[derive(Component, Clone, Copy, Debug)]
pub struct ColliderHandle3D(pub ColliderHandle);

#[derive(
    Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, engine_reflect::RegisterReflect,
)]
pub enum RigidBodyType {
    #[default]
    Dynamic,
    Kinematic,
    Static,
}

#[derive(Component, Clone, Debug, Reflect, engine_reflect::RegisterReflect)]
pub enum ColliderShape3D {
    Box { half_extents: Vec3 },
    Sphere { radius: f32 },
    Capsule { half_height: f32, radius: f32 },
    Trimesh,
}

impl Default for ColliderShape3D {
    fn default() -> Self {
        Self::Box {
            half_extents: Vec3::splat(0.5),
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, engine_reflect::RegisterReflect)]
pub struct PhysicsMaterial {
    #[engine_reflect(range(min = 0.0, max = 1.0))]
    pub restitution: f32,
    #[engine_reflect(range(min = 0.0, max = 1.0))]
    pub friction: f32,
    #[engine_reflect(range(min = 0.001, max = 10000.0))]
    pub density: f32,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            restitution: 0.3,
            friction: 0.7,
            density: 1.0,
        }
    }
}

#[derive(Bundle)]
pub struct RigidBody3DBundle {
    pub body_type: RigidBodyType,
    pub shape: ColliderShape3D,
    pub material: PhysicsMaterial,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub physics_controlled: PhysicsControlled,
}

impl Default for RigidBody3DBundle {
    fn default() -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            shape: ColliderShape3D::default(),
            material: PhysicsMaterial::default(),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            physics_controlled: PhysicsControlled,
        }
    }
}

#[cfg(test)]
#[path = "components_tests.rs"]
mod tests;
