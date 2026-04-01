use notify::{EventKind, RecommendedWatcher};
use std::sync::mpsc::{self, Receiver};

pub(crate) fn build_hot_reload_watcher() -> (
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

pub(crate) fn is_hot_reload_event(kind: &EventKind) -> bool {
    matches!(kind, EventKind::Modify(_) | EventKind::Create(_))
}
