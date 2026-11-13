use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::PathMapper::PathMapper;
use operit_host_contract::FileSystemResource::FileSystemResource;

static WATCHERS: OnceLock<Mutex<Vec<Weak<DirectoryWatch>>>> = OnceLock::new();

struct DirectoryWatch {
    root: String,
    changed: Arc<dyn Fn() + Send + Sync>,
}

/// Keeps one runtime-scoped VFS directory observation alive until its owner drops it.
pub struct FileChangeSubscription(Arc<DirectoryWatch>);

/// Validates the watched directory and isolates it by its resolved backend resource.
pub(crate) fn subscribe(root: String, changed: Arc<dyn Fn() + Send + Sync>) -> FileChangeSubscription {
    let watch = Arc::new(DirectoryWatch { root, changed });
    WATCHERS.get_or_init(Default::default).lock().expect("VFS watchers mutex poisoned").push(Arc::downgrade(&watch));
    FileChangeSubscription(watch)
}

/// Delivers committed mutations once per observer, outside registry locks and across VFS aliases.
pub(crate) fn publish(paths: &[&str]) -> Result<(), String> {
    let Some(watchers) = WATCHERS.get() else { return Ok(()); };
    let active = {
        let mut watchers = watchers.lock().expect("VFS watchers mutex poisoned");
        let mut active = Vec::new();
        watchers.retain(|watch| match watch.upgrade() {
            Some(watch) => { active.push(watch); true }
            None => false,
        });
        active
    };
    for watch in active {
        for path in paths {
            if contains(&watch.root, path)? || contains(path, &watch.root)? {
                (watch.changed)();
                break;
            }
        }
    }
    Ok(())
}

/// Compares resolved backend identities directly without remapping native locators as VFS input.
fn contains(root: &str, path: &str) -> Result<bool, String> {
    match (FileSystemResource::parse(root).map_err(|error| error.to_string())?,
        FileSystemResource::parse(path).map_err(|error| error.to_string())?) {
        (Some(root), Some(path)) if root.backend == path.backend && root.root == path.root =>
            Ok(PathMapper::relativePath(&format!("/{}", root.path), &format!("/{}", path.path))?.is_some()),
        (Some(_), _) | (_, Some(_)) => Ok(false),
        (None, None) => {
            let root = PathMapper::resolveFileResourcePath(None, root)?;
            let path = PathMapper::resolveFileResourcePath(None, path)?;
            let (root, path) = if root.starts_with("/virtual/windows/") && path.starts_with("/virtual/windows/") {
                (root.to_lowercase(), path.to_lowercase())
            } else { (root, path) };
            Ok(PathMapper::relativePath(&root, &path)?.is_some())
        }
    }
}
