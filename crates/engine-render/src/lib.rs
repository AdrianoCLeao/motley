use bevy_ecs::{
    prelude::{Component, World},
    query::With,
};
use bytemuck::{Pod, Zeroable};
use engine_assets::{
    AssetId, AssetServer, MaterialData, MaterialHandle, MeshHandle, TextureHandle,
};
use engine_core::{
    Camera2d, Camera3d, EngineError, GlobalTransform, PrimaryCamera, RenderLayer2D, RenderLayer3D,
    Result, Visible,
};
use engine_math::{Mat4, Vec3};
use std::{cmp::Ordering, collections::HashMap, mem::size_of, sync::Arc};
use wgpu::util::DeviceExt;
use winit::window::Window;

const RGBA8_BYTES_PER_PIXEL: u32 = 4;

#[derive(Component, Clone, Copy, Debug)]
pub struct MeshRenderable3d {
    pub mesh: MeshHandle,
    pub texture: TextureHandle,
    pub material: MaterialHandle,
}

impl MeshRenderable3d {
    pub fn new(mesh: MeshHandle, texture: TextureHandle, material: MaterialHandle) -> Self {
        Self {
            mesh,
            texture,
            material,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct SpriteRenderable2d {
    pub texture: TextureHandle,
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

impl SpriteRenderable2d {
    pub fn new(texture: TextureHandle) -> Self {
        Self {
            texture,
            size: [1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = [width.max(0.001), height.max(0.001)];
        self
    }

    pub fn with_color(mut self, rgba: [f32; 4]) -> Self {
        self.color = rgba;
        self
    }

    pub fn with_uv_rect(mut self, uv_min: [f32; 2], uv_max: [f32; 2]) -> Self {
        self.uv_min = uv_min;
        self.uv_max = uv_max;
        self
    }
}

pub struct RenderModule {
    state: Option<RenderState>,
    clear_color: wgpu::Color,
}

impl Default for RenderModule {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderModule {
    pub fn new() -> Self {
        Self {
            state: None,
            clear_color: wgpu::Color {
                r: 0.06,
                g: 0.08,
                b: 0.12,
                a: 1.0,
            },
        }
    }

    pub fn initialize_with_window(&mut self, window: Arc<Window>, vsync: bool) -> Result<()> {
        if self.state.is_some() {
            return Ok(());
        }

        let state = RenderState::new(window, vsync)?;
        log::info!(
            target: "engine::render",
            "Render backend initialized: {} ({:?})",
            state.adapter_info.name,
            state.adapter_info.backend
        );

        self.state = Some(state);
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(state) = self.state.as_mut() {
            state.resize(width, height);
        }
    }

    pub fn tick(&mut self, world: &mut World, assets: &AssetServer) -> Result<()> {
        let Some(state) = self.state.as_mut() else {
            log::trace!(target: "engine::render", "Render tick skipped (backend not initialized)");
            return Ok(());
        };

        state.render(world, assets, self.clear_color)
    }

    pub fn backend_type_name(&self) -> &'static str {
        std::any::type_name::<wgpu::Backends>()
    }
}

struct RenderState {
    _instance: wgpu::Instance,
    _window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    adapter_info: wgpu::AdapterInfo,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_target: DepthTarget,
    pipeline_3d: Pipeline3d,
    pipeline_2d: Pipeline2d,
    gpu_meshes: HashMap<AssetId, GpuMesh>,
    gpu_textures: HashMap<AssetId, GpuTexture>,
}

struct DepthTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct Pipeline3d {
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    model_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
}

struct Pipeline2d {
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    sprite_layout: wgpu::BindGroupLayout,
    quad_vertex_buffer: wgpu::Buffer,
    quad_index_buffer: wgpu::Buffer,
    quad_index_count: u32,
}

struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

struct GpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
    revision: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Camera3dUniform {
    view_proj: [[f32; 4]; 4],
    camera_position: [f32; 4],
    light_direction: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Camera2dUniform {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    base_color: [f32; 4],
    metallic_roughness: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

impl GpuVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<GpuVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteQuadVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl SpriteQuadVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<SpriteQuadVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteInstance {
    model: [[f32; 4]; 4],
    color: [f32; 4],
    uv_rect: [f32; 4],
}

impl SpriteInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<SpriteInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Clone, Copy)]
struct DrawItem3d {
    mesh: MeshHandle,
    texture: TextureHandle,
    material: MaterialHandle,
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 4],
}

#[derive(Clone, Copy)]
struct DrawItem2d {
    texture: TextureHandle,
    model: [[f32; 4]; 4],
    color: [f32; 4],
    uv_rect: [f32; 4],
    sort_z: f32,
}

impl RenderState {
    fn new(window: Arc<Window>, vsync: bool) -> Result<Self> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| EngineError::Render(format!("failed to create surface: {error}")))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| EngineError::Render("failed to request a compatible adapter".to_owned()))?;

        let adapter_info = adapter.get_info();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("engine-render-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|error| EngineError::Render(format!("failed to request device: {error}")))?;

        let capabilities = surface.get_capabilities(&adapter);
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| {
                EngineError::Render("surface does not expose a default configuration".to_owned())
            })?;

        config.format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(config.format);
        config.present_mode = choose_present_mode(vsync, &capabilities.present_modes);
        config.alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        surface.configure(&device, &config);
        let depth_target = create_depth_target(&device, width, height);
        let pipeline_3d = create_pipeline_3d(&device, config.format);
        let pipeline_2d = create_pipeline_2d(&device, config.format);

        Ok(Self {
            _instance: instance,
            _window: window,
            surface,
            adapter_info,
            device,
            queue,
            config,
            depth_target,
            pipeline_3d,
            pipeline_2d,
            gpu_meshes: HashMap::new(),
            gpu_textures: HashMap::new(),
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_target = create_depth_target(&self.device, width, height);
    }

    fn render(
        &mut self,
        world: &mut World,
        assets: &AssetServer,
        clear_color: wgpu::Color,
    ) -> Result<()> {
        if let Some(camera_uniform) = extract_camera_uniform_3d(world) {
            self.queue.write_buffer(
                &self.pipeline_3d.camera_buffer,
                0,
                bytemuck::bytes_of(&camera_uniform),
            );
        }

        let camera_2d_uniform =
            extract_camera_uniform_2d(world, self.config.width, self.config.height);
        self.queue.write_buffer(
            &self.pipeline_2d.camera_buffer,
            0,
            bytemuck::bytes_of(&camera_2d_uniform),
        );

        let draw_items_3d = collect_draw_items_3d(world);
        for draw_item in &draw_items_3d {
            self.ensure_gpu_mesh(draw_item.mesh, assets)?;
            self.ensure_gpu_texture(draw_item.texture, assets)?;
        }

        let draw_items_2d = collect_draw_items_2d(world);
        for draw_item in &draw_items_2d {
            self.ensure_gpu_texture(draw_item.texture, assets)?;
        }

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                log::warn!(
                    target: "engine::render",
                    "Surface outdated/lost; reconfiguring swapchain"
                );
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!(target: "engine::render", "Surface acquire timeout");
                return Ok(());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(EngineError::Render(
                    "surface out of memory while acquiring frame".to_owned(),
                ));
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("engine-render-main-encoder"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("engine-render-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_target.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.encode_3d_pass(&mut encoder, &view, assets, &draw_items_3d);
        self.encode_2d_pass(&mut encoder, &view, &draw_items_2d);

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn encode_3d_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        assets: &AssetServer,
        draw_items: &[DrawItem3d],
    ) {
        let fallback_material = MaterialData {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 1.0,
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("engine-render-3d-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_target.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.pipeline_3d.pipeline);
        render_pass.set_bind_group(0, &self.pipeline_3d.camera_bind_group, &[]);

        let mut frame_model_buffers = Vec::with_capacity(draw_items.len());
        let mut frame_material_buffers = Vec::with_capacity(draw_items.len());
        let mut frame_model_bind_groups = Vec::with_capacity(draw_items.len());
        let mut frame_material_bind_groups = Vec::with_capacity(draw_items.len());

        for draw_item in draw_items {
            let Some(gpu_mesh) = self.gpu_meshes.get(&draw_item.mesh.id()) else {
                continue;
            };
            let Some(gpu_texture) = self.gpu_textures.get(&draw_item.texture.id()) else {
                continue;
            };

            let material = assets
                .material_payload(draw_item.material)
                .unwrap_or(&fallback_material);

            let model_uniform = ModelUniform {
                model: draw_item.model,
                normal: draw_item.normal,
            };
            let model_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("engine-render-model-uniform"),
                    contents: bytemuck::bytes_of(&model_uniform),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            frame_model_buffers.push(model_buffer);
            let model_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("engine-render-model-bind-group"),
                layout: &self.pipeline_3d.model_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_model_buffers
                        .last()
                        .expect("model buffer inserted")
                        .as_entire_binding(),
                }],
            });
            frame_model_bind_groups.push(model_bind_group);

            let material_uniform = MaterialUniform {
                base_color: material.base_color_factor,
                metallic_roughness: [material.metallic, material.roughness, 0.0, 0.0],
            };
            let material_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("engine-render-material-uniform"),
                        contents: bytemuck::bytes_of(&material_uniform),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
            frame_material_buffers.push(material_buffer);
            let material_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("engine-render-material-bind-group"),
                layout: &self.pipeline_3d.material_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: frame_material_buffers
                            .last()
                            .expect("material buffer inserted")
                            .as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&gpu_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&gpu_texture.sampler),
                    },
                ],
            });
            frame_material_bind_groups.push(material_bind_group);

            let model_bind_group = frame_model_bind_groups
                .last()
                .expect("model bind group inserted");
            let material_bind_group = frame_material_bind_groups
                .last()
                .expect("material bind group inserted");

            render_pass.set_bind_group(1, model_bind_group, &[]);
            render_pass.set_bind_group(2, material_bind_group, &[]);
            render_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..gpu_mesh.index_count, 0, 0..1);
        }
    }

    fn encode_2d_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        draw_items: &[DrawItem2d],
    ) {
        if draw_items.is_empty() {
            return;
        }

        let instances: Vec<SpriteInstance> = draw_items
            .iter()
            .map(|draw_item| SpriteInstance {
                model: draw_item.model,
                color: draw_item.color,
                uv_rect: draw_item.uv_rect,
            })
            .collect();

        let instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("engine-render-sprite-instance-buffer"),
                contents: bytemuck::cast_slice(instances.as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("engine-render-2d-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.pipeline_2d.pipeline);
        render_pass.set_bind_group(0, &self.pipeline_2d.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.pipeline_2d.quad_vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.set_index_buffer(
            self.pipeline_2d.quad_index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );

        let mut frame_sprite_bind_groups = Vec::new();
        let mut batch_start = 0usize;
        while batch_start < draw_items.len() {
            let texture = draw_items[batch_start].texture;
            let mut batch_end = batch_start + 1;
            while batch_end < draw_items.len() && draw_items[batch_end].texture.id() == texture.id()
            {
                batch_end += 1;
            }

            let Some(gpu_texture) = self.gpu_textures.get(&texture.id()) else {
                batch_start = batch_end;
                continue;
            };

            let sprite_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("engine-render-sprite-bind-group"),
                layout: &self.pipeline_2d.sprite_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&gpu_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&gpu_texture.sampler),
                    },
                ],
            });
            frame_sprite_bind_groups.push(sprite_bind_group);

            let bind_group = frame_sprite_bind_groups
                .last()
                .expect("sprite bind group inserted");
            render_pass.set_bind_group(1, bind_group, &[]);
            render_pass.draw_indexed(
                0..self.pipeline_2d.quad_index_count,
                0,
                batch_start as u32..batch_end as u32,
            );

            batch_start = batch_end;
        }
    }

    fn ensure_gpu_mesh(&mut self, handle: MeshHandle, assets: &AssetServer) -> Result<()> {
        if self.gpu_meshes.contains_key(&handle.id()) {
            return Ok(());
        }

        let payload = assets.mesh_payload(handle).ok_or_else(|| {
            EngineError::Render(format!(
                "mesh payload missing for handle {}",
                handle.id().value()
            ))
        })?;

        let vertices: Vec<GpuVertex> = payload
            .vertices
            .iter()
            .map(|vertex| GpuVertex {
                position: vertex.position,
                normal: vertex.normal,
                uv: vertex.uv,
            })
            .collect();

        let index_count = u32::try_from(payload.indices.len()).map_err(|_| {
            EngineError::Render("mesh index count overflow for u32 draw call".to_owned())
        })?;

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("engine-render-mesh-vertex-buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("engine-render-mesh-index-buffer"),
                contents: bytemuck::cast_slice(payload.indices.as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            });

        self.gpu_meshes.insert(
            handle.id(),
            GpuMesh {
                vertex_buffer,
                index_buffer,
                index_count,
            },
        );

        Ok(())
    }

    fn ensure_gpu_texture(&mut self, handle: TextureHandle, assets: &AssetServer) -> Result<()> {
        let payload = assets.texture_payload(handle).ok_or_else(|| {
            EngineError::Render(format!(
                "texture payload missing for handle {}",
                handle.id().value()
            ))
        })?;

        let width = payload.width.max(1);
        let height = payload.height.max(1);

        if let Some(gpu_texture) = self.gpu_textures.get_mut(&handle.id()) {
            if gpu_texture.revision == payload.revision {
                return Ok(());
            }

            if gpu_texture.width == width && gpu_texture.height == height {
                upload_rgba8_texture(
                    &self.queue,
                    &gpu_texture.texture,
                    width,
                    height,
                    payload.pixels_rgba8.as_slice(),
                )?;
                gpu_texture.revision = payload.revision;
                return Ok(());
            }
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
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
            &self.queue,
            &texture,
            width,
            height,
            payload.pixels_rgba8.as_slice(),
        )?;

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("engine-render-texture-sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        self.gpu_textures.insert(
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
}

fn upload_rgba8_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<()> {
    let row_bytes = width
        .checked_mul(RGBA8_BYTES_PER_PIXEL)
        .ok_or_else(|| EngineError::Render("texture row byte size overflow".to_owned()))?;
    let height_usize = usize::try_from(height)
        .map_err(|_| EngineError::Render("texture height conversion overflow".to_owned()))?;
    let row_bytes_usize = usize::try_from(row_bytes)
        .map_err(|_| EngineError::Render("texture row bytes conversion overflow".to_owned()))?;
    let expected_size = row_bytes_usize
        .checked_mul(height_usize)
        .ok_or_else(|| EngineError::Render("texture upload size overflow".to_owned()))?;

    if pixels.len() < expected_size {
        return Err(EngineError::Render(format!(
            "texture payload too small: expected at least {expected_size} bytes, got {}",
            pixels.len()
        )));
    }

    let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let aligned_row_bytes = if row_bytes % alignment == 0 {
        row_bytes
    } else {
        row_bytes
            .checked_add(alignment - (row_bytes % alignment))
            .ok_or_else(|| {
                EngineError::Render("aligned texture row byte size overflow".to_owned())
            })?
    };

    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    if aligned_row_bytes == row_bytes {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels[..expected_size],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(height),
            },
            extent,
        );
        return Ok(());
    }

    let aligned_row_bytes_usize = usize::try_from(aligned_row_bytes).map_err(|_| {
        EngineError::Render("aligned texture row bytes conversion overflow".to_owned())
    })?;
    let mut padded_pixels = vec![0_u8; aligned_row_bytes_usize * height_usize];

    for row in 0..height_usize {
        let src_start = row * row_bytes_usize;
        let src_end = src_start + row_bytes_usize;
        let dst_start = row * aligned_row_bytes_usize;
        let dst_end = dst_start + row_bytes_usize;
        padded_pixels[dst_start..dst_end].copy_from_slice(&pixels[src_start..src_end]);
    }

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        padded_pixels.as_slice(),
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(aligned_row_bytes),
            rows_per_image: Some(height),
        },
        extent,
    );

    Ok(())
}

