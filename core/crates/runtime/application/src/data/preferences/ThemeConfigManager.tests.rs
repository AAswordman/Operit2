use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use operit_host_api::{HostError, HostResult, RuntimeStorageEntry};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::PreferencesDataStore::FlowCancellation;
use serde_json::json;

use super::*;

/// Delegates storage to the real host while recording preference commits and injecting write failures.
struct RecordingStorageHost {
    inner: NativeRuntimeStorageHost,
    preferenceWrites: AtomicUsize,
    failPreferenceWrite: AtomicBool,
}

impl RuntimeStorageHost for RecordingStorageHost {
    /// Delegates durability to the real storage host without rewriting the file.
    fn flushFile(&self, path: &str) -> operit_host_api::HostResult<()> {
        self.inner.flushFile(path)
    }

    /// Reads the physical root from the real storage host.
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        self.inner.runtimeRootDir()
    }

    /// Reads the workspace root from the real storage host.
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        self.inner.workspaceRootDir()
    }

    /// Reads actual persisted bytes through the real storage host.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readBytes(path)
    }

    /// Reads an actual bounded storage range through the real storage host.
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        self.inner.readBytesRange(path, offset, length)
    }

    /// Records and atomically publishes one complete preference commit through the real host.
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> HostResult<()> {
        if path == OperitPaths::USER_PREFERENCES_PATH {
            if self.failPreferenceWrite.load(Ordering::SeqCst) {
                return Err(HostError::new("injected theme preference write failure"));
            }
            self.preferenceWrites.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.writeBytesAtomically(path, content)
    }

    /// Delegates ordinary nontransactional test writes to the real host.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(path, content)
    }

    /// Appends actual bytes through the real storage host.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.appendBytes(path, content)
    }

    /// Deletes an explicitly requested storage entry through the real host.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(path, recursive)
    }

    /// Checks actual storage existence through the real host.
    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(path)
    }

    /// Lists actual storage entries through the real host.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.inner.list(prefix)
    }
}

/// Owns an isolated runtime host without changing global paths or host defaults.
struct StoreFixture {
    host: Arc<RecordingStorageHost>,
}

impl StoreFixture {
    /// Creates an isolated real host with unique runtime and workspace roots.
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-theme-tests-{}", Uuid::new_v4()));
        Self {
            host: Arc::new(RecordingStorageHost {
                inner: NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")),
                preferenceWrites: AtomicUsize::new(0),
                failPreferenceWrite: AtomicBool::new(false),
            }),
        }
    }

    /// Opens another theme manager over the same actual host and shared preference transaction.
    fn manager(&self) -> ThemeConfigManager {
        ThemeConfigManager::new(self.host.clone())
    }

    /// Opens the ordinary preference reader over the same actual host and canonical path.
    fn preferences(&self) -> PreferencesDataStore {
        super::super::UserPreferencesManager::UserPreferencesManager::store(self.host.clone())
    }

    /// Reads the actual persisted ordinary preference bytes.
    fn bytes(&self) -> Vec<u8> {
        self.host
            .readBytes(OperitPaths::USER_PREFERENCES_PATH)
            .unwrap()
    }

    /// Counts successful ordinary preference commits only, excluding synchronization log writes.
    fn writes(&self) -> usize {
        self.host.preferenceWrites.load(Ordering::SeqCst)
    }
}

/// Reads the complete snapshot fixture independently of any application defaults.
fn snapshot() -> BTreeMap<String, Value> {
    serde_json::from_str(include_str!("theme_snapshot_fixture.json")).unwrap()
}

/// Changes one field while preserving all other required fixture fields.
fn changedSnapshot(key: &str, value: Value) -> BTreeMap<String, Value> {
    let mut result = snapshot();
    result.insert(key.to_string(), value);
    result
}

/// Confirms the complete Flutter wire shape and exact ordinary preference round trip.
#[test]
fn complete_snapshot_round_trips_all_ordinary_keys() {
    let mut preferences = Preferences::default();
    preferences.set(
        &stringPreferencesKey("unrelated_setting"),
        "untouched".to_string(),
    );
    let value = snapshot();
    assert_eq!(value.len(), 78);
    validateThemePreferenceSnapshot(&value).unwrap();
    writeThemePreferenceSnapshot(&mut preferences, &value).unwrap();
    assertThemePreferenceSnapshot(&preferences, &value).unwrap();
    assert_eq!(
        preferences.get(&stringPreferencesKey("bubble_rounded_corners_enabled")),
        Some(&json!(true))
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("bubble_content_padding_left")),
        Some(&json!(12))
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("bubble_ai_content_padding_right")),
        Some(&json!(12))
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("unrelated_setting")),
        Some(&json!("untouched"))
    );
}

/// Rejects every missing field and any additional field without supplying defaults.
#[test]
fn incomplete_and_unknown_snapshots_are_rejected() {
    let complete = snapshot();
    for key in complete.keys() {
        let mut incomplete = complete.clone();
        incomplete.remove(key);
        assert!(
            validateThemePreferenceSnapshot(&incomplete).is_err(),
            "{key}"
        );
    }
    assert!(
        validateThemePreferenceSnapshot(&changedSnapshot("unknownThemeOption", json!(true)))
            .is_err()
    );
}

/// Rejects JSON type substitutions, enum mismatches and out-of-range numeric values.
#[test]
fn invalid_types_enums_and_ranges_are_rejected() {
    for (key, value) in [
        ("useCustomColors", json!("false")),
        ("customPrimaryColor", json!(-1)),
        ("customSecondaryColor", json!(4294967296_u64)),
        ("cursorUserBubbleColor", json!(1.5)),
        ("backgroundImageUri", json!({})),
        ("themeMode", json!("unknown")),
        ("backgroundMediaType", json!("audio")),
        ("chatStyle", json!("unknown")),
        ("inputStyle", json!("unknown")),
        ("bubbleAiImageRenderMode", json!("unknown")),
        ("avatarShape", json!("unknown")),
        ("fontType", json!("unknown")),
        ("systemFontName", json!("unregistered-font")),
        ("backgroundImageOpacity", json!(1.01)),
        ("backgroundBlurRadius", json!(40.1)),
        ("fontScale", json!(0.84)),
        ("bubbleUserImageCropRight", json!(0.46)),
        ("bubbleAiImageRepeatStart", json!(0.01)),
        ("bubbleUserImageRepeatEnd", json!(0.96)),
        ("bubbleAiImageScale", json!(0.19)),
        ("bubbleUserContentPaddingLeft", json!(-1)),
        ("avatarCornerRadius", json!(-0.1)),
    ] {
        assert!(
            validateThemePreferenceSnapshot(&changedSnapshot(key, value)).is_err(),
            "{key}"
        );
    }
}

