#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_theme_contract::{
    sha256_hex, ThemePackageTrust, ThemeValidationReceipt, ValidatedThemePackage,
    MAX_ARCHIVE_BYTES, MAX_EXPANDED_BYTES, MAX_FILES,
};
use operit_util::RuntimeStorageLayout::{
    RUNTIME_THEME_LIBRARY_DIR_PATH, RUNTIME_THEME_NODE_SELECTION_PATH,
};
use serde::{Deserialize, Serialize};

use crate::PreferencesDataStore::{
    stringPreferencesKey, CoreNodeStateStore, Flow, Preferences, PreferencesDataStoreError,
};
use crate::RuntimeFileSyncStore::RuntimeFileSyncStore;
use crate::RuntimeStorageHost::defaultRuntimeStorageHost;

const THEME_LIBRARY_SCHEMA_VERSION: u32 = 1;
const THEME_SELECTION_KEY: &str = "selection";
const THEME_PACKAGE_FILE: &str = "package.optheme";
const THEME_ENTRY_FILE: &str = "entry.json";
const THEME_PENDING_FILE: &str = "pending.json";
const MAX_THEME_LIBRARY_ENTRIES: usize = 512;
const PRIVATE_APPEARANCE_KEY: &str = "privateAppearanceJson";
const MAX_PRIVATE_APPEARANCE_BYTES: usize = 256 * 1024;

