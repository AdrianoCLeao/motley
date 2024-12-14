use glam::*;

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3
}

impl Default for Vertex {
    fn default() -> Self {
        Vertex {
            position: Vec3::ZERO,
            normal: Vec3::ZERO
        }
    }
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material_idx: usize
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub base_color: Vec4
}

#[derive(Clone, Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>
}