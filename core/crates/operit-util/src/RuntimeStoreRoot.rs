use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq, Eq)]
/// Stores the active runtime and workspace root directories for this process.
pub struct RuntimeStoreRootConfig {
    pub data_root: PathBuf,
    pub runtime_root: PathBuf,
    pub workspace_root: PathBuf,
}

impl RuntimeStoreRootConfig {
    /// Creates a root configuration that preserves the original data-root layout.
    pub fn fromDataRoot(data_root: PathBuf) -> Self {
        Self {
            runtime_root: data_root.join("runtime"),
            workspace_root: data_root.join("workspaces"),
            data_root,
        }
    }

    /// Creates a root configuration with explicit runtime and workspace roots.
    pub fn new(data_root: PathBuf, runtime_root: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            data_root,
            runtime_root,
            workspace_root,
        }
    }
}

/// Sets the process-wide data directory used by runtime storage helpers.
#[allow(non_snake_case)]
pub fn setDefaultRuntimeStoreRoot(root_dir: PathBuf) {
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::fromDataRoot(root_dir));
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

/// Returns the configured process-wide runtime data directory.
pub fn default_data_dir() -> PathBuf {
    default_runtime_store_root_config()
        .expect("default runtime store root is not registered")
        .data_root
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

/// Builds the default runtime directory for a data root.
pub fn runtime_dir_for_data_root(data_root: &Path) -> PathBuf {
    data_root.join("runtime")
}

/// Builds the default workspace directory for a data root.
pub fn workspace_dir_for_data_root(data_root: &Path) -> PathBuf {
    data_root.join("workspaces")
}

static DEFAULT_RUNTIME_STORE_ROOT_CONFIG: OnceLock<Mutex<Option<RuntimeStoreRootConfig>>> =
    OnceLock::new();
