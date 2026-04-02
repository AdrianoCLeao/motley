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

pub(crate) fn to_relative_asset_path(root: &AssetPath, path: &AssetPath) -> Option<String> {
    let root_path = Path::new(root.as_str());
    let asset_path = Path::new(path.as_str());

    if !asset_path.is_absolute() {
        if let Ok(stripped) = asset_path.strip_prefix(root_path) {
            let relative = stripped.to_string_lossy().replace('\\', "/");
            return Some(relative.trim_start_matches('/').to_owned());
        }

        return Some(asset_path.to_string_lossy().replace('\\', "/"));
    }

    let normalized_root = normalize_disk_path(root_path);
    let normalized_asset = normalize_disk_path(asset_path);
    let stripped = normalized_asset.strip_prefix(&normalized_root).ok()?;
    let relative = stripped.to_string_lossy().replace('\\', "/");
    Some(relative.trim_start_matches('/').to_owned())
}
