use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Sets the process-wide data directory used by runtime storage helpers.
#[allow(non_snake_case)]
pub fn setDefaultRuntimeStoreRoot(root_dir: PathBuf) {
    let holder = DEFAULT_RUNTIME_STORE_ROOT.get_or_init(|| Mutex::new(None));
    let mut guard = holder
        .lock()
        .expect("default runtime store root mutex poisoned");
    *guard = Some(root_dir);
}

/// Returns the configured process-wide runtime data directory.
pub fn default_data_dir() -> PathBuf {
    default_runtime_store_root().expect("default runtime store root is not registered")
}

fn default_runtime_store_root() -> Option<PathBuf> {
    let holder = DEFAULT_RUNTIME_STORE_ROOT.get_or_init(|| Mutex::new(None));
    holder
        .lock()
        .expect("default runtime store root mutex poisoned")
        .clone()
}

static DEFAULT_RUNTIME_STORE_ROOT: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