/// Checks all four repeat intervals and allows nullable u32 color boundaries.
#[test]
fn repeat_ordering_and_nullable_colors_are_strict() {
    for (start, end) in [
        ("bubbleUserImageRepeatStart", "bubbleUserImageRepeatEnd"),
        ("bubbleUserImageRepeatYStart", "bubbleUserImageRepeatYEnd"),
        ("bubbleAiImageRepeatStart", "bubbleAiImageRepeatEnd"),
        ("bubbleAiImageRepeatYStart", "bubbleAiImageRepeatYEnd"),
    ] {
        let mut value = snapshot();
        value.insert(start.to_string(), json!(0.8));
        value.insert(end.to_string(), json!(0.8));
        assert!(validateThemePreferenceSnapshot(&value).is_err());
        value.insert(end.to_string(), json!(0.82));
        validateThemePreferenceSnapshot(&value).unwrap();
    }
    validateThemePreferenceSnapshot(&changedSnapshot("customPrimaryColor", Value::Null)).unwrap();
    validateThemePreferenceSnapshot(&changedSnapshot("customPrimaryColor", json!(0))).unwrap();
    validateThemePreferenceSnapshot(&changedSnapshot("customPrimaryColor", json!(u32::MAX)))
        .unwrap();
}

/// Initializes only schema metadata while leaving a new catalog and selection empty.
#[test]
fn initial_catalog_initializes_only_schema_metadata() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    assert!(manager.list().unwrap().is_empty());
    assert_eq!(manager.getActive().unwrap(), None);
    assert_eq!(
        manager.watch().first().unwrap(),
        ThemeConfigState {
            configs: vec![],
            activeThemeConfigId: None
        }
    );
    assert_eq!(fixture.writes(), 1);
    let values = fixture.preferences().data().unwrap();
    assert_eq!(values.entries().len(), 1);
    assert_eq!(
        values.get(&stringPreferencesKey(
            operit_store::PreferencesDataStore::PREFERENCES_SCHEMA_VERSION_KEY_NAME
        )),
        Some(&json!(4))
    );
}

/// Runs real persisted CRUD without implicitly applying newly created configurations.
#[test]
fn independent_crud_preserves_custom_appearance() {
    let fixture = StoreFixture::new();
    let ordinary = fixture.preferences();
    ordinary
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("theme_mode"), "dark".to_string())
        })
        .unwrap();
    let manager = fixture.manager();
    let config = manager
        .create("First theme".to_string(), snapshot())
        .unwrap();
    assert_eq!(manager.get(config.id.clone()).unwrap(), config);
    assert_eq!(manager.getActive().unwrap(), None);
    let updated = manager
        .update(
            config.id.clone(),
            "Renamed".to_string(),
            changedSnapshot("themeMode", json!("light")),
        )
        .unwrap();
    assert_eq!(updated.createdAt, config.createdAt);
    assert!(updated.updatedAt >= updated.createdAt);
    assert_eq!(
        ordinary
            .data()
            .unwrap()
            .get(&stringPreferencesKey("theme_mode")),
        Some(&json!("dark"))
    );
    manager.delete(config.id.clone()).unwrap();
    assert!(manager.list().unwrap().is_empty());
    assert!(manager.get(config.id).is_err());
}

/// Applies the full snapshot and active identifier through one actual preference commit.
#[test]
fn apply_commits_snapshot_and_selection_once() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let config = manager
        .create(
            "Selected".to_string(),
            changedSnapshot("themeMode", json!("dark")),
        )
        .unwrap();
    let writes = fixture.writes();
    assert_eq!(manager.apply(config.id.clone()).unwrap(), config);
    assert_eq!(fixture.writes(), writes + 1);
    let ordinary = fixture.preferences().data().unwrap();
    assertThemePreferenceSnapshot(&ordinary, &config.snapshot).unwrap();
    assert_eq!(
        serde_json::from_value::<Option<String>>(
            ordinary
                .get(&stringPreferencesKey(ACTIVE_THEME_CONFIG_ID_KEY))
                .unwrap()
                .clone()
        )
        .unwrap(),
        Some(config.id.clone())
    );
    assert_eq!(fixture.manager().getActive().unwrap(), Some(config));
}

/// Synchronizes active named updates and ordinary settings saves in the same canonical commits.
#[test]
fn active_update_and_ordinary_save_keep_named_snapshot_synchronized() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let config = manager.create("Selected".to_string(), snapshot()).unwrap();
    manager.apply(config.id.clone()).unwrap();
    let writes = fixture.writes();
    let updated = manager
        .update(
            config.id.clone(),
            "Edited name".to_string(),
            changedSnapshot("fontScale", json!(1.2)),
        )
        .unwrap();
    assert_eq!(fixture.writes(), writes + 1);
    assertThemePreferenceSnapshot(&fixture.preferences().data().unwrap(), &updated.snapshot)
        .unwrap();
    let edited = changedSnapshot("customUserAvatarUri", json!("runtime/avatar.png"));
    let saved = manager.saveAppearance(edited.clone()).unwrap().unwrap();
    assert_eq!(fixture.writes(), writes + 2);
    assert_eq!(saved.id, config.id);
    assert_eq!(saved.name, "Edited name");
    assert_eq!(saved.createdAt, config.createdAt);
    assert_eq!(saved.snapshot, edited);
    assert_eq!(manager.getActive().unwrap(), Some(saved.clone()));
    assertThemePreferenceSnapshot(&fixture.preferences().data().unwrap(), &saved.snapshot).unwrap();
}

