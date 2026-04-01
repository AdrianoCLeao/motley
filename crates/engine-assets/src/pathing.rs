use engine_core::{EngineError, Result};
use std::path::{Component, Path, PathBuf};

use crate::AssetPath;

pub(crate) fn resolve_disk_path(root: &AssetPath, relative_path: &str) -> Result<PathBuf> {
    if relative_path.trim().is_empty() {
        return Err(EngineError::AssetLoad {
            path: relative_path.to_owned(),
            reason: "path cannot be empty".to_owned(),
        });
    }

    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(EngineError::AssetLoad {
            path: relative_path.to_owned(),
            reason: "path must be root-relative without traversal segments".to_owned(),
        });
    }

    let path = Path::new(root.as_str()).join(relative);
    let normalized = normalize_disk_path(&path);

    let normalized_root = normalize_disk_path(Path::new(root.as_str()));
    if normalized.is_absolute()
        && normalized_root.is_absolute()
        && !normalized.starts_with(&normalized_root)
    {
        return Err(EngineError::AssetLoad {
            path: relative_path.to_owned(),
            reason: "resolved path escaped asset root".to_owned(),
        });
    }

    Ok(normalized)
}

pub(crate) fn normalize_disk_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn to_asset_path(path: &Path) -> AssetPath {
    AssetPath::new(path.to_string_lossy().replace('\\', "/"))
}
