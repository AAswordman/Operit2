use super::*;
use operit_store::ExtensionStore::ExtensionStore;
use operit_store::PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore};

/// Removes failed bundled ownership and invalid external sources through the same deletion path.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_package_deletion_removes_builtin_and_external_installations() {
    let _test_guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    ui_resource_tests::configure_package_hosts(&mut hosts);
    let app = OperitApplication::newWithContext(hosts);
    let packages = app.packageManager();
    let mut manager = packages.lock().unwrap();
    manager.loadAvailablePackages();
    let name = "com.operit.goal_mode_bundle";
    let store = ExtensionStore::new(storage.clone());
    let mut record = store.record("package", name).unwrap();
    assert_eq!(record.scope, "space");
    let record_path = storage
        .list("runtime/extensions/space/records")
        .unwrap()
        .into_iter()
        .find(|entry| {
            serde_json::from_slice::<Value>(&storage.readBytes(&entry.path).unwrap()).unwrap()["id"]
                == name
        })
        .unwrap()
        .path;
    record.scope = "device".into();
    record.sourceName.clear();
    // Non-builtin duplicates must keep failing instead of being auto-merged.
    record.settings["builtin"] = json!(false);
    record
        .settings
        .as_object_mut()
        .unwrap()
        .remove("installationScope");
    storage
        .writeBytes(
            &record_path.replace("/space/", "/device/"),
            &serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();

    manager.loadAvailablePackages();
    let issue = manager
        .getToolPkgLoadIssues()
        .into_iter()
        .find(|issue| issue.packageName.as_deref() == Some(name))
        .unwrap();
    assert!(issue.message.contains("ownership conflict"), "{issue:?}");
    manager.deleteToolPkgLoadIssue(issue).unwrap();
    assert!(!storage.exists(&record_path).unwrap());
    assert!(!storage.exists(&record_path.replace("/space/", "/device/")).unwrap());
    manager.loadAvailablePackages();
    assert!(store.record("package", name).is_err());
    assert!(manager.getToolPkgContainerRuntime(name).is_none());
    assert!(manager.getBundledExternalPackageCandidates().iter()
        .any(|candidate| candidate.packageName == name));
    let imported = manager.importBundledExternalToolPkgContainer(name);
    assert!(imported.starts_with("Successfully"), "{imported}");
    assert!(manager.getToolPkgContainerRuntime(name).is_some());

    // A malformed archive has neither a package ID nor an ownership record.
    let paths = operit_store::RuntimeStorePaths::RuntimeStorePaths::default();
    let broken = paths.runtime_storage_path("runtime/extensions/device/packages/broken.toolpkg");
    std::fs::write(&broken, b"invalid archive").unwrap();
    manager.loadAvailablePackages();
    let issue = manager
        .getToolPkgLoadIssues()
        .into_iter()
        .find(|issue| std::path::Path::new(&issue.sourcePath) == broken)
        .unwrap();
    assert!(issue.packageName.is_none());
    manager.deleteToolPkgLoadIssue(issue).unwrap();
    assert!(!broken.exists());
    assert!(store.record("package", name).is_ok());

    // Deleting a duplicate source must preserve the valid installation of the same ID.
    let extra = "com.operit.context_limiter";
    let imported = manager.importBundledExternalToolPkgContainer(extra);
    assert!(imported.starts_with("Successfully"), "{imported}");
    let valid = store.record("package", extra).unwrap();
    let original = paths
        .runtime_storage_path("runtime/extensions/device/packages")
        .join(&valid.sourceName);
    std::fs::write(&original, b"corrupted installed archive").unwrap();
    manager.loadAvailablePackages();
    let issue = manager
        .getToolPkgLoadIssues()
        .into_iter()
        .find(|issue| std::path::Path::new(&issue.sourcePath) == original)
        .unwrap();
    assert!(issue.packageName.is_none());
    manager.deleteToolPkgLoadIssue(issue).unwrap();
    assert!(store.record("package", extra).is_err());
    let imported = manager.importBundledExternalToolPkgContainer(extra);
    assert!(imported.starts_with("Successfully"), "{imported}");
    let duplicate =
        paths.runtime_storage_path("runtime/extensions/device/packages/zzz_duplicate.toolpkg");
    std::fs::copy(original, &duplicate).unwrap();
    manager.loadAvailablePackages();
    let issue = manager
        .getToolPkgLoadIssues()
        .into_iter()
        .find(|issue| std::path::Path::new(&issue.sourcePath) == duplicate)
        .unwrap();
    assert_eq!(issue.code, "duplicate_package");
    manager.deleteToolPkgLoadIssue(issue).unwrap();
    assert!(!duplicate.exists());
    assert_eq!(store.record("package", extra).unwrap().scope, "device");
    assert!(manager.getToolPkgContainerRuntime(extra).is_some());
}

