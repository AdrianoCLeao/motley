use engine_core::{EngineError, HardeningConfig, Result};
use image::GenericImageView;
use std::path::Path;

use crate::{MeshData, MeshVertex, TextureData};

pub(crate) fn load_texture_payload(
    path: &Path,
    hardening: &HardeningConfig,
) -> Result<TextureData> {
    let image = image::open(path).map_err(|error| EngineError::AssetLoad {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;

    let (width, height) = image.dimensions();
    if width > hardening.max_texture_dimension || height > hardening.max_texture_dimension {
        return Err(EngineError::AssetLoad {
            path: path.display().to_string(),
            reason: format!(
                "texture dimensions {}x{} exceed configured limit {}",
                width, height, hardening.max_texture_dimension
            ),
        });
    }

    let estimated_payload_bytes = width as u64 * height as u64 * 4;
    if estimated_payload_bytes > hardening.max_texture_payload_bytes as u64 {
        return Err(EngineError::AssetLoad {
            path: path.display().to_string(),
            reason: format!(
                "texture payload {} bytes exceeds configured limit {}",
                estimated_payload_bytes, hardening.max_texture_payload_bytes
            ),
        });
    }

    let pixels_rgba8 = image.to_rgba8().into_raw();

    Ok(TextureData {
        width,
        height,
        pixels_rgba8,
        revision: 0,
    })
}

pub(crate) fn load_mesh_payload(path: &Path, hardening: &HardeningConfig) -> Result<MeshData> {
    let (document, buffers, _images) =
        gltf::import(path).map_err(|error| EngineError::AssetLoad {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;

    let mut mesh_name = None;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for mesh in document.meshes() {
        if mesh_name.is_none() {
            mesh_name = mesh.name().map(str::to_owned);
        }

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .ok_or_else(|| EngineError::AssetLoad {
                    path: path.display().to_string(),
                    reason: "gltf primitive is missing POSITION attribute".to_owned(),
                })?
                .collect();

            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|iter| iter.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|iter| iter.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            if normals.len() != positions.len() || uvs.len() != positions.len() {
                return Err(EngineError::AssetLoad {
                    path: path.display().to_string(),
                    reason: "gltf vertex attribute lengths do not match".to_owned(),
                });
            }

            let base_index = u32::try_from(vertices.len()).map_err(|_| EngineError::AssetLoad {
                path: path.display().to_string(),
                reason: "mesh has too many vertices for u32 index buffer".to_owned(),
            })?;
            let primitive_vertex_count =
                u32::try_from(positions.len()).map_err(|_| EngineError::AssetLoad {
                    path: path.display().to_string(),
                    reason: "primitive vertex count overflow".to_owned(),
                })?;

            if vertices.len() + positions.len() > hardening.max_mesh_vertices {
                return Err(EngineError::AssetLoad {
                    path: path.display().to_string(),
                    reason: format!(
                        "mesh vertex count exceeds configured limit {}",
                        hardening.max_mesh_vertices
                    ),
                });
            }

            for ((position, normal), uv) in positions.into_iter().zip(normals).zip(uvs) {
                vertices.push(MeshVertex {
                    position,
                    normal,
                    uv,
                });
            }

            let primitive_indices: Vec<u32> = if let Some(read_indices) = reader.read_indices() {
                read_indices
                    .into_u32()
                    .map(|index| base_index + index)
                    .collect()
            } else {
                (0..primitive_vertex_count)
                    .map(|index| base_index + index)
                    .collect()
            };

            if indices.len() + primitive_indices.len() > hardening.max_mesh_indices {
                return Err(EngineError::AssetLoad {
                    path: path.display().to_string(),
                    reason: format!(
                        "mesh index count exceeds configured limit {}",
                        hardening.max_mesh_indices
                    ),
                });
            }

            indices.extend(primitive_indices);
        }
    }

    if vertices.is_empty() {
        return Err(EngineError::AssetLoad {
            path: path.display().to_string(),
            reason: "gltf file contains no renderable primitives".to_owned(),
        });
    }

    let fallback_name = path
        .file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed-mesh".to_owned());

    Ok(MeshData {
        name: mesh_name.unwrap_or(fallback_name),
        vertices,
        indices,
    })
}