fn create_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("engine-render-depth-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    DepthTarget {
        _texture: texture,
        view,
    }
}

fn create_pipeline_3d(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Pipeline3d {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("engine-render-mesh3d-shader"),
        source: wgpu::ShaderSource::Wgsl(MESH3D_SHADER.into()),
    });

    let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("engine-render-camera3d-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let model_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("engine-render-model-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("engine-render-material-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("engine-render-camera3d-uniform"),
        size: size_of::<Camera3dUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("engine-render-camera3d-bind-group"),
        layout: &camera_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("engine-render-mesh3d-pipeline-layout"),
        bind_group_layouts: &[&camera_layout, &model_layout, &material_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("engine-render-mesh3d-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[GpuVertex::layout()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    Pipeline3d {
        pipeline,
        camera_buffer,
        camera_bind_group,
        model_layout,
        material_layout,
    }
}

fn create_pipeline_2d(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Pipeline2d {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("engine-render-sprite2d-shader"),
        source: wgpu::ShaderSource::Wgsl(SPRITE2D_SHADER.into()),
    });

    let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("engine-render-camera2d-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let sprite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("engine-render-sprite-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("engine-render-camera2d-uniform"),
        size: size_of::<Camera2dUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("engine-render-camera2d-bind-group"),
        layout: &camera_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });

    let quad_vertices = [
        SpriteQuadVertex {
            position: [-0.5, -0.5],
            uv: [0.0, 1.0],
        },
        SpriteQuadVertex {
            position: [0.5, -0.5],
            uv: [1.0, 1.0],
        },
        SpriteQuadVertex {
            position: [0.5, 0.5],
            uv: [1.0, 0.0],
        },
        SpriteQuadVertex {
            position: [-0.5, 0.5],
            uv: [0.0, 0.0],
        },
    ];
    let quad_indices = [0u16, 1, 2, 0, 2, 3];

    let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("engine-render-sprite-quad-vertex-buffer"),
        contents: bytemuck::cast_slice(quad_vertices.as_slice()),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let quad_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("engine-render-sprite-quad-index-buffer"),
        contents: bytemuck::cast_slice(quad_indices.as_slice()),
        usage: wgpu::BufferUsages::INDEX,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("engine-render-sprite2d-pipeline-layout"),
        bind_group_layouts: &[&camera_layout, &sprite_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("engine-render-sprite2d-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[SpriteQuadVertex::layout(), SpriteInstance::layout()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    Pipeline2d {
        pipeline,
        camera_buffer,
        camera_bind_group,
        sprite_layout,
        quad_vertex_buffer,
        quad_index_buffer,
        quad_index_count: quad_indices.len() as u32,
    }
}

fn extract_camera_uniform_3d(world: &mut World) -> Option<Camera3dUniform> {
    let mut query = world.query_filtered::<(&Camera3d, &GlobalTransform), With<PrimaryCamera>>();
    let (camera, global_transform) = query.iter(world).next()?;

    let view = Mat4::from(global_transform.0.inverse());
    let view_proj = camera.projection_matrix() * view;
    let translation = global_transform.translation();

    Some(Camera3dUniform {
        view_proj: view_proj.to_cols_array_2d(),
        camera_position: [translation.x, translation.y, translation.z, 1.0],
        light_direction: [0.6, -1.0, 0.2, 0.0],
    })
}

fn extract_camera_uniform_2d(
    world: &mut World,
    viewport_width: u32,
    viewport_height: u32,
) -> Camera2dUniform {
    let mut query = world.query_filtered::<(&Camera2d, &GlobalTransform), With<PrimaryCamera>>();

    let view_proj = if let Some((camera, global_transform)) = query.iter(world).next() {
        let view = Mat4::from(global_transform.0.inverse());
        camera.projection_matrix() * view
    } else {
        let width = viewport_width.max(1) as f32;
        let height = viewport_height.max(1) as f32;
        Mat4::orthographic_rh(
            -width * 0.5,
            width * 0.5,
            -height * 0.5,
            height * 0.5,
            -1.0,
            1.0,
        )
    };

    Camera2dUniform {
        view_proj: view_proj.to_cols_array_2d(),
    }
}

fn collect_draw_items_3d(world: &mut World) -> Vec<DrawItem3d> {
    let mut draw_items = Vec::new();
    let mut query = world.query_filtered::<
        (&GlobalTransform, &MeshRenderable3d),
        (With<Visible>, With<RenderLayer3D>),
    >();

    for (global_transform, mesh_renderable) in query.iter(world) {
        let model = Mat4::from(global_transform.0);
        let normal = model.inverse().transpose();

        draw_items.push(DrawItem3d {
            mesh: mesh_renderable.mesh,
            texture: mesh_renderable.texture,
            material: mesh_renderable.material,
            model: model.to_cols_array_2d(),
            normal: normal.to_cols_array_2d(),
        });
    }

    draw_items
}

fn collect_draw_items_2d(world: &mut World) -> Vec<DrawItem2d> {
    let mut draw_items = Vec::new();
    let mut query = world.query_filtered::<
        (&GlobalTransform, &SpriteRenderable2d),
        (With<Visible>, With<RenderLayer2D>),
    >();

    for (global_transform, sprite) in query.iter(world) {
        let model = Mat4::from(global_transform.0)
            * Mat4::from_scale(Vec3::new(sprite.size[0], sprite.size[1], 1.0));
        let translation = global_transform.translation();

        draw_items.push(DrawItem2d {
            texture: sprite.texture,
            model: model.to_cols_array_2d(),
            color: sprite.color,
            uv_rect: [
                sprite.uv_min[0],
                sprite.uv_min[1],
                sprite.uv_max[0],
                sprite.uv_max[1],
            ],
            sort_z: translation.z,
        });
    }

    draw_items.sort_by(|left, right| {
        left.sort_z
            .partial_cmp(&right.sort_z)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.texture.id().value().cmp(&right.texture.id().value()))
    });

    draw_items
}

