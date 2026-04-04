use std::fs;
use std::path::PathBuf;

use egui_dock::DockState;
use engine_core::{EngineError, Result};
use serde::{Deserialize, Serialize};

use crate::layout::{create_default_layout, Tab};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "create_default_layout")]
    pub dock_state: DockState<Tab>,
    #[serde(default)]
    pub recent_files: Vec<PathBuf>,
    pub last_opened_scene: Option<PathBuf>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            dock_state: create_default_layout(),
            recent_files: Vec::new(),
            last_opened_scene: None,
        }
    }
}

impl EditorConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("motley")
            .join("engine-editor")
            .join("config.ron")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let Ok(contents) = fs::read_to_string(&path) else {
            return Self::default();
        };

        match ron::from_str::<Self>(&contents) {
            Ok(config) => config,
            Err(error) => {
                log::warn!(
                    target: "engine::editor",
                    "Failed to parse editor config at {}: {}",
                    path.display(),
                    error
                );
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                EngineError::Config(format!(
                    "failed to create config directory '{}': {}",
                    parent.display(),
                    error
                ))
            })?;
        }

        let payload = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new())
            .map_err(|error| EngineError::Config(format!("failed to serialize config: {}", error)))?;

        fs::write(&path, payload).map_err(|error| {
            EngineError::Config(format!("failed to write config '{}': {}", path.display(), error))
        })
    }

    pub fn touch_recent_file(&mut self, path: PathBuf) {
        self.recent_files.retain(|entry| entry != &path);
        self.recent_files.insert(0, path.clone());
        self.recent_files.truncate(10);
        self.last_opened_scene = Some(path);
    }
}
