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
    /// Confirms synchronous in-memory publication without copying or rewriting bytes.
    fn flushFile(&self, path: &str) -> operit_host_api::HostResult<()> {
        if !self.exists(path)? {
            return Err(operit_host_api::HostError::new("runtime storage entry not found"));
        }
        Ok(())
    }

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
    /// Atomically replaces one in-memory file.
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        self.writeBytes(path, content)
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
    assert!(ops.iter().all(|op| op.domain == "runtime_file"));
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

/// Resolves concurrent writes by choosing one complete settings entity.
#[test]
fn concurrent_settings_choose_one_atomic_installation_record() {
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
        .all(|op| op.domain == "runtime_file"));
    transfer(&a, &b, &ops_a).unwrap();
    transfer(&b, &a, &ops_b).unwrap();
    let settings_a = store_a.record("skill", "demo").unwrap().settings;
    let settings_b = store_b.record("skill", "demo").unwrap().settings;
    assert_eq!(settings_a, settings_b);
    assert!(
        settings_a == json!({"visible": false, "label": "original"})
            || settings_a == json!({"visible": true, "label": "new label"})
    );
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
        schemaVersion: 0,
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
    .edit(|prefs| {
        prefs.set(
            &stringPreferencesKey("skill_visible_old"),
            "false".to_owned(),
        )
    })
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
fn shared_mcp_definition_and_flags_roundtrip_via_installation_record() {
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
    assert!(ops.iter().all(|op| op.domain == "runtime_file"));
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
    store
        .registerDevice(
            "package",
            id,
            &format!("{id}.toolpkg"),
            json!({
                "members": members, "enabledNames": [], "disabledNames": [],
                "subpackageStates": {}, "order": 0, "builtin": false,
                "installationId": "00000000000000000000000000000000"
            }),
        )
        .unwrap();
}

/// Resolves ownership directly from persisted records without a runtime package manager.
#[test]
fn package_owner_resolves_container_and_subpackage() {
    let store = ExtensionStore::new(host());
    register_package_owner(&store, "demo", &["demo", "demo.tools"]);
    assert_eq!(store.packageOwner("demo").unwrap().id, "demo");
    assert_eq!(store.packageOwner(" demo.tools ").unwrap().id, "demo");
    assert_eq!(
        store.packageOwner("missing").unwrap_err(),
        "Extension is not registered: package:missing"
    );
}

/// Rejects ambiguous membership instead of selecting an arbitrary container.
#[test]
fn package_owner_rejects_conflicting_membership() {
    let store = ExtensionStore::new(host());
    register_package_owner(&store, "first", &["first", "shared"]);
    register_package_owner(&store, "second", &["second", "shared"]);
    assert_eq!(
        store.packageOwner("shared").unwrap_err(),
        "Extension ownership conflict: package:shared"
    );
    register_package_owner(&store, "shared", &["shared"]);
    assert_eq!(
        store.packageOwner("shared").unwrap_err(),
        "Extension ownership conflict: package:shared"
    );
}