/// Preserves the existing character owner and data across scope repair, uninstallation and reinstallation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn builtin_character_package_can_be_deleted_and_reinstalled() {
    let _test_guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    ui_resource_tests::configure_package_hosts(&mut hosts);
    let app = OperitApplication::newWithContext(hosts);
    let packages = app.packageManager();
    let mut manager = packages.lock().unwrap();
    manager.loadAvailablePackages();
    let name = "com.operit.character_cards";
    let store = ExtensionStore::new(storage.clone());
    let original = store.record("package", name).unwrap();
    assert_eq!(original.settings["builtin"], true);
    assert_eq!(original.scope, "device");
    assert!(manager.getToolPkgContainerRuntime(name).is_some());
    let root = ExtensionStore::spaceDataPath(name).unwrap();
    let data_path = format!("{root}/preserved-user-data.bin");
    storage.writeBytes(&data_path, b"existing character data").unwrap();
    manager.setPackageScope(name, "space").unwrap();
    let mut shared = store.record("package", name).unwrap();
    shared.settings["installationScope"] = json!("space");
    shared.settings["order"] = json!(17);
    store.setSettings("package", name, shared.settings).unwrap();
    manager.loadAvailablePackages();
    let repaired = store.record("package", name).unwrap();
    assert_eq!(repaired.scope, "space");
    assert_eq!(repaired.settings["order"], 17);
    assert_eq!(repaired.settings["installationId"], original.settings["installationId"]);
    assert!(!repaired.settings.as_object().unwrap().contains_key("installationScope"));
    assert!(manager.getToolPkgContainerRuntime(name).is_some());
    manager.setPackageScope(name, "device").unwrap();
    assert_eq!(store.record("package", name).unwrap().settings["order"], 17);
    assert_eq!(storage.readBytes(&data_path).unwrap(), b"existing character data");
    assert!(manager.deletePackage(name));
    for _ in 0..2 {
        manager.loadAvailablePackages();
        assert!(store.record("package", name).is_err());
        assert!(manager.getToolPkgContainerRuntime(name).is_none());
        assert!(!manager.getEnabledPackageNames().iter().any(|enabled| enabled == name));
    }
    assert!(manager.getBundledExternalPackageCandidates().iter()
        .any(|candidate| candidate.packageName == name));
    let imported = manager.importBundledExternalToolPkgContainer(name);
    assert!(imported.starts_with("Successfully"), "{imported}");
    assert!(manager.getToolPkgContainerRuntime(name).is_some());
    assert_eq!(storage.readBytes(&data_path).unwrap(), b"existing character data");
}

/// Loading shared bundled ownership never merges device-only activation or ordering into it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_builtin_owner_keeps_its_own_settings() {
    let _test_guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    ui_resource_tests::configure_package_hosts(&mut hosts);
    let app = OperitApplication::newWithContext(hosts);
    let packages = app.packageManager();
    let mut manager = packages.lock().unwrap();
    manager.loadAvailablePackages();
    let name = "com.operit.goal_mode_bundle";
    let store = ExtensionStore::new(storage.clone());
    let canonical = store.record("package", name).unwrap();
    assert_eq!(canonical.scope, "space");
    let space_record_path = storage
        .list("runtime/extensions/space/records")
        .unwrap()
        .into_iter()
        .find(|entry| {
            serde_json::from_slice::<Value>(&storage.readBytes(&entry.path).unwrap()).unwrap()["id"]
                == name
        })
        .unwrap()
        .path;
    let mut legacy = canonical.clone();
    legacy.scope = "device".into();
    legacy
        .settings
        .as_object_mut()
        .unwrap()
        .remove("installationScope");
    legacy.settings["enabledNames"] = json!(["legacy_toggle"]);
    legacy.settings["order"] = json!(9);
    let device_record_path = space_record_path.replace("/space/", "/device/");
    storage
        .writeBytes(&device_record_path, &serde_json::to_vec(&legacy).unwrap())
        .unwrap();
    assert!(store.record("package", name).is_err());

    manager.loadAvailablePackages();

    assert!(manager
        .getToolPkgLoadIssues()
        .iter()
        .all(|issue| issue.packageName.as_deref() != Some(name)));
    let merged = store.record("package", name).unwrap();
    assert_eq!(merged.scope, "space");
    assert_eq!(merged.settings, canonical.settings);
    assert!(!storage.exists(&device_record_path).unwrap());
    assert!(manager.getToolPkgContainerRuntime(name).is_some());
}

