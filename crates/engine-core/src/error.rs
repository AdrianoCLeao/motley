use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Render error: {0}")]
    Render(String),
    #[error("Asset loading failed for '{path}': {reason}")]
    AssetLoad { path: String, reason: String },
    #[error("Physics world is not initialized")]
    PhysicsNotInitialized,
    #[error("Audio backend unavailable: {0}")]
    Audio(String),
    #[error("Windowing error: {0}")]
    Window(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;