/// Reads the committed scope after an installation moves without retaining runtime state.
#[test]
fn package_owner_tracks_scope_moves() {
    let storage = host();
    let store = ExtensionStore::new(storage.clone());
    storage
        .writeBytes(
            "runtime/extensions/device/packages/demo.toolpkg",
            b"archive",
        )
        .unwrap();
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
    assert_eq!(
        ExtensionStore::configPathForScope("first_import", "device").unwrap(),
        "runtime/extensions/device/plugins/configs/first_import"
    );
    assert_eq!(
        ExtensionStore::configPathForScope("first_import", "space").unwrap(),
        "runtime/extensions/space/plugins/configs/first_import"
    );
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

/// Migrates the legacy package-manager strings before importing extension settings.
#[test]
fn legacy_package_preferences_migrate_to_native_json_before_registration() {
    let storage = host();
    let path = operit_util::RuntimeStorageLayout::PACKAGE_MANAGER_PREFERENCES_PATH;
    storage.writeBytes(path, &serde_json::to_vec(&json!({
        "imported_packages": "[\"demo\",\"demo.tools\",\"other\"]",
        "disabled_packages": "[\"demo.disabled\",\"other.disabled\"]",
        "toolpkg_order": "[\"other\",\"demo\"]",
        "toolpkg_subpackage_states": "{\"demo.tools\":true,\"demo.disabled\":false,\"other.tools\":true}",
        "unrelated": "preserve me"
    })).unwrap()).unwrap();
    let store = ExtensionStore::new(storage.clone());
    register_package_owner(&store, "demo", &["demo", "demo.tools", "demo.disabled"]);
    let record = store.record("package", "demo").unwrap();
    assert_eq!(record.settings["enabledNames"], json!(["demo", "demo.tools"]));
    assert_eq!(record.settings["disabledNames"], json!(["demo.disabled"]));
    assert_eq!(record.settings["order"], json!(1));
    assert_eq!(record.settings["subpackageStates"], json!({"demo.tools": true, "demo.disabled": false}));
    let prefs = PreferencesDataStore::newWithStorage(storage.clone(), path).data().unwrap();
    assert_eq!(prefs.getTyped::<Vec<String>>(&stringPreferencesKey("imported_packages")).unwrap().unwrap(), vec!["demo", "demo.tools", "other"]);
    assert_eq!(prefs.getTyped::<BTreeMap<String, bool>>(&stringPreferencesKey("toolpkg_subpackage_states")).unwrap().unwrap()["demo.disabled"], false);
    assert_eq!(prefs.getTyped::<String>(&stringPreferencesKey("unrelated")).unwrap().unwrap(), "preserve me");
    let migrated = storage.readBytes(path).unwrap();
    register_package_owner(&store, "other", &["other", "other.tools", "other.disabled"]);
    assert_eq!(storage.readBytes(path).unwrap(), migrated);
    assert_eq!(store.record("package", "other").unwrap().settings["enabledNames"], json!(["other"]));
}

/// Rejects malformed legacy package state without registering a partial installation.
#[test]
fn malformed_legacy_package_preferences_do_not_register_or_overwrite_data() {
    for (key, encoded) in [
        ("imported_packages", "["),
        ("disabled_packages", "[1]"),
        ("toolpkg_order", "{}"),
        ("toolpkg_subpackage_states", "{\"demo\":1}"),
    ] {
        let storage = host();
        let path = operit_util::RuntimeStorageLayout::PACKAGE_MANAGER_PREFERENCES_PATH;
        let original = serde_json::to_vec(&json!({key: encoded})).unwrap();
        storage.writeBytes(path, &original).unwrap();
        let store = ExtensionStore::new(storage.clone());
        assert!(store.registerDevice("package", "demo", "demo.toolpkg", json!({
            "members": ["demo"], "enabledNames": [], "disabledNames": [],
            "subpackageStates": {}, "order": 0, "builtin": false,
            "installationId": "00000000000000000000000000000000"
        })).is_err());
        assert!(store.records("package").unwrap().is_empty());
        assert_eq!(storage.readBytes(path).unwrap(), original);
    }
}

/// Rejects incompatible installation and scope moves before touching package data or records.
#[test]
fn fixed_package_installation_scope_rejects_registration_and_moves_atomically() {
    for (scope, other) in [("device", "space"), ("space", "device")] {
        let storage = host();
        let store = ExtensionStore::new(storage.clone());
        let settings = json!({
            "members":["fixed"], "enabledNames":[], "disabledNames":[],
            "subpackageStates":{}, "order":0, "builtin":false,
            "installationId":"00000000000000000000000000000000", "installationScope":scope
        });
        assert!(store.registerInScope("package", "fixed", "fixed.toolpkg", settings.clone(), other).is_err());
        assert!(store.records("package").unwrap().is_empty());
        store.registerInScope("package", "fixed", "fixed.toolpkg", settings, scope).unwrap();
        let path = format!("{}/fixed.toolpkg", ExtensionStore::root("package", scope).unwrap());
        storage.writeBytes(&path, b"original-package").unwrap();
        let original = store.record("package", "fixed").unwrap();
        assert!(store.moveScope("package", "fixed", other).is_err());
        assert_eq!(store.record("package", "fixed").unwrap(), original);
        assert_eq!(storage.readBytes(&path).unwrap(), b"original-package");
        assert!(!storage.exists(&format!("{}/fixed.toolpkg", ExtensionStore::root("package", other).unwrap())).unwrap());
    }
}

/// Failed packages can be removed even when device and space both claim the same ID.
#[test]
fn deleting_failed_package_removes_conflicting_owners_and_preserves_other_packages() {
    let storage = host();
    let store = ExtensionStore::new(storage.clone());
    register_package_owner(&store, "broken", &["broken"]);
    register_package_owner(&store, "healthy", &["healthy"]);
    let mut shared = store.record("package", "broken").unwrap();
    shared.scope = "space".into();
    store.write(&shared).unwrap();
    for scope in ["device", "space"] {
        storage
            .writeBytes(
                &format!("runtime/extensions/{scope}/packages/broken.toolpkg"),
                b"broken",
            )
            .unwrap();
    }
    assert!(store
        .record("package", "broken")
        .unwrap_err()
        .contains("ownership conflict"));
    let before = clock(&storage);
    assert!(store.deleteFailedPackageInstallations("broken").unwrap());
    assert!(!store.deleteFailedPackageInstallations("broken").unwrap());
    assert_eq!(store.records("package").unwrap().len(), 1);
    assert_eq!(store.record("package", "healthy").unwrap().id, "healthy");
    for scope in ["device", "space"] {
        assert!(!storage
            .exists(&format!(
                "runtime/extensions/{scope}/packages/broken.toolpkg"
            ))
            .unwrap());
    }
    assert!(operations(&storage, &before)
        .iter()
        .any(|operation| operation.operation == "delete"
            && operation.entityId.ends_with("broken.toolpkg")));
}


/// Retains configuration, data, identity and user settings across custom installs and explicit removal.
#[test]
fn builtin_customization_preserves_data_and_supports_custom_updates() {
    for scope in ["device", "space"] {
        let storage = host();
        let store = ExtensionStore::new(storage.clone());
        store
            .registerInScope(
                "package",
                "demo",
                "",
                json!({
                    "members": ["demo", "demo:tools"], "enabledNames": ["demo"],
                    "disabledNames": ["demo:tools"], "subpackageStates": {"demo:tools": false},
                    "order": 7, "builtin": true,
                    "installationId": "00000000000000000000000000000000"
                }),
                scope,
            )
            .unwrap();
        let config = format!(
            "{}/settings.json",
            ExtensionStore::configPathForScope("demo", scope).unwrap()
        );
        storage.writeBytes(&config, b"user configuration").unwrap();
        let local = format!(
            "{}/notes.txt",
            ExtensionStore::localDataPath("demo").unwrap()
        );
        let shared = format!(
            "{}/memory.sqlite",
            ExtensionStore::spaceDataPath("demo").unwrap()
        );
        storage.writeBytes(&local, b"local data").unwrap();
        storage.writeBytes(&shared, b"shared data").unwrap();
        let root = ExtensionStore::root("package", scope).unwrap();
        storage
            .writeBytes(&format!("{root}/custom.toolpkg"), b"first custom version")
            .unwrap();
        store
            .customizePackage(
                "demo",
                "custom.toolpkg",
                vec!["demo".into(), "demo:custom".into()],
            )
            .unwrap();
        let custom = store.record("package", "demo").unwrap();
        assert_eq!(custom.sourceName, "custom.toolpkg");
        assert_eq!(custom.settings["builtin"], false);
        assert_eq!(custom.settings["customized"], true);
        assert_eq!(custom.settings["enabledNames"], json!(["demo"]));
        assert_eq!(store.packageOwner("demo:custom").unwrap().id, "demo");
        storage
            .writeBytes(
                &format!("{root}/updated.toolpkg"),
                b"updated custom version",
            )
            .unwrap();
        store
            .customizePackage(
                "demo",
                "updated.toolpkg",
                vec!["demo".into(), "demo:custom".into()],
            )
            .unwrap();
        assert!(!storage.exists(&format!("{root}/custom.toolpkg")).unwrap());
        assert_eq!(storage.readBytes(&config).unwrap(), b"user configuration");
        store.removePackageCustomization("demo").unwrap();
        let builtin = store.record("package", "demo").unwrap();
        assert_eq!(builtin.id, "demo");
        assert_eq!(builtin.scope, scope);
        assert_eq!(builtin.sourceName, "");
        assert_eq!(builtin.settings["builtin"], true);
        assert_eq!(builtin.settings["customized"], false);
        assert_eq!(builtin.settings["members"], json!(["demo", "demo:tools"]));
        assert_eq!(builtin.settings["enabledNames"], json!(["demo"]));
        assert_eq!(builtin.settings["disabledNames"], json!(["demo:tools"]));
        assert_eq!(
            builtin.settings["subpackageStates"],
            json!({"demo:tools": false})
        );
        assert_eq!(builtin.settings["order"], 7);
        assert_eq!(store.packageOwner("demo:tools").unwrap().id, "demo");
        assert!(!storage.exists(&format!("{root}/updated.toolpkg")).unwrap());
        assert_eq!(storage.readBytes(&config).unwrap(), b"user configuration");
        assert_eq!(storage.readBytes(&local).unwrap(), b"local data");
        assert_eq!(storage.readBytes(&shared).unwrap(), b"shared data");
    }
}

/// Synchronizes custom source installation and explicit removal through existing Space file operations.
#[test]
fn shared_builtin_customization_syncs_source_changes_without_deleting_configuration() {
    let source = host();
    let target = host();
    let store = ExtensionStore::new(source.clone());
    store
        .registerInScope(
            "package",
            "demo",
            "",
            json!({
                "members": ["demo"], "enabledNames": ["demo"], "disabledNames": [],
                "subpackageStates": {}, "order": 0, "builtin": true,
                "installationId": "00000000000000000000000000000000"
            }),
            "space",
        )
        .unwrap();
    let config = format!(
        "{}/settings.json",
        ExtensionStore::configPathForScope("demo", "space").unwrap()
    );
    RuntimeFileSyncStore::new(source.clone(), RUNTIME_SYNC_DIR_PATH)
        .writeBytes(&config, b"shared configuration")
        .unwrap();
    transfer(&source, &target, &operations(&source, &SyncClock::empty())).unwrap();
    let before_install = clock(&source);
    let archive = "runtime/extensions/space/packages/custom.toolpkg";
    source.writeBytes(archive, b"custom archive").unwrap();
    store
        .customizePackage("demo", "custom.toolpkg", vec!["demo".into()])
        .unwrap();
    transfer(&source, &target, &operations(&source, &before_install)).unwrap();
    let peer = ExtensionStore::new(target.clone());
    assert_eq!(
        peer.record("package", "demo").unwrap().settings["customized"],
        true
    );
    assert_eq!(target.readBytes(archive).unwrap(), b"custom archive");
    let before_remove = clock(&source);
    store.removePackageCustomization("demo").unwrap();
    transfer(&source, &target, &operations(&source, &before_remove)).unwrap();
    let restored = peer.record("package", "demo").unwrap();
    assert_eq!(restored.settings["builtin"], true);
    assert_eq!(restored.sourceName, "");
    assert!(!target.exists(archive).unwrap());
    assert_eq!(target.readBytes(&config).unwrap(), b"shared configuration");
}

/// Shared ownership replaces the bundled local record without merging independent local settings.
#[test]
fn shared_builtin_owner_replaces_local_state() {
    let storage = host();
    let store = ExtensionStore::new(storage.clone());
    let settings = |enabled: Value, order: u64| {
        let subpackageEnabled = enabled
            .as_array()
            .is_some_and(|values| !values.is_empty());
        json!({
            "members": ["bundle", "bundle_tools"], "enabledNames": enabled,
            "disabledNames": [], "subpackageStates": {"bundle_tools": subpackageEnabled},
            "order": order, "builtin": true,
            "installationId": "00000000000000000000000000000000"
        })
    };
    store
        .registerDevice("package", "bundle", "", settings(json!(["legacy"]), 7))
        .unwrap();
    let mut canonical = store.record("package", "bundle").unwrap();
    canonical.scope = "space".into();
    canonical.settings = settings(json!(["canonical"]), 2);
    canonical.settings["installationScope"] = json!("space");
    store.write(&canonical).unwrap();
    assert!(store
        .record("package", "bundle")
        .unwrap_err()
        .contains("ownership conflict"));

    assert!(store
        .acceptsBuiltinPackage("bundle")
        .unwrap());
    let merged = store.record("package", "bundle").unwrap();
    assert_eq!(merged.scope, "space");
    assert_eq!(merged.settings["enabledNames"], json!(["canonical"]));
    assert_eq!(merged.settings["order"], json!(2));
    assert_eq!(merged.settings["installationScope"], json!("space"));
    assert!(store
        .records("package")
        .unwrap()
        .iter()
        .all(|record| record.scope == "space"));
    assert!(store.acceptsBuiltinPackage("bundle").unwrap());
}

/// Genuine ownership conflicts must keep failing instead of being auto-merged.
#[test]
fn builtin_scope_reconciliation_refuses_non_builtin_duplicates() {
    let storage = host();
    let store = ExtensionStore::new(storage);
    register_package_owner(&store, "bundle", &["bundle"]);
    let mut duplicate = store.record("package", "bundle").unwrap();
    duplicate.scope = "space".into();
    duplicate.settings["installationScope"] = json!("space");
    store.write(&duplicate).unwrap();

    assert!(store.acceptsBuiltinPackage("bundle").unwrap_err().contains("ownership conflict"));
    assert!(store
        .record("package", "bundle")
        .unwrap_err()
        .contains("ownership conflict"));
}

/// Keeps installation source and configuration in the same synchronized entity.
#[test]
fn installation_settings_use_only_the_installation_record() {
    let storage = host();
    let store = ExtensionStore::new(storage.clone());
    store.registerInScope("skill", "demo", "demo", json!({"visible": true}), "space").unwrap();
    let before = clock(&storage);
    store.setSettings("skill", "demo", json!({"visible": false})).unwrap();
    let changes = operations(&storage, &before);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].domain, "runtime_file");
    assert_eq!(changes[0].entityId, recordPath("skill", "demo", "space").unwrap());
    assert!(storage.list("runtime/extensions/space/settings").unwrap().is_empty());
}

