use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::{HostSecretStore, RuntimeSqliteHost, RuntimeStorageHost};

use crate::RuntimeStorePaths::RuntimeStorePaths;

/// Registers the runtime storage host used by store helpers and repositories.
#[allow(non_snake_case)]
pub fn setDefaultRuntimeStorageHost(host: Arc<dyn RuntimeStorageHost>) {
    let holder = DEFAULT_RUNTIME_STORAGE_HOST.get_or_init(|| Mutex::new(None));
    let mut guard = holder
        .lock()
        .expect("default runtime storage host mutex poisoned");
    *guard = Some(host);
}

/// Registers the SQLite host used by database-backed runtime stores.
#[allow(non_snake_case)]
pub fn setDefaultRuntimeSqliteHost(host: Arc<dyn RuntimeSqliteHost>) {
    let holder = DEFAULT_RUNTIME_SQLITE_HOST.get_or_init(|| Mutex::new(None));
    let mut guard = holder
        .lock()
        .expect("default runtime sqlite host mutex poisoned");
    *guard = Some(host);
}

/// Registers the host secret store used by encrypted runtime stores.
#[allow(non_snake_case)]
pub fn setDefaultHostSecretStore(host: Arc<dyn HostSecretStore>) {
    let holder = DEFAULT_HOST_SECRET_STORE.get_or_init(|| Mutex::new(None));
    let mut guard = holder
        .lock()
        .expect("default host secret store mutex poisoned");
    *guard = Some(host);
}

/// Returns the registered runtime storage host.
#[allow(non_snake_case)]
pub fn defaultRuntimeStorageHost() -> Arc<dyn RuntimeStorageHost> {
    let holder = DEFAULT_RUNTIME_STORAGE_HOST.get_or_init(|| Mutex::new(None));
    let guard = holder
        .lock()
        .expect("default runtime storage host mutex poisoned");
    match guard.as_ref() {
        Some(host) => Arc::clone(host),
        None => panic!("default runtime storage host is not registered"),
    }
}

/// Returns the registered runtime SQLite host.
#[allow(non_snake_case)]
pub fn defaultRuntimeSqliteHost() -> Arc<dyn RuntimeSqliteHost> {
    let holder = DEFAULT_RUNTIME_SQLITE_HOST.get_or_init(|| Mutex::new(None));
    let guard = holder
        .lock()
        .expect("default runtime sqlite host mutex poisoned");
    match guard.as_ref() {
        Some(host) => Arc::clone(host),
        None => panic!("default runtime sqlite host is not registered"),
    }
}

/// Returns the registered host secret store.
#[allow(non_snake_case)]
pub fn defaultHostSecretStore() -> Arc<dyn HostSecretStore> {
    let holder = DEFAULT_HOST_SECRET_STORE.get_or_init(|| Mutex::new(None));
    let guard = holder
        .lock()
        .expect("default host secret store mutex poisoned");
    match guard.as_ref() {
        Some(host) => Arc::clone(host),
        None => panic!("default host secret store is not registered"),
    }
}

/// Returns the registered host secret store when one is available.
#[allow(non_snake_case)]
pub fn defaultHostSecretStoreOption() -> Option<Arc<dyn HostSecretStore>> {
    let holder = DEFAULT_HOST_SECRET_STORE.get_or_init(|| Mutex::new(None));
    holder
        .lock()
        .expect("default host secret store mutex poisoned")
        .as_ref()
        .map(Arc::clone)
}

/// Converts an absolute runtime path into the host-relative storage path.
#[allow(non_snake_case)]
pub fn runtimeStoragePath(path: &Path) -> String {
    let paths = RuntimeStorePaths::default();
    if let Ok(relative) = path.strip_prefix(paths.runtime_dir()) {
        return format!("runtime/{}", relative.to_string_lossy().replace('\\', "/"));
    }
    if let Ok(relative) = path.strip_prefix(paths.workspace_dir()) {
        return format!(
            "workspaces/{}",
            relative.to_string_lossy().replace('\\', "/")
        );
    }
    panic!(
        "runtime storage path must be under {} or {}",
        paths.runtime_dir().display(),
        paths.workspace_dir().display()
    )
}

static DEFAULT_RUNTIME_STORAGE_HOST: OnceLock<Mutex<Option<Arc<dyn RuntimeStorageHost>>>> =
    OnceLock::new();
static DEFAULT_RUNTIME_SQLITE_HOST: OnceLock<Mutex<Option<Arc<dyn RuntimeSqliteHost>>>> =
    OnceLock::new();
static DEFAULT_HOST_SECRET_STORE: OnceLock<Mutex<Option<Arc<dyn HostSecretStore>>>> =
    OnceLock::new();
