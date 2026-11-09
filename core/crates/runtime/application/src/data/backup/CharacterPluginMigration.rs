//! Imports Operit1 character and memory data into plugin-owned storage.
//! Configuration uses native plugin preferences; memory objects use their separate storage owner.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::CharacterPluginLegacyData::*;
use crate::data::preferences::ThemeConfigManager::{ThemeConfig, ThemeConfigManager};
use crate::data::preferences::ThemePreferenceSnapshot::readLegacyCharacterThemeSnapshot;
use operit_host_api::PluginStorage::{
    StorageKind, StorageMutation, StorageRequest, StorageResponse, StoragePreferenceEdit,
};
use operit_host_api::{RuntimeSqliteHost, RuntimeStorageHost};
use operit_store::PluginStorage::PluginStorageSession;
use operit_store::PreferencesDataStore::{stringPreferencesKey, Preferences, PreferencesDataStore};
use operit_store::SyncOperationStore::SyncOperationStore;
use operit_util::AppLogger::AppLogger;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
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
    pub active: Option<Value>,
    pub memory_preferences: Option<Value>,
}

/// Uses the production Host storage dispatcher for bounded and version-checked migration writes.
pub struct CharacterPluginWriter {
    session: PluginStorageSession,
    sqlite: Arc<dyn RuntimeSqliteHost>,
    storage: Arc<dyn RuntimeStorageHost>,
    device_id: String,
    handle: String,
    preferences: BTreeMap<String, String>,
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
                    path: format!("{directory}/memory.sqlite"),
                    kind: StorageKind::Objects,
                    merge: Some(serde_json::from_str(include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../../plugins/packages/buildin/character_cards/src/storage/memory-merge.json"
                    ))).map_err(|error| error.to_string())?),
                },
                &device_id,
            )
            .map_err(|error| error.to_string())?;
        let StorageResponse::Open(response) = response else {
            return Err("Storage open returned a different operation result".to_string());
        };
        let handle = response.handle;
        let declarations: BTreeMap<String, operit_host_api::PluginStorage::StoragePreferencesMergeDescriptor> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"), "/../../../../plugins/packages/buildin/character_cards/src/storage/merge.json"
        ))).map_err(|error| error.to_string())?;
        let mut preferences = BTreeMap::new();
        for (name, merge) in declarations {
            let response = session.requestPreferences(storage.clone(), StorageRequest::OpenPreferences {
                path: format!("{directory}/{name}.preferences.json"), merge: Some(merge),
            }).map_err(|error| error.to_string())?;
            let StorageResponse::Open(response) = response else { return Err("Preference open returned an unexpected result".into()); };
            preferences.insert(name, response.handle);
        }
        let result = Self {
            session,
            sqlite,
            storage,
            device_id,
            handle,
            preferences,
            directory,
        };
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

    /// Selects a plugin-owned JSON file and key independently of memory objects.
    fn preferenceLocation(&self, collection: &str, key: &str) -> Option<(&str, String)> {
        let (owner, key) = match collection {
            "cards" | "groups" | "tags" | "stores" | "conversationGroups" => (collection, format!("record_{key}")),
            "meta" => ("meta", key.to_string()),
            "imports" => ("meta", format!("import_{key}")),
            "chat_bindings" => ("chats", format!("binding_{key}")),
            "chat_drafts" => ("chats", format!("draft_{key}")),
            _ => return None,
        };
        Some((self.preferences.get(owner).expect("declared preference owner"), key))
    }

    /// Reads an exact object entry, preserving the distinction between absence and a stored value.
    pub fn get(&self, collection: &str, key: &str) -> Result<Option<Value>, String> {
        if let Some((handle, key)) = self.preferenceLocation(collection, key) {
            let snapshot = self.request(StorageRequest::PreferencesData { handle: handle.into() })?;
            return snapshot["values"].get(&key).map(|value| {
                Ok(json!({"value":value,"version":serde_json::to_string(value).map_err(|error| error.to_string())?}))
            }).transpose();
        }
        let entry = self.request(StorageRequest::Get {
            handle: self.handle.clone(), collection: collection.into(), key: key.into(),
        })?;
        Ok((!entry.is_null()).then_some(entry))
    }

    /// Enumerates bounded pages without reading or rewriting whole collection files.
    pub fn list(&self, collection: &str) -> Result<Vec<Value>, String> {
        if let Some((handle, prefix)) = self.preferenceLocation(collection, "") {
            let snapshot = self.request(StorageRequest::PreferencesData { handle: handle.into() })?;
            let values = snapshot["values"].as_object().ok_or("Preference data returned invalid values")?;
            let mut result = Vec::new();
            for (key, value) in values {
                if let Some(key) = key.strip_prefix(&prefix) {
                    result.push(json!({"key":key,"value":value,"version":serde_json::to_string(value).map_err(|error| error.to_string())?}));
                }
            }
            return Ok(result);
        }
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

    /// Publishes each preference owner through its native edit and memory through its own object transaction.
    fn commit(&self, mutations: Vec<StorageMutation>) -> Result<(), String> {
        let mut owners: BTreeMap<String, Vec<(String, StorageMutation)>> = BTreeMap::new();
        let mut memory = Vec::new();
        for mutation in mutations {
            if let Some((handle, key)) = self.preferenceLocation(&mutation.collection, &mutation.key) {
                owners.entry(handle.into()).or_default().push((key, mutation));
            } else { memory.push(mutation); }
        }
        for (handle, mutations) in owners {
            let started = self.request(StorageRequest::PreferencesBeginEdit { handle: handle.clone() })?;
            let token = requiredString(&started, "token")?.to_string();
            let values = started["snapshot"]["values"].as_object().ok_or("Preference edit returned invalid values")?;
            let edits = (|| {
                let mut edits = Vec::new();
                for (key, mutation) in mutations {
                    let current = values.get(&key).map(serde_json::to_string).transpose().map_err(|error| error.to_string())?;
                    if mutation.check_version && current != mutation.expected_version { return Err(format!("Preference import edit conflict: {key}")); }
                    edits.push(if mutation.deleted { StoragePreferenceEdit::Remove { key } } else { StoragePreferenceEdit::Set { key, value: mutation.value } });
                }
                Ok(edits)
            })();
            let edits = match edits {
                Ok(edits) => edits,
                Err(error) => {
                    self.request(StorageRequest::PreferencesCancelEdit { handle, token })?;
                    return Err(error);
                }
            };
            let schema_version = started["snapshot"]["schemaVersion"].as_u64().ok_or("Preference schema version is missing")? as u32;
            self.request(StorageRequest::PreferencesFinishEdit { handle, token, edits, schema_version })?;
        }
        if !memory.is_empty() { self.request(StorageRequest::Commit { handle: self.handle.clone(), mutations: memory, guards: Vec::new() })?; }
        Ok(())
    }

    /// Removes one exact imported object without overwriting concurrent plugin edits.
    pub fn remove(&self, collection: &str, key: &str, version: &str) -> Result<(), String> {
        self.commit(vec![StorageMutation {
            collection: collection.into(), key: key.into(), value: Value::Null,
            deleted: true, check_version: true, expected_version: Some(version.into()),
        }])
    }

    /// Adopts one complete JSON object without replacing a different saved record.
    pub fn adoptRecord(&self, collection: &str, key: &str, value: Value) -> Result<(), String> {
        if let Some(existing) = self.get(collection, key)? {
            if existing["value"] != value {
                return Err(format!("Import would replace {collection}/{key}"));
            }
            return Ok(());
        }
        self.commit(vec![self.mutation(collection, key, value.clone())?])?;
        Ok(())
    }

    /// Writes an explicit migration with a durable in-progress marker and a final completion marker.
    /// Restarting an interrupted migration repeats source records; a completed migration never runs twice.
    pub fn import(&self, source: &str, mut data: CharacterMigrationData) -> Result<(), String> {
        if self.get("imports", source)?.is_some() {
            return Ok(());
        }
        for card in &mut data.cards {
            migrateCharacterModelBinding(card);
        }
        let dropped = dropUnresolvableRecords(&mut data);
        if dropped != DroppedLegacyRecords::default() {
            AppLogger::w(
                "CharacterPluginMigration",
                &format!(
                    "source={source} removed {} tag attachments, {} shared memory mounts, {} group members, {} relationships and {} document chunks with missing legacy targets",
                    dropped.tagAttachments, dropped.sharedMemoryMounts, dropped.groupMembers, dropped.relationships, dropped.documentChunks
                ),
            );
        }
        validateMigrationData(&data)?;
        let existing = self.get(META_COLLECTION, "state")?;
        let mut metadata = match existing {
            Some(entry) => entry["value"].clone(),
            None => json!({"version":1,"active":null}),
        };
        if metadata["version"] != 1 { return Err("Unsupported character schema version".into()); }
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
        // Released identities remain stable; the new plugin has no privileged default-card field.
        for card in data.cards {
            let mut value = serde_json::to_value(&card).map_err(|error| error.to_string())?;
            value.as_object_mut().ok_or("Character must be an object")?.remove("isDefault");
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
        let mut identities = BTreeSet::new();
        for (collection, key, _) in &rows {
            if !identities.insert((collection, key)) {
                return Err(format!("Duplicate imported record: {collection}/{key}"));
            }
        }
        if metadata["active"].is_null() {
            return Err("Character migration has no active selection".to_string());
        }
        metadata["migrationStatus"] = json!("writing");
        metadata["migrationSource"] = json!(source);
        self.commit(vec![self.mutation(
            META_COLLECTION,
            "state",
            metadata.clone(),
        )?])?;
        for (owner, bytes) in data.documents {
            let target = format!("{}/{}", self.directory, ownerDocumentPath(&owner));
            operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore::new(
                self.storage.clone(),
                RUNTIME_SYNC_DIR_PATH,
            )
            .writeBytes(&target, &bytes)?;
        }
        for page in rows.chunks(999) {
            let mut mutations = Vec::new();
            for (collection, key, value) in page {
                mutations.push(self.mutation(collection, key, value.clone())?);
            }
            mutations.push(self.mutation(META_COLLECTION, "state", metadata.clone())?);
            self.commit(mutations)?;
        }
        metadata["migrationStatus"] = json!("complete");
        self.commit(vec![
            self.mutation(META_COLLECTION, "state", metadata)?,
            self.mutation(
                "imports",
                source,
                json!({"complete":true}),
            )?,
        ])?;
        super::CharacterPluginLayout::prepareMetadata(self.storage.clone(),&format!("{}/meta.preferences.json",self.directory))
    }
}

impl Drop for CharacterPluginWriter {
    /// Releases the retained migration handle independently of its database's persistent lifetime.
    fn drop(&mut self) {
        for handle in self.preferences.values() {
            let _ = self.request(StorageRequest::Close { handle: handle.clone() });
        }
        let _ = self.request(StorageRequest::Close {
            handle: self.handle.clone(),
        });
    }
}

/// Converts a verified storage identity into the public Files API path used by plugin records.
pub(crate) fn publicResourcePath(path: &str) -> Result<String, String> {
    for (storage, public) in [
        ("runtime/", "/operitapp/data"),
        ("workspaces/", "/operitapp/workspaces"),
    ] {
        if let Some(relative) = path.strip_prefix(storage) {
            return operit_tools::files::PathMapper::PathMapper::joinVfsPath(public, relative);
        }
    }
    Err(format!("Resource is outside public runtime roots: {path}"))
}

/// Counts legacy references removed because their target is absent from the same snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DroppedLegacyRecords {
    pub tagAttachments: usize,
    pub sharedMemoryMounts: usize,
    pub groupMembers: usize,
    pub relationships: usize,
    pub documentChunks: usize,
}

