pub use glam;

pub type Vec2 = glam::Vec2;
pub type Vec3 = glam::Vec3;
pub type Quat = glam::Quat;
pub type Mat4 = glam::Mat4;

pub fn module_name() -> &'static str {
    "engine-math"
}

pub fn identity() -> Mat4 {
    Mat4::IDENTITY
}