/// A later peer's built-in initialization cannot replace an explicit shared installation.
#[test]
fn late_builtin_initialization_preserves_explicit_shared_installation() {
    let installed = host();
    let fresh = host();
    let owner = ExtensionStore::new(installed.clone());
    let peer = ExtensionStore::new(fresh.clone());
    let settings = json!({
        "members": ["demo"], "enabledNames": ["demo"], "disabledNames": [],
        "subpackageStates": {}, "order": 9, "builtin": false,
        "installationId": "00000000000000000000000000000000"
    });
    installed.writeBytes("runtime/extensions/space/packages/demo.toolpkg", b"archive").unwrap();
    owner.registerInScope("package", "demo", "demo.toolpkg", settings.clone(), "space").unwrap();
    let mut initial = settings;
    initial["builtin"] = json!(true);
    initial["order"] = json!(0);
    initial["enabledNames"] = json!([]);
    initial["installationId"] = json!("11111111111111111111111111111111");
    peer.registerInScope("package", "demo", "", initial, "space").unwrap();
    let explicit = operations(&installed, &SyncClock::empty());
    let mut bootstrap = operations(&fresh, &SyncClock::empty());
    for op in &mut bootstrap { op.createdAt += 60000; }
    transfer(&fresh, &installed, &bootstrap).unwrap();
    transfer(&installed, &fresh, &explicit).unwrap();
    let expected = owner.record("package", "demo").unwrap();
    assert_eq!(expected.sourceName, "demo.toolpkg");
    assert_eq!(expected.settings["builtin"], false);
    assert_eq!(expected.settings["order"], 9);
    assert_eq!(expected.settings["installationId"], "00000000000000000000000000000000");
    assert_eq!(peer.record("package", "demo").unwrap(), expected);
}

