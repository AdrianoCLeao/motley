use engine_core::{EngineError, Result};
use image::GenericImageView;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    sync::mpsc::{self, Receiver, TryRecvError},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetPath(String);

impl AssetPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(u64);

impl AssetId {
    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    id: AssetId,
    generation: u32,
    marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    pub fn id(self) -> AssetId {
        self.id
    }

    pub fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AssetState {
    #[default]
    Loaded,
    Failed,
}

#[derive(Debug, Clone, Copy)]
struct HandleRecord {
    generation: u32,
    state: AssetState,
}

#[derive(Default)]
struct HandleRegistry {
    path_to_id: HashMap<AssetPath, AssetId>,
    id_to_record: HashMap<AssetId, HandleRecord>,
}

impl HandleRegistry {
    fn get_or_create<T>(&mut self, path: AssetPath, id_source: &AtomicU64) -> Handle<T> {
        if let Some(id) = self.path_to_id.get(&path).copied() {
            let record = self.id_to_record.get(&id).copied().unwrap_or(HandleRecord {
                generation: 1,
                state: AssetState::Loaded,
            });

            return Handle {
                id,
                generation: record.generation,
                marker: PhantomData,
            };
        }

        let id = AssetId(id_source.fetch_add(1, Ordering::Relaxed));
        self.path_to_id.insert(path, id);
        self.id_to_record.insert(
            id,
            HandleRecord {
                generation: 1,
                state: AssetState::Loaded,
            },
        );

        Handle {
            id,
            generation: 1,
            marker: PhantomData,
        }
    }

    fn state_for<T>(&self, handle: Handle<T>) -> Option<AssetState> {
        let record = self.id_to_record.get(&handle.id)?;
        if record.generation != handle.generation {
            return None;
        }
        Some(record.state)
    }

    fn mark_failed<T>(&mut self, handle: Handle<T>) {
        if let Some(record) = self.id_to_record.get_mut(&handle.id) {
            record.state = AssetState::Failed;
        }
    }