/// Explicitly selects custom appearance without changing or automatically replacing a saved theme.
#[test]
fn explicit_custom_selection_preserves_named_records() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let named = manager.create("Named".to_string(), snapshot()).unwrap();
    manager.apply(named.id.clone()).unwrap();
    let custom = changedSnapshot("themeMode", json!("light"));
    let writes = fixture.writes();
    manager.useCustomAppearance(custom.clone()).unwrap();
    assert_eq!(fixture.writes(), writes + 1);
    assert_eq!(manager.getActive().unwrap(), None);
    assert_eq!(manager.get(named.id.clone()).unwrap(), named);
    assertThemePreferenceSnapshot(&fixture.preferences().data().unwrap(), &custom).unwrap();
    manager.delete(named.id).unwrap();
    assert!(manager.list().unwrap().is_empty());
    assert_eq!(manager.saveAppearance(custom).unwrap(), None);
}

/// Rejects active deletion and unknown IDs without changing persisted bytes or commit counts.
#[test]
fn rejected_deletion_and_unknown_ids_do_not_mutate_bytes() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let named = manager.create("Named".to_string(), snapshot()).unwrap();
    manager.apply(named.id.clone()).unwrap();
    let bytes = fixture.bytes();
    let writes = fixture.writes();
    assert!(manager.delete(named.id).is_err());
    assert!(manager.get("missing".to_string()).is_err());
    assert!(manager.apply("missing".to_string()).is_err());
    assert!(manager
        .update("missing".to_string(), "Valid".to_string(), snapshot())
        .is_err());
    assert!(manager.delete("missing".to_string()).is_err());
    assert_eq!(fixture.bytes(), bytes);
    assert_eq!(fixture.writes(), writes);
}

/// Rejects bad snapshots and blank names before any real preference mutation is committed.
#[test]
fn invalid_mutations_do_not_change_bytes() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let named = manager.create("Named".to_string(), snapshot()).unwrap();
    let bytes = fixture.bytes();
    let writes = fixture.writes();
    assert!(manager.create(" ".to_string(), snapshot()).is_err());
    assert!(manager
        .create("Bad".to_string(), changedSnapshot("fontScale", json!(10)))
        .is_err());
    assert!(manager
        .update(named.id, " ".to_string(), snapshot())
        .is_err());
    assert!(manager
        .saveAppearance(changedSnapshot("missing", json!(true)))
        .is_err());
    assert!(manager
        .useCustomAppearance(changedSnapshot("bubbleAiImageScale", json!(0)))
        .is_err());
    assert_eq!(fixture.bytes(), bytes);
    assert_eq!(fixture.writes(), writes);
}

/// Propagates corrupt catalog JSON through read, watch and mutation instead of treating it as empty.
#[test]
fn corrupt_catalog_json_propagates_without_mutating_bytes() {
    for invalid in ["{", "null", "{}", "[1]"] {
        let fixture = StoreFixture::new();
        fixture
            .preferences()
            .edit(|preferences| {
                preferences.set(
                    &stringPreferencesKey("theme_config_known"),
                    invalid.to_string(),
                )
            })
            .unwrap();
        let bytes = fixture.bytes();
        let writes = fixture.writes();
        let manager = fixture.manager();
        assert!(manager.list().is_err());
        assert!(manager.getActive().is_err());
        assert!(manager.watch().first().is_err());
        assert!(manager.create("New".to_string(), snapshot()).is_err());
        assert!(manager.useCustomAppearance(snapshot()).is_err());
        assert_eq!(fixture.bytes(), bytes);
        assert_eq!(fixture.writes(), writes);
    }
}

/// Rejects missing snapshot fields, unknown record fields, mismatched IDs and dangling active IDs.
#[test]
fn invalid_saved_records_and_selections_are_not_repaired() {
    let valid = ThemeConfig {
        id: "known".to_string(),
        name: "Known".to_string(),
        snapshot: snapshot(),
        createdAt: 1,
        updatedAt: 1,
    };
    let mut incomplete = valid.clone();
    incomplete.snapshot.remove("fontScale");
    let mut unknown_record = serde_json::to_value(&valid).unwrap();
    unknown_record
        .as_object_mut()
        .unwrap()
        .insert("unknown".to_string(), json!(true));
    let mut wrong_identity = valid.clone();
    wrong_identity.id = "another".into();
    for record in [json!(incomplete), unknown_record, json!(wrong_identity)] {
        let fixture = StoreFixture::new();
        fixture
            .preferences()
            .edit(|preferences| {
                preferences.set(&stringPreferencesKey("theme_config_known"), record.clone())
            })
            .unwrap();
        let before = fixture.bytes();
        assert!(fixture.manager().list().is_err());
        assert!(fixture.manager().saveAppearance(snapshot()).is_err());
        assert_eq!(fixture.bytes(), before);
    }
    for active in ["\"deleted\"", "\"\"", "not JSON", "7"] {
        let fixture = StoreFixture::new();
        fixture
            .preferences()
            .edit(|preferences| {
                preferences.set(
                    &stringPreferencesKey("theme_config_known"),
                    json!(valid.clone()),
                );
                preferences.set(
                    &stringPreferencesKey(ACTIVE_THEME_CONFIG_ID_KEY),
                    active.clone(),
                );
            })
            .unwrap();
        let before = fixture.bytes();
        assert!(fixture.manager().getActive().is_err());
        assert!(fixture.manager().apply(valid.id.clone()).is_err());
        assert_eq!(fixture.bytes(), before);
    }
}

/// Rejects an externally divergent active ordinary preference without overwriting either source.
#[test]
fn active_ordinary_divergence_is_reported() {
    for (key, value) in [
        ("theme_mode", "dark"),
        ("custom_primary_color", "null"),
        ("custom_user_avatar_uri", "null"),
    ] {
        let fixture = StoreFixture::new();
        let manager = fixture.manager();
        let named = manager.create("Named".to_string(), snapshot()).unwrap();
        manager.apply(named.id).unwrap();
        fixture
            .preferences()
            .edit(|preferences| preferences.set(&stringPreferencesKey(key), value.clone()))
            .unwrap();
        let before = fixture.bytes();
        assert!(manager.getActive().is_err(), "{key}");
        assert!(manager.watch().first().is_err(), "{key}");
        assert!(manager.saveAppearance(snapshot()).is_err(), "{key}");
        assert_eq!(fixture.bytes(), before);
    }
}