/// Raw node state stays readable even when the selected Space package is missing.
/// Availability is checked separately; an observer must never replace a broken selection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeNodeState {
    pub schema_version: u32,
    pub selection: Option<ThemeSelection>,
    pub private_appearance_json: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeLibraryAvailability {
    Ready,
    WaitingForSync,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeLibraryItem {
    pub theme_id: String,
    pub archive_sha256: String,
    pub entry: Option<ThemeLibraryEntry>,
    pub availability: ThemeLibraryAvailability,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeLibraryEntry {
    pub schema_version: u32,
    pub theme_id: String,
    pub version: String,
    pub display_name: String,
    pub author_name: String,
    pub license: String,
    pub platforms: Vec<String>,
    pub archive_sha256: String,
    pub archive_bytes: usize,
    pub expanded_bytes: usize,
    pub manifest_sha256: String,
    pub theme_sha256: String,
    pub package_path: String,
    pub trust: ThemePackageTrust,
    pub validation: ThemeValidationReceipt,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSelection {
    pub schema_version: u32,
    pub theme_id: String,
    pub version: String,
    pub archive_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeInstallStatus {
    Imported,
    AlreadyPresent,
    Applied,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeInstallReceipt {
    pub schema_version: u32,
    pub status: ThemeInstallStatus,
    pub entry: ThemeLibraryEntry,
    pub selection: Option<ThemeSelection>,
    pub selection_changed: bool,
    pub validation: ThemeValidationReceipt,
}

#[derive(Clone)]
pub struct ThemeLibraryStore {
    storage: Arc<dyn RuntimeStorageHost>,
    file_sync: RuntimeFileSyncStore,
}

impl ThemeLibraryStore {
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self {
            file_sync: RuntimeFileSyncStore::new(storage.clone(), "runtime/sync".to_string()),
            storage,
        }
    }

    pub fn native() -> Self {
        Self::new(defaultRuntimeStorageHost())
    }

    /// Imports a sealed, already L0-L4 validated package into Space storage.
    /// It never changes the CoreNode selection.
    #[allow(non_snake_case)]
    pub fn importValidated(
        &self,
        package: &ValidatedThemePackage,
    ) -> Result<ThemeInstallReceipt, String> {
        let entry = entryFromPackage(package)?;
        let root = packageRoot(&entry)?;
        let entry_path = format!("{root}/{THEME_ENTRY_FILE}");
        let package_path = format!("{root}/{THEME_PACKAGE_FILE}");
        let pending_path = format!("{root}/{THEME_PENDING_FILE}");

        if self.storage.exists(&pending_path).map_err(|error| error.to_string())? {
            return Err("theme library contains an unfinished import transaction".to_string());
        }

        if self
            .storage
            .exists(&entry_path)
            .map_err(|error| error.to_string())?
        {
            let stored = self.readEntryAt(&entry_path)?;
            let bytes = self.readPackageBytes(&stored)?;
            if stored != entry || bytes != package.sealed_archive_bytes() {
                return Err(
                    "theme library contains an entry with conflicting sealed bytes".to_string(),
                );
            }
            return self.receipt(
                ThemeInstallStatus::AlreadyPresent,
                stored,
                false,
            );
        }
        let commit = || -> Result<(), String> {
            let pending = serde_json::to_vec(&entry).map_err(|error| error.to_string())?;
            self.storage
                .writeBytes(&pending_path, &pending)
                .map_err(|error| error.to_string())?;
            self.file_sync
                .writeBytes(&package_path, package.sealed_archive_bytes())?;
            let entry_bytes = serde_json::to_vec(&entry).map_err(|error| error.to_string())?;
            self.file_sync.writeBytes(&entry_path, &entry_bytes)?;
            self.storage
                .delete(&pending_path, false)
                .map_err(|error| error.to_string())?;
            Ok(())
        };
        if let Err(error) = commit() {
            let mut rollback_errors = Vec::new();
            for path in [&entry_path, &package_path] {
                if let Err(rollback_error) = self.file_sync.delete(path) {
                    rollback_errors.push(format!("{path}: {rollback_error}"));
                }
            }
            // Do not remove the visibility barrier when rollback is incomplete.
            if rollback_errors.is_empty() {
                if let Err(rollback_error) = self.file_sync.delete(&pending_path) {
                    rollback_errors.push(format!("{pending_path}: {rollback_error}"));
                }
            }
            if !rollback_errors.is_empty() {
                return Err(format!("theme import failed: {error}; rollback errors: {}", rollback_errors.join("; ")));
            }
            return Err(error);
        }
        self.cleanupSupersededVersions(&entry.theme_id, &entry.version, &entry.archive_sha256);
        self.receipt(ThemeInstallStatus::Imported, entry, false)
    }

    /// Lists committed entries only. A package or pending transaction without
    /// entry.json is not a usable library record.
    #[allow(non_snake_case)]
    pub fn listThemes(&self) -> Result<Vec<ThemeLibraryEntry>, String> {
        let mut entries = self.listLibrary()?.into_iter().map(|item| {
            if item.availability != ThemeLibraryAvailability::Ready {
                return Err(format!("theme library item is not ready: {} {}", item.theme_id, item.archive_sha256));
            }
            item.entry.ok_or_else(|| "ready theme has no metadata".to_string())
        }).collect::<Result<Vec<_>, String>>()?;
        entries.sort_by(|left, right| {
            (
                left.theme_id.as_str(),
                left.version.as_str(),
                left.archive_sha256.as_str(),
            )
                .cmp(&(
                    right.theme_id.as_str(),
                    right.version.as_str(),
                    right.archive_sha256.as_str(),
                ))
        });
        Ok(entries)
    }

    #[allow(non_snake_case)]
    pub fn theme(
        &self,
        themeId: &str,
        archiveSha256: &str,
    ) -> Result<ThemeLibraryEntry, String> {
        validateThemeIdentity(themeId, archiveSha256)?;
        let path = format!(
            "{}/{}/{}/{THEME_ENTRY_FILE}",
            RUNTIME_THEME_LIBRARY_DIR_PATH, themeId, archiveSha256
        );
        let pending_path = format!(
            "{}/{}/{}/{THEME_PENDING_FILE}", RUNTIME_THEME_LIBRARY_DIR_PATH, themeId, archiveSha256,
        );
        if self.storage.exists(&pending_path).map_err(|error| error.to_string())? {
            return Err("theme package transaction is unfinished or its sync is incomplete".to_string());
        }
        self.readEntryAt(&path)
    }

    #[allow(non_snake_case)]
    pub fn packageBytes(&self, entry: &ThemeLibraryEntry) -> Result<Vec<u8>, String> {
        let stored = self.theme(&entry.theme_id, &entry.archive_sha256)?;
        if stored != *entry {
            return Err("theme library entry changed during package read".to_string());
        }
        self.readPackageBytes(&stored)
    }

    /// Applies only a library entry and writes only the local CoreNode choice.
    /// The package bytes remain Space-owned and are never copied into selection.
    #[allow(non_snake_case)]
    pub fn applyValidated(
        &self,
        package: &ValidatedThemePackage,
    ) -> Result<ThemeInstallReceipt, String> {
        let expected = entryFromPackage(package)?;
        let entry = self.theme(&expected.theme_id, &expected.archive_sha256)?;
        if entry != expected {
            return Err("theme library changed after full package validation".to_string());
        }
        let next = ThemeSelection {
            schema_version: THEME_LIBRARY_SCHEMA_VERSION,
            theme_id: entry.theme_id.clone(),
            version: entry.version.clone(),
            archive_sha256: entry.archive_sha256.clone(),
        };
        let selection_store = selectionStore(self.storage.clone());
        let value = serde_json::to_string(&next).map_err(|error| error.to_string())?;
        let changed = selection_store.try_edit_result(|preferences| {
            let previous = selectionFromPreferences(preferences)
                .map_err(PreferencesDataStoreError::Message)?;
            let changed = previous.as_ref() != Some(&next);
            if changed { preferences.set(&stringPreferencesKey(THEME_SELECTION_KEY), value); }
            Ok::<bool, PreferencesDataStoreError>(changed)
        }).map_err(|error| error.to_string())?;
        self.cleanupSupersededVersions(&entry.theme_id, &entry.version, &entry.archive_sha256);
        Ok(ThemeInstallReceipt {
            schema_version: THEME_LIBRARY_SCHEMA_VERSION,
            status: ThemeInstallStatus::Applied,
            validation: package.receipt().clone(),
            selection: Some(next),
            selection_changed: changed,
            entry,
        })
    }

    #[allow(non_snake_case)]
    pub fn currentSelection(&self) -> Result<Option<ThemeSelection>, String> {
        let selection = readSelection(&selectionStore(self.storage.clone()))?;
        let Some(selection) = selection else {
            return Ok(None);
        };
        let entry = self.theme(&selection.theme_id, &selection.archive_sha256)?;
        if entry.version != selection.version {
            return Err("theme selection version differs from its Space library entry".to_string());
        }
        let _ = self.readPackageBytes(&entry)?;
        Ok(Some(selection))
    }

    pub fn nodeState(&self) -> Result<ThemeNodeState, String> {
        nodeStateFromPreferences(
            &selectionStore(self.storage.clone()).data().map_err(|error| error.to_string())?,
        )
    }

    /// All engines observe the same node-local store; no UI polling or prompt subscription.
    pub fn nodeStateFlow(&self) -> Flow<ThemeNodeState> {
        selectionStore(self.storage.clone()).dataFlow().mapResult(|preferences| {
            nodeStateFromPreferences(&preferences).map_err(PreferencesDataStoreError::Message)
        })
    }

    /// Fresh internal-test state only. Old character/group keys are not read or migrated.
    pub fn initializeAppearance(&self, snapshotJson: &str) -> Result<ThemeNodeState, String> {
        checkPrivateSnapshot(snapshotJson)?;
        let defaults: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(snapshotJson).map_err(|error| error.to_string())?;
        let store = selectionStore(self.storage.clone());
        store.edit(|preferences| {
            let key = stringPreferencesKey(PRIVATE_APPEARANCE_KEY);
            match preferences.get(&key).cloned() {
                None => {
                    preferences.set(&key, snapshotJson.to_string());
                }
                Some(existing) => {
                    if let Ok(mut current) =
                        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&existing)
                    {
                        let mut changed = false;
                        for (field, default_value) in &defaults {
                            if !current.contains_key(field) {
                                current.insert(field.clone(), default_value.clone());
                                changed = true;
                            }
                        }
                        if changed {
                            if let Ok(serialized) = serde_json::to_string(&current) {
                                preferences.set(&key, serialized);
                            }
                        }
                    }
                }
            }
        }).map_err(|error| error.to_string())?;
        self.nodeState()
    }

    /// Atomic partial edits avoid replacing changes made by another Flutter engine.
    /// An immutable package allows only node/user settings, not visual edits.
    pub fn updateAppearance(
        &self,
        updates: std::collections::BTreeMap<String, String>,
    ) -> Result<ThemeNodeState, String> {
        let store = selectionStore(self.storage.clone());
        store.try_edit_result(|preferences| {
            let state = nodeStateFromPreferences(preferences)
                .map_err(PreferencesDataStoreError::Message)?;
            let source = state.private_appearance_json.ok_or_else(||
                PreferencesDataStoreError::Message("node appearance has not been initialized".to_string()))?;
            let mut snapshot: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&source)?;
            for (key, value) in &updates {
                if !snapshot.contains_key(key) {
                    return Err(PreferencesDataStoreError::Message(format!("unknown appearance field: {key}")));
                }
                if state.selection.is_some() && !isNodeThemeSetting(key) {
                    return Err(PreferencesDataStoreError::Message(
                        "package visuals are immutable; select personal appearance to edit them".to_string()));
                }
                snapshot.insert(key.clone(), serde_json::from_str(value)?);
            }
            let value = serde_json::to_string(&snapshot)?;
            checkPrivateSnapshot(&value).map_err(PreferencesDataStoreError::Message)?;
            preferences.set(&stringPreferencesKey(PRIVATE_APPEARANCE_KEY), value);
            Ok(())
        }).map_err(|error| error.to_string())?;
        self.nodeState()
    }

    /// Explicit user action, never invoked in response to a missing package.
    pub fn selectPersonalAppearance(&self) -> Result<ThemeNodeState, String> {
        let store = selectionStore(self.storage.clone());
        store.try_edit_result(|preferences| {
            if preferences.get(&stringPreferencesKey(PRIVATE_APPEARANCE_KEY)).is_none() {
                return Err(PreferencesDataStoreError::Message("node appearance has not been initialized".to_string()));
            }
            preferences.remove(&stringPreferencesKey(THEME_SELECTION_KEY));
            Ok(())
        }).map_err(|error| error.to_string())?;
        self.nodeState()
    }

    /// Removes a package entry from the Space theme library and records sync tombstones.
    /// Rejects deleting the package currently selected on this CoreNode.
    #[allow(non_snake_case)]
    pub fn deleteTheme(&self, themeId: &str, archiveSha256: &str) -> Result<(), String> {
        validateThemeIdentity(themeId, archiveSha256)?;
        let active_selection = readSelection(&selectionStore(self.storage.clone()))?;
        if active_selection
            .as_ref()
            .is_some_and(|sel| sel.theme_id == themeId && sel.archive_sha256 == archiveSha256)
        {
            return Err(
                "cannot delete the theme currently selected on this node; switch to personal appearance first"
                    .to_string(),
            );
        }
        let theme_dir = format!("{RUNTIME_THEME_LIBRARY_DIR_PATH}/{themeId}");
        let archive_dir = format!("{theme_dir}/{archiveSha256}");
        let entry_path = format!("{archive_dir}/{THEME_ENTRY_FILE}");
        let package_path = format!("{archive_dir}/{THEME_PACKAGE_FILE}");
        let pending_path = format!("{archive_dir}/{THEME_PENDING_FILE}");
        let dir_exists = self.storage.exists(&archive_dir).map_err(|error| error.to_string())?;
        let entry_exists = self.storage.exists(&entry_path).map_err(|error| error.to_string())?;
        let package_exists = self.storage.exists(&package_path).map_err(|error| error.to_string())?;
        let pending_exists = self.storage.exists(&pending_path).map_err(|error| error.to_string())?;
        if !dir_exists && !entry_exists && !package_exists && !pending_exists {
            return Err(format!(
                "theme library entry does not exist: {themeId} {archiveSha256}"
            ));
        }
        if entry_exists {
            self.file_sync.delete(&entry_path)?;
        }
        if package_exists {
            self.file_sync.delete(&package_path)?;
        }
        if pending_exists {
            self.storage
                .delete(&pending_path, false)
                .map_err(|error| error.to_string())?;
        }
        if self.storage.exists(&archive_dir).map_err(|error| error.to_string())? {
            self.storage
                .delete(&archive_dir, true)
                .map_err(|error| error.to_string())?;
        }
        if self.storage.exists(&theme_dir).map_err(|error| error.to_string())? {
            if let Ok(remaining) = self.storage.list(&theme_dir) {
                if remaining.is_empty() {
                    let _ = self.storage.delete(&theme_dir, true);
                }
            }
        }
        Ok(())
    }

    /// A damaged item is reported independently; it is never treated as ready.
    /// Metadata may arrive before the package blob during Space synchronization.
    pub fn listLibrary(&self) -> Result<Vec<ThemeLibraryItem>, String> {
        let mut items = Vec::new();
        let themes = self.storage.list(RUNTIME_THEME_LIBRARY_DIR_PATH).map_err(|error| error.to_string())?;
        if themes.len() > MAX_THEME_LIBRARY_ENTRIES {
            return Err("theme library directory budget exceeded".to_string());
        }
        for theme in themes {
            if !theme.isDirectory { continue; }
            let theme_id = directSegment(RUNTIME_THEME_LIBRARY_DIR_PATH, &theme.path)?;
            let archives = self.storage.list(&theme.path).map_err(|error| error.to_string())?;
            if archives.len() + items.len() > MAX_THEME_LIBRARY_ENTRIES {
                return Err("theme library directory budget exceeded".to_string());
            }
            for archive in archives {
                if !archive.isDirectory { continue; }
                if items.len() >= MAX_THEME_LIBRARY_ENTRIES {
                    return Err("theme library entry budget exceeded".to_string());
                }
                let digest = directSegment(&theme.path, &archive.path)?;
                if validateThemeIdentity(theme_id, digest).is_ok() {
                    if let Ok(children) = self.storage.list(&archive.path) {
                        if children.is_empty() {
                            let _ = self.storage.delete(&archive.path, true);
                            continue;
                        }
                    }
                }
                let mut item = ThemeLibraryItem {
                    theme_id: theme_id.to_string(), archive_sha256: digest.to_string(),
                    entry: None, availability: ThemeLibraryAvailability::Invalid, message: None,
                };
                let inspect = || -> Result<(Option<ThemeLibraryEntry>, ThemeLibraryAvailability), String> {
                    validateThemeIdentity(theme_id, digest)?;
                    let children = self.storage.list(&archive.path).map_err(|error| error.to_string())?;
                    let path = format!("{}/{THEME_ENTRY_FILE}", archive.path);
                    let pending_path = format!("{}/{THEME_PENDING_FILE}", archive.path);
                    let has_entry = children.iter().any(|child| !child.isDirectory && child.path == path);
                    let has_pending = children.iter().any(|child| child.path == pending_path);
                    if !has_entry {
                        return Ok((None, ThemeLibraryAvailability::WaitingForSync));
                    }
                    let entry = self.readEntryAt(&path)?;
                    if has_pending {
                        return Ok((Some(entry), ThemeLibraryAvailability::WaitingForSync));
                    }
                    let Some(pkg_file) = children.iter().find(|child| !child.isDirectory && child.path == entry.package_path) else {
                        return Ok((Some(entry), ThemeLibraryAvailability::WaitingForSync));
                    };
                    if pkg_file.size >= 0 && pkg_file.size as usize != entry.archive_bytes {
                        return Err("theme package bytes differ from the library entry".to_string());
                    }
                    Ok((Some(entry), ThemeLibraryAvailability::Ready))
                };
                match inspect() {
                    Ok((entry, availability)) => { item.entry = entry; item.availability = availability; }
                    Err(error) => item.message = Some(error),
                }
                items.push(item);
            }
        }
        items.sort_by(|left, right| (&left.theme_id, &left.archive_sha256).cmp(&(&right.theme_id, &right.archive_sha256)));
        Ok(items)
    }

    fn receipt(
        &self,
        status: ThemeInstallStatus,
        entry: ThemeLibraryEntry,
        selection_changed: bool,
    ) -> Result<ThemeInstallReceipt, String> {
        Ok(ThemeInstallReceipt {
            schema_version: THEME_LIBRARY_SCHEMA_VERSION,
            status,
            validation: entry.validation.clone(),
            // An import receipt describes only the import. A broken active
            // package must not turn a committed import into an apparent failure.
            selection: None,
            selection_changed,
            entry,
        })
    }

    fn readEntryAt(&self, path: &str) -> Result<ThemeLibraryEntry, String> {
        let bytes = self.storage.readBytesRange(path, 0, MAX_PRIVATE_APPEARANCE_BYTES + 1)
            .map_err(|error| error.to_string())?;
        if bytes.len() > MAX_PRIVATE_APPEARANCE_BYTES {
            return Err("theme library metadata byte budget exceeded".to_string());
        }
        let entry: ThemeLibraryEntry =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        validateEntry(&entry)?;
        if path != format!("{}/{THEME_ENTRY_FILE}", packageRoot(&entry)?) {
            return Err("theme entry identity does not match its registered directory".to_string());
        }
        Ok(entry)
    }

    fn cleanupSupersededVersions(&self, themeId: &str, version: &str, keepArchiveSha256: &str) {
        let active_sha = readSelection(&selectionStore(self.storage.clone()))
            .ok()
            .flatten()
            .filter(|sel| sel.theme_id == themeId)
            .map(|sel| sel.archive_sha256);
        let theme_dir = format!("{RUNTIME_THEME_LIBRARY_DIR_PATH}/{themeId}");
        let Ok(archives) = self.storage.list(&theme_dir) else {
            return;
        };
        for archive in archives {
            if !archive.isDirectory {
                continue;
            }
            let Ok(digest) = directSegment(&theme_dir, &archive.path) else {
                continue;
            };
            if digest == keepArchiveSha256 || active_sha.as_deref() == Some(digest) {
                continue;
            }
            let entry_path = format!("{}/{THEME_ENTRY_FILE}", archive.path);
            let Ok(stored) = self.readEntryAt(&entry_path) else {
                continue;
            };
            if stored.version == version {
                let _ = self.file_sync.delete(&entry_path);
                let _ = self.file_sync.delete(&stored.package_path);
                let _ = self.storage.delete(&archive.path, true);
            }
        }
    }

    fn readPackageBytes(&self, entry: &ThemeLibraryEntry) -> Result<Vec<u8>, String> {
        let bytes = self
            .storage
            .readBytesRange(&entry.package_path, 0, entry.archive_bytes + 1)
            .map_err(|error| error.to_string())?;
        if bytes.len() != entry.archive_bytes || sha256_hex(&bytes) != entry.archive_sha256 {
            return Err("theme package bytes differ from the library entry".to_string());
        }
        Ok(bytes)
    }


}

fn entryFromPackage(package: &ValidatedThemePackage) -> Result<ThemeLibraryEntry, String> {
    let manifest = package.manifest();
    validateThemeIdentity(&manifest.theme_id, package.archive_sha256())?;
    let root = format!(
        "{}/{}/{}",
        RUNTIME_THEME_LIBRARY_DIR_PATH,
        manifest.theme_id,
        package.archive_sha256()
    );
    Ok(ThemeLibraryEntry {
        schema_version: THEME_LIBRARY_SCHEMA_VERSION,
        theme_id: manifest.theme_id.clone(),
        version: manifest.version.clone(),
        display_name: manifest.display_name.clone(),
        author_name: manifest.author.name.clone(),
        license: manifest.license.clone(),
        platforms: manifest.platforms.clone(),
        archive_sha256: package.archive_sha256().to_string(),
        archive_bytes: package.archive_bytes(),
        expanded_bytes: package.expanded_bytes(),
        manifest_sha256: package.receipt().manifest_sha256.clone(),
        theme_sha256: package.receipt().theme_sha256.clone(),
        package_path: format!("{root}/{THEME_PACKAGE_FILE}"),
        trust: package.receipt().trust,
        validation: package.receipt().clone(),
    })
}

fn validateEntry(entry: &ThemeLibraryEntry) -> Result<(), String> {
    if entry.schema_version != THEME_LIBRARY_SCHEMA_VERSION {
        return Err("unsupported theme library schema".to_string());
    }
    validateThemeIdentity(&entry.theme_id, &entry.archive_sha256)?;
    let root = packageRoot(entry)?;
    if entry.package_path != format!("{root}/{THEME_PACKAGE_FILE}")
        || entry.archive_bytes == 0
        || entry.archive_bytes > MAX_ARCHIVE_BYTES
        || entry.expanded_bytes > MAX_EXPANDED_BYTES
        || entry.validation.validator_version != 1
        || entry.validation.files.len() > MAX_FILES
        || entry.expanded_bytes != entry.validation.expanded_bytes
        || entry.validation.archive_sha256 != entry.archive_sha256
        || entry.validation.archive_bytes != entry.archive_bytes
        || entry.validation.manifest_sha256 != entry.manifest_sha256
        || entry.validation.theme_sha256 != entry.theme_sha256
        || entry.validation.trust != entry.trust
    {
        return Err("theme library entry does not match its validation receipt".to_string());
    }
    Ok(())
}

fn packageRoot(entry: &ThemeLibraryEntry) -> Result<String, String> {
    validateThemeIdentity(&entry.theme_id, &entry.archive_sha256)?;
    Ok(format!(
        "{}/{}/{}",
        RUNTIME_THEME_LIBRARY_DIR_PATH, entry.theme_id, entry.archive_sha256
    ))
}

fn validateThemeIdentity(themeId: &str, archiveSha256: &str) -> Result<(), String> {
    if themeId.is_empty()
        || themeId.len() > 128
        || !themeId.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'-'
        })
        || themeId.contains("..")
        || themeId.starts_with('.')
        || themeId.ends_with('.')
    {
        return Err("theme id is not a registered storage segment".to_string());
    }
    if archiveSha256.len() != 64
        || !archiveSha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("theme archive SHA-256 is invalid".to_string());
    }
    Ok(())
}