const MESH3D_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_direction: vec4<f32>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    normal: mat4x4<f32>,
};

struct MaterialUniform {
    base_color: vec4<f32>,
    metallic_roughness: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> model: ModelUniform;
@group(2) @binding(0) var<uniform> material: MaterialUniform;
@group(2) @binding(1) var t_albedo: texture_2d<f32>;
@group(2) @binding(2) var s_albedo: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = model.model * vec4<f32>(input.position, 1.0);
    output.clip_position = camera.view_proj * world_pos;
    output.world_position = world_pos.xyz;
    output.world_normal = normalize((model.normal * vec4<f32>(input.normal, 0.0)).xyz);
    output.uv = input.uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let albedo_sample = textureSample(t_albedo, s_albedo, input.uv);
    let base_color = albedo_sample * material.base_color;
    let metallic = clamp(material.metallic_roughness.x, 0.0, 1.0);
    let roughness = clamp(material.metallic_roughness.y, 0.04, 1.0);

    let n = normalize(input.world_normal);
    let l = normalize(-camera.light_direction.xyz);
    let v = normalize(camera.camera_position.xyz - input.world_position);
    let h = normalize(l + v);

    let ndotl = max(dot(n, l), 0.0);
    let ndoth = max(dot(n, h), 0.0);
    let spec_power = mix(256.0, 4.0, roughness);
    let specular_strength = pow(ndoth, spec_power);