/// Propagates the original write error and keeps both real bytes and the observed state unchanged.
#[test]
fn host_write_failure_preserves_committed_state() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let named = manager.create("Named".to_string(), snapshot()).unwrap();
    let before = fixture.bytes();
    let state = manager.watch().first().unwrap();
    fixture
        .host
        .failPreferenceWrite
        .store(true, Ordering::SeqCst);
    let error = manager.apply(named.id).unwrap_err();
    match error {
        PreferencesDataStoreError::Host(error) => {
            assert_eq!(error.message, "injected theme preference write failure")
        }
        other => panic!("unexpected error type: {other}"),
    }
    assert_eq!(fixture.bytes(), before);
    assert_eq!(manager.watch().first().unwrap(), state);
}

/// Shares real committed changes across manager instances without sharing catalogs across runtime hosts.
#[test]
fn actual_runtime_hosts_isolate_catalogs_and_selection() {
    let first = StoreFixture::new();
    let second = StoreFixture::new();
    let named = first
        .manager()
        .create("First runtime".to_string(), snapshot())
        .unwrap();
    first.manager().apply(named.id.clone()).unwrap();
    assert_eq!(first.manager().getActive().unwrap(), Some(named.clone()));
    assert!(second.manager().list().unwrap().is_empty());
    assert_eq!(second.manager().getActive().unwrap(), None);
    assert!(second.manager().get(named.id).is_err());
    assert_eq!(second.writes(), 1);
}

/// Emits one coherent state per shared commit and no state for rejected mutations.
#[test]
fn watch_observes_cross_context_commits_once() {
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
        ThemeObserverScheduler,
    ));
    let fixture = StoreFixture::new();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let observedForCallback = observed.clone();
    let cancellation = FlowCancellation::new();
    let subscription = fixture
        .manager()
        .watch()
        .subscribeWithCancellation(cancellation, move |state| {
            observedForCallback.lock().unwrap().push(state);
        })
        .unwrap();
    let writer = fixture.manager();
    let named = writer.create("Watched".to_string(), snapshot()).unwrap();
    writer.apply(named.id.clone()).unwrap();
    writer
        .saveAppearance(changedSnapshot("themeMode", json!("dark")))
        .unwrap();
    assert!(writer.delete(named.id.clone()).is_err());
    {
        let states = observed.lock().unwrap();
        assert_eq!(states.len(), 4);
        assert!(states[0].configs.is_empty());
        assert_eq!(states[1].activeThemeConfigId, None);
        assert_eq!(states[2].activeThemeConfigId, Some(named.id.clone()));
        assert_eq!(
            states[3].configs[0].snapshot.get("themeMode"),
            Some(&json!("dark"))
        );
    }
    subscription.cancel();
    writer.useCustomAppearance(snapshot()).unwrap();
    assert_eq!(observed.lock().unwrap().len(), 4);
}

/// Removes the obsolete ordinary flag in the same apply commit so it cannot override theme_mode.
#[test]
fn apply_clears_only_obsolete_ordinary_theme_override() {
    let fixture = StoreFixture::new();
    fixture
        .preferences()
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("use_system_theme"), json!(true));
            preferences.set(&stringPreferencesKey("app_language"), "en".to_string());
        })
        .unwrap();
    let manager = fixture.manager();
    let named = manager
        .create(
            "Dark".to_string(),
            changedSnapshot("themeMode", json!("dark")),
        )
        .unwrap();
    let writes = fixture.writes();
    manager.apply(named.id).unwrap();
    assert_eq!(fixture.writes(), writes + 1);
    let ordinary = fixture.preferences().data().unwrap();
    assert!(ordinary
        .get(&stringPreferencesKey("use_system_theme"))
        .is_none());
    assert_eq!(
        ordinary.get(&stringPreferencesKey("theme_mode")),
        Some(&json!("dark"))
    );
    assert_eq!(
        ordinary.get(&stringPreferencesKey("app_language")),
        Some(&json!("en"))
    );
}

/// Keeps partial edits, saved configuration and ordinary appearance in a single real preference commit.
#[test]
fn partial_appearance_edits_update_only_the_active_named_configuration() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let first = manager.create("First".to_string(), snapshot()).unwrap();
    let second = manager.create("Second".to_string(), snapshot()).unwrap();
    manager.apply(first.id.clone()).unwrap();
    let writes = fixture.writes();
    let changed = manager
        .patchAppearance(
            snapshot(),
            BTreeMap::from([("font_scale".to_string(), json!(1.2))]),
            vec![],
        )
        .unwrap()
        .unwrap();
    assert_eq!(fixture.writes(), writes + 1);
    assert_eq!(changed.id, first.id);
    assert_eq!(changed.snapshot["fontScale"], json!(1.2));
    fixture
        .manager()
        .patchAppearance(
            snapshot(),
            BTreeMap::from([("theme_mode".to_string(), json!("dark"))]),
            vec![],
        )
        .unwrap();
    let current = fixture.manager().getActive().unwrap().unwrap();
    assert_eq!(current.snapshot["fontScale"], json!(1.2));
    assert_eq!(current.snapshot["themeMode"], json!("dark"));
    assert_eq!(manager.get(second.id.clone()).unwrap(), second);
    manager.apply(second.id.clone()).unwrap();
    let restored = manager.apply(first.id.clone()).unwrap();
    assert_eq!(restored.snapshot, current.snapshot);
    assertThemePreferenceSnapshot(&fixture.preferences().data().unwrap(), &current.snapshot)
        .unwrap();
}

/// Rejects malformed edits before storage writes and retains the original active configuration.
#[test]
fn partial_appearance_rejects_invalid_fields_and_values_without_writing() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let config = manager.create("Strict".to_string(), snapshot()).unwrap();
    manager.apply(config.id.clone()).unwrap();
    let before = fixture.bytes();
    for values in [
        BTreeMap::from([("theme_configs".to_string(), json!("[]"))]),
        BTreeMap::from([("font_scale".to_string(), json!("broken"))]),
        BTreeMap::from([("font_scale".to_string(), json!(9))]),
        BTreeMap::from([("theme_mode".to_string(), json!("unknown"))]),
    ] {
        assert!(manager.patchAppearance(snapshot(), values, vec![]).is_err());
        assert_eq!(fixture.bytes(), before);
    }
    assert!(manager
        .patchAppearance(
            snapshot(),
            BTreeMap::from([("font_scale".to_string(), json!(1.1))]),
            vec!["font_scale".to_string()]
        )
        .is_err());
    assert_eq!(manager.getActive().unwrap().unwrap().id, config.id);
}

