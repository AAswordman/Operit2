//! One removable migration boundary for Core v27 and Operit1 character and memory data.
//! Domain records use the same typed Host SQL tables as the plugin.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::CharacterPluginLegacyData::*;
use crate::data::preferences::ThemeConfigManager::{ThemeConfig, ThemeConfigManager};
use crate::data::preferences::ThemePreferenceSnapshot::readLegacyCharacterThemeSnapshot;
use operit_host_api::PluginStorage::{
    StorageKind, StorageMutation, StorageRequest, StorageResponse,
};
use operit_host_api::{RuntimeSqliteHost, RuntimeStorageHost};
use operit_store::PluginStorage::PluginStorageSession;
use operit_store::PreferencesDataStore::{stringPreferencesKey, Preferences, PreferencesDataStore};
use operit_store::RuntimeStorageHost::{
    defaultRuntimeSqliteHost, defaultRuntimeStorageHost, runtimeStoragePath,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::SqliteStore::{SqliteRowGet, SqliteStore, SqliteValue};
use operit_store::SyncOperationStore::SyncOperationStore;
use operit_util::AppLogger::AppLogger;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH;
use operit_util::RuntimeStorageLayout::{
    API_PREFERENCES_PATH, CHARACTER_CARDS_PREFERENCES_PATH, CHARACTER_GROUPS_PREFERENCES_PATH,
    PROMPT_TAGS_PREFERENCES_PATH, SHARED_MEMORY_STORES_PREFERENCES_PATH,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PLUGIN_ID: &str = "com.operit.character_cards";
pub const LEGACY_BINDING_NAMESPACE: &str = "com.operit.character_cards.migration";
const META_COLLECTION: &str = "meta";

/// Carries actual source records and documents without recreating the removed Core managers.
#[derive(Default)]
pub struct CharacterMigrationData {
    pub cards: Vec<CharacterCard>,
    pub groups: Vec<CharacterGroupCard>,
    pub tags: Vec<PromptTag>,
    pub stores: Vec<SharedMemoryStore>,
    pub owners: Vec<Value>,
    pub records: Vec<(String, String, Value)>,
    pub documents: BTreeMap<String, Vec<u8>>,
    pub resources: BTreeMap<String, Vec<u8>>,
    pub active: Option<Value>,
    pub memory_preferences: Option<Value>,
    pub cleanup_paths: Vec<String>,
}

/// Uses the production Host storage dispatcher for bounded and version-checked migration writes.
pub struct CharacterPluginWriter {
    relational: super::CharacterRelationalStorage::CharacterRelationalStorage,
    session: PluginStorageSession,
    sqlite: Arc<dyn RuntimeSqliteHost>,
    storage: Arc<dyn RuntimeStorageHost>,
    device_id: String,
    handle: String,
    pub directory: String,
}

impl CharacterPluginWriter {
    /// Opens the stable Space namespace independently of the package installation scope.
    pub fn open(
        sqlite: Arc<dyn RuntimeSqliteHost>,
        storage: Arc<dyn RuntimeStorageHost>,
    ) -> Result<Self, String> {
        let directory = operit_store::ExtensionStore::ExtensionStore::spaceDataPath(PLUGIN_ID)?;
        let device_id = SyncOperationStore::new(storage.clone(), RUNTIME_SYNC_DIR_PATH)
            .localDeviceId()
            .map_err(|error| error.to_string())?;
        let session = PluginStorageSession::default();
        let response = session
            .request(
                sqlite.clone(),
                storage.clone(),
                StorageRequest::Open {
                    path: format!("{directory}/characters.sqlite"),
                    kind: StorageKind::Objects,
                },
                &device_id,
            )
            .map_err(|error| error.to_string())?;
        let StorageResponse::Open(response) = response else {
            return Err("Storage open returned a different operation result".to_string());
        };
        let handle = response.handle;
        let relational = super::CharacterRelationalStorage::CharacterRelationalStorage::open(
            sqlite.clone(),
            storage.clone(),
            device_id.clone(),
            &directory,
        )?;
        let result = Self {
            relational,
            session,
            sqlite,
            storage,
            device_id,
            handle,
            directory,
        };
        result.adoptObjectRecords()?;
        Ok(result)
    }

    /// Dispatches one request and propagates the original Host error.
    fn request(&self, request: StorageRequest) -> Result<Value, String> {
        self.session
            .request(
                self.sqlite.clone(),
                self.storage.clone(),
                request,
                &self.device_id,
            )
            .map_err(|error| error.to_string())
            .and_then(|response| serde_json::to_value(response).map_err(|error| error.to_string()))
    }

    /// Reads an exact object entry, preserving the distinction between absence and a stored value.
    pub fn get(&self, collection: &str, key: &str) -> Result<Option<Value>, String> {
        if collection != "imports" {
            return self.relational.get(collection, key);
        }
        self.legacyGet(collection, key)
    }

    /// Reads only the explicitly selected legacy source or migration provenance.
    fn legacyGet(&self, collection: &str, key: &str) -> Result<Option<Value>, String> {
        let entry = self.request(StorageRequest::Get {
            handle: self.handle.clone(),
            collection: collection.to_string(),
            key: key.to_string(),
        })?;
        Ok((!entry.is_null()).then_some(entry))
    }

    /// Enumerates bounded pages without reading or rewriting whole collection files.
    pub fn list(&self, collection: &str) -> Result<Vec<Value>, String> {
        if collection != "imports" {
            return self.relational.list(collection);
        }
        self.legacyList(collection)
    }

    /// Reads the released objects source during its one-time schema adoption.
    fn legacyList(&self, collection: &str) -> Result<Vec<Value>, String> {
        let mut result = Vec::new();
        let mut after = None;
        loop {
            let page = self.request(StorageRequest::List {
                handle: self.handle.clone(),
                collection: collection.to_string(),
                after: after.clone(),
                limit: 1000,
            })?;
            let rows = page
                .as_array()
                .ok_or("Storage list did not return an array")?;
            if let Some(last) = rows.last() {
                after = Some(requiredString(last, "key")?.to_string());
            }
            result.extend(rows.iter().cloned());
            if rows.len() < 1000 {
                return Ok(result);
            }
        }
    }

    /// Creates one exact-version mutation rather than overwriting a newer object blindly.
    fn mutation(
        &self,
        collection: &str,
        key: &str,
        value: Value,
    ) -> Result<StorageMutation, String> {
        let expected_version = self
            .get(collection, key)?
            .as_ref()
            .map(|entry| requiredString(entry, "version").map(ToString::to_string))
            .transpose()?;
        Ok(StorageMutation {
            collection: collection.to_string(),
            key: key.to_string(),
            value,
            deleted: false,
            check_version: true,
            expected_version,
        })
    }

    /// Commits one bounded migration transaction through the plugin storage journal.
    fn commit(&self, mutations: Vec<StorageMutation>) -> Result<(), String> {
        for mutation in mutations {
            if mutation.collection == "imports" {
                self.request(StorageRequest::Commit {
                    handle: self.handle.clone(),
                    mutations: vec![mutation],
                })?;
            } else {
                if mutation.deleted {
                    return Err("Migration cannot delete a relational business record".into());
                }
                self.relational.put(&[(
                    mutation.collection,
                    mutation.key,
                    mutation.value,
                    mutation.expected_version,
                )])?;
            }
        }
        Ok(())
    }

    /// Adopts released v28 object records once, verifies every value, and retains source provenance.
    fn adoptObjectRecords(&self) -> Result<(), String> {
        const MARKER: &str = "objects-to-relational-v1";
        if self.legacyGet("imports", MARKER)?.is_some() {
            return self.removeAdoptedObjectRecords();
        }
        for collection in [
            "cards",
            "groups",
            "tags",
            "stores",
            "conversationGroups",
            "owners",
            "memories",
            "links",
            "chunks",
            "candidates",
            "embeddings",
            "meta",
        ] {
            for entry in self.legacyList(collection)? {
                let key = requiredString(&entry, "key")?;
                let expected = self
                    .relational
                    .comparisonValue(collection, &entry["value"])?;
                let existing = self.relational.get(collection, key)?;
                match existing {
                    Some(existing) => {
                        if existing["value"] != expected {
                            return Err(format!(
                                "Interrupted relational adoption differs at {collection}/{key}"
                            ));
                        }
                    }
                    None => self.relational.put(&[(
                        collection.into(),
                        key.into(),
                        entry["value"].clone(),
                        None,
                    )])?,
                }
                let checked = self
                    .relational
                    .get(collection, key)?
                    .ok_or("Adopted relational record missing")?;
                if checked["value"] != expected {
                    return Err(format!(
                        "Relational adoption verification failed at {collection}/{key}"
                    ));
                }
            }
        }
        self.request(StorageRequest::Commit {
            handle: self.handle.clone(),
            mutations: vec![StorageMutation {
                collection: "imports".into(),
                key: MARKER.into(),
                value: json!({"complete":true}),
                deleted: false,
                check_version: true,
                expected_version: None,
            }],
        })?;
        self.removeAdoptedObjectRecords()
    }

    /// Removes verified business sources while retaining migration provenance and exact source versions.
    fn removeAdoptedObjectRecords(&self) -> Result<(), String> {
        for collection in [
            "cards",
            "groups",
            "tags",
            "stores",
            "conversationGroups",
            "owners",
            "memories",
            "links",
            "chunks",
            "candidates",
            "embeddings",
            "meta",
        ] {
            let records = self.legacyList(collection)?;
            for page in records.chunks(1000) {
                let mutations = page
                    .iter()
                    .map(|entry| {
                        Ok(StorageMutation {
                            collection: collection.into(),
                            key: requiredString(entry, "key")?.into(),
                            value: Value::Null,
                            deleted: true,
                            check_version: true,
                            expected_version: Some(requiredString(entry, "version")?.into()),
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                self.request(StorageRequest::Commit {
                    handle: self.handle.clone(),
                    mutations,
                })?;
            }
        }
        Ok(())
    }

    /// Persists one idempotent binding or historical snapshot through the shared typed writer.
    pub fn adoptRecord(&self, collection: &str, key: &str, value: Value) -> Result<(), String> {
        let expected = self.relational.comparisonValue(collection, &value)?;
        let existing = self.relational.get(collection, key)?;
        if let Some(existing) = existing {
            if existing["value"] != expected {
                return Err(format!("Migration would replace {collection}/{key}"));
            }
            return Ok(());
        }
        self.relational
            .put(&[(collection.into(), key.into(), value.clone(), None)])?;
        if self
            .relational
            .get(collection, key)?
            .ok_or("Migrated record missing")?["value"]
            != expected
        {
            return Err(format!(
                "Migrated relational record verification failed: {collection}/{key}"
            ));
        }
        Ok(())
    }

    /// Writes an explicit migration with a durable in-progress marker and a final completion marker.
    /// Restarting an interrupted migration repeats source records; a completed migration never runs twice.
    pub fn import(&self, source: &str, mut data: CharacterMigrationData) -> Result<(), String> {
        if self.get("imports", source)?.is_some() {
            return Ok(());
        }
        let dropped = dropUnresolvableRecords(&mut data);
        if dropped != DroppedLegacyRecords::default() {
            AppLogger::w(
                "CharacterPluginMigration",
                &format!(
                    "source={source} dropped {} relationships and {} document chunks whose legacy owner was deleted without a reference cleanup",
                    dropped.relationships, dropped.documentChunks
                ),
            );
        }
        validateMigrationData(&data)?;
        let theme_targets = data
            .cards
            .iter()
            .map(|card| ("cards".to_string(), card.id.clone()))
            .chain(
                data.groups
                    .iter()
                    .map(|group| ("groups".to_string(), group.id.clone())),
            )
            .collect::<Vec<_>>();
        let existing = self.get(META_COLLECTION, "state")?;
        let mut metadata = match existing {
            Some(entry) => entry["value"].clone(),
            None => json!({"version":3,"nextId":"1","active":null}),
        };
        if metadata["version"] != 3 {
            return Err("Unsupported character database metadata".to_string());
        }
        if metadata["migrationStatus"] == "writing" && metadata["migrationSource"] != source {
            return Err("Another character migration is incomplete".to_string());
        }
        if let Some(preferences) = data.memory_preferences {
            validateGlobalMemoryPreferences(&preferences)?;
            metadata["memoryPreferences"] = preferences;
        }
        validateGlobalMemoryPreferences(&metadata["memoryPreferences"])?;
        if let Some(active) = data.active {
            metadata["active"] = active;
        }
        let mut rows = data.records;
        let imported_ids = data
            .cards
            .iter()
            .map(|card| card.id.clone())
            .collect::<BTreeSet<_>>();
        if data.cards.iter().filter(|card| card.isDefault).count() != 1 {
            return Err("Imported characters must have exactly one default".to_string());
        }
        for entry in self.list("cards")? {
            let mut card = entry["value"].clone();
            let id = requiredString(&card, "id")?.to_string();
            if card["isDefault"] == true && !imported_ids.contains(&id) {
                card["isDefault"] = json!(false);
                rows.push(("cards".to_string(), id, card));
            }
        }
        for card in data.cards {
            let mut value = serde_json::to_value(&card).map_err(|error| error.to_string())?;
            value["themeConfigId"] = Value::Null;
            rows.push(("cards".to_string(), card.id, value));
        }
        for group in data.groups {
            let mut value = serde_json::to_value(&group).map_err(|error| error.to_string())?;
            value["themeConfigId"] = Value::Null;
            rows.push(("groups".to_string(), group.id, value));
        }
        for tag in data.tags {
            rows.push((
                "tags".to_string(),
                tag.id.clone(),
                serde_json::to_value(tag).map_err(|error| error.to_string())?,
            ));
        }
        for store in data.stores {
            rows.push((
                "stores".to_string(),
                store.id.clone(),
                serde_json::to_value(store).map_err(|error| error.to_string())?,
            ));
        }
        for owner in data.owners {
            rows.push((
                "owners".to_string(),
                requiredString(&owner, "ownerKey")?.to_string(),
                owner,
            ));
        }
        let mut next_id = requiredString(&metadata, "nextId")?
            .parse::<i64>()
            .map_err(|error| error.to_string())?;
        let mut identities = BTreeSet::new();
        for (collection, key, value) in &rows {
            if !identities.insert((collection, key)) {
                return Err(format!("Duplicate imported record: {collection}/{key}"));
            }
            let id = if value.get("record").is_some() {
                &value["record"]["id"]
            } else {
                &value["id"]
            };
            if let Some(id) = id.as_str() {
                if id.bytes().all(|byte| byte.is_ascii_digit()) && !id.is_empty() {
                    let id = id.parse::<i64>().map_err(|error| error.to_string())?;
                    next_id = next_id.max(
                        id.checked_add(1)
                            .ok_or("Character identity space exhausted")?,
                    );
                }
            }
        }
        if metadata["active"].is_null() {
            return Err("Character migration has no active selection".to_string());
        }
        metadata["nextId"] = json!(next_id.to_string());
        metadata["migrationStatus"] = json!("writing");
        metadata["migrationSource"] = json!(source);
        self.commit(vec![self.mutation(
            META_COLLECTION,
            "state",
            metadata.clone(),
        )?])?;
        for (relative, bytes) in data.resources {
            let target = format!("{}/{}", self.directory, relative);
            operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore::new(
                self.storage.clone(),
                RUNTIME_SYNC_DIR_PATH,
            )
            .writeBytes(&target, &bytes)?;
            if self
                .storage
                .readBytes(&target)
                .map_err(|error| error.to_string())?
                != bytes
            {
                return Err(format!("Migrated resource verification failed: {target}"));
            }
        }
        for (owner, bytes) in data.documents {
            let target = format!("{}/{}", self.directory, ownerDocumentPath(&owner));
            operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore::new(
                self.storage.clone(),
                RUNTIME_SYNC_DIR_PATH,
            )
            .writeBytes(&target, &bytes)?;
            if self
                .storage
                .readBytes(&target)
                .map_err(|error| error.to_string())?
                != bytes
            {
                return Err(format!("Migrated document verification failed: {owner}"));
            }
        }
        for page in rows.chunks(999) {
            let mut mutations = Vec::new();
            for (collection, key, value) in page {
                mutations.push(self.mutation(collection, key, value.clone())?);
            }
            mutations.push(self.mutation(META_COLLECTION, "state", metadata.clone())?);
            self.commit(mutations)?;
            for (collection, key, expected) in page {
                let entry = self
                    .get(collection, key)?
                    .ok_or("Migrated record is missing")?;
                if entry["value"] != self.relational.comparisonValue(collection, expected)? {
                    return Err(format!(
                        "Migrated record verification failed: {collection}/{key}"
                    ));
                }
            }
        }
        metadata["migrationStatus"] = json!("complete");
        self.commit(vec![
            self.mutation(META_COLLECTION, "state", metadata)?,
            self.mutation(
                "imports",
                source,
                json!({"complete":true,"cleanupPaths":data.cleanup_paths,"cleanupComplete":false,"themeTargets":theme_targets}),
            )?,
        ])
    }
}

impl Drop for CharacterPluginWriter {
    /// Releases the retained migration handle independently of its database's persistent lifetime.
    fn drop(&mut self) {
        let _ = self.request(StorageRequest::Close {
            handle: self.handle.clone(),
        });
    }
}

/// Converts a declared host or old runtime path without guessing platform-specific path syntax.
fn legacyResourceStoragePath(source: &str) -> Result<String, String> {
    for (public, storage) in [
        ("/app/data/", "runtime/"),
        ("/app/workspaces/", "workspaces/"),
    ] {
        if let Some(relative) = source.strip_prefix(public) {
            return Ok(format!("{storage}{relative}"));
        }
        if source.starts_with(storage) {
            return Ok(source.to_string());
        }
    }
    let paths = RuntimeStorePaths::default();
    let path = std::path::Path::new(source);
    let workspace_root = paths.workspace_dir();
    for (root, prefix) in [
        (paths.runtime_dir(), "runtime/"),
        (workspace_root.as_path(), "workspaces/"),
    ] {
        if let Ok(relative) = path.strip_prefix(root) {
            return Ok(format!(
                "{prefix}{}",
                relative.to_string_lossy().replace('\\', "/")
            ));
        }
    }
    Err(format!(
        "Legacy resource is outside declared runtime storage: {source}"
    ))
}

/// Converts a verified storage identity into the public Files API path used by plugin records.
pub(crate) fn publicResourcePath(path: &str) -> Result<String, String> {
    for (storage, public) in [
        ("runtime/", "/app/data"),
        ("workspaces/", "/app/workspaces"),
    ] {
        if let Some(relative) = path.strip_prefix(storage) {
            return operit_tools::files::PathMapper::PathMapper::joinVfsPath(public, relative);
        }
    }
    Err(format!("Resource is outside public runtime roots: {path}"))
}

/// Identifies resources under either legacy owner root, including references across owners.
fn legacyResourceRelativePath(path: &str) -> Option<&str> {
    use operit_util::RuntimeStorageLayout::{
        DATA_MEMORY_CHARACTERS_DIR_PATH, DATA_MEMORY_SHARED_DIR_PATH,
    };
    [DATA_MEMORY_CHARACTERS_DIR_PATH, DATA_MEMORY_SHARED_DIR_PATH]
        .iter()
        .find_map(|root| {
            path.strip_prefix(root)
                .and_then(|relative| relative.strip_prefix('/'))
        })
}

/// Rejects unsafe resource segments before constructing a plugin destination.
fn validateResourcePath(path: &str) -> Result<(), String> {
    if path.split('/').any(|part| {
        part.is_empty() || part == "." || part == ".." || part.contains('\\') || part.contains(':')
    }) {
        return Err(format!("Invalid legacy resource path: {path}"));
    }
    Ok(())
}

/// Counts the legacy records an import dropped because their owning record is absent from the same snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DroppedLegacyRecords {
    pub relationships: usize,
    pub documentChunks: usize,
}

/// Drops relationships and document chunks whose owning record is absent from the same snapshot.
/// The retired Core deleted memories without clearing their references, and the plugin storage rejects
/// any record with an unresolvable endpoint, so only resolvable records can be imported.
/// Records with malformed fields stay for validateMigrationData to report precisely.
pub fn dropUnresolvableRecords(data: &mut CharacterMigrationData) -> DroppedLegacyRecords {
    let mut dropped = DroppedLegacyRecords::default();
    let mut memories = BTreeSet::new();
    let mut documents = BTreeSet::new();
    for (collection, _, envelope) in &data.records {
        if collection != "memories" {
            continue;
        }
        let Ok(owner) = requiredString(envelope, "ownerKey") else {
            continue;
        };
        let record = &envelope["record"];
        if let Ok(id) = requiredString(record, "id") {
            memories.insert((owner.to_string(), id.to_string()));
        }
        if record["isDocumentNode"] == true {
            if let Ok(uuid) = requiredString(record, "uuid") {
                documents.insert((owner.to_string(), uuid.to_string()));
            }
        }
    }
    data.records.retain(|(collection, _, envelope)| {
        let Ok(owner) = requiredString(envelope, "ownerKey") else {
            return true;
        };
        let record = &envelope["record"];
        let endpoint = |field: &str| {
            requiredString(record, field).map(|value| (owner.to_string(), value.to_string()))
        };
        match collection.as_str() {
            "links" => {
                let (Ok(source), Ok(target)) =
                    (endpoint("sourceMemoryId"), endpoint("targetMemoryId"))
                else {
                    return true;
                };
                if memories.contains(&source) && memories.contains(&target) {
                    true
                } else {
                    dropped.relationships += 1;
                    false
                }
            }
            "chunks" => match endpoint("memoryUuid") {
                Ok(memory) if documents.contains(&memory) => true,
                Ok(_) => {
                    dropped.documentChunks += 1;
                    false
                }
                Err(_) => true,
            },
            _ => true,
        }
    });
    dropped
}

/// Checks domain references before any migration write or source deletion.
fn validateMigrationData(data: &CharacterMigrationData) -> Result<(), String> {
    let cards = data
        .cards
        .iter()
        .map(|card| card.id.as_str())
        .collect::<BTreeSet<_>>();
    let groups = data
        .groups
        .iter()
        .map(|group| group.id.as_str())
        .collect::<BTreeSet<_>>();
    let tags = data
        .tags
        .iter()
        .map(|tag| tag.id.as_str())
        .collect::<BTreeSet<_>>();
    let stores = data
        .stores
        .iter()
        .map(|store| store.id.as_str())
        .collect::<BTreeSet<_>>();
    let owners = data
        .owners
        .iter()
        .map(|owner| requiredString(owner, "ownerKey"))
        .collect::<Result<BTreeSet<_>, _>>()?;
    for card in &data.cards {
        if !owners.contains(format!("character:{}", card.id).as_str()) {
            return Err("Imported character owner is missing".to_string());
        }
        for tag in &card.attachedTagIds {
            if !tags.contains(tag.as_str()) {
                return Err(format!("Missing imported tag: {tag}"));
            }
        }
        if card.memoryBindingMode == "SHARED"
            && !card
                .sharedMemoryId
                .as_ref()
                .is_some_and(|id| stores.contains(id.as_str()))
        {
            return Err("Imported shared binding is invalid".to_string());
        }
        for mount in &card.sharedMemoryMounts {
            if !stores.contains(mount.sharedMemoryId.as_str()) {
                return Err("Imported memory mount is missing".to_string());
            }
        }
    }
    for group in &data.groups {
        let value = serde_json::to_value(group).map_err(|error| error.to_string())?;
        for member in value["members"]
            .as_array()
            .ok_or("Invalid imported group members")?
        {
            if !cards.contains(requiredString(member, "characterCardId")?) {
                return Err("Imported group character is missing".to_string());
            }
        }
    }
    for owner in &owners {
        if !data.documents.contains_key(*owner) {
            return Err(format!("Imported owner document is missing: {owner}"));
        }
    }
    let mut memories = BTreeSet::new();
    let mut uuids = BTreeSet::new();
    for (collection, _, envelope) in &data.records {
        let owner = requiredString(envelope, "ownerKey")?;
        if !owners.contains(owner) {
            return Err("Imported record owner is missing".to_string());
        }
        if collection == "memories" {
            memories.insert((owner, requiredString(&envelope["record"], "id")?));
            if envelope["record"]["isDocumentNode"] == true {
                uuids.insert((owner, requiredString(&envelope["record"], "uuid")?));
            }
        }
    }
    for (collection, _, envelope) in &data.records {
        let owner = requiredString(envelope, "ownerKey")?;
        let record = &envelope["record"];
        if collection == "links" {
            for field in ["sourceMemoryId", "targetMemoryId"] {
                if !memories.contains(&(owner, requiredString(record, field)?)) {
                    return Err("Imported relationship target is missing".to_string());
                }
            }
        }
        if collection == "chunks"
            && !uuids.contains(&(owner, requiredString(record, "memoryUuid")?))
        {
            return Err("Imported document chunk parent is missing".to_string());
        }
    }
    let active = data
        .active
        .as_ref()
        .ok_or("Imported active selection is missing")?;
    if let Some(card) = active.get("CharacterCard") {
        if !cards.contains(requiredString(card, "id")?) {
            return Err("Imported active character is missing".to_string());
        }
    } else if let Some(group) = active.get("CharacterGroup") {
        if !groups.contains(requiredString(group, "id")?) {
            return Err("Imported active group is missing".to_string());
        }
    } else {
        return Err("Invalid imported active selection".to_string());
    }
    Ok(())
}

/// Identifies individual keys whose business owner has moved into character plugin storage.
pub fn isRetiredCharacterPreferenceKey(path: &str, key: &str) -> bool {
    (path == operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH
        && (key.starts_with("character_card_theme_") || key.starts_with("character_group_theme_")))
        || (path == API_PREFERENCES_PATH && key == "enable_memory_auto_update")
}

/// Identifies retired source operations so synchronization cannot recreate migrated owner directories.
pub fn isRetiredCharacterOperation(
    operation: &operit_store::SyncOperationStore::SyncOperation,
) -> bool {
    use operit_util::RuntimeStorageLayout::{
        CHARACTER_CARDS_PREFERENCES_PATH, CHARACTER_GROUPS_PREFERENCES_PATH,
        DATA_MEMORY_CHARACTERS_DIR_PATH, DATA_MEMORY_SHARED_DIR_PATH, PROMPT_TAGS_PREFERENCES_PATH,
        SHARED_MEMORY_STORES_PREFERENCES_PATH,
    };
    if operation.domain == "preferences" {
        if let Some(path) = operation.payload.get("storagePath").and_then(Value::as_str) {
            if operation.operation == "transaction" {
                if let Some(entries) = operation.payload.get("entries").and_then(Value::as_array) {
                    if !entries.is_empty()
                        && entries.iter().all(|entry| {
                            entry
                                .get("key")
                                .and_then(Value::as_str)
                                .is_some_and(|key| isRetiredCharacterPreferenceKey(path, key))
                        })
                    {
                        return true;
                    }
                }
            } else if operation
                .payload
                .get("key")
                .and_then(Value::as_str)
                .is_some_and(|key| isRetiredCharacterPreferenceKey(path, key))
            {
                return true;
            }
        }
    }
    let path = match operation.domain.as_str() {
        "objectbox" => return true,
        "runtime_file" => Some(operation.entityId.as_str()),
        "preferences" => operation.payload.get("storagePath").and_then(Value::as_str),
        _ => return false,
    };
    let Some(path) = path else {
        return false;
    };
    [
        CHARACTER_CARDS_PREFERENCES_PATH,
        CHARACTER_GROUPS_PREFERENCES_PATH,
        PROMPT_TAGS_PREFERENCES_PATH,
        SHARED_MEMORY_STORES_PREFERENCES_PATH,
    ]
    .contains(&path)
        || [DATA_MEMORY_CHARACTERS_DIR_PATH, DATA_MEMORY_SHARED_DIR_PATH]
            .iter()
            .any(|root| {
                path == *root
                    || path
                        .strip_prefix(root)
                        .is_some_and(|relative| relative.starts_with('/'))
            })
}

/// Requires an actual string from the declared source schema.
fn requiredString<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing or invalid migration field: {field}"))
}

/// Encodes the plugin's canonical owner document path using JavaScript encodeURIComponent semantics.
pub fn ownerDocumentPath(owner: &str) -> String {
    let mut encoded = String::new();
    for byte in owner.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    format!("owners/{encoded}/USER.md")
}

/// Creates the plugin's actual initial owner metadata while imported settings retain their source values.
pub fn ownerMetadata(owner: &str, settings: MemorySettings, search: MemorySearchConfig) -> Value {
    json!({"ownerKey":owner,"userDocumentPath":ownerDocumentPath(owner),"settings":settings,"searchConfig":search,
        "rebuildTask":null,"rebuildProgress":{"status":"idle","totalChats":0,"completedChats":0,
        "totalWindows":0,"completedWindows":0,"totalSourceMessages":0,"processedSourceMessages":0,
        "failedWindows":0,"currentChatTitle":"","lastError":""}})
}

/// Reports whether a released queue record still resolves against the current chat history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateSourceState {
    /// The record now carries the plugin's exact source message revision.
    Migrated,
    /// The source message no longer exists, so this pending extraction can never run.
    Unresolvable,
}

/// Converts a released automatic history cutoff or selected-user timestamp into its actual message revision.
/// A source chat that lost the message leaves the record unresolvable instead of inventing a revision.
pub fn migrateCandidateTrigger(
    record: &mut Value,
    messages: &[(i64, i64, String)],
) -> Result<CandidateSourceState, String> {
    let candidate: MemoryAutoSaveCandidate =
        serde_json::from_value(record.clone()).map_err(|error| error.to_string())?;
    let timestamp = match candidate.sourceType.as_str() {
        MemoryAutoSaveCandidate::SOURCE_TYPE_REPLY_FINALIZED_AUTO => {
            let Some(timestamp) = messages
                .iter()
                .filter(|message| message.0 <= candidate.triggerMessageTimestamp)
                .map(|message| message.0)
                .max()
            else {
                return Ok(CandidateSourceState::Unresolvable);
            };
            timestamp
        }
        MemoryAutoSaveCandidate::SOURCE_TYPE_SELECTED_USER_MESSAGE => {
            candidate.triggerMessageTimestamp
        }
        source => return Err(format!("Unknown legacy candidate source type: {source}")),
    };
    let sources = messages
        .iter()
        .filter(|message| message.0 == timestamp)
        .collect::<Vec<_>>();
    if sources.len() != 1 {
        return Ok(CandidateSourceState::Unresolvable);
    }
    let source = sources[0];
    let valid_sender = match candidate.sourceType.as_str() {
        MemoryAutoSaveCandidate::SOURCE_TYPE_REPLY_FINALIZED_AUTO => {
            source.2 == "ai" || source.2 == "assistant"
        }
        MemoryAutoSaveCandidate::SOURCE_TYPE_SELECTED_USER_MESSAGE => source.2 == "user",
        _ => unreachable!("candidate source type was checked above"),
    };
    if !valid_sender || source.1 < 0 || source.1 > i64::from(i32::MAX) {
        return Ok(CandidateSourceState::Unresolvable);
    }
    record["triggerMessageTimestamp"] = json!(source.0);
    record["triggerVariantIndex"] = json!(source.1);
    Ok(CandidateSourceState::Migrated)
}

/// Reads the bounded source revisions from Core SQLite before converting a released queue record.
fn migrateStoredCandidateTrigger(
    database: &SqliteStore,
    record: &mut Value,
) -> Result<CandidateSourceState, String> {
    let rows = database.queryRows(
        "SELECT timestamp, selectedVariantIndex, sender FROM messages WHERE chatId=?1 AND timestamp<=?2 ORDER BY timestamp DESC LIMIT 2",
        vec![
            SqliteValue::Text(requiredString(record, "chatId")?.to_string()),
            SqliteValue::Integer(record["triggerMessageTimestamp"].as_i64().ok_or("Invalid candidate timestamp")?),
        ],
    ).map_err(|error| error.to_string())?;
    let messages = rows
        .iter()
        .map(|row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .collect::<Result<Vec<_>, operit_store::SqliteStore::SqliteStoreError>>()
        .map_err(|error| error.to_string())?;
    migrateCandidateTrigger(record, &messages)
}

/// Converts declared ObjectBox numeric identity fields to the plugin's lossless decimal strings.
pub fn ownerRecord(
    collection: &str,
    owner: &str,
    mut record: Value,
) -> Result<(String, String, Value), String> {
    match collection {
        "memories" => {
            serde_json::from_value::<Memory>(record.clone()).map_err(|error| error.to_string())?;
        }
        "links" => {
            serde_json::from_value::<MemoryLink>(record.clone())
                .map_err(|error| error.to_string())?;
        }
        "chunks" => {
            serde_json::from_value::<DocumentChunk>(record.clone())
                .map_err(|error| error.to_string())?;
        }
        "candidates" => {
            serde_json::from_value::<MemoryAutoSaveCandidate>(record.clone())
                .map_err(|error| error.to_string())?;
        }
        _ => return Err(format!("Unknown migrated owner collection: {collection}")),
    }
    for field in ["id", "sourceMemoryId", "targetMemoryId"] {
        if let Some(value) = record.get_mut(field) {
            let id = value
                .as_i64()
                .ok_or_else(|| format!("Invalid legacy identity: {field}"))?;
            *value = json!(id.to_string());
        }
    }
    for field in ["tags", "properties"] {
        if let Some(values) = record.get_mut(field) {
            for value in values
                .as_array_mut()
                .ok_or("Invalid legacy nested identities")?
            {
                let id = value["id"]
                    .as_i64()
                    .ok_or("Invalid legacy nested identity")?;
                value["id"] = json!(id.to_string());
            }
        }
    }
    let key = format!("{owner}/{}", requiredString(&record, "id")?);
    Ok((
        collection.to_string(),
        key,
        json!({"ownerKey":owner,"record":record}),
    ))
}

/// Reads an existing legacy preference file through its established encryption and storage layer.
fn preferences(path: std::path::PathBuf) -> Result<Preferences, String> {
    PreferencesDataStore::new(path)
        .data()
        .map_err(|error| error.to_string())
}

/// Requires a persisted field rather than manufacturing a replacement for damaged source data.
fn preference<'a>(source: &'a Preferences, key: &str) -> Result<&'a str, String> {
    source
        .get(&stringPreferencesKey(key))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Legacy preference is missing: {key}"))
}

/// Reads the legacy ordered identity list; an unset list represents an uncreated collection.
fn identityList(source: &Preferences, key: &str) -> Result<Vec<String>, String> {
    let values = source
        .get(&stringPreferencesKey(key))
        .map(|raw| {
            serde_json::from_str::<Vec<String>>(
                raw.as_str()
                    .ok_or("Legacy identity list must be encoded text")?,
            )
            .map_err(|error| error.to_string())
        })
        .transpose()?;
    Ok(values.into_iter().flatten().collect())
}

/// Projects the old declared preference suffixes into their corresponding structured record fields.
fn preferenceRecord(
    source: &Preferences,
    prefix: &str,
    id: &str,
    strings: &[(&str, &str)],
    nullable: &[(&str, &str)],
    json_fields: &[(&str, &str)],
    numbers: &[(&str, &str)],
) -> Result<Value, String> {
    let mut value = json!({"id":id});
    for (field, suffix) in strings {
        value[*field] = json!(preference(source, &format!("{prefix}_{id}_{suffix}"))?);
    }
    for (field, suffix) in nullable {
        value[*field] =
            json!(source.get(&stringPreferencesKey(&format!("{prefix}_{id}_{suffix}"))));
    }
    for (field, suffix) in json_fields {
        value[*field] =
            serde_json::from_str(preference(source, &format!("{prefix}_{id}_{suffix}"))?)
                .map_err(|error| error.to_string())?;
    }
    for (field, suffix) in numbers {
        value[*field] = json!(preference(source, &format!("{prefix}_{id}_{suffix}"))?
            .parse::<i64>()
            .map_err(|error| error.to_string())?);
    }
    Ok(value)
}

/// Transfers the original global policy once; subsequent reads/writes belong to the plugin.
fn legacyGlobalMemoryPreferences(paths: &RuntimeStorePaths) -> Result<Value, String> {
    let api = crate::data::preferences::ApiPreferences::ApiPreferences::new(
        paths.runtime_dir().to_path_buf(),
    )
    .snapshot()
    .map_err(|error| error.to_string())?;
    let boolean = |key: &str, default: bool| -> Result<bool, String> {
        api.get(&stringPreferencesKey(key))
            .map(|raw| {
                serde_json::from_value::<bool>(raw.clone())
                    .map_err(|_| format!("Invalid legacy boolean: {key}"))
            })
            .transpose()
            .map(|value| value.unwrap_or(default))
    };
    let value = json!({
        "userDescriptionEnabled": !boolean("disable_user_preference_description", false)?,
        "autoUpdateEnabled": boolean("enable_memory_auto_update", true)?,
    });
    validateGlobalMemoryPreferences(&value)?;
    Ok(value)
}

fn validateGlobalMemoryPreferences(value: &Value) -> Result<(), String> {
    if !value["userDescriptionEnabled"].is_boolean() || !value["autoUpdateEnabled"].is_boolean() {
        return Err("Invalid global memory preferences".to_string());
    }
    Ok(())
}

/// Collects all v27 preference records using the existing schema and portable host paths.
fn legacyPreferences(paths: &RuntimeStorePaths) -> Result<CharacterMigrationData, String> {
    let cards = preferences(paths.runtime_storage_path(CHARACTER_CARDS_PREFERENCES_PATH))?;
    let groups = preferences(paths.runtime_storage_path(CHARACTER_GROUPS_PREFERENCES_PATH))?;
    let tags = preferences(paths.runtime_storage_path(PROMPT_TAGS_PREFERENCES_PATH))?;
    let stores = preferences(paths.runtime_storage_path(SHARED_MEMORY_STORES_PREFERENCES_PATH))?;
    let mut data = CharacterMigrationData::default();
    data.memory_preferences = Some(legacyGlobalMemoryPreferences(paths)?);
    for id in identityList(&cards, "character_card_list")? {
        let mut value = preferenceRecord(
            &cards,
            "character_card",
            &id,
            &[
                ("name", "name"),
                ("description", "description"),
                ("characterSetting", "character_setting"),
                ("openingStatement", "opening_statement"),
                ("otherContentChat", "other_content_chat"),
                ("otherContentVoice", "other_content_voice"),
                ("advancedCustomPrompt", "advanced_custom_prompt"),
                ("marks", "marks"),
                ("chatModelBindingMode", "chat_model_binding_mode"),
                ("memoryBindingMode", "memory_binding_mode"),
            ],
            &[
                ("avatarUri", "avatar_uri"),
                ("chatModelId", "chat_model_id"),
                ("ttsConfigId", "tts_config_id"),
                ("sharedMemoryId", "shared_memory_id"),
            ],
            &[
                ("attachedTagIds", "attached_tag_ids"),
                ("sharedMemoryMounts", "shared_memory_mounts"),
            ],
            &[("createdAt", "created_at"), ("updatedAt", "updated_at")],
        )?;
        // The released default-card writer represents disabled tool access by an absent key.
        let tool_key =
            stringPreferencesKey(&format!("character_card_{id}_tool_access_config_json"));
        let tool_access = match cards.get(&tool_key) {
            Some(raw) => serde_json::from_str::<CharacterCardToolAccessConfig>(
                raw.as_str()
                    .ok_or("Legacy tool access config must be encoded text")?,
            )
            .map_err(|error| error.to_string())?,
            None => CharacterCardToolAccessConfig::default(),
        };
        value["toolAccessConfig"] =
            serde_json::to_value(tool_access).map_err(|error| error.to_string())?;
        value["isDefault"] = json!(
            preference(&cards, &format!("character_card_{id}_is_default"))?
                .parse::<bool>()
                .map_err(|error| error.to_string())?
        );
        data.cards
            .push(serde_json::from_value(value).map_err(|error| error.to_string())?);
    }
    for id in identityList(&groups, "character_group_list")? {
        let value =
            serde_json::from_str(preference(&groups, &format!("character_group_{id}_data"))?)
                .map_err(|error| error.to_string())?;
        data.groups.push(value);
    }
    for id in identityList(&tags, "prompt_tag_list")? {
        let value = preferenceRecord(
            &tags,
            "prompt_tag",
            &id,
            &[
                ("name", "name"),
                ("description", "description"),
                ("promptContent", "prompt_content"),
                ("tagType", "tag_type"),
            ],
            &[],
            &[],
            &[("createdAt", "created_at"), ("updatedAt", "updated_at")],
        )?;
        data.tags
            .push(serde_json::from_value(value).map_err(|error| error.to_string())?);
    }
    for id in identityList(&stores, "shared_memory_store_list")? {
        let value = preferenceRecord(
            &stores,
            "shared_memory_store",
            &id,
            &[("name", "name")],
            &[],
            &[],
            &[("createdAt", "created_at"), ("updatedAt", "updated_at")],
        )?;
        data.stores
            .push(serde_json::from_value(value).map_err(|error| error.to_string())?);
    }
    let group = groups
        .get(&stringPreferencesKey("active_character_group_id"))
        .filter(|id| id.as_str() != Some(""));
    data.active = Some(match group {
        Some(id) => json!({"CharacterGroup":{"id":id}}),
        None => json!({"CharacterCard":{"id":preference(&cards,"active_character_card_id")?}}),
    });
    Ok(data)
}

/// Reads only existing ObjectBox databases, preserving every declared entity payload and exact identity.
fn legacyEntities(path: &str, entity: &str) -> Result<Vec<Value>, String> {
    let storage = defaultRuntimeStorageHost();
    if !storage.exists(path).map_err(|error| error.to_string())? {
        return Ok(Vec::new());
    }
    let store = SqliteStore::openWithHost(defaultRuntimeSqliteHost(), path)
        .map_err(|error| error.to_string())?;
    store
        .queryRows(
            "SELECT payload FROM objectbox_entities WHERE entity_type=?1 ORDER BY id",
            vec![SqliteValue::Text(entity.to_string())],
        )
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|row| {
            let payload: String = row.get(0).map_err(|error| error.to_string())?;
            serde_json::from_str(&payload).map_err(|error| error.to_string())
        })
        .collect()
}

/// Resolves the original human-name binding exactly once and rejects missing or ambiguous source records.
pub fn resolveBinding(cards: &[Value], groups: &[Value], old: &Value) -> Result<Value, String> {
    let selection = if let Some(id) = old["characterGroupId"].as_str().filter(|id| !id.is_empty()) {
        if groups.iter().filter(|group| group["id"] == id).count() != 1 {
            return Err(format!("Missing legacy character group: {id}"));
        }
        format!("group:{id}")
    } else {
        let matches: Vec<_> = match old["characterCardName"]
            .as_str()
            .filter(|name| !name.is_empty())
        {
            Some(name) => cards.iter().filter(|card| card["name"] == name).collect(),
            None => cards
                .iter()
                .filter(|card| card["isDefault"] == true)
                .collect(),
        };
        if matches.len() != 1 {
            return Err(format!(
                "Legacy character binding is missing or ambiguous: {old}"
            ));
        }
        format!("card:{}", requiredString(matches[0], "id")?)
    };
    Ok(json!({"version":1,"selection":selection}))
}

/// Publishes committed drafts and retires drafts whose Core chat was never committed.
fn reconcileChatDrafts(
    database: &SqliteStore,
    writer: &CharacterPluginWriter,
) -> Result<(), String> {
    for draft in writer.list("chat_drafts")? {
        let id = requiredString(&draft, "key")?;
        let chat = database
            .queryOne(
                "SELECT pluginExtensions FROM chats WHERE id=?1",
                vec![SqliteValue::Text(id.into())],
            )
            .map_err(|error| error.to_string())?;
        if let Some(chat) = chat {
            let extensions: BTreeMap<String, Value> = serde_json::from_str(
                &chat
                    .get::<_, String>(0)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            if extensions.get(PLUGIN_ID) == Some(&json!({"recordId":id,"schemaVersion":1})) {
                writer.adoptRecord("chat_bindings", id, draft["value"].clone())?;
            }
        }
        writer
            .relational
            .delete("chat_drafts", id, requiredString(&draft, "version")?)?;
    }
    Ok(())
}

/// Removes bindings only when Core has a committed deletion and no surviving chat identity.
fn reconcileDeletedChatBindings(
    database: &SqliteStore,
    writer: &CharacterPluginWriter,
) -> Result<(), String> {
    let deleted = database.queryRows(
        "SELECT DISTINCT d.chatId FROM sync_sql_deletions d LEFT JOIN chats c ON c.id=d.chatId WHERE d.tableName='chats' AND c.id IS NULL",
        Vec::new(),
    ).map_err(|error| error.to_string())?;
    for row in deleted {
        let id: String = row.get(0).map_err(|error| error.to_string())?;
        if let Some(binding) = writer.get("chat_bindings", &id)? {
            writer
                .relational
                .delete("chat_bindings", &id, requiredString(&binding, "version")?)?;
        }
    }
    Ok(())
}

/// Converts legacy bindings and frozen message snapshots before removing their Core payloads.
fn adoptCoreExtensions(
    paths: RuntimeStorePaths,
    writer: &CharacterPluginWriter,
) -> Result<(), String> {
    let database = operit_store::db::AppDatabase::AppDatabase::getDatabase(paths)
        .map_err(|error| error.to_string())?;
    adoptCoreConfigurationRows(database.store(), writer)
}

/// Migrates committed Core references using the same SQL Host as upgrade and archive import.
fn adoptCoreConfigurationRows(
    database: &SqliteStore,
    writer: &CharacterPluginWriter,
) -> Result<(), String> {
    reconcileChatDrafts(database, writer)?;
    reconcileDeletedChatBindings(database, writer)?;
    let cards = writer
        .list("cards")?
        .into_iter()
        .map(|entry| entry["value"].clone())
        .collect::<Vec<_>>();
    let groups = writer
        .list("groups")?
        .into_iter()
        .map(|entry| entry["value"].clone())
        .collect::<Vec<_>>();
    for table in [
        "chats",
        "sync_sql_chat_rows",
        "messages",
        "message_variants",
        "sync_sql_message_rows",
        "sync_sql_message_variant_rows",
    ] {
        let columns = if table == "chats" {
            "rowid,pluginExtensions,id"
        } else {
            "rowid,pluginExtensions"
        };
        let rows = database
            .queryRows(&format!("SELECT {columns} FROM {table}"), Vec::new())
            .map_err(|error| error.to_string())?;
        for row in rows {
            let row_id: i64 = row.get(0).map_err(|error| error.to_string())?;
            let mut extensions: BTreeMap<String, Value> =
                serde_json::from_str(&row.get::<_, String>(1).map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
            let legacy = extensions.remove(LEGACY_BINDING_NAMESPACE);
            let stored = extensions.remove(PLUGIN_ID);
            if legacy.is_none() && stored.is_none() {
                continue;
            }
            if legacy.is_some() && stored.is_some() {
                return Err("Migration has conflicting character namespaces".into());
            }
            let value = match (legacy, stored) {
                (Some(legacy), None) => resolveBinding(&cards, &groups, &legacy)?,
                (None, Some(stored)) => stored,
                _ => unreachable!("Character namespace presence was checked"),
            };
            match table {
                "chats" => {
                    let id: String = row.get(2).map_err(|error| error.to_string())?;
                    if value.get("schemaVersion") != Some(&json!(1)) {
                        writer.adoptRecord("chat_bindings", &id, value)?;
                    }
                }
                "sync_sql_chat_rows" => {}
                _ => {
                    if value.get("schemaVersion") == Some(&json!(1)) {
                        continue;
                    }
                    let key = format!("migration:{table}:{row_id}");
                    writer.adoptRecord("message_snapshots", &key, value)?;
                    extensions.insert(PLUGIN_ID.into(), json!({"recordId":key,"schemaVersion":1}));
                }
            }
            let encoded = serde_json::to_string(&extensions).map_err(|error| error.to_string())?;
            database
                .execute(
                    &format!("UPDATE {table} SET pluginExtensions=?1 WHERE rowid=?2"),
                    vec![SqliteValue::Text(encoded), SqliteValue::Integer(row_id)],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    database.execute(
        "UPDATE sync_sql_operations SET schemaVersion=8 WHERE domain='chat' AND schemaVersion=7",
        Vec::new(),
    ).map_err(|error| error.to_string())?;
    Ok(())
}

/// Migrates released Core preferences and memory exactly once before any character plugin runtime opens.
pub fn migrateLegacyCharacters(paths: RuntimeStorePaths) -> Result<(), String> {
    let storage = defaultRuntimeStorageHost();
    let database_path = format!(
        "{}/characters.sqlite",
        operit_store::ExtensionStore::ExtensionStore::spaceDataPath(PLUGIN_ID)?
    );
    let source_exists = storage
        .exists(&runtimeStoragePath(
            &paths.runtime_storage_path(CHARACTER_CARDS_PREFERENCES_PATH),
        ))
        .map_err(|error| error.to_string())?;
    let database_exists = storage
        .exists(&database_path)
        .map_err(|error| error.to_string())?;
    let relational_exists = storage
        .exists(&format!(
            "{}/characters-relational.sqlite",
            operit_store::ExtensionStore::ExtensionStore::spaceDataPath(PLUGIN_ID)?
        ))
        .map_err(|error| error.to_string())?;
    if !source_exists && !database_exists && !relational_exists {
        let database = operit_store::db::AppDatabase::AppDatabase::getDatabase(paths)
            .map_err(|error| error.to_string())?;
        database.store().execute(
            "UPDATE sync_sql_operations SET schemaVersion=8 WHERE domain='chat' AND schemaVersion=7",
            Vec::new(),
        ).map_err(|error| error.to_string())?;
        return Ok(());
    }
    let writer = CharacterPluginWriter::open(defaultRuntimeSqliteHost(), storage.clone())?;
    // Upgrades completed v3 plugin migrations as well as imports made before these flags
    // became plugin-owned. Never replace a value already saved by the plugin.
    if let Some(entry) = writer.get(META_COLLECTION, "state")? {
        let mut metadata = entry["value"].clone();
        if !metadata.is_object() || metadata["version"] != 3 {
            return Err("Unsupported character database metadata".to_string());
        }
        if metadata.get("memoryPreferences").is_none() && metadata["migrationStatus"] != "writing" {
            metadata["memoryPreferences"] = legacyGlobalMemoryPreferences(&paths)?;
            writer.commit(vec![writer.mutation(META_COLLECTION, "state", metadata)?])?;
        }
    }
    if !source_exists && writer.get("imports", "operit2-v27")?.is_none() {
        return adoptCoreExtensions(paths, &writer);
    }
    if writer.get("imports", "operit2-v27")?.is_none() {
        let mut data = legacyPreferences(&paths)?;
        let owners = data
            .cards
            .iter()
            .map(|card| format!("character:{}", card.id))
            .chain(
                data.stores
                    .iter()
                    .map(|store| format!("shared:{}", store.id)),
            )
            .collect::<Vec<_>>();
        for owner in owners {
            let root = legacyMemoryRoot(&owner)?;
            data.cleanup_paths.push(root.clone());
            let (memory_settings, search) = readLegacyMemorySettings(storage.clone(), &owner)?;
            data.owners
                .push(ownerMetadata(&owner, memory_settings, search));
            let mut dropped_candidates = 0usize;
            for (collection, entity, path) in [
                ("memories", "Memory", format!("{root}/Memory.sqlite")),
                ("chunks", "DocumentChunk", format!("{root}/Memory.sqlite")),
                ("links", "MemoryLink", format!("{root}/MemoryLink.sqlite")),
                (
                    "candidates",
                    "MemoryAutoSaveCandidate",
                    format!("{root}/MemoryAutoSaveCandidate.sqlite"),
                ),
            ] {
                for mut record in legacyEntities(&path, entity)? {
                    if collection == "candidates" {
                        let database =
                            operit_store::db::AppDatabase::AppDatabase::getDatabase(paths.clone())
                                .map_err(|error| error.to_string())?;
                        // The retired app deleted chats without clearing their queued extractions,
                        // and the plugin requires a resolvable source revision, so stale queue
                        // records are dropped rather than failing the whole migration.
                        if migrateStoredCandidateTrigger(database.store(), &mut record)?
                            == CandidateSourceState::Unresolvable
                        {
                            dropped_candidates += 1;
                            continue;
                        }
                    }
                    if collection == "memories" {
                        for field in ["documentPath", "chunkIndexFilePath"] {
                            if let Some(source) = record[field].as_str() {
                                let source = legacyResourceStoragePath(source)?;
                                if let Some(relative) = legacyResourceRelativePath(&source) {
                                    validateResourcePath(relative)?;
                                    let destination = format!(
                                        "{}/resources/{relative}",
                                        ownerDocumentPath(&owner).trim_end_matches("/USER.md")
                                    );
                                    let bytes = storage
                                        .readBytes(&source)
                                        .map_err(|error| error.to_string())?;
                                    data.resources.insert(destination.clone(), bytes);
                                    record[field] = json!(publicResourcePath(&format!(
                                        "{}/{destination}",
                                        writer.directory
                                    ))?);
                                } else {
                                    storage
                                        .readBytes(&source)
                                        .map_err(|error| error.to_string())?;
                                    record[field] = json!(publicResourcePath(&source)?);
                                }
                            }
                        }
                    }
                    data.records.push(ownerRecord(collection, &owner, record)?);
                }
            }
            if dropped_candidates > 0 {
                AppLogger::w(
                    "CharacterPluginMigration",
                    &format!(
                        "owner={owner} dropped {dropped_candidates} queued memory extractions whose source chat history no longer exists"
                    ),
                );
            }
            let bytes = readLegacyUserMarkdown(storage.as_ref(), &owner)?;
            data.documents.insert(owner, bytes);
        }
        for path in [
            paths.runtime_storage_path(CHARACTER_CARDS_PREFERENCES_PATH),
            paths.runtime_storage_path(CHARACTER_GROUPS_PREFERENCES_PATH),
            paths.runtime_storage_path(PROMPT_TAGS_PREFERENCES_PATH),
            paths.runtime_storage_path(SHARED_MEMORY_STORES_PREFERENCES_PATH),
        ] {
            data.cleanup_paths.push(runtimeStoragePath(&path));
        }
        writer.import("operit2-v27", data)?;
    }
    writer.migrateCoreThemes()?;
    adoptCoreExtensions(paths, &writer)?;
    cleanupLegacyCharacters(&writer)
}

/// Deletes only the verified migration's exact legacy owners and dedicated preference files through the Host.
fn cleanupLegacyCharacters(writer: &CharacterPluginWriter) -> Result<(), String> {
    use operit_util::RuntimeStorageLayout::{
        CHARACTER_CARDS_PREFERENCES_PATH, CHARACTER_GROUPS_PREFERENCES_PATH,
        DATA_MEMORY_CHARACTERS_DIR_PATH, DATA_MEMORY_SHARED_DIR_PATH, PROMPT_TAGS_PREFERENCES_PATH,
        SHARED_MEMORY_STORES_PREFERENCES_PATH,
    };
    let entry = writer
        .get("imports", "operit2-v27")?
        .ok_or("Core character migration completion is missing")?;
    let mut marker = entry["value"].clone();
    if marker["complete"] != true {
        return Err("Cannot delete incomplete character migration sources".to_string());
    }
    let paths = marker["cleanupPaths"]
        .as_array()
        .ok_or("Migration cleanup paths are missing")?;
    let mut validated_paths = Vec::new();
    for path in paths {
        let path = path.as_str().ok_or("Invalid cleanup path")?;
        let memory_owner = [DATA_MEMORY_CHARACTERS_DIR_PATH, DATA_MEMORY_SHARED_DIR_PATH]
            .iter()
            .any(|root| {
                path.strip_prefix(&format!("{root}/"))
                    .is_some_and(|segment| {
                        !segment.is_empty()
                            && segment != "."
                            && segment != ".."
                            && !segment.contains('/')
                    })
            });
        let preference = [
            CHARACTER_CARDS_PREFERENCES_PATH,
            CHARACTER_GROUPS_PREFERENCES_PATH,
            PROMPT_TAGS_PREFERENCES_PATH,
            SHARED_MEMORY_STORES_PREFERENCES_PATH,
        ]
        .iter()
        .any(|expected| path == *expected);
        if !memory_owner && !preference {
            return Err(format!(
                "Migration cleanup path is outside the declared legacy data: {path}"
            ));
        }
        validated_paths.push((path, memory_owner));
    }
    let api_preferences =
        crate::data::preferences::ApiPreferences::ApiPreferences::store(writer.storage.clone());
    api_preferences
        .try_edit_result(
            |saved| -> Result<(), operit_store::PreferencesDataStore::PreferencesDataStoreError> {
                let key = stringPreferencesKey("enable_memory_auto_update");
                if saved.contains(&key) {
                    let metadata = writer
                        .get(META_COLLECTION, "state")
                        .map_err(
                            operit_store::PreferencesDataStore::PreferencesDataStoreError::Message,
                        )?
                        .ok_or_else(|| {
                            operit_store::PreferencesDataStore::PreferencesDataStoreError::Message(
                                "Migrated memory policy metadata is missing".to_string(),
                            )
                        })?;
                    validateGlobalMemoryPreferences(&metadata["value"]["memoryPreferences"])
                        .map_err(
                            operit_store::PreferencesDataStore::PreferencesDataStoreError::Message,
                        )?;
                    let source =
                        serde_json::from_value::<bool>(saved.get(&key).ok_or_else(|| {
                            operit_store::PreferencesDataStore::PreferencesDataStoreError::Message(
                                "Legacy memory policy disappeared during cleanup".into(),
                            )
                        })?.clone())
                        .map_err(|error| {
                            operit_store::PreferencesDataStore::PreferencesDataStoreError::Message(
                                error.to_string(),
                            )
                        })?;
                    if metadata["value"]["memoryPreferences"]["autoUpdateEnabled"] != json!(source)
                    {
                        return Err(
                            operit_store::PreferencesDataStore::PreferencesDataStoreError::Message(
                                "Legacy memory policy differs from its verified destination".into(),
                            ),
                        );
                    }
                    saved.remove(&key);
                }
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
    if marker["cleanupComplete"] == true {
        return Ok(());
    }
    for (path, memory_owner) in validated_paths {
        if writer
            .storage
            .exists(path)
            .map_err(|error| error.to_string())?
        {
            writer
                .storage
                .delete(path, memory_owner)
                .map_err(|error| error.to_string())?;
        }
    }
    for root in [DATA_MEMORY_CHARACTERS_DIR_PATH, DATA_MEMORY_SHARED_DIR_PATH] {
        if writer
            .storage
            .exists(root)
            .map_err(|error| error.to_string())?
            && writer
                .storage
                .list(root)
                .map_err(|error| error.to_string())?
                .is_empty()
        {
            writer
                .storage
                .delete(root, true)
                .map_err(|error| error.to_string())?;
        }
    }
    marker["cleanupComplete"] = json!(true);
    writer.commit(vec![writer.mutation("imports", "operit2-v27", marker)?])
}

/// Retains the exact source target and a complete independent theme for restartable adoption.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ThemeBinding {
    collection: String,
    key: String,
    config: ThemeConfig,
}

/// Persists the conversion inputs before either configuration or role bindings are changed.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ThemeMigrationPlan {
    bindings: Vec<ThemeBinding>,
    activeConfigId: Option<String>,
    sourceEntries: Vec<(String, String)>,
    complete: bool,
}

/// Returns the exact released preference namespace for a declared role target.
fn themePrefix(collection: &str, id: &str) -> Result<String, String> {
    match collection {
        "cards" => Ok(format!("character_card_theme_{id}_")),
        "groups" => Ok(format!("character_group_theme_{id}_")),
        _ => Err(format!(
            "Invalid legacy theme target collection: {collection}"
        )),
    }
}

impl CharacterPluginWriter {
    /// Migrates every imported role theme, verifies both owners, then removes only its exact source keys.
    pub fn migrateThemes(
        &self,
        source: &str,
        source_preferences: &Preferences,
        targets: &[(String, String)],
        clean_preferences: bool,
    ) -> Result<(), String> {
        let marker_key = format!("{source}:themes-v1");
        let plan: ThemeMigrationPlan = match self.get("imports", &marker_key)? {
            Some(entry) if entry["value"]["complete"] == true => return Ok(()),
            Some(entry) => {
                serde_json::from_value(entry["value"].clone()).map_err(|error| error.to_string())?
            }
            None => {
                let plan = self.planThemes(source, source_preferences, targets)?;
                self.commit(vec![self.mutation(
                    "imports",
                    &marker_key,
                    serde_json::to_value(&plan).map_err(|error| error.to_string())?,
                )?])?;
                plan
            }
        };
        if plan.complete {
            return Ok(());
        }
        let manager = ThemeConfigManager::new(self.storage.clone());
        let configs = plan
            .bindings
            .iter()
            .map(|binding| binding.config.clone())
            .collect::<Vec<_>>();
        manager
            .importLegacyConfigurations(&configs, plan.activeConfigId.as_deref())
            .map_err(|error| error.to_string())?;
        // Validate every retained role before publishing any binding batch.
        let mut rows = Vec::new();
        for binding in &plan.bindings {
            let entry = self
                .get(&binding.collection, &binding.key)?
                .ok_or_else(|| format!("Migrated theme target is missing: {}", binding.key))?;
            let mut role = entry["value"].clone();
            match role.get("themeConfigId") {
                Some(Value::Null) => role["themeConfigId"] = json!(binding.config.id),
                Some(Value::String(id)) if id == &binding.config.id => {}
                _ => {
                    return Err(format!(
                        "Migration would replace an existing theme binding: {}",
                        binding.key
                    ));
                }
            }
            rows.push((binding.collection.clone(), binding.key.clone(), role));
        }
        for page in rows.chunks(1000) {
            let mutations = page
                .iter()
                .map(|(collection, key, role)| self.mutation(collection, key, role.clone()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            self.commit(mutations)?;
        }
        for binding in &plan.bindings {
            let entry = self
                .get(&binding.collection, &binding.key)?
                .ok_or("Migrated theme target disappeared during verification")?;
            if entry["value"]["themeConfigId"] != binding.config.id
                || manager
                    .get(binding.config.id.clone())
                    .map_err(|error| error.to_string())?
                    != binding.config
            {
                return Err(format!(
                    "Migrated theme binding verification failed: {}",
                    binding.key
                ));
            }
        }
        if clean_preferences {
            let preferences =
                crate::data::preferences::UserPreferencesManager::UserPreferencesManager::store(
                    self.storage.clone(),
                );
            preferences.try_edit_result(|saved| -> Result<(), operit_store::PreferencesDataStore::PreferencesDataStoreError> {
                let source_keys = plan.sourceEntries.iter().map(|(key, _)| key.as_str()).collect::<BTreeSet<_>>();
                for (key, _) in saved.iterEntries() {
                    if (key.starts_with("character_card_theme_") || key.starts_with("character_group_theme_"))
                        && !source_keys.contains(key)
                    {
                        return Err(operit_store::PreferencesDataStore::PreferencesDataStoreError::Message(
                            format!("Legacy theme source added a key before cleanup: {key}")));
                    }
                }
                for (key, expected) in &plan.sourceEntries {
                    if let Some(actual) = saved.get(&stringPreferencesKey(key)) {
                        if actual != expected {
                            return Err(operit_store::PreferencesDataStore::PreferencesDataStoreError::Message(
                                format!("Legacy theme source changed before cleanup: {key}")));
                        }
                    }
                }
                for (key, _) in &plan.sourceEntries {
                    saved.remove(&stringPreferencesKey(key));
                }
                Ok(())
            }).map_err(|error| error.to_string())?;
            let saved = preferences.data().map_err(|error| error.to_string())?;
            if plan
                .sourceEntries
                .iter()
                .any(|(key, _)| saved.contains(&stringPreferencesKey(key)))
            {
                return Err("Legacy theme preference cleanup verification failed".to_string());
            }
        }
        self.commit(vec![self.mutation(
            "imports",
            &marker_key,
            json!({"complete":true}),
        )?])
    }

    /// Decodes the released sparse theme schema using its declared initial values and exact target IDs.
    fn planThemes(
        &self,
        source: &str,
        preferences: &Preferences,
        targets: &[(String, String)],
    ) -> Result<ThemeMigrationPlan, String> {
        let state = self
            .get("meta", "state")?
            .ok_or("Character selection metadata is missing")?;
        let active = &state["value"]["active"];
        let mut plan = ThemeMigrationPlan {
            bindings: Vec::new(),
            activeConfigId: None,
            sourceEntries: Vec::new(),
            complete: false,
        };
        let mut identities = BTreeSet::new();
        for (collection, id) in targets {
            if !identities.insert((collection, id)) {
                return Err(format!("Duplicate legacy theme target: {collection}/{id}"));
            }
            let role = self
                .get(collection, id)?
                .ok_or_else(|| format!("Legacy theme target is missing: {id}"))?;
            let role = &role["value"];
            let prefix = themePrefix(collection, id)?;
            let (snapshot, entries) = readLegacyCharacterThemeSnapshot(preferences, &prefix)
                .map_err(|error| error.to_string())?;
            let config = ThemeConfig {
                id: format!("legacy-theme:{source}:{collection}:{id}"),
                name: requiredString(role, "name")?.to_string(),
                snapshot,
                createdAt: role["createdAt"]
                    .as_i64()
                    .ok_or("Invalid legacy theme creation time")?,
                updatedAt: role["updatedAt"]
                    .as_i64()
                    .ok_or("Invalid legacy theme update time")?,
            };
            match role.get("themeConfigId") {
                Some(Value::Null) => {}
                Some(Value::String(value)) if value == &config.id => {}
                _ => {
                    return Err(format!(
                        "Migration would replace an existing theme binding: {id}"
                    ));
                }
            }
            if (collection == "cards" && active["CharacterCard"]["id"] == *id)
                || (collection == "groups" && active["CharacterGroup"]["id"] == *id)
            {
                plan.activeConfigId = Some(config.id.clone());
            }
            plan.bindings.push(ThemeBinding {
                collection: collection.clone(),
                key: id.clone(),
                config,
            });
            plan.sourceEntries.extend(entries);
        }
        let migrated_keys = plan
            .sourceEntries
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<BTreeSet<_>>();
        for (key, _) in preferences.iterEntries() {
            if (key.starts_with("character_card_theme_")
                || key.starts_with("character_group_theme_"))
                && !migrated_keys.contains(key)
            {
                return Err(format!(
                    "Legacy theme preference has no imported role target: {key}"
                ));
            }
        }
        Ok(plan)
    }

    /// Recovers the Core migration's original card IDs from its exact cleanup manifest, excluding later cards.
    pub fn migrateCoreThemes(&self) -> Result<(), String> {
        let marker = self
            .get("imports", "operit2-v27")?
            .ok_or("Core character import is missing")?;
        let cleanup = marker["value"]["cleanupPaths"]
            .as_array()
            .ok_or("Core cleanup manifest is missing")?;
        let mut targets = Vec::new();
        if let Some(declared) = marker["value"].get("themeTargets") {
            targets =
                serde_json::from_value(declared.clone()).map_err(|error| error.to_string())?;
        } else {
            // Upgrade the completed pre-theme migration manifest using its original owner paths.
            for entry in self.list("cards")? {
                let id = requiredString(&entry["value"], "id")?;
                let root = legacyMemoryRoot(&format!("character:{id}"))?;
                if cleanup
                    .iter()
                    .any(|path| path.as_str() == Some(root.as_str()))
                {
                    targets.push(("cards".to_string(), id.to_string()));
                }
            }
            for entry in self.list("groups")? {
                targets.push((
                    "groups".to_string(),
                    requiredString(&entry["value"], "id")?.to_string(),
                ));
            }
        }
        let preferences =
            crate::data::preferences::UserPreferencesManager::UserPreferencesManager::store(
                self.storage.clone(),
            )
            .data()
            .map_err(|error| error.to_string())?;
        self.migrateThemes("operit2-v27", &preferences, &targets, true)
    }
}

#[cfg(test)]
#[path = "CharacterPluginMigrationTests.rs"]
mod tests;
