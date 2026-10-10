use super::*;
use crate::PreferencesDataStore::{PreferencesDataStore, PreferencesSyncedEntry};
use crate::SyncOperationStore::{SyncClock, SyncOperation, SyncOperationStore};
use operit_host_api::{HostError, HostResult, RuntimeStorageEntry};
use serde_json::json;
use std::sync::Mutex;

#[derive(Default)]
struct MemoryStorage {
    files: Mutex<BTreeMap<String, Vec<u8>>>,
    fail_write: Mutex<Option<String>>,
}
impl RuntimeStorageHost for MemoryStorage {
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| HostError::new(format!("Missing {path}")))
    }
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let bytes = self.readBytes(path)?;
        let start = (offset as usize).min(bytes.len());
        Ok(bytes[start..start.saturating_add(length).min(bytes.len())].to_vec())
    }
    fn writeBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        let mut failure = self.fail_write.lock().unwrap();
        if failure.as_deref() == Some(path) {
            failure.take();
            return Err(HostError::new("injected write failure"));
        }
        self.files.lock().unwrap().insert(path.into(), bytes.into());
        Ok(())
    }
    fn appendBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.files
            .lock()
            .unwrap()
            .entry(path.into())
            .or_default()
            .extend_from_slice(bytes);
        Ok(())
    }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.files
            .lock()
            .unwrap()
            .retain(|key, _| key != path && !(recursive && key.starts_with(&format!("{path}/"))));
        Ok(())
    }
    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self
            .files
            .lock()
            .unwrap()
            .keys()
            .any(|key| key == path || key.starts_with(&format!("{path}/"))))
    }
    fn list(&self, path: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let mut entries = BTreeMap::new();
        for (key, bytes) in self.files.lock().unwrap().iter() {
            if let Some(relative) = key.strip_prefix(&format!("{path}/")) {
                let name = relative.split('/').next().unwrap();
                let full = format!("{path}/{name}");
                let dir = relative.contains('/');
                entries.insert(
                    full.clone(),
                    RuntimeStorageEntry {
                        path: full,
                        isDirectory: dir,
                        size: if dir { 0 } else { bytes.len() as i64 },
                    },
                );
            }
        }
        Ok(entries.into_values().collect())
    }
}
fn host() -> Arc<MemoryStorage> {
    Arc::new(MemoryStorage::default())
}
fn operations(host: &Arc<MemoryStorage>, clock: &SyncClock) -> Vec<SyncOperation> {
    SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH)
        .operationsSince(clock, &[], 10000)
        .unwrap()
}
fn clock(host: &Arc<MemoryStorage>) -> SyncClock {
    SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH)
        .localClock()
        .unwrap()
}
/// Uses only production generic sync APIs; deliberately no ExtensionStore receive hook.
fn transfer(
    source: &Arc<MemoryStorage>,
    target: &Arc<MemoryStorage>,
    ops: &[SyncOperation],
) -> Result<(), String> {
    let store = SyncOperationStore::new(target.clone(), RUNTIME_SYNC_DIR_PATH);
    let decisions = store
        .shouldApplyOperations(ops)
        .map_err(|e| e.to_string())?;
    for (op, apply) in ops.iter().zip(decisions) {
        if !apply {
            continue;
        }
        if op.domain == "runtime_file" {
            if let Some(reference) = RuntimeFileSyncStore::requiredBlob(op)? {
                let bytes = RuntimeFileSyncStore::new(source.clone(), RUNTIME_SYNC_DIR_PATH)
                    .readBlobChunk(&reference.contentHash, 0, reference.size)?;
                RuntimeFileSyncStore::new(target.clone(), RUNTIME_SYNC_DIR_PATH)
                    .writeBlob(&reference, &bytes)?;
            }
            RuntimeFileSyncStore::applySyncedOperation(
                target.clone(),
                RUNTIME_SYNC_DIR_PATH,
                &op.entityId,
                &op.operation,
                op.payload.clone(),
            )?;
        } else if op.domain == "preferences" {
            PreferencesDataStore::applySyncedEntriesWithStorage(
                target.clone(),
                &[PreferencesSyncedEntry::fromOperation(op).map_err(|e| e.to_string())?],
            )
            .map_err(|e| e.to_string())?;
        } else {
            return Err(format!("Unexpected domain {}", op.domain));
        }
    }
    store
        .recordAppliedOperations(ops)
        .map_err(|e| e.to_string())?;
    store.appendOperations(ops).map_err(|e| e.to_string())?;
    Ok(())
}
fn skill(host: &Arc<MemoryStorage>) -> ExtensionStore {
    let store = ExtensionStore::new(host.clone());
    host.writeBytes("runtime/extensions/device/skills/demo/SKILL.md", b"# Demo")
        .unwrap();
    host.writeBytes(
        "runtime/extensions/device/skills/demo/scripts/helper.py",
        b"print(1)",
    )
    .unwrap();
    store
        .registerDevice(
            "skill",
            "demo",
            "demo",
            json!({"visible": true, "label": "original"}),
        )
        .unwrap();
    store
}