/// Resets explicitly requested fields to their declared initial values while retaining other appearance and preference data.
#[test]
fn partial_appearance_reset_preserves_unrelated_values_and_renaming_preserves_edits() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let initial = snapshot();
    let config = manager
        .create("Original".to_string(), initial.clone())
        .unwrap();
    manager.apply(config.id.clone()).unwrap();
    manager
        .patchAppearance(
            initial.clone(),
            BTreeMap::from([
                ("font_scale".to_string(), json!(1.2)),
                (
                    "custom_user_avatar_uri".to_string(),
                    json!("host:///avatars/user.png"),
                ),
            ]),
            vec![],
        )
        .unwrap();
    let reset = manager
        .patchAppearance(
            initial.clone(),
            BTreeMap::new(),
            vec!["font_scale".to_string()],
        )
        .unwrap()
        .unwrap();
    assert_eq!(reset.snapshot["fontScale"], initial["fontScale"]);
    assert_eq!(
        reset.snapshot["customUserAvatarUri"],
        json!("host:///avatars/user.png")
    );
    let renamed = manager
        .rename(config.id.clone(), "Renamed".to_string())
        .unwrap();
    assert_eq!(renamed.snapshot, reset.snapshot);
    assert_eq!(renamed.name, "Renamed");
    assert_eq!(manager.getActive().unwrap().unwrap(), renamed);
}

/// Initializes only explicit custom appearance without manufacturing a saved configuration or active selection.
#[test]
fn partial_appearance_without_a_selected_configuration_keeps_an_empty_catalog() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    assert!(manager
        .patchAppearance(
            snapshot(),
            BTreeMap::from([("theme_mode".to_string(), json!("light"))]),
            vec![]
        )
        .unwrap()
        .is_none());
    assert!(manager.list().unwrap().is_empty());
    assert!(manager.getActive().unwrap().is_none());
    let preferences = fixture.preferences().data().unwrap();
    assert_eq!(
        preferences.get(&stringPreferencesKey("theme_mode")),
        Some(&json!("light"))
    );
}

/// Executes preference observer callbacks synchronously in this isolated theme-owner test.
struct ThemeObserverScheduler;

impl operit_host_api::HostRuntimeTaskSchedulerHost for ThemeObserverScheduler {
    /// Supplies the deterministic observer clock used by this test host.
    fn monotonicTimeMillis(&self) -> operit_host_api::HostResult<u64> {
        Ok(0)
    }
    /// Publishes one committed preference callback before the writer acknowledges the change.
    fn scheduleHostRuntimeTask(
        &self,
        _: &str,
        task: operit_host_api::HostRuntimeTask,
    ) -> operit_host_api::HostResult<()> {
        task();
        Ok(())
    }
    /// Rejects asynchronous work outside this test's declared observer boundary.
    fn scheduleHostRuntimeAsyncTask(
        &self,
        _: &str,
        _: operit_host_api::HostRuntimeAsyncTask,
    ) -> operit_host_api::HostResult<()> {
        Err(HostError::new(
            "Theme observer tests do not schedule async work",
        ))
    }
    /// Rejects cooperative futures outside this test's declared synchronous observer boundary.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        _: &str,
        _: operit_host_api::HostRuntimeAsyncTask,
    ) -> operit_host_api::HostResult<()> {
        Err(HostError::new(
            "Theme observer tests do not schedule cooperative async work",
        ))
    }
    /// Rejects delayed work outside this test's declared observer boundary.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: operit_host_api::HostRuntimeTask,
    ) -> operit_host_api::HostResult<()> {
        Err(HostError::new(
            "Theme observer tests do not schedule delayed work",
        ))
    }
    /// Rejects delayed futures outside this observer test's synchronous boundary.
    fn waitForHostRuntimeDelay(&self, _: u64) -> operit_host_api::HostRuntimeTurnFuture {
        Box::pin(async {
            Err(HostError::new(
                "Theme observer tests do not wait for delays",
            ))
        })
    }
    /// Finishes the explicitly synchronous observer turn.
    fn waitForHostRuntimeTaskTurn(&self) -> operit_host_api::HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }
}

/// Migrates known user values in version one while preserving ordinary strings exactly.
#[test]
fn user_schema_one_migration_preserves_declared_native_types() {
    let fixture = StoreFixture::new();
    fixture.host.writeBytes(OperitPaths::USER_PREFERENCES_PATH,&serde_json::to_vec(&json!({
        "__operit_preferences_schema_version":"1", "font_scale":"1.25", "use_custom_colors":"true", "custom_primary_color":"4278190080",
        "long_pasted_text_input_enabled":"false", "long_pasted_text_input_threshold":"5000", "theme_configs":"[]", "active_theme_config_id":"null", "app_language":"true"
    })).unwrap()).unwrap();
    let preferences = fixture.preferences();
    let values = preferences.data().unwrap();
    assert_eq!(preferences.schemaVersion().unwrap(), 4);
    assert_eq!(
        values.get(&stringPreferencesKey("font_scale")),
        Some(&json!(1.25))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("use_custom_colors")),
        Some(&json!(true))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("custom_primary_color")),
        Some(&json!(4278190080_u32))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("long_pasted_text_input_enabled")),
        Some(&json!(false))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("long_pasted_text_input_threshold")),
        Some(&json!(5000))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("theme_configs")),
        None
    );
    assert_eq!(
        values.get(&stringPreferencesKey("active_theme_config_id")),
        Some(&Value::Null)
    );
    assert_eq!(
        values.get(&stringPreferencesKey("app_language")),
        Some(&json!("true"))
    );
}

