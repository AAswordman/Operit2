use std::sync::Arc;

use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::repository::RuntimeStorageRepository::RuntimeStorageRepository;
use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{
    runtimeStorageOwnership, RuntimeStorageOwnership, RUNTIME_APPLICATION_ZOOM_PATH,
};

/// Exercises the same repository calls used by Flutter, with a real storage Host.
#[test]
fn application_zoom_is_persistent_and_does_not_create_a_sync_log() {
    let root = std::env::temp_dir().join(format!("operit-zoom-{}", uuid::Uuid::new_v4()));
    let runtime = root.join("runtime");
    setDefaultRuntimeStorageHost(Arc::new(NativeRuntimeStorageHost::new(
        runtime.clone(),
        root.join("workspaces"),
    )));
    let store = RuntimeStorageRepository::new();
    let path = store.applicationZoomPath();
    assert_eq!(path, RUNTIME_APPLICATION_ZOOM_PATH);
    assert_eq!(
        runtimeStorageOwnership(&path).unwrap(),
        RuntimeStorageOwnership::CoreNode
    );
    assert_eq!(store.readText(path.clone()).unwrap(), None);
    store.writeText(path.clone(), "1.3".into()).unwrap();
    assert_eq!(
        RuntimeStorageRepository::new()
            .readText(path.clone())
            .unwrap(),
        Some("1.3".into())
    );
    assert_eq!(
        std::fs::read_to_string(runtime.join("client/application_zoom.local")).unwrap(),
        "1.3"
    );
    assert!(
        !runtime.join("sync").exists(),
        "local zoom must not produce sync metadata or operations"
    );
    store.writeText(path, "1.0".into()).unwrap();
    assert!(!runtime.join("sync").exists());
    std::fs::remove_dir_all(root).unwrap();
}
