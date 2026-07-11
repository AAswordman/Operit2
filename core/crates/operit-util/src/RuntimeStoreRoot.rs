use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq, Eq)]
/// Stores the active runtime and workspace root directories for this process.
pub struct RuntimeStoreRootConfig {
    pub runtime_root: PathBuf,
    pub workspace_root: PathBuf,
}

impl RuntimeStoreRootConfig {
    /// Creates a root configuration with explicit runtime and workspace roots.
    pub fn new(runtime_root: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            runtime_root,
            workspace_root,
        }
    }
}

/// Sets the process-wide runtime and workspace storage directories.
#[allow(non_snake_case)]
pub fn setDefaultRuntimeStoreRootConfig(config: RuntimeStoreRootConfig) {
    let holder = DEFAULT_RUNTIME_STORE_ROOT_CONFIG.get_or_init(|| Mutex::new(None));
    let mut guard = holder
        .lock()
        .expect("default runtime store root config mutex poisoned");
    *guard = Some(config);
}

/// Returns the configured runtime directory.
pub fn default_runtime_dir() -> PathBuf {
    default_runtime_store_root_config()
        .expect("default runtime store root is not registered")
        .runtime_root
}

/// Returns the configured workspace collection directory.
pub fn default_workspace_dir() -> PathBuf {
    default_runtime_store_root_config()
        .expect("default runtime store root is not registered")
        .workspace_root
}

/// Returns the configured process-wide storage root config.
pub fn default_runtime_store_root_config() -> Option<RuntimeStoreRootConfig> {
    let holder = DEFAULT_RUNTIME_STORE_ROOT_CONFIG.get_or_init(|| Mutex::new(None));
    holder
        .lock()
        .expect("default runtime store root config mutex poisoned")
        .clone()
}

static DEFAULT_RUNTIME_STORE_ROOT_CONFIG: OnceLock<Mutex<Option<RuntimeStoreRootConfig>>> =
    OnceLock::new();