fn selectionStore(storage: Arc<dyn RuntimeStorageHost>) -> CoreNodeStateStore {
    CoreNodeStateStore::newWithStorage(storage, RUNTIME_THEME_NODE_SELECTION_PATH)
}

fn readSelection(store: &CoreNodeStateStore) -> Result<Option<ThemeSelection>, String> {
    let preferences = store.data().map_err(|error| error.to_string())?;
    selectionFromPreferences(&preferences)
}

fn selectionFromPreferences(preferences: &Preferences) -> Result<Option<ThemeSelection>, String> {
    let Some(value) = preferences.get(&stringPreferencesKey(THEME_SELECTION_KEY)) else {
        return Ok(None);
    };
    let selection: ThemeSelection =
        serde_json::from_str(value).map_err(|error| error.to_string())?;
    if selection.schema_version != THEME_LIBRARY_SCHEMA_VERSION {
        return Err("unsupported theme selection schema".to_string());
    }
    validateThemeIdentity(&selection.theme_id, &selection.archive_sha256)?;
    Ok(Some(selection))
}

fn nodeStateFromPreferences(preferences: &Preferences) -> Result<ThemeNodeState, String> {
    let appearance = preferences.get(&stringPreferencesKey(PRIVATE_APPEARANCE_KEY)).cloned();
    if let Some(value) = &appearance { checkPrivateSnapshot(value)?; }
    Ok(ThemeNodeState {
        schema_version: THEME_LIBRARY_SCHEMA_VERSION,
        selection: selectionFromPreferences(preferences)?,
        private_appearance_json: appearance,
    })
}

