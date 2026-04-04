use egui_dock::{DockState, NodeIndex};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tab {
    Viewport,
    SceneTree,
    Inspector,
    AssetBrowser,
    Console,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Viewport => "Viewport",
            Self::SceneTree => "Scene",
            Self::Inspector => "Inspector",
            Self::AssetBrowser => "Assets",
            Self::Console => "Console",
        }
    }
}

pub fn create_default_layout() -> DockState<Tab> {
    let mut state = DockState::new(vec![Tab::Viewport]);
    let surface = state.main_surface_mut();

    let [_center, _left] = surface.split_left(NodeIndex::root(), 0.20, vec![Tab::SceneTree]);
    let [_center, _right] = surface.split_right(NodeIndex::root(), 0.75, vec![Tab::Inspector]);
    let _ = surface.split_below(
        NodeIndex::root(),
        0.75,
        vec![Tab::AssetBrowser, Tab::Console],
    );

    state
}
