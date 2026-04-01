use std::collections::HashMap;

use engine_assets::{AssetId, AssetServer, MeshHandle, MeshVertex, TextureHandle};
use engine_core::{EngineError, Result};
use wgpu::util::DeviceExt;

use crate::{texture_upload::upload_rgba8_texture, GpuMesh, GpuTexture, GpuVertex};

pub(crate) fn build_gpu_vertices(vertices: &[MeshVertex]) -> Vec<GpuVertex> {
    vertices
        .iter()
        .map(|vertex| GpuVertex {
            position: vertex.position,
            normal: vertex.normal,
            uv: vertex.uv,
        })
        .collect()
}

pub(crate) fn compute_index_count(indices_len: usize) -> Result<u32> {
    u32::try_from(indices_len)
        .map_err(|_| EngineError::Render("mesh index count overflow for u32 draw call".to_owned()))
}

pub(crate) fn ensure_gpu_mesh(
    device: &wgpu::Device,
    gpu_meshes: &mut HashMap<AssetId, GpuMesh>,
    handle: MeshHandle,
    assets: &AssetServer,
) -> Result<()> {
    if gpu_meshes.contains_key(&handle.id()) {
        return Ok(());
    }

    let payload = assets.mesh_payload(handle).ok_or_else(|| {
        EngineError::Render(format!(
            "mesh payload missing for handle {}",
            handle.id().value()
        ))
    })?;

    let vertices = build_gpu_vertices(payload.vertices.as_slice());
    let index_count = compute_index_count(payload.indices.len())?;

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("engine-render-mesh-vertex-buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("engine-render-mesh-index-buffer"),
        contents: bytemuck::cast_slice(payload.indices.as_slice()),
        usage: wgpu::BufferUsages::INDEX,
    });

    gpu_meshes.insert(
        handle.id(),
        GpuMesh {
            vertex_buffer,
            index_buffer,
            index_count,
        },
    );

    Ok(())
}

pub(crate) fn ensure_gpu_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    gpu_textures: &mut HashMap<AssetId, GpuTexture>,
    handle: TextureHandle,
    assets: &AssetServer,
) -> Result<()> {
    let payload = assets.texture_payload(handle).ok_or_else(|| {
        EngineError::Render(format!(
            "texture payload missing for handle {}",
            handle.id().value()
        ))
    })?;

    let width = payload.width.max(1);
    let height = payload.height.max(1);

    if let Some(gpu_texture) = gpu_textures.get_mut(&handle.id()) {
        if gpu_texture.revision == payload.revision {
            return Ok(());
        }

        if gpu_texture.width == width && gpu_texture.height == height {
            upload_rgba8_texture(
                queue,
                &gpu_texture.texture,
                width,
                height,
                payload.pixels_rgba8.as_slice(),
            )?;
            gpu_texture.revision = payload.revision;
            return Ok(());
        }
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("engine-render-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    upload_rgba8_texture(
        queue,
        &texture,
        width,
        height,
        payload.pixels_rgba8.as_slice(),
    )?;

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("engine-render-texture-sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    gpu_textures.insert(
        handle.id(),
        GpuTexture {
            texture,
            view,
            sampler,
            width,
            height,
            revision: payload.revision,
        },
    );

    Ok(())
}