/// Shared installation ownership overrides obsolete device-local retirement preferences.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_builtin_modes_ignore_device_local_retirement() {
    let _test_guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    ui_resource_tests::configure_package_hosts(&mut hosts);
    let app = OperitApplication::newWithContext(hosts);
    let packages = app.packageManager();
    let mut manager = packages.lock().unwrap();
    manager.loadAvailablePackages();
    let store = ExtensionStore::new(storage.clone());
    let modes = [
        ("com.operit.goal_mode_bundle", "goal_mode.toolpkg"),
        ("com.operit.plan_mode_bundle", "plan_mode.toolpkg"),
    ];
    for (name, _) in modes {
        let mut record = store.record("package", name).unwrap();
        assert_eq!(record.settings["builtin"], true);
        assert!(record.sourceName.is_empty());
        record.settings["order"] = json!(9);
        store.setSettings("package", name, record.settings).unwrap();
    }
    let preferences = CoreNodeStateStore::newWithStorage(
        storage.clone(),
        "runtime/extensions/device/manager.preferences.json",
    );
    preferences.edit(|prefs| {
        prefs.set(
            &stringPreferencesKey("removed_builtin_package_assets"),
            json!(["goal_mode.toolpkg", "plan_mode.toolpkg"]),
        );
    }).unwrap();
    manager.loadAvailablePackages();
    for (name, _) in modes {
        assert!(manager.getToolPkgContainerRuntime(name).is_some(),
            "Shared package {name} was hidden by local retirement preferences");
        assert!(!manager.getBundledExternalPackageCandidates().iter()
            .any(|candidate| candidate.packageName == name));
        let record = store.record("package", name).unwrap();
        assert_eq!(record.scope, "space");
        assert_eq!(record.sourceName, "");
        assert_eq!(record.settings["builtin"], true);
        assert_eq!(record.settings["order"], 9);
    }
    manager.loadAvailablePackages();
    for (name, _) in modes {
        assert!(manager.getToolPkgContainerRuntime(name).is_some());
        assert!(manager.getToolPkgLoadIssues().iter()
            .all(|issue| issue.packageName.as_deref() != Some(name)));
    }
}

/// Keeps bundled provenance separate from ownership and uses independent scope ranks for every hook.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn builtin_scope_and_user_order_share_one_execution_order() {
    let _test_guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    ui_resource_tests::configure_package_hosts(&mut hosts);
    let app = OperitApplication::newWithContext(hosts);
    let packages = app.packageManager();
    packages.lock().unwrap().loadAvailablePackages();
    let scopes = app.getExtensionScopes("package".into()).unwrap();
    assert!(scopes.values().all(|scope| scope == "device" || scope == "space"));
    let name = "com.operit.character_cards";
    assert_eq!(scopes[name], "device");
    let mut manager = packages.lock().unwrap();
    assert!(!manager.getBundledExternalPackageCandidates().iter()
        .any(|candidate| candidate.packageName == name));
    manager.setPackageScope(name, "space").unwrap();
    assert!(!manager.getBundledExternalPackageCandidates().iter()
        .any(|candidate| candidate.packageName == name));
    let store = ExtensionStore::new(storage.clone());
    assert_eq!(store.record("package", name).unwrap().scope, "space");
    let mut requested = manager.getToolPkgContainerOrder();
    requested.reverse();
    manager.setToolPkgContainerOrder(requested.clone()).unwrap();
    let mut expected = Vec::new();
    for scope in ["space", "device"] {
        let members = requested.iter().filter(|id| store.record("package", id).unwrap().scope == scope);
        for (rank, id) in members.enumerate() {
            assert_eq!(store.record("package", id).unwrap().settings["order"], rank);
            expected.push(id.clone());
        }
    }
    assert_eq!(manager.getToolPkgContainerOrder(), expected);
    assert_eq!(manager.getToolPkgContainerRuntimes().into_iter().map(|runtime| runtime.packageName).collect::<Vec<_>>(), expected);
    let enabled = manager.getEnabledPackageNames();
    assert_eq!(manager.getEnabledToolPkgContainerRuntimes().into_iter().map(|runtime| runtime.packageName).collect::<Vec<_>>(),
        expected.iter().filter(|id| enabled.contains(id)).cloned().collect::<Vec<_>>());
    let sync = operit_store::SyncOperationStore::SyncOperationStore::new(storage.clone(), "runtime/sync");
    let before = sync.localClock().unwrap();
    let mut local_order = expected.iter().filter(|id| store.record("package", id).unwrap().scope == "device")
        .cloned().collect::<Vec<_>>();
    assert!(local_order.len() > 1);
    local_order.reverse();
    let mut reordered = expected.iter().filter(|id| store.record("package", id).unwrap().scope == "space")
        .cloned().collect::<Vec<_>>();
    reordered.extend(local_order);
    manager.setToolPkgContainerOrder(reordered.clone()).unwrap();
    assert_eq!(manager.getToolPkgContainerOrder(), reordered);
    assert_eq!(sync.localClock().unwrap(), before, "Device-only ordering must not rewrite shared ranks");
    manager.setPackageScope(name, "device").unwrap();
    assert!(!manager.getBundledExternalPackageCandidates().iter()
        .any(|candidate| candidate.packageName == name));
    drop(manager);
    let scopes = app.getExtensionScopes("package".into()).unwrap();
    assert_eq!(scopes[name], "device");
    assert_eq!(scopes["character_memory"], "device");
}

