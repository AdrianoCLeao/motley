use glam::*;
use crate::model::Texture;

/*
The `Vertex` struct represents a single vertex in a 3D mesh. It includes position and normal
data, which are essential for rendering and lighting calculations. The `Default` trait provides
a default vertex with zeroed position and normal.
*/
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2
}

impl Default for Vertex {
    fn default() -> Self {
        Vertex {
            position: Vec3::ZERO,
            normal: Vec3::ZERO,
            tex_coord: Vec2::ZERO
        }
    }
}

/*
The `Mesh` struct represents a collection of vertices and indices forming a 3D object. It
also stores a reference to the material index used for rendering the mesh.
*/
#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material_idx: usize
}

/*
The `Material` struct defines the appearance of a mesh using a base color stored as a `Vec4`.
The `Default` trait initializes it with a white color.
*/
#[derive(Clone, Debug)]
pub struct Material {
    pub base_color: Vec4,
    pub base_color_texture: Option<Texture>
}

impl Default for Material {
    fn default() -> Self {
        Material {
            base_color: Vec4::ONE,
            base_color_texture: None
        }
    }
}

/*
The `Model` struct aggregates multiple meshes and their associated materials, representing
a complete 3D object that can be rendered.
*/
#[derive(Clone, Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>
}

/*
Processes a single GLTF node, extracting its meshes and associated materials. This function reads
vertex positions, normals, and indices, and maps them to custom `Mesh` and `Vertex` structs.
It also handles material assignment and updates the `materials` array accordingly.
*/
fn process_node_recursive(
    node: &gltf::Node,
    buffers: &[gltf::buffer::Data],
    meshes: &mut Vec<Mesh>,
    materials: &mut [Material],
    file_path: &str,
    parent_transform: Mat4,
) {
    let transform = parent_transform * Mat4::from_cols_array_2d(&node.transform().matrix());

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            if primitive.mode() == gltf::mesh::Mode::Triangles {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let positions = reader
                    .read_positions()
                    .expect("Vertices precisam ter posições")
                    .map(Vec3::from)
                    .map(|pos| transform.transform_point3(pos))
                    .collect::<Vec<_>>();

                let mut vertices: Vec<Vertex> = positions
                    .into_iter()
                    .map(|position| Vertex {
                        position,
                        ..Default::default()
                    })
                    .collect();

                if let Some(normals) = reader.read_normals() {
                    for (i, normal) in normals.enumerate() {
                        vertices[i].normal = transform
                            .transform_vector3(Vec3::from(normal))
                            .normalize();
                    }
                }

                if let Some(tex_coords) = reader.read_tex_coords(0) {
                    for (i, tex_coord) in tex_coords.into_f32().enumerate() {
                        vertices[i].tex_coord = Vec2::from(tex_coord);
                    }
                }

                let indices = reader
                    .read_indices()
                    .map(|read_indices| read_indices.into_u32().collect())
                    .expect("Índices são necessários");

                let material_idx = primitive.material().index().unwrap_or(0);

                meshes.push(Mesh {
                    vertices,
                    indices,
                    material_idx,
                });
            }
        }
    }

    for child in node.children() {
        process_node_recursive(
            &child,
            buffers,
            meshes,
            materials,
            file_path,
            transform,
        );
    }
}

/*
Loads a 3D model from a GLTF file. It parses the document, processes the nodes to extract
meshes and materials, and assembles them into a `Model` struct for further use.
*/
pub fn load_model(file_path: &str) -> Model {
    let (document, buffers, _images) = gltf::import(file_path).expect("Falha ao carregar modelo.");

    let mut meshes = Vec::new();
    let mut materials = vec![Material::default(); document.materials().len()];
    if materials.is_empty() {
        materials.push(Material::default());
    }

    let root_transform = Mat4::IDENTITY;

    for node in document.nodes() {
        process_node_recursive(
            &node,
            &buffers,
            &mut meshes,
            &mut materials,
            file_path,
            root_transform,
        );
    }

    Model { meshes, materials }
}