/// Moves bundled ownership through the existing sync path and preserves the selected subpackage flags.
#[test]
fn builtin_scope_moves_sync_flags_and_allows_deleting_bundled_installations() {
    let source = host();
    let target = host();
    let store = ExtensionStore::new(source.clone());
    let peer = ExtensionStore::new(target.clone());
    let settings = json!({
        "members": ["demo", "demo_tools"], "enabledNames": ["demo"], "disabledNames": [],
        "subpackageStates": {"demo_tools": false}, "order": 7, "builtin": true,
        "installationId": "00000000000000000000000000000000"
    });
    store.registerDevice("package", "demo", "", settings.clone()).unwrap();
    let mut local_settings = settings.clone();
    local_settings["subpackageStates"]["demo_tools"] = json!(true);
    peer.registerDevice("package", "demo", "", local_settings).unwrap();
    let local_config = format!("{}/settings.json", ExtensionStore::configPathForScope("demo", "device").unwrap());
    let shared_config = format!("{}/settings.json", ExtensionStore::configPathForScope("demo", "space").unwrap());
    let private_data = format!("{}/notes.txt", ExtensionStore::localDataPath("demo").unwrap());
    source.writeBytes(&local_config, b"shared configuration").unwrap();
    target.writeBytes(&local_config, b"old local configuration").unwrap();
    target.writeBytes(&private_data, b"private device data").unwrap();
    store.moveScope("package", "demo", "space").unwrap();
    transfer(&source, &target, &operations(&source, &SyncClock::empty())).unwrap();
    let before_adoption = clock(&target);
    assert!(peer.acceptsBuiltinPackage("demo").unwrap());
    assert_eq!(peer.record("package", "demo").unwrap(), store.record("package", "demo").unwrap());
    assert_eq!(clock(&target), before_adoption);
    assert!(!target.exists(&recordPath("package", "demo", "device").unwrap()).unwrap());
    assert!(!target.exists(&local_config).unwrap());
    assert_eq!(target.readBytes(&shared_config).unwrap(), b"shared configuration");
    assert_eq!(target.readBytes(&private_data).unwrap(), b"private device data");
    let before_toggle = clock(&source);
    let mut updated = store.record("package", "demo").unwrap().settings;
    updated["subpackageStates"]["demo_tools"] = json!(true);
    store.setSettings("package", "demo", updated).unwrap();
    transfer(&source, &target, &operations(&source, &before_toggle)).unwrap();
    assert_eq!(peer.record("package", "demo").unwrap().settings["subpackageStates"]["demo_tools"], true);
    let before_move = clock(&target);
    peer.moveScope("package", "demo", "device").unwrap();
    transfer(&target, &source, &operations(&target, &before_move)).unwrap();
    assert!(!store.acceptsBuiltinPackage("demo").unwrap());
    assert!(store.record("package", "demo").is_err());
    assert!(peer.acceptsBuiltinPackage("demo").unwrap());
    assert_eq!(target.readBytes(&local_config).unwrap(), b"shared configuration");
    assert_eq!(target.readBytes(&private_data).unwrap(), b"private device data");
    let local = peer.record("package", "demo").unwrap();
    assert_eq!(local.scope, "device");
    assert_eq!(local.settings["subpackageStates"]["demo_tools"], true);
    assert_eq!(local.settings["order"], 7);
    peer.delete("package", "demo").unwrap();
    assert!(peer.record("package", "demo").is_err());
    assert!(!peer.acceptsBuiltinPackage("demo").unwrap());
    assert!(!target.exists(&local_config).unwrap());
    assert_eq!(target.readBytes(&private_data).unwrap(), b"private device data");
}

