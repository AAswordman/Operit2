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

    /// Records ordinary preference writes and delegates successful commits to the real host.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        if path == OperitPaths::USER_PREFERENCES_PATH {
            if self.failPreferenceWrite.load(Ordering::SeqCst) {
                return Err(HostError::new("injected theme preference write failure"));
            }
            self.preferenceWrites.fetch_add(1, Ordering::SeqCst);
        }
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
        PreferencesDataStore::newWithStorage(self.host.clone(), OperitPaths::USER_PREFERENCES_PATH)
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
        Some(&"true".to_string())
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("bubble_content_padding_left")),
        Some(&"12".to_string())
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("bubble_ai_content_padding_right")),
        Some(&"12".to_string())
    );
    assert_eq!(
        preferences.get(&stringPreferencesKey("unrelated_setting")),
        Some(&"untouched".to_string())
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

/// Keeps a new catalog genuinely empty without writing or binding an implicit default theme.
#[test]
fn initial_catalog_is_empty_and_read_only() {
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
    assert_eq!(fixture.writes(), 0);
    assert!(!fixture
        .host
        .exists(OperitPaths::USER_PREFERENCES_PATH)
        .unwrap());
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
        Some(&"dark".to_string())
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
        serde_json::from_str::<Option<String>>(
            ordinary
                .get(&stringPreferencesKey(ACTIVE_THEME_CONFIG_ID_KEY))
                .unwrap()
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
                    &stringPreferencesKey(THEME_CONFIGS_KEY),
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

/// Rejects missing snapshot fields, unknown record fields, duplicate IDs and dangling active IDs.
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
    for catalog in [
        json!([incomplete]),
        json!([unknown_record]),
        json!([valid.clone(), valid.clone()]),
    ] {
        let fixture = StoreFixture::new();
        fixture
            .preferences()
            .edit(|preferences| {
                preferences.set(
                    &stringPreferencesKey(THEME_CONFIGS_KEY),
                    catalog.to_string(),
                )
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
                    &stringPreferencesKey(THEME_CONFIGS_KEY),
                    json!([valid.clone()]).to_string(),
                );
                preferences.set(
                    &stringPreferencesKey(ACTIVE_THEME_CONFIG_ID_KEY),
                    active.to_string(),
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
            .edit(|preferences| preferences.set(&stringPreferencesKey(key), value.to_string()))
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
    assert_eq!(second.writes(), 0);
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
            preferences.set(
                &stringPreferencesKey("use_system_theme"),
                "true".to_string(),
            );
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
        Some(&"dark".to_string())
    );
    assert_eq!(
        ordinary.get(&stringPreferencesKey("app_language")),
        Some(&"en".to_string())
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
            BTreeMap::from([("font_scale".to_string(), "1.2".to_string())]),
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
            BTreeMap::from([("theme_mode".to_string(), "dark".to_string())]),
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
        BTreeMap::from([("theme_configs".to_string(), "[]".to_string())]),
        BTreeMap::from([("font_scale".to_string(), "broken".to_string())]),
        BTreeMap::from([("font_scale".to_string(), "9".to_string())]),
        BTreeMap::from([("theme_mode".to_string(), "unknown".to_string())]),
    ] {
        assert!(manager.patchAppearance(snapshot(), values, vec![]).is_err());
        assert_eq!(fixture.bytes(), before);
    }
    assert!(manager
        .patchAppearance(
            snapshot(),
            BTreeMap::from([("font_scale".to_string(), "1.1".to_string())]),
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
                ("font_scale".to_string(), "1.2".to_string()),
                (
                    "custom_user_avatar_uri".to_string(),
                    "host:///avatars/user.png".to_string(),
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
            BTreeMap::from([("theme_mode".to_string(), "light".to_string())]),
            vec![]
        )
        .unwrap()
        .is_none());
    assert!(manager.list().unwrap().is_empty());
    assert!(manager.getActive().unwrap().is_none());
    let preferences = fixture.preferences().data().unwrap();
    assert_eq!(
        preferences.get(&stringPreferencesKey("theme_mode")),
        Some(&"light".to_string())
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