    fn mark_loaded<T>(&mut self, handle: Handle<T>) {
        if let Some(record) = self.id_to_record.get_mut(&handle.id) {
            record.state = AssetState::Loaded;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextureAsset;

#[derive(Debug, Clone, Copy)]
pub struct MeshAsset;

#[derive(Debug, Clone, Copy)]
pub struct MaterialAsset;

pub type TextureHandle = Handle<TextureAsset>;
pub type MeshHandle = Handle<MeshAsset>;
pub type MaterialHandle = Handle<MaterialAsset>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub pixels_rgba8: Vec<u8>,
    #[serde(default)]
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshData {
    pub name: String,
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialData {
    pub base_color_factor: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

pub struct AssetServer {
    root: AssetPath,
    next_id: AtomicU64,
    next_texture_revision: u64,
    textures: HandleRegistry,
    meshes: HandleRegistry,
    materials: HandleRegistry,
    texture_payloads: HashMap<AssetId, TextureData>,
    mesh_payloads: HashMap<AssetId, MeshData>,
    material_payloads: HashMap<AssetId, MaterialData>,
    hot_reload_watcher: Option<RecommendedWatcher>,
    hot_reload_rx: Option<Receiver<notify::Result<notify::Event>>>,
    tracked_texture_files: HashMap<PathBuf, TextureHandle>,
}

impl AssetServer {
    pub fn new(root: impl Into<String>) -> Self {
        let (hot_reload_watcher, hot_reload_rx) = build_hot_reload_watcher();

        Self {
            root: AssetPath::new(root),
            next_id: AtomicU64::new(1),
            next_texture_revision: 1,
            textures: HandleRegistry::default(),
            meshes: HandleRegistry::default(),
            materials: HandleRegistry::default(),
            texture_payloads: HashMap::new(),
            mesh_payloads: HashMap::new(),
            material_payloads: HashMap::new(),
            hot_reload_watcher,
            hot_reload_rx,
            tracked_texture_files: HashMap::new(),
        }
    }

    pub fn root(&self) -> &AssetPath {
        &self.root
    }

    pub fn resolve_path(&self, relative_path: &str) -> Result<AssetPath> {
        let disk_path = self.resolve_disk_path(relative_path)?;
        Ok(to_asset_path(&disk_path))
    }

    fn resolve_disk_path(&self, relative_path: &str) -> Result<PathBuf> {
        if relative_path.trim().is_empty() {
            return Err(EngineError::AssetLoad {
                path: relative_path.to_owned(),
                reason: "path cannot be empty".to_owned(),
            });
        }

        let path = Path::new(self.root.as_str()).join(relative_path);
        Ok(normalize_disk_path(&path))
    }

    pub fn load_texture_handle(&mut self, relative_path: &str) -> Result<TextureHandle> {
        let disk_path = self.resolve_disk_path(relative_path)?;
        let path = to_asset_path(&disk_path);
        let handle = self.textures.get_or_create(path.clone(), &self.next_id);

        match load_texture_payload(&disk_path) {
            Ok(mut payload) => {
                payload.revision = self.allocate_texture_revision();
                self.texture_payloads.insert(handle.id(), payload);
                self.textures.mark_loaded(handle);
                self.register_texture_watch(&disk_path, handle);
            }
            Err(error) => {
                self.textures.mark_failed(handle);
                return Err(error);
            }
        }

        log::trace!(
            target: "engine::assets",
            "Resolved texture handle {} for {}",
            handle.id().value(),
            path.as_str()
        );
        Ok(handle)
    }

    pub fn load_mesh_handle(&mut self, relative_path: &str) -> Result<MeshHandle> {
        let disk_path = self.resolve_disk_path(relative_path)?;
        let path = to_asset_path(&disk_path);
        let handle = self.meshes.get_or_create(path.clone(), &self.next_id);

        match load_mesh_payload(&disk_path) {
            Ok(payload) => {
                self.mesh_payloads.insert(handle.id(), payload);
                self.meshes.mark_loaded(handle);
            }
            Err(error) => {
                self.meshes.mark_failed(handle);
                return Err(error);
            }
        }

        log::trace!(
            target: "engine::assets",
            "Resolved mesh handle {} for {}",
            handle.id().value(),
            path.as_str()
        );
        Ok(handle)
    }

    pub fn load_material_handle(&mut self, relative_path: &str) -> Result<MaterialHandle> {
        let disk_path = self.resolve_disk_path(relative_path)?;
        let path = to_asset_path(&disk_path);
        let handle = self.materials.get_or_create(path.clone(), &self.next_id);
        self.material_payloads.insert(
            handle.id(),
            MaterialData {
                base_color_factor: [1.0, 1.0, 1.0, 1.0],
                metallic: 0.0,
                roughness: 1.0,
            },
        );
        self.materials.mark_loaded(handle);
        log::trace!(
            target: "engine::assets",
            "Resolved material handle {} for {}",
            handle.id().value(),
            path.as_str()
        );
        Ok(handle)
    }

    pub fn texture_state(&self, handle: TextureHandle) -> Option<AssetState> {
        self.textures.state_for(handle)
    }

    pub fn mesh_state(&self, handle: MeshHandle) -> Option<AssetState> {
        self.meshes.state_for(handle)
    }

    pub fn material_state(&self, handle: MaterialHandle) -> Option<AssetState> {
        self.materials.state_for(handle)
    }

    pub fn mark_texture_failed(&mut self, handle: TextureHandle) {
        self.textures.mark_failed(handle);
    }

    pub fn texture_payload(&self, handle: TextureHandle) -> Option<&TextureData> {
        self.texture_payloads.get(&handle.id())
    }

    pub fn mesh_payload(&self, handle: MeshHandle) -> Option<&MeshData> {
        self.mesh_payloads.get(&handle.id())
    }

    pub fn material_payload(&self, handle: MaterialHandle) -> Option<&MaterialData> {
        self.material_payloads.get(&handle.id())
    }

    pub fn poll_texture_hot_reload(&mut self) -> usize {
        let Some(rx) = self.hot_reload_rx.as_ref() else {
            return 0;
        };

        let mut changed_files = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(Ok(event)) => {
                    if !is_hot_reload_event(&event.kind) {
                        continue;
                    }

                    for path in event.paths {
                        changed_files.push(normalize_disk_path(&path));
                    }
                }
                Ok(Err(error)) => {
                    log::warn!(target: "engine::assets", "hot-reload event error: {error}");
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }

        let mut reload_count = 0;
        for file_path in changed_files {
            let Some(handle) = self.tracked_texture_files.get(&file_path).copied() else {
                continue;
            };

            match load_texture_payload(&file_path) {
                Ok(mut payload) => {
                    payload.revision = self.allocate_texture_revision();
                    self.texture_payloads.insert(handle.id(), payload);
                    self.textures.mark_loaded(handle);
                    reload_count += 1;
                    log::info!(
                        target: "engine::assets",
                        "hot-reloaded texture {}",
                        file_path.display()
                    );
                }
                Err(error) => {
                    self.textures.mark_failed(handle);
                    log::warn!(
                        target: "engine::assets",
                        "failed to hot-reload texture {}: {}",
                        file_path.display(),
                        error
                    );
                }
            }
        }

        reload_count
    }

    fn allocate_texture_revision(&mut self) -> u64 {
        let revision = self.next_texture_revision;
        self.next_texture_revision = self.next_texture_revision.saturating_add(1);
        revision
    }

    fn register_texture_watch(&mut self, disk_path: &Path, handle: TextureHandle) {
        let normalized = normalize_disk_path(disk_path);
        self.tracked_texture_files
            .insert(normalized.clone(), handle);

        let Some(watcher) = self.hot_reload_watcher.as_mut() else {
            return;
        };

        if let Err(error) = watcher.watch(&normalized, RecursiveMode::NonRecursive) {
            log::warn!(
                target: "engine::assets",
                "failed to watch texture {} for hot-reload: {}",
                normalized.display(),
                error
            );
        }
    }
}

pub struct AssetModule {
    server: AssetServer,
}

impl AssetModule {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            server: AssetServer::new(root),
        }
    }

    pub fn load_stub(&self, relative_path: &str) -> Result<AssetPath> {
        let path = self.server.resolve_path(relative_path)?;
        log::trace!(target: "engine::assets", "Loading asset stub: {}", path.as_str());
        Ok(path)
    }

    pub fn load_texture_handle(&mut self, relative_path: &str) -> Result<TextureHandle> {
        self.server.load_texture_handle(relative_path)
    }

    pub fn load_mesh_handle(&mut self, relative_path: &str) -> Result<MeshHandle> {
        self.server.load_mesh_handle(relative_path)
    }

    pub fn load_material_handle(&mut self, relative_path: &str) -> Result<MaterialHandle> {
        self.server.load_material_handle(relative_path)
    }

    pub fn asset_server(&self) -> &AssetServer {
        &self.server
    }

    pub fn asset_server_mut(&mut self) -> &mut AssetServer {
        &mut self.server
    }

    pub fn poll_texture_hot_reload(&mut self) -> usize {
        self.server.poll_texture_hot_reload()
    }

    pub fn supported_formats() -> &'static [&'static str] {
        &["png", "jpeg", "gltf", "glb", "ron", "ogg", "wav", "mp3"]
    }
}

pub fn module_name() -> &'static str {
    "engine-assets"
}

fn load_texture_payload(path: &Path) -> Result<TextureData> {
    let image = image::open(path).map_err(|error| EngineError::AssetLoad {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;

    let (width, height) = image.dimensions();
    let pixels_rgba8 = image.to_rgba8().into_raw();

    Ok(TextureData {
        width,
        height,
        pixels_rgba8,
        revision: 0,
    })
}

fn load_mesh_payload(path: &Path) -> Result<MeshData> {
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

            for ((position, normal), uv) in positions.into_iter().zip(normals).zip(uvs) {
                vertices.push(MeshVertex {
                    position,
                    normal,
                    uv,
                });
            }

            if let Some(read_indices) = reader.read_indices() {
                indices.extend(read_indices.into_u32().map(|index| base_index + index));
            } else {
                indices.extend((0..primitive_vertex_count).map(|index| base_index + index));
            }
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

fn build_hot_reload_watcher() -> (
    Option<RecommendedWatcher>,
    Option<Receiver<notify::Result<notify::Event>>>,
) {
    let (tx, rx) = mpsc::channel();
    match notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    }) {
        Ok(watcher) => (Some(watcher), Some(rx)),
        Err(error) => {
            log::warn!(
                target: "engine::assets",
                "texture hot-reload watcher unavailable: {}",
                error
            );
            (None, None)
        }
    }
}

fn is_hot_reload_event(kind: &EventKind) -> bool {
    matches!(kind, EventKind::Modify(_) | EventKind::Create(_))
}

fn normalize_disk_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn to_asset_path(path: &Path) -> AssetPath {
    AssetPath::new(path.to_string_lossy().replace('\\', "/"))
}