/// Preserves plugin data during bundled uninstallation and synchronizes only installation removal.
#[test]
fn builtin_deletion_retires_installations_and_syncs_shared_removal() {
    for scope in ["device", "space"] {
        for failed in [false, true] {
            let source = host();
            let target = host();
            let store = ExtensionStore::new(source.clone());
            let peer = ExtensionStore::new(target.clone());
            let settings = json!({
                "members": ["demo"], "enabledNames": ["demo"], "disabledNames": [],
                "subpackageStates": {}, "order": 0, "builtin": true,
                "installationId": "00000000000000000000000000000000"
            });
            store.registerInScope("package", "demo", "", settings.clone(), scope).unwrap();
            let mut data_files = Vec::new();
            for data_scope in ["device", "space"] {
                let root = ExtensionStore::dataPathForScope("demo", data_scope).unwrap();
                for name in ["cards.preferences.json", "groups.preferences.json", "memory.sqlite", "owner/USER.md"] {
                    let path = format!("{root}/{name}");
                    let bytes = format!("existing {data_scope} {name}").into_bytes();
                    source.writeBytes(&path, &bytes).unwrap();
                    data_files.push((path, bytes));
                }
            }
            if scope == "space" {
                transfer(&source, &target, &operations(&source, &SyncClock::empty())).unwrap();
                assert!(peer.acceptsBuiltinPackage("demo").unwrap());
            }
            let before_delete = clock(&source);
            if failed {
                assert!(store.deleteFailedPackageInstallations("demo").unwrap());
                assert!(!store.deleteFailedPackageInstallations("demo").unwrap());
            } else {
                store.delete("package", "demo").unwrap();
            }
            assert!(store.record("package", "demo").is_err());
            assert!(!ExtensionStore::new(source.clone()).acceptsBuiltinPackage("demo").unwrap());
            for (path, bytes) in &data_files {
                assert_eq!(source.readBytes(path).unwrap(), *bytes, "Uninstallation changed plugin data: {path}");
            }
            assert!(operations(&source, &before_delete).iter()
                .all(|operation| !operation.entityId.starts_with("runtime/plugin_data/")));
            if scope == "space" {
                transfer(&source, &target, &operations(&source, &before_delete)).unwrap();
                assert!(peer.record("package", "demo").is_err());
                assert!(!peer.acceptsBuiltinPackage("demo").unwrap());
            } else {
                assert!(operations(&source, &before_delete).is_empty());
                assert!(peer.acceptsBuiltinPackage("demo").unwrap());
                store.registerDevice("package", "demo", "", settings).unwrap();
                for (path, bytes) in &data_files {
                    assert_eq!(source.readBytes(path).unwrap(), *bytes, "Reinstallation changed plugin data: {path}");
                }
                assert!(store.acceptsBuiltinPackage("demo").unwrap());
                store.delete("package", "demo").unwrap();
                assert!(!store.acceptsBuiltinPackage("demo").unwrap());
            }
        }
    }
}