#[test]
fn scope_move_syncs_real_files_and_settings_without_embedded_bundles() {
    let source = host();
    let target = host();
    let store = skill(&source);
    assert!(
        operations(&source, &SyncClock::empty()).is_empty(),
        "device content must stay local"
    );
    store.moveScope("skill", "demo", "space").unwrap();
    let ops = operations(&source, &SyncClock::empty());
    assert!(ops
        .iter()
        .any(|op| op.entityId == "runtime/extensions/space/skills/demo/SKILL.md"));
    assert!(ops.iter().any(|op| op.domain == "preferences"));
    transfer(&source, &target, &ops).unwrap();
    assert_eq!(
        target
            .readBytes("runtime/extensions/space/skills/demo/SKILL.md")
            .unwrap(),
        b"# Demo"
    );
    let record = ExtensionStore::new(target.clone())
        .record("skill", "demo")
        .unwrap();
    assert_eq!(record.scope, "space");
    assert!(record.files.is_empty());
    let stored: Value = serde_json::from_slice(
        &target
            .readBytes(&recordPath("skill", "demo", "space").unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(stored.get("files").is_none());
    assert!(!source
        .exists("runtime/extensions/device/skills/demo/SKILL.md")
        .unwrap());
}

#[test]
fn concurrent_settings_merge_without_republishing_installation() {
    let a = host();
    let b = host();
    let store_a = skill(&a);
    store_a.moveScope("skill", "demo", "space").unwrap();
    transfer(&a, &b, &operations(&a, &SyncClock::empty())).unwrap();
    let initial_a = clock(&a);
    let initial_b = clock(&b);
    let store_b = ExtensionStore::new(b.clone());
    let mut settings_a = store_a.record("skill", "demo").unwrap().settings;
    let mut settings_b = store_b.record("skill", "demo").unwrap().settings;
    settings_a["visible"] = json!(false);
    settings_b["label"] = json!("new label");
    store_a.setSettings("skill", "demo", settings_a).unwrap();
    store_b.setSettings("skill", "demo", settings_b).unwrap();
    let ops_a = operations(&a, &initial_a);
    let ops_b = operations(&b, &initial_b);
    assert!(!ops_a.is_empty() && !ops_b.is_empty());
    assert!(ops_a
        .iter()
        .chain(&ops_b)
        .all(|op| op.domain == "preferences"));
    transfer(&a, &b, &ops_a).unwrap();
    transfer(&b, &a, &ops_b).unwrap();
    for store in [&store_a, &store_b] {
        let settings = store.record("skill", "demo").unwrap().settings;
        assert_eq!(settings["visible"], false);
        assert_eq!(settings["label"], "new label");
    }
}

#[test]
fn failed_file_apply_retries_even_after_descriptor_was_received() {
    let source = host();
    let target = host();
    let store = skill(&source);
    store.moveScope("skill", "demo", "space").unwrap();
    let mut ops = operations(&source, &SyncClock::empty());
    ops.sort_by_key(|op| !op.entityId.contains("/records/"));
    *target.fail_write.lock().unwrap() =
        Some("runtime/extensions/space/skills/demo/scripts/helper.py".into());
    assert!(transfer(&source, &target, &ops).is_err());
    transfer(&source, &target, &ops).unwrap();
    assert_eq!(
        target
            .readBytes("runtime/extensions/space/skills/demo/scripts/helper.py")
            .unwrap(),
        b"print(1)"
    );
}

#[test]
fn moving_back_to_device_removes_remote_files_through_generic_tombstones() {
    let source = host();
    let target = host();
    let store = skill(&source);
    store.moveScope("skill", "demo", "space").unwrap();
    transfer(&source, &target, &operations(&source, &SyncClock::empty())).unwrap();
    let before = clock(&source);
    store.moveScope("skill", "demo", "device").unwrap();
    let ops = operations(&source, &before);
    assert!(ops
        .iter()
        .any(|op| op.operation == "delete" && op.entityId.ends_with("helper.py")));
    transfer(&source, &target, &ops).unwrap();
    assert!(!target
        .exists("runtime/extensions/space/skills/demo/SKILL.md")
        .unwrap());
    assert!(ExtensionStore::new(target)
        .records("skill")
        .unwrap()
        .is_empty());
    assert_eq!(store.record("skill", "demo").unwrap().scope, "device");
}

#[test]
fn interrupted_scope_move_keeps_source_and_can_resume() {
    let source = host();
    let store = skill(&source);
    *source.fail_write.lock().unwrap() = Some(recordPath("skill", "demo", "space").unwrap());
    assert!(store.moveScope("skill", "demo", "space").is_err());
    assert!(source
        .exists("runtime/extensions/device/skills/demo/SKILL.md")
        .unwrap());
    store.moveScope("skill", "demo", "space").unwrap();
    assert_eq!(store.record("skill", "demo").unwrap().scope, "space");
    assert!(!source
        .exists("runtime/extensions/device/skills/demo/SKILL.md")
        .unwrap());
}

#[test]
fn local_mcp_cannot_be_shared_and_creates_no_sync_operations() {
    let source = host();
    let store = ExtensionStore::new(source.clone());
    store
        .registerDevice(
            "mcp",
            "local",
            "",
            json!({"server": {"command": "node"}, "metadata": {}}),
        )
        .unwrap();
    assert!(store.moveScope("mcp", "local", "space").is_err());
    assert_eq!(store.record("mcp", "local").unwrap().scope, "device");
    assert!(operations(&source, &SyncClock::empty()).is_empty());
}

#[test]
fn generic_file_tracking_only_publishes_changed_files_and_recursive_tombstones() {
    let source = host();
    let store = skill(&source);
    store.moveScope("skill", "demo", "space").unwrap();
    let before = clock(&source);
    let path = "runtime/extensions/space/skills/demo/scripts/helper.py";
    RuntimeFileSyncStore::new(source.clone(), RUNTIME_SYNC_DIR_PATH)
        .trackChanges(&[path.into()], || {
            source
                .writeBytes(path, b"print(2)")
                .map_err(|e| e.to_string())
        })
        .unwrap();
    let ops = operations(&source, &before);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].entityId, path);
    let before_delete = clock(&source);
    let root = "runtime/extensions/space/skills/demo";
    RuntimeFileSyncStore::new(source.clone(), RUNTIME_SYNC_DIR_PATH)
        .trackChanges(&[root.into()], || {
            source.delete(root, true).map_err(|e| e.to_string())
        })
        .unwrap();
    let ops = operations(&source, &before_delete);
    assert_eq!(ops.len(), 2);
    assert!(ops.iter().all(|op| op.operation == "delete"));
}