fn checkPrivateSnapshot(value: &str) -> Result<(), String> {
    if value.len() > MAX_PRIVATE_APPEARANCE_BYTES {
        return Err("node appearance byte budget exceeded".to_string());
    }
    let _: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(value).map_err(|error| error.to_string())?;
    Ok(())
}

fn isNodeThemeSetting(key: &str) -> bool {
    matches!(
        key,
        "themeMode"
            | "fontScale"
            | "customUserAvatarUri"
            | "customAiAvatarUri"
            | "bubbleShowAvatar"
            | "avatarShape"
            | "avatarCornerRadius"
            | "chatStyle"
            | "bubbleWideLayoutEnabled"
            | "inputStyle"
            | "chatInputFloating"
            | "inputBackgroundColor"
            | "inputRoundedCornersEnabled"
            | "inputUseImage"
            | "inputImageUri"
            | "inputImageRenderMode"
            | "inputImageCropLeft"
            | "inputImageCropTop"
            | "inputImageCropRight"
            | "inputImageCropBottom"
            | "inputImageRepeatStart"
            | "inputImageRepeatEnd"
            | "inputImageRepeatYStart"
            | "inputImageRepeatYEnd"
            | "inputImageScale"
            | "inputSettingsAsset"
            | "inputAttachmentAsset"
            | "inputMicAsset"
            | "inputSendAsset"
            | "inputStopAsset"
            | "inputSettingsAssetSize"
            | "inputAttachmentAssetSize"
            | "inputMicAssetSize"
            | "inputSendAssetSize"
            | "inputStopAssetSize"
            | "transparentSurfaceEnabled"
            | "backgroundImageOpacity"
            | "useBackgroundBlur"
            | "backgroundBlurRadius"
            | "videoBackgroundMuted"
            | "videoBackgroundLoop"
            | "bubbleUserRoundedCornersEnabled"
            | "bubbleAiRoundedCornersEnabled"
            | "bubbleUserContentPaddingLeft"
            | "bubbleUserContentPaddingRight"
            | "bubbleAiContentPaddingLeft"
            | "bubbleAiContentPaddingRight"
            | "cursorUserBubbleColor"
            | "bubbleUserBubbleColor"
            | "bubbleAiBubbleColor"
            | "bubbleUserTextColor"
            | "bubbleAiTextColor"
            | "bubbleUserImageRenderMode"
            | "bubbleUserImageCropLeft"
            | "bubbleUserImageCropTop"
            | "bubbleUserImageCropRight"
            | "bubbleUserImageCropBottom"
            | "bubbleUserImageRepeatStart"
            | "bubbleUserImageRepeatEnd"
            | "bubbleUserImageRepeatYStart"
            | "bubbleUserImageRepeatYEnd"
            | "bubbleUserImageScale"
            | "bubbleAiImageRenderMode"
            | "bubbleAiImageCropLeft"
            | "bubbleAiImageCropTop"
            | "bubbleAiImageCropRight"
            | "bubbleAiImageCropBottom"
            | "bubbleAiImageRepeatStart"
            | "bubbleAiImageRepeatEnd"
            | "bubbleAiImageRepeatYStart"
            | "bubbleAiImageRepeatYEnd"
            | "bubbleAiImageScale"
            | "showThinkingProcess"
            | "showModelProvider"
            | "showModelName"
            | "showRoleName"
            | "showUserName"
            | "showMessageTokenStats"
            | "showMessageTimingStats"
            | "showMessageTimestamp"
            | "showInputProcessingStatus"
    )
}

fn directSegment<'a>(parent: &str, path: &'a str) -> Result<&'a str, String> {
    let segment = path.strip_prefix(&format!("{parent}/"))
        .ok_or_else(|| "theme listing escaped its registered parent".to_string())?;
    if segment.is_empty() || segment.contains('/') || segment.contains('\\') {
        return Err("theme listing is not a direct registered child".to_string());
    }
    Ok(segment)
}

#[cfg(test)]
#[path = "tests/ThemeLibraryStoreTests.rs"]
mod tests;
