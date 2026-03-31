use engine_core::{EngineError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPath(String);

impl AssetPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct AssetModule {
    root: AssetPath,
}

impl AssetModule {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: AssetPath::new(root),
        }
    }

    pub fn load_stub(&self, relative_path: &str) -> Result<AssetPath> {
        if relative_path.trim().is_empty() {
            return Err(EngineError::AssetLoad {
                path: relative_path.to_owned(),
                reason: "path cannot be empty".to_owned(),
            });
        }

        log::trace!(target: "engine::assets", "Loading asset stub: {}", relative_path);
        Ok(AssetPath::new(format!(
            "{}/{}",
            self.root.as_str(),
            relative_path
        )))
    }

    pub fn supported_formats() -> &'static [&'static str] {
        &["png", "jpeg", "gltf", "ron"]
    }
}

pub fn module_name() -> &'static str {
    "engine-assets"
}