/// Detects same-version bundle changes and updates both ownership scopes without losing settings or data.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bundled_source_updates_preserve_device_and_space_installations() {
    let _test_guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    ui_resource_tests::configure_package_hosts(&mut hosts);
    let app = OperitApplication::newWithContext(hosts);
    let packages = app.packageManager();
    let mut manager = packages.lock().unwrap();
    manager.loadAvailablePackages();
    let store = ExtensionStore::new(storage.clone());
    let paths = operit_store::RuntimeStorePaths::RuntimeStorePaths::default();
    for name in ["com.operit.character_cards", "com.operit.context_limiter"] {
        let imported = manager.importBundledExternalToolPkgContainer(name);
        assert!(imported.starts_with("Successfully"), "{imported}");
        let data_path = format!(
            "{}/bundle-update-data.bin",
            ExtensionStore::spaceDataPath(name).unwrap()
        );
        storage
            .writeBytes(&data_path, b"preserved user data")
            .unwrap();
        for scope in ["device", "space"] {
            manager.setPackageScope(name, scope).unwrap();
            if scope == "device" {
                manager.disableToolPkgContainer(name);
            } else {
                manager.enableToolPkgContainer(name);
            }
            let mut record = store.record("package", name).unwrap();
            record.settings["updateTestConfig"] = json!({"value": "retained", "count": 7});
            store.setSettings("package", name, record.settings).unwrap();
            manager.loadAvailablePackages();
            assert!(!manager
                .getBundledExternalPackageCandidates()
                .iter()
                .any(|item| item.packageName == name));
            let before = store.record("package", name).unwrap();
            let version = manager.getToolPkgContainerRuntime(name).unwrap().version;
            let enabled = manager.getEnabledPackageNames();
            let source = paths
                .runtime_storage_path(&ExtensionStore::root("package", scope).unwrap())
                .join(&before.sourceName);
            let current = std::fs::read(&source).unwrap();
            // A ZIP comment changes source bytes without changing the manifest version or runtime code.
            let mut archive =
                zip::ZipWriter::new_append(std::io::Cursor::new(current.clone())).unwrap();
            archive.set_comment("previous application bundle");
            std::fs::write(&source, archive.finish().unwrap().into_inner()).unwrap();
            manager.loadAvailablePackages();
            let updates = manager.getBundledExternalPackageCandidates();
            let update = updates
                .iter()
                .find(|item| item.packageName == name)
                .unwrap();
            assert_eq!(update.version, version);
            let imported = manager.importBundledExternalToolPkgContainer(name);
            assert!(imported.starts_with("Successfully"), "{imported}");
            let after = store.record("package", name).unwrap();
            assert_eq!(after.scope, scope);
            assert_eq!(after.sourceName, before.sourceName);
            assert_eq!(after.settings, before.settings);
            assert_eq!(manager.getEnabledPackageNames(), enabled);
            assert_eq!(
                storage.readBytes(&data_path).unwrap(),
                b"preserved user data"
            );
            assert_eq!(std::fs::read(&source).unwrap(), current);
            assert!(!manager
                .getBundledExternalPackageCandidates()
                .iter()
                .any(|item| item.packageName == name));
        }
    }
}
