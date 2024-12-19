pub mod loader;
pub mod texture;

pub use loader::{load_model, Material, Model, Vertex};
pub use texture::{Texture, load_texture};