/// Migrates only API-owned boolean, counter, and map values without guessing text values.
#[test]
fn api_schema_one_migration_preserves_declared_native_types() {
    let fixture = StoreFixture::new();
    fixture.host.writeBytes(OperitPaths::API_PREFERENCES_PATH,&serde_json::to_vec(&json!({
        "__operit_preferences_schema_version":"1", "enable_tools":"false", "token_input_test":"23", "mcp_startup_timeout_seconds":"15",
        "feature_toggles_json":"{\"feature\":true}", "tool_prompt_visibility_json":"{\"tool\":false}", "api_key":"false"
    })).unwrap()).unwrap();
    let store = super::super::ApiPreferences::ApiPreferences::store(fixture.host.clone());
    let values = store.data().unwrap();
    assert_eq!(store.schemaVersion().unwrap(), 2);
    assert_eq!(
        values.get(&stringPreferencesKey("enable_tools")),
        Some(&json!(false))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("token_input_test")),
        Some(&json!(23))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("mcp_startup_timeout_seconds")),
        Some(&json!(15))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("feature_toggles_json")),
        Some(&json!({"feature":true}))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("tool_prompt_visibility_json")),
        Some(&json!({"tool":false}))
    );
    assert_eq!(
        values.get(&stringPreferencesKey("api_key")),
        Some(&json!("false"))
    );
}

/// Keeps new input appearance fields in ordinary preferences and active saved configurations.
#[test]
fn input_appearance_extensions_share_atomic_named_theme_commits() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let mut initial = snapshot();
    initial.insert("inputBackgroundColor".into(), Value::Null);
    initial.insert("inputSendAsset".into(), Value::Null);
    initial.insert("inputSendAssetSize".into(), json!(26.0));
    let saved = manager.create("Shared input appearance".into(), initial.clone()).unwrap();
    manager.apply(saved.id.clone()).unwrap();
    let before = fixture.writes();
    manager.patchAppearance(initial.clone(), BTreeMap::from([
        ("input_background_color".into(), json!(0xff123456u32)),
        ("input_send_asset".into(), json!("host:///icons/send.svg")),
        ("input_send_asset_size".into(), json!(30.0)),
    ]), Vec::new()).unwrap();
    assert_eq!(fixture.writes(), before + 1);
    let preferences = fixture.preferences().data().unwrap();
    assert_eq!(preferences.get(&stringPreferencesKey("input_send_asset")), Some(&json!("host:///icons/send.svg")));
    let updated = manager.get(saved.id).unwrap();
    assert_eq!(updated.snapshot["inputBackgroundColor"], json!(0xff123456u32));
    assert_eq!(updated.snapshot["inputSendAssetSize"], json!(30.0));
    validateThemePreferenceSnapshot(&snapshot()).unwrap();
}

/// Clears every extended template field when applying an original 78-field configuration.
#[test]
fn released_named_theme_clears_previous_template_input_preferences() {
    use super::super::ThemePreferenceSnapshot::{EXTENDED_SNAPSHOT_FIELDS, FieldKind};
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let released = manager.create("Released appearance".into(), snapshot()).unwrap();
    let visual: Value = serde_json::from_slice(&operit_plugin_theme::normalize_draft(b"{}").unwrap().canonical_bytes()).unwrap();
    let mut extended_values = BTreeMap::new();
    for field in EXTENDED_SNAPSHOT_FIELDS {
        let value = match field.kind {
            FieldKind::Boolean => json!(true),
            FieldKind::OptionalString => json!("runtime/data/user_assets/theme/template.svg"),
            FieldKind::OptionalColor => json!(0xff123456u32),
            FieldKind::OptionalPalette => json!(serde_json::to_string(&visual["palette"]).unwrap()),
            FieldKind::Enum(values) => json!(values[0]),
            FieldKind::Number { min, .. } => json!(min),
            FieldKind::OptionalEnum(values) => json!(values[0]),
        };
        extended_values.insert(field.preference_key.to_owned(), value);
    }
    fixture.preferences().edit(|preferences| {
        for field in EXTENDED_SNAPSHOT_FIELDS {
            preferences.set(&stringPreferencesKey(field.preference_key), extended_values[field.preference_key].clone());
        }
        preferences.set(&stringPreferencesKey("unrelated_setting"), json!("untouched"));
    }).unwrap();
    manager.apply(released.id).unwrap();
    let preferences = fixture.preferences().data().unwrap();
    for field in EXTENDED_SNAPSHOT_FIELDS {
        assert!(preferences.get(&stringPreferencesKey(field.preference_key)).is_none(), "{}", field.preference_key);
    }
    assert_eq!(preferences.get(&stringPreferencesKey("unrelated_setting")), Some(&json!("untouched")));
}

/// Preserves copied complete palettes independently and clears them when applying a released seed configuration.
#[test]
fn copied_template_palette_survives_configuration_inheritance() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let draft = operit_plugin_theme::normalize_draft(b"{}").unwrap();
    let visual: Value = serde_json::from_slice(&draft.canonical_bytes()).unwrap();
    let palette = json!(serde_json::to_string(&visual["palette"]).unwrap());
    let mut template = snapshot();
    template.insert("themePaletteJson".into(), palette.clone());
    let original = manager.create("Template copy".into(), template.clone()).unwrap();
    manager.apply(original.id.clone()).unwrap();
    let inherited = manager.create("Inherited copy".into(), manager.get(original.id.clone()).unwrap().snapshot).unwrap();
    manager.apply(inherited.id).unwrap();
    assert_eq!(manager.get(original.id).unwrap().snapshot, template);
    assert_eq!(fixture.preferences().data().unwrap().get(&stringPreferencesKey("theme_palette_json")), Some(&palette));
    let released = manager.create("Released appearance".into(), snapshot()).unwrap();
    manager.apply(released.id).unwrap();
    assert!(fixture.preferences().data().unwrap().get(&stringPreferencesKey("theme_palette_json")).is_none());
}

/// Rejects malformed copied palettes before any ordinary preference mutation.
#[test]
fn malformed_configuration_palette_is_rejected_without_writes() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let writes = fixture.writes();
    for palette in ["not json", "{}", r#"{"light":{},"dark":{}}"#] {
        let mut config = snapshot();
        config.insert("themePaletteJson".into(), json!(palette));
        assert!(manager.create("Invalid palette".into(), config).is_err());
    }
    assert_eq!(fixture.writes(), writes);
}

