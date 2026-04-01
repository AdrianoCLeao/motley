use bevy_ecs::system::Resource;

#[derive(Resource, Debug, Clone, Copy)]
pub struct HardeningConfig {
    pub max_buffered_input_events: usize,
    pub max_registered_gamepads: usize,
    pub max_texture_dimension: u32,
    pub max_texture_payload_bytes: usize,
    pub max_mesh_vertices: usize,
    pub max_mesh_indices: usize,
    pub max_draw_items_3d: usize,
    pub max_draw_items_2d: usize,
}

impl HardeningConfig {
    pub const DEFAULT_MAX_BUFFERED_INPUT_EVENTS: usize = 4_096;
    pub const DEFAULT_MAX_REGISTERED_GAMEPADS: usize = 16;
    pub const DEFAULT_MAX_TEXTURE_DIMENSION: u32 = 8_192;
    pub const DEFAULT_MAX_TEXTURE_PAYLOAD_BYTES: usize = 256 * 1024 * 1024;
    pub const DEFAULT_MAX_MESH_VERTICES: usize = 1_000_000;
    pub const DEFAULT_MAX_MESH_INDICES: usize = 3_000_000;
    pub const DEFAULT_MAX_DRAW_ITEMS_3D: usize = 200_000;
    pub const DEFAULT_MAX_DRAW_ITEMS_2D: usize = 200_000;
}

impl Default for HardeningConfig {
    fn default() -> Self {
        Self {
            max_buffered_input_events: Self::DEFAULT_MAX_BUFFERED_INPUT_EVENTS,
            max_registered_gamepads: Self::DEFAULT_MAX_REGISTERED_GAMEPADS,
            max_texture_dimension: Self::DEFAULT_MAX_TEXTURE_DIMENSION,
            max_texture_payload_bytes: Self::DEFAULT_MAX_TEXTURE_PAYLOAD_BYTES,
            max_mesh_vertices: Self::DEFAULT_MAX_MESH_VERTICES,
            max_mesh_indices: Self::DEFAULT_MAX_MESH_INDICES,
            max_draw_items_3d: Self::DEFAULT_MAX_DRAW_ITEMS_3D,
            max_draw_items_2d: Self::DEFAULT_MAX_DRAW_ITEMS_2D,
        }
    }
}