/// Removes stale list references while retaining role identities and primary selections for user rebinding.
pub fn dropUnresolvableRecords(data: &mut CharacterMigrationData) -> DroppedLegacyRecords {
    let mut dropped = DroppedLegacyRecords::default();
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
    for card in &mut data.cards {
        let card_id = &card.id;
        card.attachedTagIds.retain(|tag_id| {
            if tags.contains(tag_id.as_str()) {
                return true;
            }
            AppLogger::w(
                "CharacterPluginMigration",
                &format!("card={card_id} removed reference to missing legacy tagId={tag_id}"),
            );
            dropped.tagAttachments += 1;
            false
        });
        card.sharedMemoryMounts.retain(|mount| {
            if stores.contains(mount.sharedMemoryId.as_str()) {
                return true;
            }
            AppLogger::w(
                "CharacterPluginMigration",
                &format!(
                    "card={card_id} removed mount to missing legacy sharedMemoryId={}",
                    mount.sharedMemoryId
                ),
            );
            dropped.sharedMemoryMounts += 1;
            false
        });
    }
    let cards = data
        .cards
        .iter()
        .map(|card| card.id.as_str())
        .collect::<BTreeSet<_>>();
    for group in &mut data.groups {
        let group_id = &group.id;
        group.members.retain(|member| {
            if cards.contains(member.characterCardId.as_str()) {
                return true;
            }
            AppLogger::w(
                "CharacterPluginMigration",
                &format!(
                    "group={group_id} removed reference to missing legacy characterCardId={}",
                    member.characterCardId
                ),
            );
            dropped.groupMembers += 1;
            false
        });
    }
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

/// Converts the documented legacy fixed mode while retaining unresolved selections for user rebinding.
fn migrateCharacterModelBinding(card: &mut CharacterCard) {
    if card.chatModelBindingMode == "FIXED_CONFIG" {
        card.chatModelBindingMode = "FIXED_MODEL".to_string();
    }
}

/// Requires only a readable active selection; deleted reference targets are cleaned independently.
fn validateMigrationData(data: &CharacterMigrationData) -> Result<(), String> {
    let active = data
        .active
        .as_ref()
        .ok_or("Imported active selection is missing")?;
    let active = active.as_object().ok_or("Invalid imported active selection")?;
    if active.len() != 1 {
        return Err("Invalid imported active selection".to_string());
    }
    // Preserve deleted role selections exactly; explicit activation and execution validate their targets.
    let selection = active
        .get("CharacterCard")
        .or_else(|| active.get("CharacterGroup"))
        .ok_or("Invalid imported active selection")?;
    if requiredString(selection, "id")?.is_empty() {
        return Err("Imported active selection identity is empty".to_string());
    }
    Ok(())
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

fn validateGlobalMemoryPreferences(value: &Value) -> Result<(), String> {
    if !value["userDescriptionEnabled"].is_boolean() || !value["autoUpdateEnabled"].is_boolean() {
        return Err("Invalid global memory preferences".to_string());
    }
    Ok(())
}

/// Resolves unique historical targets and leaves missing or ambiguous references unbound.
pub fn resolveBinding(
    cards: &[Value],
    groups: &[Value],
    old: &Value,
) -> Result<Option<Value>, String> {
    let selection = if let Some(id) = old["characterGroupId"].as_str().filter(|id| !id.is_empty()) {
        if groups.iter().filter(|group| group["id"] == id).count() != 1 {
            return Ok(None);
        }
        format!("group:{id}")
    } else {
        let Some(name) = old["characterCardName"]
            .as_str()
            .filter(|name| !name.is_empty())
        else {
            return Ok(Some(json!({"version":1,"selection":"model:chat"})));
        };
        let matches: Vec<_> = cards.iter().filter(|card| card["name"] == name).collect();
        if matches.len() != 1 {
            return Ok(None);
        }
        format!("card:{}", requiredString(matches[0], "id")?)
    };
    Ok(Some(json!({"version":1,"selection":selection})))
}

/// Retains the exact source target and a complete independent theme for restartable adoption.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ThemeBinding {
    collection: String,
    key: String,
    config: ThemeConfig,
}

/// Holds allocated theme identities until migration completes.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ThemeMigrationPlan {
    bindings: Vec<ThemeBinding>,
    activeConfigId: Option<String>,
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
    /// Imports role themes from the Operit1 snapshot without changing source preferences.
    pub fn migrateThemes(
        &self,
        source: &str,
        source_preferences: &Preferences,
        targets: &[(String, String)],
    ) -> Result<(), String> {
        let marker_key = format!("{source}:themes");
        let plan: ThemeMigrationPlan = match self.get("imports", &marker_key)? {
            Some(entry) if entry["value"]["complete"] == true => return Ok(()),
            Some(entry) => {
                serde_json::from_value(entry["value"].clone()).map_err(|error| error.to_string())?
            }
            None => {
                let plan = self.planThemes(source_preferences, targets)?;
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
        let mut rows = Vec::new();
        let mut configs = Vec::new();
        for binding in &plan.bindings {
            let Some(entry) = self.get(&binding.collection, &binding.key)? else {
                continue;
            };
            let mut role = entry["value"].clone();
            match role.get("themeConfigId") {
                Some(Value::Null) => role["themeConfigId"] = json!(binding.config.id),
                Some(Value::String(id)) if id == &binding.config.id => {}
                _ => continue,
            }
            configs.push(binding.config.clone());
            rows.push((binding.collection.clone(), binding.key.clone(), role));
        }
        let retained = configs
            .iter()
            .map(|config| config.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut abandoned = plan
            .bindings
            .iter()
            .filter(|binding| !retained.contains(binding.config.id.as_str()))
            .map(|binding| binding.config.clone())
            .collect::<Vec<_>>();
        if !abandoned.is_empty() {
            let mut referenced = BTreeSet::new();
            for collection in ["cards", "groups"] {
                for entry in self.list(collection)? {
                    if let Some(id) = entry["value"]["themeConfigId"].as_str() {
                        referenced.insert(id.to_string());
                    }
                }
            }
            abandoned.retain(|config| !referenced.contains(&config.id));
        }
        let manager = ThemeConfigManager::new(self.storage.clone());
        let imported = manager
            .importLegacyConfigurations(&configs, plan.activeConfigId.as_deref(), &abandoned)
            .map_err(|error| error.to_string())?;
        rows.retain(|(_, _, role)| {
            role["themeConfigId"]
                .as_str()
                .is_some_and(|id| imported.contains(id))
        });
        for page in rows.chunks(1000) {
            let mutations = page
                .iter()
                .map(|(collection, key, role)| self.mutation(collection, key, role.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            self.commit(mutations)?;
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
        preferences: &Preferences,
        targets: &[(String, String)],
    ) -> Result<ThemeMigrationPlan, String> {
        let state = self.get("meta", "state")?;
        let active = state.as_ref().map(|entry| &entry["value"]["active"]);
        let mut plan = ThemeMigrationPlan {
            bindings: Vec::new(),
            activeConfigId: None,
            complete: false,
        };
        let mut identities = BTreeSet::new();
        for (collection, id) in targets {
            if !identities.insert((collection, id)) {
                continue;
            }
            let Some(entry) = self.get(collection, id)? else {
                continue;
            };
            let role = &entry["value"];
            if role.get("themeConfigId") != Some(&Value::Null) {
                continue;
            }
            let Ok(prefix) = themePrefix(collection, id) else {
                continue;
            };
            let Ok((snapshot, _)) = readLegacyCharacterThemeSnapshot(preferences, &prefix) else {
                continue;
            };
            let (Some(name), Some(createdAt), Some(updatedAt)) = (
                role["name"].as_str(),
                role["createdAt"].as_i64(),
                role["updatedAt"].as_i64(),
            ) else {
                continue;
            };
            let config = ThemeConfig {
                // Persist the allocated identity before publishing either owner.
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                snapshot,
                createdAt,
                updatedAt,
            };
            if active.is_some_and(|active| {
                (collection == "cards" && active["CharacterCard"]["id"] == *id)
                    || (collection == "groups" && active["CharacterGroup"]["id"] == *id)
            }) {
                plan.activeConfigId = Some(config.id.clone());
            }
            plan.bindings.push(ThemeBinding {
                collection: collection.clone(),
                key: id.clone(),
                config,
            });
        }
        Ok(plan)
    }

}

#[cfg(test)]
#[path = "CharacterPluginMigrationTests.rs"]
mod tests;