/// Explicit replacement repairs invalid ordinary appearance without modifying valid saved configurations.
#[test]
fn explicit_custom_appearance_repairs_invalid_preferences_and_preserves_catalog() {
    let fixture = StoreFixture::new();
    let manager = fixture.manager();
    let saved = manager.create("Saved appearance".into(), snapshot()).unwrap();
    manager.apply(saved.id.clone()).unwrap();
    fixture.preferences().edit(|preferences| {
        preferences.set(&stringPreferencesKey("font_scale"), json!("invalid"));
    }).unwrap();
    assert!(manager.list().is_err());
    let writes = fixture.writes();
    manager.useCustomAppearance(snapshot()).unwrap();
    assert_eq!(fixture.writes(), writes + 1);
    assert_eq!(manager.get(saved.id.clone()).unwrap(), saved);
    assert!(manager.getActive().unwrap().is_none());
    assert_eq!(fixture.preferences().data().unwrap().get(&stringPreferencesKey("font_scale")), Some(&snapshot()["fontScale"]));
}


/// Migrates through the user preferences owner before any theme manager is opened.
#[test]
fn user_schema_two_migration_preserves_records_and_active_appearance() {
    let fixture = StoreFixture::new();
    let first = ThemeConfig {
        id: Uuid::new_v4().to_string(), name: "First".into(), snapshot: snapshot(),
        createdAt: 1, updatedAt: 2,
    };
    let second = ThemeConfig {
        id: Uuid::new_v4().to_string(), name: "Second".into(),
        snapshot: changedSnapshot("fontScale", json!(1.2)), createdAt: 3, updatedAt: 4,
    };
    let mut saved = Preferences::default();
    saved.set(&stringPreferencesKey("__operit_preferences_schema_version"), 2);
    saved.set(&stringPreferencesKey("theme_configs"), json!([first, second]));
    saved.set(&stringPreferencesKey("active_theme_config_id"), json!(second.id));
    writeThemePreferenceSnapshot(&mut saved, &second.snapshot).unwrap();
    fixture.host.writeBytes(OperitPaths::USER_PREFERENCES_PATH, &serde_json::to_vec(&saved).unwrap()).unwrap();
    let writes = fixture.writes();
    let entries = fixture.preferences().data().unwrap();
    assert_eq!(fixture.writes(), writes + 1);
    assert_eq!(entries.get(&stringPreferencesKey("__operit_preferences_schema_version")), Some(&json!(4)));
    let migratedBytes = fixture.bytes();
    let manager = fixture.manager();
    assert_eq!(manager.list().unwrap(), vec![first.clone(), second.clone()]);
    assert_eq!(manager.getActive().unwrap(), Some(second.clone()));
    assert_eq!(fixture.writes(), writes + 1);
    assert_eq!(fixture.bytes(), migratedBytes);
    assert!(!entries.contains(&stringPreferencesKey("theme_configs")));
    assert_eq!(entries.get(&stringPreferencesKey(&format!("theme_config_{}", first.id))), Some(&json!(first)));
    assert_eq!(entries.get(&stringPreferencesKey(&format!("theme_config_{}", second.id))), Some(&json!(second)));
}

/// Keeps the schema-two file intact when catalog migration rejects conflicting or invalid records.
#[test]
fn user_schema_two_migration_rejects_invalid_catalogs_without_persisting_changes() {
    let first = ThemeConfig {
        id: Uuid::new_v4().to_string(),
        name: "First".into(),
        snapshot: snapshot(),
        createdAt: 1,
        updatedAt: 2,
    };
    let second = ThemeConfig {
        id: Uuid::new_v4().to_string(),
        name: "Second".into(),
        ..first.clone()
    };
    let mut invalid = second.clone();
    invalid.snapshot.remove("fontScale");
    for (catalog, destination) in [
        (json!([first, first]), None),
        (json!([first, second]), Some(second.clone())),
        (json!([first, invalid]), None),
        (json!({"invalid": "catalog"}), None),
    ] {
        let fixture = StoreFixture::new();
        let mut saved = Preferences::default();
        saved.set(&stringPreferencesKey("__operit_preferences_schema_version"), 2);
        saved.set(&stringPreferencesKey("theme_configs"), catalog);
        saved.set(&stringPreferencesKey("active_theme_config_id"), json!(first.id));
        if let Some(config) = destination {
            saved.set(
                &stringPreferencesKey(&format!("theme_config_{}", config.id)),
                json!(config),
            );
        }
        let original = serde_json::to_vec(&saved).unwrap();
        fixture.host.writeBytes(OperitPaths::USER_PREFERENCES_PATH, &original).unwrap();
        let writes = fixture.writes();
        let store = fixture.preferences();
        assert!(store.data().is_err());
        assert_eq!(fixture.writes(), writes);
        assert_eq!(fixture.bytes(), original);
        assert!(store.data().is_err());
        assert_eq!(fixture.bytes(), original);
    }
}

/// Keeps two independently synchronized themes instead of replacing a shared catalog array.
#[test]
fn synchronized_themes_preserve_independent_identity_records() {
    use operit_store::SyncOperationStore::{SyncClock, SyncOperationStore};
    use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
    let source = StoreFixture::new();
    let target = StoreFixture::new();
    let first = source
        .manager()
        .create("Shared name".into(), snapshot())
        .unwrap();
    let second = target
        .manager()
        .create(
            "Shared name".into(),
            changedSnapshot("fontScale", json!(1.2)),
        )
        .unwrap();
    let sourceJournal = SyncOperationStore::new(source.host.clone(), RUNTIME_SYNC_DIR_PATH);
    let targetJournal = SyncOperationStore::new(target.host.clone(), RUNTIME_SYNC_DIR_PATH);
    let operations = sourceJournal
        .operationsSince(&SyncClock::empty(), &["preferences".into()], 512)
        .unwrap();
    let history = targetJournal.preferenceHistory().unwrap();
    operit_store::PreferenceMerge::applyOperations(
        target.host.clone(),
        &crate::data::preferences::PreferenceSpaceMerge::schema(&operations),
        &history,
        &operations,
        |_, _| true,
    )
    .unwrap();
    let manager = target.manager();
    assert_eq!(manager.list().unwrap().len(), 2);
    let mut mergedFirst = manager.get(first.id.clone()).unwrap();
    let mut mergedSecond = manager.get(second.id.clone()).unwrap();
    // The shared catalog resolves duplicate display names while preserving
    // both identities, snapshots, and timestamps.
    let mut names = vec![mergedFirst.name.clone(), mergedSecond.name.clone()];
    names.sort();
    assert_eq!(names, vec!["Shared name", "Shared name（1）"]);
    mergedFirst.name = first.name.clone();
    mergedSecond.name = second.name.clone();
    assert_eq!(mergedFirst, first);
    assert_eq!(mergedSecond, second);
}

