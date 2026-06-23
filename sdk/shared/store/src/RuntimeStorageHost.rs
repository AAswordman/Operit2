use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::{RuntimeSqliteHost, RuntimeStorageHost};

use crate::RuntimeStorePaths::RuntimeStorePaths;

#[allow(non_snake_case)]
pub fn setDefaultRuntimeStorageHost(host: Arc<dyn RuntimeStorageHost>) {
    let holder = DEFAULT_RUNTIME_STORAGE_HOST.get_or_init(|| Mutex::new(None));
    let mut guard = holder
        .lock()
        .expect("default runtime storage host mutex poisoned");
    *guard = Some(host);
}

#[allow(non_snake_case)]
pub fn setDefaultRuntimeSqliteHost(host: Arc<dyn RuntimeSqliteHost>) {
    let holder = DEFAULT_RUNTIME_SQLITE_HOST.get_or_init(|| Mutex::new(None));
    let mut guard = holder
        .lock()
        .expect("default runtime sqlite host mutex poisoned");
    *guard = Some(host);
}

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

#[allow(non_snake_case)]
pub fn runtimeStoragePath(path: &Path) -> String {
    let root = RuntimeStorePaths::default().root_dir().to_path_buf();
    let relative = path
        .strip_prefix(&root)
        .unwrap_or_else(|_| panic!("runtime storage path must be under {}", root.display()));
    relative.to_string_lossy().replace('\\', "/")
}

static DEFAULT_RUNTIME_STORAGE_HOST: OnceLock<Mutex<Option<Arc<dyn RuntimeStorageHost>>>> =
    OnceLock::new();
static DEFAULT_RUNTIME_SQLITE_HOST: OnceLock<Mutex<Option<Arc<dyn RuntimeSqliteHost>>>> =
    OnceLock::new();