    let diffuse = base_color.rgb * ndotl * (1.0 - metallic);
    let specular = vec3<f32>(specular_strength) * mix(0.04, 1.0, metallic);
    let ambient = base_color.rgb * 0.08;

    let color = ambient + diffuse + specular;
    return vec4<f32>(color, base_color.a);
}
"#;

const SPRITE2D_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;

struct VertexInput {
    @location(0) local_position: vec2<f32>,
    @location(1) quad_uv: vec2<f32>,
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) color: vec4<f32>,
    @location(7) uv_rect: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world_position = model * vec4<f32>(input.local_position, 0.0, 1.0);

    output.clip_position = camera.view_proj * world_position;
    output.uv = mix(input.uv_rect.xy, input.uv_rect.zw, input.quad_uv);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(t_sprite, s_sprite, input.uv);
    return texel * input.color;
}
"#;

fn choose_present_mode(vsync: bool, supported_modes: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    if vsync {
        return wgpu::PresentMode::Fifo;
    }

    for preferred in [
        wgpu::PresentMode::Immediate,
        wgpu::PresentMode::Mailbox,
        wgpu::PresentMode::FifoRelaxed,
        wgpu::PresentMode::Fifo,
    ] {
        if supported_modes.contains(&preferred) {
            return preferred;
        }
    }

    wgpu::PresentMode::Fifo
}

pub fn module_name() -> &'static str {
    "engine-render"
}