/// Repairs installed schema-one, schema-two and schema-three color representations before any active-theme validation.
#[test]
fn legacy_color_migration_covers_existing_user_schema_versions() {
    use super::super::ThemePreferenceSnapshot::{snapshotFields, FieldKind};
    for version in [1, 2, 3] {
        let fixture = StoreFixture::new();
        let canonical = snapshot();
        let mut saved = Preferences::default();
        writeThemePreferenceSnapshot(&mut saved, &canonical).unwrap();
        let mut old_snapshot = canonical;
        for field in snapshotFields().filter(|field| matches!(field.kind, FieldKind::OptionalColor))
        {
            old_snapshot.insert(field.json_key.to_string(), json!(-16711936));
            saved.set(
                &stringPreferencesKey(field.preference_key),
                if version == 1 {
                    json!("-16711936")
                } else {
                    json!(-16711936)
                },
            );
        }
        old_snapshot.insert("retiredHistoricalSetting".to_string(), json!(true));
        let old = ThemeConfig {
            id: "legacy-color".into(),
            name: "My theme".into(),
            snapshot: old_snapshot,
            createdAt: 10,
            updatedAt: 20,
        };
        saved.set(
            &stringPreferencesKey("__operit_preferences_schema_version"),
            json!(version),
        );
        saved.set(
            &stringPreferencesKey("active_theme_config_id"),
            if version == 1 {
                json!("\"legacy-color\"")
            } else {
                json!("legacy-color")
            },
        );
        match version {
            1 => saved.set(
                &stringPreferencesKey("theme_configs"),
                serde_json::to_string(&vec![old.clone()]).unwrap(),
            ),
            2 => saved.set(&stringPreferencesKey("theme_configs"), json!([old.clone()])),
            3 => saved.set(
                &stringPreferencesKey("theme_config_legacy-color"),
                json!(old.clone()),
            ),
            _ => unreachable!(),
        }
        saved.set(&stringPreferencesKey("unrelated"), json!({"keep":true}));
        fixture
            .host
            .writeBytes(
                OperitPaths::USER_PREFERENCES_PATH,
                &serde_json::to_vec(&saved).unwrap(),
            )
            .unwrap();
        let entries = fixture.preferences().data().unwrap();
        assert_eq!(fixture.preferences().schemaVersion().unwrap(), 4);
        let mut expected = old;
        expected.snapshot.remove("retiredHistoricalSetting");
        for field in snapshotFields().filter(|field| matches!(field.kind, FieldKind::OptionalColor))
        {
            expected
                .snapshot
                .insert(field.json_key.to_string(), json!(4278255360_u32));
            assert_eq!(
                entries.get(&stringPreferencesKey(field.preference_key)),
                Some(&json!(4278255360_u32))
            );
        }
        let manager = fixture.manager();
        assert_eq!(manager.getActive().unwrap(), Some(expected.clone()));
        assert_eq!(manager.get(expected.id.clone()).unwrap(), expected);
        assert_eq!(
            entries.get(&stringPreferencesKey("unrelated")),
            Some(&json!({"keep":true}))
        );
        assert!(entries
            .get(&stringPreferencesKey("theme_configs"))
            .is_none());
        let committed = fixture.bytes();
        fixture.preferences().data().unwrap();
        assert_eq!(fixture.bytes(), committed);
    }
}

/// Rejects values that do not encode a 32-bit ARGB integer without manufacturing a replacement color.
#[test]
fn legacy_color_conversion_checks_exact_bit_width() {
    use super::super::ThemePreferenceSnapshot::{
        decodeLegacyThemePreference, snapshotFields, FieldKind,
    };
    for field in snapshotFields().filter(|field| matches!(field.kind, FieldKind::OptionalColor)) {
        for raw in [
            json!(-2147483649_i64),
            json!(4294967296_i64),
            json!(1.5),
            json!(true),
            json!("invalid"),
        ] {
            assert!(
                decodeLegacyThemePreference(field, raw).is_err(),
                "{}",
                field.json_key
            );
        }
        assert_eq!(
            decodeLegacyThemePreference(field, json!(i32::MIN)).unwrap(),
            json!(2147483648_u32)
        );
        assert_eq!(
            decodeLegacyThemePreference(field, json!(u32::MAX)).unwrap(),
            json!(u32::MAX)
        );
    }
}

/// Migrates all 78 fields emitted by the October 8 writer (425074726), including already installed schema markers.
#[test]
fn october_eighth_preferences_migrate_all_released_field_types() {
    let released: Value =
        serde_json::from_str(include_str!("theme_preferences_oct08_fixture.json")).unwrap();
    let expected: BTreeMap<String, Value> =
        serde_json::from_value(released["snapshot"].clone()).unwrap();
    for version in [0, 1, 2, 3] {
        let fixture = StoreFixture::new();
        let mut saved = released["preferences"].clone();
        if version != 0 {
            saved["__operit_preferences_schema_version"] = json!(version.to_string());
        }
        fixture
            .host
            .writeBytes(
                OperitPaths::USER_PREFERENCES_PATH,
                &serde_json::to_vec(&saved).unwrap(),
            )
            .unwrap();
        let initial_writes = fixture.writes();
        let preferences = fixture.preferences();
        let migrated = preferences.data().unwrap();
        assert_eq!(preferences.schemaVersion().unwrap(), 4);
        assert_eq!(
            readThemePreferenceSnapshot(&migrated, &snapshot()).unwrap(),
            expected
        );
        assert_eq!(fixture.writes(), initial_writes + 1);
        let committed = fixture.bytes();
        preferences.data().unwrap();
        assert_eq!(fixture.bytes(), committed);
        assert_eq!(fixture.writes(), initial_writes + 1);
    }
}