#[test]
fn legacy_bundle_upgrade_retries_actual_files_without_overwriting_local_edits() {
    let source = host();
    let record = ExtensionRecord {
        kind: "skill".into(),
        id: "legacy".into(),
        scope: "space".into(),
        sourceName: "legacy".into(),
        settings: json!({"visible": true}),
        files: BTreeMap::from([
            ("content/legacy/SKILL.md".into(), STANDARD.encode(b"old")),
            ("content/legacy/extra.txt".into(), STANDARD.encode(b"extra")),
        ]),
    };
    source
        .writeBytes(
            &recordPath("skill", "legacy", "space").unwrap(),
            &serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
    source
        .writeBytes(
            "runtime/extensions/space/skills/legacy/SKILL.md",
            b"user edit",
        )
        .unwrap();
    *source.fail_write.lock().unwrap() =
        Some("runtime/extensions/space/skills/legacy/extra.txt".into());
    let store = ExtensionStore::new(source.clone());
    assert!(store.records("skill").is_err());
    assert!(store.record("skill", "legacy").unwrap().files.is_empty());
    assert_eq!(
        source
            .readBytes("runtime/extensions/space/skills/legacy/SKILL.md")
            .unwrap(),
        b"user edit"
    );
    assert_eq!(
        source
            .readBytes("runtime/extensions/space/skills/legacy/extra.txt")
            .unwrap(),
        b"extra"
    );
}

#[test]
fn applied_notifications_are_storage_scoped_and_detach_on_drop() {
    let a: Arc<dyn RuntimeStorageHost> = host();
    let b: Arc<dyn RuntimeStorageHost> = host();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = calls.clone();
    let subscription = crate::SyncAppliedChanges::subscribe(a.clone(), move |_| {
        seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    let source = host();
    let store = skill(&source);
    store.moveScope("skill", "demo", "space").unwrap();
    let ops = operations(&source, &SyncClock::empty());
    crate::SyncAppliedChanges::publish(&b, &ops);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    crate::SyncAppliedChanges::publish(&a, &ops);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    drop(subscription);
    crate::SyncAppliedChanges::publish(&a, &ops);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn legacy_layout_migration_preserves_files_visibility_and_mcp_without_sharing_secrets() {
    let source = host();
    let store = ExtensionStore::new(source.clone());
    source
        .writeBytes("runtime/extensions/skills/old/SKILL.md", b"legacy skill")
        .unwrap();
    source
        .writeBytes("runtime/extensions/packages/demo.js", b"legacy package")
        .unwrap();
    source
        .writeBytes(
            "runtime/extensions/device/packages/demo.js",
            b"newer package",
        )
        .unwrap();
    source.writeBytes("runtime/extensions/mcp/mcp_config.json", serde_json::to_string(&json!({
        "mcpServers": {"remote": {"url": "https://example.test/mcp", "headers": {"Authorization": "secret"}}}
    })).unwrap().as_bytes()).unwrap();
    PreferencesDataStore::newWithStorage(
        source.clone(),
        operit_util::RuntimeStorageLayout::SKILL_VISIBILITY_PREFERENCES_PATH,
    )
    .edit(|prefs| prefs.set(&stringPreferencesKey("skill_visible_old"), "false".into()))
    .unwrap();
    let before = clock(&source);
    store.migrateLegacyLayout().unwrap();
    store
        .registerDevice("skill", "old", "old", json!({"visible": true}))
        .unwrap();
    assert_eq!(
        store.record("skill", "old").unwrap().settings["visible"],
        false
    );
    assert_eq!(
        source
            .readBytes("runtime/extensions/device/skills/old/SKILL.md")
            .unwrap(),
        b"legacy skill"
    );
    assert_eq!(
        source
            .readBytes("runtime/extensions/device/packages/demo.js")
            .unwrap(),
        b"newer package"
    );
    assert_eq!(store.record("mcp", "remote").unwrap().scope, "device");
    assert!(source
        .exists("runtime/extensions/skills/old/SKILL.md")
        .unwrap());
    assert!(operations(&source, &before).is_empty());
    store.delete("skill", "old").unwrap();
    store.delete("mcp", "remote").unwrap();
    store.migrateLegacyLayout().unwrap();
    assert!(store.records("mcp").unwrap().is_empty());
    assert!(!source
        .exists("runtime/extensions/device/skills/old/SKILL.md")
        .unwrap());
}

#[test]
fn broken_mcp_record_does_not_break_skill_catalog() {
    let source = host();
    let store = skill(&source);
    source
        .writeBytes(
            "runtime/extensions/device/records/mcp-broken.json",
            b"not json",
        )
        .unwrap();
    assert!(store.records("mcp").is_err());
    assert_eq!(store.records("skill").unwrap().len(), 1);
}

#[test]
fn observer_panic_cannot_undo_a_committed_sync_or_prevent_other_observers() {
    let source = host();
    let store = skill(&source);
    store.moveScope("skill", "demo", "space").unwrap();
    let storage: Arc<dyn RuntimeStorageHost> = source.clone();
    let _broken = crate::SyncAppliedChanges::subscribe(storage.clone(), |_| {
        panic!("broken business observer")
    });
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = calls.clone();
    let _healthy = crate::SyncAppliedChanges::subscribe(storage.clone(), move |_| {
        seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    crate::SyncAppliedChanges::publish(&storage, &operations(&source, &SyncClock::empty()));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn shared_mcp_definition_and_flags_roundtrip_via_preferences() {
    let source = host();
    let target = host();
    let store = ExtensionStore::new(source.clone());
    store.registerDevice("mcp", "remote", "", json!({"server": {"url": "https://example.test/mcp", "disabled": false}, "metadata": {"name": "Remote"}})).unwrap();
    store.moveScope("mcp", "remote", "space").unwrap();
    transfer(&source, &target, &operations(&source, &SyncClock::empty())).unwrap();
    let before = clock(&source);
    let mut settings = store.record("mcp", "remote").unwrap().settings;
    settings["server"]["disabled"] = json!(true);
    store.setSettings("mcp", "remote", settings).unwrap();
    let ops = operations(&source, &before);
    assert!(ops.iter().all(|op| op.domain == "preferences"));
    transfer(&source, &target, &ops).unwrap();
    assert_eq!(
        ExtensionStore::new(target)
            .record("mcp", "remote")
            .unwrap()
            .settings["server"]["disabled"],
        true
    );
}

#[test]
fn package_config_changes_do_not_republish_archive_or_installation_record() {
    let source = host();
    let target = host();
    let store = ExtensionStore::new(source.clone());
    source
        .writeBytes(
            "runtime/extensions/device/packages/demo.toolpkg",
            b"archive bytes",
        )
        .unwrap();
    store.registerDevice("package", "demo", "demo.toolpkg", json!({"members": ["demo"], "enabledNames": [], "disabledNames": [], "subpackageStates": {}, "order": 0, "builtin": false, "installationId": "00000000000000000000000000000000"})).unwrap();
    let config = format!("{}/env.json", store.configPath("demo").unwrap());
    source.writeBytes(&config, b"original config").unwrap();
    store.moveScope("package", "demo", "space").unwrap();
    transfer(&source, &target, &operations(&source, &SyncClock::empty())).unwrap();
    let before = clock(&source);
    let shared = format!("{}/env.json", store.configPath("demo").unwrap());
    RuntimeFileSyncStore::new(source.clone(), RUNTIME_SYNC_DIR_PATH)
        .trackChanges(&[shared.clone()], || {
            source
                .writeBytes(&shared, b"updated config")
                .map_err(|e| e.to_string())
        })
        .unwrap();
    let ops = operations(&source, &before);
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].entityId, shared);
    transfer(&source, &target, &ops).unwrap();
    assert_eq!(target.readBytes(&shared).unwrap(), b"updated config");
    assert_eq!(
        target
            .readBytes("runtime/extensions/space/packages/demo.toolpkg")
            .unwrap(),
        b"archive bytes"
    );
}

/// Registers one package fixture with explicit container membership.
fn register_package_owner(store: &ExtensionStore, id: &str, members: &[&str]) {
    store.registerDevice("package", id, &format!("{id}.toolpkg"), json!({
        "members": members, "enabledNames": [], "disabledNames": [],
        "subpackageStates": {}, "order": 0, "builtin": false,
        "installationId": "00000000000000000000000000000000"
    })).unwrap();
}

/// Resolves ownership directly from persisted records without a runtime package manager.
#[test]
fn package_owner_resolves_container_and_subpackage() {
    let store = ExtensionStore::new(host());
    register_package_owner(&store, "demo", &["demo", "demo.tools"]);
    assert_eq!(store.packageOwner("demo").unwrap().id, "demo");
    assert_eq!(store.packageOwner(" demo.tools ").unwrap().id, "demo");
    assert_eq!(store.packageOwner("missing").unwrap_err(),
        "Extension is not registered: package:missing");
}

/// Rejects ambiguous membership instead of selecting an arbitrary container.
#[test]
fn package_owner_rejects_conflicting_membership() {
    let store = ExtensionStore::new(host());
    register_package_owner(&store, "first", &["first", "shared"]);
    register_package_owner(&store, "second", &["second", "shared"]);
    assert_eq!(store.packageOwner("shared").unwrap_err(),
        "Extension ownership conflict: package:shared");
    register_package_owner(&store, "shared", &["shared"]);
    assert_eq!(store.packageOwner("shared").unwrap_err(),
        "Extension ownership conflict: package:shared");
}

/// Reads the committed scope after an installation moves without retaining runtime state.
#[test]
fn package_owner_tracks_scope_moves() {
    let storage = host();
    let store = ExtensionStore::new(storage.clone());
    storage.writeBytes("runtime/extensions/device/packages/demo.toolpkg", b"archive").unwrap();
    register_package_owner(&store, "demo", &["demo", "demo.tools"]);
    assert_eq!(store.packageOwner("demo.tools").unwrap().scope, "device");
    store.moveScope("package", "demo", "space").unwrap();
    assert_eq!(store.packageOwner("demo.tools").unwrap().scope, "space");
}

/// Resolves device and space registration paths before any installation record exists.
#[test]
fn configuration_paths_exist_before_package_registration() {
    let store = ExtensionStore::new(host());
    assert!(store.records("package").unwrap().is_empty());
    assert_eq!(ExtensionStore::configPathForScope("first_import", "device").unwrap(),
        "runtime/extensions/device/plugins/configs/first_import");
    assert_eq!(ExtensionStore::configPathForScope("first_import", "space").unwrap(),
        "runtime/extensions/space/plugins/configs/first_import");
    assert!(store.records("package").unwrap().is_empty());
    assert!(ExtensionStore::configPathForScope("first_import", "invalid_scope").is_err());
    assert!(ExtensionStore::configPathForScope("", "device").is_err());
}

/// Uses the same stable path before and after an installation record is committed.
#[test]
fn configuration_path_does_not_change_after_registration() {
    let store = ExtensionStore::new(host());
    let path = ExtensionStore::configPathForScope("first_import", "device").unwrap();
    register_package_owner(&store, "first_import", &["first_import"]);
    assert_eq!(store.configPath("first_import").unwrap(), path);
}

/// Verifies retiring managed code preserves configuration and business data and requires archival first.
#[test]
fn retired_device_package_keeps_data_and_requires_archived_source() {
    let host = Arc::new(MemoryStorage::default());
    let store = ExtensionStore::new(host.clone());
    let source = "runtime/extensions/device/packages/daily_life.js";
    host.writeBytes(source, b"legacy-bundled-code").unwrap();
    store.registerDevice("package", "daily_life", "daily_life.js", json!({
        "builtin": false, "installationId": "00000000000000000000000000000001", "members": ["daily_life"], "enabledNames": ["daily_life"],
        "disabledNames": [], "order": 0, "subpackageStates": {}
    })).unwrap();
    let config = format!("{}/reminders.json", ExtensionStore::configPathForScope("daily_life", "device").unwrap());
    let data = format!("{}/memory.db", ExtensionStore::dataPathForScope("daily_life", "device").unwrap());
    host.writeBytes(&config, b"configuration").unwrap();
    host.writeBytes(&data, b"business-data").unwrap();
    assert!(store.retireDevicePackageSource("daily_life", "daily_life.js").is_err());
    assert!(store.retireDevicePackageSource("daily_life", "other.js").is_err());
    host.delete(source, false).unwrap();
    store.retireDevicePackageSource("daily_life", "daily_life.js").unwrap();
    assert!(store.record("package", "daily_life").is_err());
    assert_eq!(host.readBytes(&config).unwrap(), b"configuration");
    assert_eq!(host.readBytes(&data).unwrap(), b"business-data");
}