/// Retires the duplicate settings copy without letting it replace descriptor ownership.
#[test]
fn legacy_settings_migrate_once_without_changing_the_installation_source() {
    let storage = host();
    let store = ExtensionStore::new(storage.clone());
    let mut record = ExtensionRecord {
        schemaVersion: 0, kind: "package".into(), id: "demo".into(), scope: "space".into(),
        sourceName: "demo.toolpkg".into(), files: BTreeMap::new(),
        settings: json!({
            "members": ["demo"], "enabledNames": [], "disabledNames": [],
            "subpackageStates": {}, "order": 0, "builtin": false,
            "installationId": "00000000000000000000000000000000"
        }),
    };
    let mut old_settings = record.settings.clone();
    old_settings["builtin"] = json!(true);
    old_settings["installationId"] = json!("11111111111111111111111111111111");
    old_settings["order"] = json!(7);
    let preferences = store.legacySettingsStore(&record).unwrap();
    preferences.edit(|prefs| prefs.set(&stringPreferencesKey("settings"), old_settings)).unwrap();
    storage.writeBytes(&recordPath("package", "demo", "space").unwrap(), &serde_json::to_vec(&record).unwrap()).unwrap();
    record.schemaVersion = 1;
    record.settings["order"] = json!(7);
    assert_eq!(store.record("package", "demo").unwrap(), record);
    let migrated = clock(&storage);
    preferences.edit(|prefs| prefs.set(&stringPreferencesKey("settings"), json!({"order": 99}))).unwrap();
    assert_eq!(store.record("package", "demo").unwrap(), record);
    let changes = operations(&storage, &migrated);
    assert!(changes.iter().all(|op| op.domain == "preferences"));
}
