#![allow(non_snake_case)]

use super::*;
use std::collections::BTreeMap;
use std::sync::Mutex;
use operit_host_api::{HostError, HostResult, RuntimeStorageEntry};

#[derive(Default)]
struct MemoryStorage {
    files: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl RuntimeStorageHost for MemoryStorage {
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> { None }
    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> { None }
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.files.lock().unwrap().get(path).cloned()
            .ok_or_else(|| HostError::new(format!("missing {path}")))
    }
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let bytes = self.readBytes(path)?;
        let start = (offset as usize).min(bytes.len());
        Ok(bytes[start..start.saturating_add(length).min(bytes.len())].to_vec())
    }
    fn writeBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.files.lock().unwrap().insert(path.into(), bytes.into()); Ok(())
    }
    fn appendBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.files.lock().unwrap().entry(path.into()).or_default().extend_from_slice(bytes); Ok(())
    }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.files.lock().unwrap().retain(|key, _|
            key != path && !(recursive && key.starts_with(&format!("{path}/")))); Ok(())
    }
    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.files.lock().unwrap().keys().any(|key|
            key == path || key.starts_with(&format!("{path}/"))))
    }
    fn list(&self, path: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let mut entries = BTreeMap::new();
        for (key, bytes) in self.files.lock().unwrap().iter() {
            if let Some(relative) = key.strip_prefix(&format!("{path}/")) {
                let name = relative.split('/').next().unwrap();
                let full = format!("{path}/{name}");
                let directory = relative.contains('/');
                entries.insert(full.clone(), RuntimeStorageEntry {
                    path: full, isDirectory: directory, size: if directory { 0 } else { bytes.len() as i64 },
                });
            }
        }
        Ok(entries.into_values().collect())
    }
}

fn setup() -> (Arc<MemoryStorage>, ThemeLibraryStore) {
    let storage = Arc::new(MemoryStorage::default());
    let store = ThemeLibraryStore::new(storage.clone());
    // Minimal private-state fixture; this is not a public package/Snapshot validator.
    store.initializeAppearance(r#"{"themeMode":"system","fontScale":1.0,"useBackgroundImage":false,"bubbleShowAvatar":true}"#).unwrap();
    (storage, store)
}

fn selectMissing(storage: &Arc<MemoryStorage>) -> ThemeSelection {
    let selection = ThemeSelection {
        schema_version: 1, theme_id: "com.example.missing".into(),
        version: "1.0.0".into(), archive_sha256: "a".repeat(64),
    };
    selectionStore(storage.clone()).edit(|preferences|
        preferences.set(&stringPreferencesKey(THEME_SELECTION_KEY), serde_json::to_string(&selection).unwrap())).unwrap();
    selection
}

#[test]
fn nodeAppearanceDoesNotReadOrMigrateRolePreferences() {
    let (storage, store) = setup();
    let legacy = b"{\"character_card_theme_example_theme_mode\":\"dark\"}";
    storage.writeBytes("runtime/config/preferences/user_preferences.preferences.json", legacy).unwrap();
    store.initializeAppearance(r#"{"themeMode":"light"}"#).unwrap();
    let appearance: serde_json::Value = serde_json::from_str(&store.nodeState().unwrap().private_appearance_json.unwrap()).unwrap();
    assert_eq!(appearance["themeMode"], "system");
    assert_eq!(storage.readBytes("runtime/config/preferences/user_preferences.preferences.json").unwrap(), legacy);
    assert!(storage.files.lock().unwrap().keys().all(|path| !path.contains("migration")));
}

#[test]
fn partialEditsAreAtomicAndNodeOwned() {
    let (storage, store) = setup();
    store.updateAppearance(BTreeMap::from([("fontScale".into(), "1.2".into())])).unwrap();
    let before = store.nodeState().unwrap();
    assert!(store.updateAppearance(BTreeMap::from([
        ("themeMode".into(), "\"dark\"".into()), ("unknownField".into(), "true".into()),
    ])).is_err());
    assert_eq!(store.nodeState().unwrap(), before);
    assert!(storage.files.lock().unwrap().keys().all(|path| path == RUNTIME_THEME_NODE_SELECTION_PATH));
}

#[test]
fn selectedMissingPackageIsObservableAndNeverAutoReplaced() {
    let (storage, store) = setup();
    let selection = selectMissing(&storage);
    assert!(store.currentSelection().is_err());
    assert_eq!(store.nodeState().unwrap().selection, Some(selection));
    store.selectPersonalAppearance().unwrap();
    assert!(store.nodeState().unwrap().selection.is_none());
}

#[test]
fn packageVisualsAreImmutableWhileNodeSettingsRemainEditable() {
    let (storage, store) = setup();
    let selected = selectMissing(&storage);
    assert!(store.updateAppearance(BTreeMap::from([("useBackgroundImage".into(), "true".into())])).is_err());
    store.updateAppearance(BTreeMap::from([
        ("themeMode".into(), "\"dark\"".into()),
        ("bubbleShowAvatar".into(), "false".into()),
    ])).unwrap();
    let state = store.nodeState().unwrap();
    assert_eq!(state.selection, Some(selected));
    let appearance: serde_json::Value = serde_json::from_str(&state.private_appearance_json.unwrap()).unwrap();
    assert_eq!(appearance["useBackgroundImage"], false);
    assert_eq!(appearance["themeMode"], "dark");
    assert_eq!(appearance["bubbleShowAvatar"], false);
}

#[test]
fn invalidLibraryItemDoesNotHideAnIndependentlyWaitingItem() {
    let (storage, store) = setup();
    storage.writeBytes(&format!("{}/com.example.bad/{}/entry.json", RUNTIME_THEME_LIBRARY_DIR_PATH, "b".repeat(64)), b"{}").unwrap();
    storage.writeBytes(&format!("{}/com.example.waiting/{}/pending.json", RUNTIME_THEME_LIBRARY_DIR_PATH, "c".repeat(64)), b"{}").unwrap();
    let items = store.listLibrary().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].availability, ThemeLibraryAvailability::Invalid);
    assert_eq!(items[1].availability, ThemeLibraryAvailability::WaitingForSync);
    assert!(items[0].message.is_some());
}

#[test]
fn unfinishedTransactionIsNotReadableAsAnAppliedPackage() {
    let (storage, store) = setup();
    let digest = "d".repeat(64);
    storage.writeBytes(&format!("{}/com.example.pending/{digest}/pending.json", RUNTIME_THEME_LIBRARY_DIR_PATH), b"{}").unwrap();
    assert!(store.theme("com.example.pending", &digest).unwrap_err().contains("transaction is unfinished"));
    assert_eq!(store.listLibrary().unwrap()[0].availability, ThemeLibraryAvailability::WaitingForSync);
}

#[test]
fn deleteThemeRemovesUnselectedEntryAndRejectsSelectedEntry() {
    let (storage, store) = setup();
    let selected = selectMissing(&storage);
    storage.writeBytes(
        &format!("{}/{}/{}/entry.json", RUNTIME_THEME_LIBRARY_DIR_PATH, selected.theme_id, selected.archive_sha256),
        b"{}",
    ).unwrap();
    assert!(store.deleteTheme(&selected.theme_id, &selected.archive_sha256).is_err());

    let other_digest = "e".repeat(64);
    storage.writeBytes(
        &format!("{}/com.example.removable/{other_digest}/entry.json", RUNTIME_THEME_LIBRARY_DIR_PATH),
        b"{}",
    ).unwrap();
    storage.writeBytes(
        &format!("{}/com.example.removable/{other_digest}/package.optheme", RUNTIME_THEME_LIBRARY_DIR_PATH),
        b"zip",
    ).unwrap();
    assert_eq!(store.listLibrary().unwrap().len(), 2);
    store.deleteTheme("com.example.removable", &other_digest).unwrap();
    let remaining = store.listLibrary().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].theme_id, selected.theme_id);
}
