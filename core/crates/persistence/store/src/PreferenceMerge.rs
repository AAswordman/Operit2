//! Executes the shared preference merge declarations for Rust owners and plugins.
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_host_api::PluginStorage::{
    StoragePreferencesMergeCatalog, StoragePreferencesMergeDescriptor,
    StoragePreferencesMergeKey, StoragePreferencesMergePathSegment,
};
pub use operit_host_api::PluginStorage::PLUGIN_MERGE_DECLARATION_KEY;
use serde_json::Value;
use crate::MergeRules::uniqueNames;

use crate::PreferencesDataStore::{
    stringPreferencesKey, Preferences, PreferencesDataStore, PreferencesDataStoreError,
    PreferencesSyncedEntry,
};
use crate::SyncOperationStore::SyncOperation;

/// Selects immutable merge declarations and explicit identity decisions without decoding business values.
pub(crate) fn isMergeMetadata(operation: &SyncOperation) -> bool {
    operation.domain == "preferences" && (operation.operation == "alias"
        || operation.payload["key"] == PLUGIN_MERGE_DECLARATION_KEY
        || operation.payload["entries"].as_array().is_some_and(|entries| {
            entries.iter().any(|entry| entry["key"] == PLUGIN_MERGE_DECLARATION_KEY)
        }))
}

/// Binds the shared data declaration to its owning preference store.
#[derive(Clone, Debug)]
pub struct PreferenceMergeStore {
    pub path: String,
    pub encrypted: bool,
    pub open: fn(Arc<dyn RuntimeStorageHost>, &str) -> PreferencesDataStore,
    pub decode: Option<fn(&mut PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError>>,
    pub merge: StoragePreferencesMergeDescriptor,
}

impl PreferenceMergeStore {
    /// Declares an owner of ordinary typed values without catalog or reference side effects.
    pub fn values(
        path: impl Into<String>,
        encrypted: bool,
        open: fn(Arc<dyn RuntimeStorageHost>, &str) -> PreferencesDataStore,
        decode: fn(&mut PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError>,
    ) -> Self {
        Self {
            path: path.into(),
            encrypted,
            open,
            decode: Some(decode),
            merge: StoragePreferencesMergeDescriptor {
                version: 1,
                catalog: None,
                references: Vec::new(),
                projection: None,
            },
        }
    }
}

/// Collects store ownership without introducing another merge declaration model.
#[derive(Clone, Debug, Default)]
pub struct PreferenceMergeSchema {
    pub stores: Vec<PreferenceMergeStore>,
}

impl PreferenceMergeSchema {
    /// Resolves explicit native files and the plugin API's Space-owned preference namespace.
    /// An unrelated file is not an implicit preference owner.
    fn owner(&self, path: &str) -> Result<Option<PreferenceMergeStore>, String> {
        if let Some(owner) = self.stores.iter().find(|store| store.path == path) {
            return Ok(Some(owner.clone()));
        }
        use operit_util::RuntimeStorageLayout::{PLUGIN_DATA_SPACE_DIR_PATH, PLUGIN_DATA_DEVICE_DIR_PATH};
        if ![PLUGIN_DATA_SPACE_DIR_PATH, PLUGIN_DATA_DEVICE_DIR_PATH].iter()
            .any(|root| path.starts_with(&format!("{root}/"))) {
            return Ok(None);
        }
        let canonical = operit_util::PluginStoragePaths::resolveStoragePath(path)?;
        if canonical != path || !path.ends_with(".preferences.json") {
            return Err("Invalid synchronized plugin preference path".into());
        }
        use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership};
        if runtimeStorageOwnership(path)? != RuntimeStorageOwnership::Space {
            return Err("Synchronized preferences require Space-owned storage".into());
        }
        Ok(Some(PreferenceMergeStore {
            path: path.to_owned(), encrypted: false,
            open: |storage, path| PreferencesDataStore::newWithStorage(storage, path),
            decode: None,
            merge: StoragePreferencesMergeDescriptor {
                version: 1, catalog: None, references: Vec::new(), projection: None,
            },
        }))
    }

    /// Iterates the catalogs together with their explicit storage ownership.
    fn catalogs(&self) -> impl Iterator<Item = (&PreferenceMergeStore, &StoragePreferencesMergeCatalog)> {
        self.stores.iter().filter_map(|store| store.merge.catalog.as_ref().map(|catalog| (store, catalog)))
    }
}

/// Validates the shared declaration contract before owner-specific namespace checks.
fn validateDeclaration(descriptor: &StoragePreferencesMergeDescriptor) -> Result<(), String> {
    if descriptor.version != 1 || descriptor.references.len() > 256 {
        return Err("Invalid preference merge declaration".into());
    }
    if let Some(catalog) = &descriptor.catalog {
        if catalog.key_prefix.is_empty() || catalog.identity_field.is_empty()
            || catalog.name_field.is_empty() || catalog.identity_field == catalog.name_field
            || catalog.ignored_fields.iter().any(|field| field == &catalog.name_field)
            || catalog.ignored_fields.len() > 64 || catalog.retired_keys.len() > 64 {
            return Err("Invalid preference merge catalog declaration".into());
        }
    }
    for reference in &descriptor.references {
        if reference.target.is_empty() || reference.target.len() > 512
            || !reference.target.ends_with(".preferences.json")
            || !reference.target.starts_with("runtime/")
            || reference.target.split('/').any(|part| part.is_empty() || part == "." || part == "..")
            || reference.target.chars().any(|character| character.is_control() || character == '\\')
            || reference.path.len() > 32 {
            return Err("Invalid preference merge reference declaration".into());
        }
        match &reference.key {
            StoragePreferencesMergeKey::Exact { key } if key.is_empty() || key == PLUGIN_MERGE_DECLARATION_KEY =>
                return Err("Invalid preference reference key".into()),
            StoragePreferencesMergeKey::Records { prefix, .. } if prefix.is_empty() =>
                return Err("Invalid preference record selector".into()),
            _ => {}
        }
        for (index, segment) in reference.path.iter().enumerate() {
            match segment {
                StoragePreferencesMergePathSegment::Field { name }
                | StoragePreferencesMergePathSegment::Variant { name } if name.is_empty() =>
                    return Err("Invalid preference reference path".into()),
                StoragePreferencesMergePathSegment::Identity { .. } if index + 1 != reference.path.len() =>
                    return Err("Preference identity selector must be terminal".into()),
                _ => {}
            }
        }
    }
    if let Some(projection) = &descriptor.projection {
        if descriptor.catalog.is_none() || projection.selection_key.is_empty()
            || projection.source_field.is_empty() || projection.fields.len() > 256
            || projection.retired_keys.len() > 64 {
            return Err("Invalid preference projection declaration".into());
        }
    }
    Ok(())
}

/// Validates aggregate limits and relationships after every owner's shared contract is checked.
fn validateSchema(schema: &PreferenceMergeSchema) -> Result<(), String> {
    let mut paths = BTreeSet::new();
    let mut catalogs = BTreeMap::new();
    let mut referenceCount = 0;
    let mut projectionCount = 0;
    for store in &schema.stores {
        validateDeclaration(&store.merge)?;
        if store.path.is_empty()
            || store.path.len() > 512
            || store.path.chars().any(char::is_control)
            || !paths.insert(store.path.as_str())
        {
            return Err("Invalid or duplicate preference merge store declaration".into());
        }
        if let Some(catalog) = &store.merge.catalog {
            catalogs.insert(store.path.as_str(), catalog);
        }
        referenceCount += store.merge.references.len();
        projectionCount += usize::from(store.merge.projection.is_some());
    }
    if catalogs.len() > 64 || referenceCount > 256 || projectionCount > 64 {
        return Err("Preference merge declaration exceeds resource limits".into());
    }
    for store in &schema.stores {
        let mut references = BTreeSet::new();
        for reference in &store.merge.references {
            if !references.insert((&reference.key, &reference.path)) {
                return Err("Invalid preference merge reference declaration".into());
            }
        }
        if let Some(projection) = &store.merge.projection {
            if !store.merge.references.iter().any(|reference| {
                reference.key
                    == StoragePreferencesMergeKey::Exact {
                        key: projection.selection_key.clone(),
                    }
                    && reference.path.is_empty()
                    && reference.target == store.path
            }) {
                return Err("Invalid preference projection declaration".into());
            }
            let catalog = catalogs
                .get(store.path.as_str())
                .ok_or("Projection requires a declared entity catalog")?;
            let mut projectedKeys = BTreeSet::new();
            for key in projection.fields.values() {
                if key.is_empty()
                    || key == &projection.selection_key
                    || keyMatches(key, catalog)
                    || !projectedKeys.insert(key)
                {
                    return Err("Invalid or overlapping preference projection field".into());
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
struct ConfigRecord {
    id: String,
    key: String,
    value: Value,
}

/// Checks plugin storage ownership before applying the shared declaration validator.
pub(crate) fn validatePluginDeclaration(path: &str, descriptor: &StoragePreferencesMergeDescriptor) -> Result<(), String> {
    let canonical = operit_util::PluginStoragePaths::resolveStoragePath(path)?;
    if canonical != path || !path.ends_with(".preferences.json") {
        return Err("Invalid plugin preference merge declaration".into());
    }
    validateDeclaration(descriptor)
}

/// Recovers immutable plugin contracts from already decoded preference history.
fn pluginDeclarations(entries: &[PreferencesSyncedEntry]) -> Result<BTreeMap<String, StoragePreferencesMergeDescriptor>, String> {
    let mut declarations = BTreeMap::new();
    for entry in entries {
        if !entry.keys().contains(&PLUGIN_MERGE_DECLARATION_KEY) { continue; }
        let path = entry.storagePath().to_owned();
        if entry.encrypted() { return Err("Plugin preference declarations require an unencrypted plugin store".into()); }
        let mut extracted = false;
        entry.clone().rewriteRootEntries(|key, value| {
            if key != PLUGIN_MERGE_DECLARATION_KEY { return Ok(()); }
            let value = value.ok_or_else(|| PreferencesDataStoreError::Message("Preference merge declarations cannot be removed".into()))?;
            let descriptor: StoragePreferencesMergeDescriptor = serde_json::from_value(value.clone())?;
            validatePluginDeclaration(&path, &descriptor).map_err(PreferencesDataStoreError::Message)?;
            match declarations.entry(path.clone()) {
                Entry::Vacant(entry) => { entry.insert(descriptor); }
                Entry::Occupied(entry) if entry.get() != &descriptor =>
                    return Err(PreferencesDataStoreError::Message("Conflicting preference merge declarations".into())),
                Entry::Occupied(_) => {}
            }
            extracted = true;
            Ok(())
        }).map_err(|error| error.to_string())?;
        if !extracted && entry.identityAlias().is_none() { return Err("Preference merge declarations must be complete immutable records".into()); }
    }
    if declarations.len() > 64 { return Err("Plugin preference merge declaration limit exceeded".into()); }
    Ok(declarations)
}

/// Reports a dataset commit or terminal skip without sharing failure boundaries with other datasets.
pub struct PreferenceMergeOutcome {
    pub paths: BTreeSet<String>,
    pub operations: Vec<SyncOperation>,
    /// Terminally ignores unowned, older or unversioned operations without applying their values.
    pub skipped: bool,
    pub result: Result<BTreeMap<String, String>, String>,
}

/// Applies each dataset independently; stable references do not join commit or failure boundaries.
pub fn applyIndependentOperations<F>(
    storage: Arc<dyn RuntimeStorageHost>,
    schema: &PreferenceMergeSchema,
    history: &[SyncOperation],
    incoming: &[SyncOperation],
    retain: F,
) -> Vec<PreferenceMergeOutcome>
where
    F: Fn(&str, &str) -> bool + Copy,
{
    let mut stores = PreferenceStores {
        storage,
        schema,
        opened: BTreeMap::new(),
    };
    let mut outcomes = Vec::new();
    let mut owners = BTreeMap::<String, Vec<SyncOperation>>::new();
    let mut candidates = BTreeMap::<String, Vec<SyncOperation>>::new();
    let mut pending = BTreeMap::<String, Vec<SyncOperation>>::new();
    let mut versions = BTreeMap::<String, u32>::new();
    let mut skipped = BTreeSet::new();
    for operation in incoming {
        match PreferencesSyncedEntry::operationPath(operation) {
            Ok(path) => candidates.entry(path.to_owned()).or_default().push(operation.clone()),
            Err(error) => outcomes.push(PreferenceMergeOutcome {
                paths: BTreeSet::new(), operations: vec![operation.clone()], skipped: false,
                result: Err(format!("op={}: {error}", operation.opId)),
            }),
        }
    }
    for (path, operations) in candidates {
        let owner = match schema.owner(&path) {
            Ok(Some(owner)) => owner,
            Ok(None) => {
                outcomes.push(PreferenceMergeOutcome {
                    paths: BTreeSet::from([path]), operations, skipped: true,
                    result: Ok(BTreeMap::new()),
                });
                continue;
            }
            Err(error) => {
                outcomes.push(PreferenceMergeOutcome {
                    paths: BTreeSet::from([path]), operations, skipped: false, result: Err(error),
                });
                continue;
            }
        };
        // A malformed transaction rejects that transaction, not independent operations in its file.
        let mut decoded = Vec::new();
        for operation in operations {
            let entry = PreferencesSyncedEntry::fromOperation(&operation)
                .map_err(|error| error.to_string())
                .and_then(|entry| if entry.encrypted() == owner.encrypted { Ok(entry) }
                    else { Err(format!("Conflicting preference encryption policy: {path}")) });
            match entry {
                Ok(entry) => decoded.push((operation, entry)),
                Err(error) => {
                    skipped.insert(path.clone());
                    outcomes.push(PreferenceMergeOutcome {
                        paths: BTreeSet::from([path.clone()]), operations: vec![operation],
                        skipped: false, result: Err(error),
                    });
                }
            }
        }
        if decoded.is_empty() { continue; }
        let version = match mergeAttempt(&path, || stores.version(&path, owner.encrypted)) {
            Ok(version) => version,
            Err(error) => {
                skipped.insert(path.clone());
                outcomes.push(PreferenceMergeOutcome {
                    paths: BTreeSet::from([path]), operations: decoded.into_iter().map(|(op, _)| op).collect(),
                    skipped: false, result: Err(error),
                });
                continue;
            }
        };
        versions.insert(path.clone(), version);
        for (operation, entry) in decoded {
            if entry.schemaVersion().is_none_or(|remote| remote < version) {
                skipped.insert(path.clone());
                outcomes.push(PreferenceMergeOutcome {
                    paths: BTreeSet::from([path.clone()]), operations: vec![operation],
                    skipped: true, result: Ok(BTreeMap::new()),
                });
                continue;
            }
            match entry.requireSchemaVersion(version) {
                Ok(()) => pending.entry(path.clone()).or_default().push(operation),
                Err(error) => {
                    skipped.insert(path.clone());
                    outcomes.push(PreferenceMergeOutcome {
                        paths: BTreeSet::from([path.clone()]), operations: vec![operation],
                        skipped: false, result: Err(error.to_string()),
                    });
                }
            }
        }
    }
    // History is committed context, not a request to rematerialize every dataset.
    // Catalogs and reference owners participate in identity projection; ordinary
    // datasets only participate when this batch actually changes them.
    let preparing = history.is_empty() && incoming.is_empty();
    let mut participating = pending.keys().cloned().collect::<BTreeSet<_>>();
    participating.extend(schema.stores.iter().filter(|store| {
        preparing || store.merge.catalog.is_some() || !store.merge.references.is_empty()
    }).map(|store| store.path.clone()));
    for operation in history.iter().chain(incoming) {
        if operation.domain != "preferences" { continue; }
        let declaresMerge = operation.payload["key"] == PLUGIN_MERGE_DECLARATION_KEY
            || operation.payload["entries"].as_array().is_some_and(|entries| {
                entries.iter().any(|entry| entry["key"] == PLUGIN_MERGE_DECLARATION_KEY)
            });
        if declaresMerge {
            if let Ok(entry) = PreferencesSyncedEntry::fromOperation(operation) {
                if stores.version(entry.storagePath(), entry.encrypted())
                    .is_ok_and(|version| entry.schemaVersion() == Some(version)) {
                    participating.insert(entry.storagePath().to_owned());
                }
            }
        }
    }
    // A rejected operation cannot trigger preparation or reference projection for its dataset.
    participating.retain(|path| !skipped.contains(path) || pending.contains_key(path));
    for operation in history
        .iter()
        .chain(pending.values().flatten())
        .filter(|operation| operation.domain == "preferences")
    {
        if let Some(path) = operation.payload.get("storagePath").and_then(Value::as_str)
            .filter(|path| participating.contains(*path)) {
            let compatible = versions.get(path).copied().or_else(|| {
                let entry = PreferencesSyncedEntry::fromOperation(operation).ok()?;
                stores.version(path, entry.encrypted()).ok()
            }).is_some_and(|version| operation.payload.get("schemaVersion").and_then(Value::as_u64) == Some(u64::from(version)));
            if compatible {
                owners.entry(path.into()).or_default().push(operation.clone());
            }
        }
    }
    for path in participating {
        owners.entry(path).or_default();
    }
    let mut aliases = BTreeMap::new();
    let mut committed = Vec::new();
    for (path, history) in owners {
        let operations = pending.remove(&path).unwrap_or_default();
        let index = outcomes.len();
        let result = mergeAttempt(&path, || applyDeclaredOperations(
            &mut stores, &path, &history, &BTreeMap::new(), retain,
        ));
        if let Ok(decisions) = &result {
            aliases.extend(decisions.clone());
            committed.push((index, path.clone(), history));
        }
        outcomes.push(PreferenceMergeOutcome {
            paths: BTreeSet::from([path]),
            operations,
            skipped: false,
            result,
        });
    }
    // Reference projection uses only alias decisions whose owning dataset has committed.
    if !aliases.is_empty() {
        for (index, path, history) in committed {
            outcomes[index].result = mergeAttempt(&path, || applyDeclaredOperations(
                &mut stores, &path, &history, &aliases, retain,
            ));
        }
    }
    outcomes
}

/// Owner callbacks may panic; keep the other datasets and the next attempt usable.
fn mergeAttempt<T>(path: &str, operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation))
        .map_err(|payload| {
            let detail = payload.downcast_ref::<String>().map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied()).unwrap_or("non-string panic");
            format!("Preference merge panicked path={path}: {detail}")
        })?
}

/// Reuses each owner's DataStore throughout one batch, including reference projection.
struct PreferenceStores<'a> {
    storage: Arc<dyn RuntimeStorageHost>,
    schema: &'a PreferenceMergeSchema,
    opened: BTreeMap<String, (bool, PreferencesDataStore)>,
}

impl PreferenceStores<'_> {
    /// Opens one store per path while preserving its declared encryption policy.
    fn open(&mut self, path: &str, encrypted: bool) -> Result<&PreferencesDataStore, String> {
        let (expected, store) = match self.opened.entry(path.to_owned()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let owner = self.schema.owner(path)?
                    .ok_or_else(|| format!("Undeclared preference merge owner: {path}"))?;
                if owner.encrypted != encrypted {
                    return Err(format!("Conflicting preference encryption policy: {path}"));
                }
                let store = (owner.open)(self.storage.clone(), path);
                entry.insert((encrypted, store))
            }
        };
        if *expected != encrypted {
            return Err(format!("Conflicting preference encryption policy: {path}"));
        }
        Ok(store)
    }

    /// Checks the replay version without creating a file or committing schema-only state.
    fn version(&mut self, path: &str, encrypted: bool) -> Result<u32, String> {
        self.open(path, encrypted)?
            .syncSchemaVersion()
            .map_err(|error| error.to_string())
    }
}

/// Applies all requested datasets while preserving a separate durable commit for each owner.
pub fn applyOperations<F>(
    storage: Arc<dyn RuntimeStorageHost>,
    schema: &PreferenceMergeSchema,
    history: &[SyncOperation],
    incoming: &[SyncOperation],
    retain: F,
) -> Result<BTreeMap<String, String>, String>
where
    F: Fn(&str, &str) -> bool + Copy,
{
    let mut references = BTreeMap::new();
    let mut errors = Vec::new();
    for outcome in applyIndependentOperations(storage, schema, history, incoming, retain) {
        match outcome.result {
            Ok(map) => references.extend(map),
            Err(error) => errors.push(format!(
                "{}: {error}",
                outcome.paths.into_iter().collect::<Vec<_>>().join(",")
            )),
        }
    }
    if errors.is_empty() {
        Ok(references)
    } else {
        Err(errors.join("; "))
    }
}

/// Decodes historical representations with the owner contract before staging a single dataset.
fn prepareOwner(
    schema: &PreferenceMergeSchema,
    path: &str,
    history: &[SyncOperation],
) -> Result<(PreferenceMergeSchema, Vec<PreferencesSyncedEntry>), String> {
    let batch = PreferencesDataStore::prepareSyncedEntries(&[], history)
        .map_err(|error| error.to_string())?;
    let mut entries = batch
        .into_iter()
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    let declarations = pluginDeclarations(&entries)?;
    let mut owner = PreferenceMergeSchema {
        stores: schema
            .stores
            .iter()
            .filter(|store| store.path == path)
            .cloned()
            .collect(),
    };
    for (path, merge) in declarations {
        owner.stores.push(PreferenceMergeStore {
            path,
            encrypted: false,
            open: |storage, path| PreferencesDataStore::newWithStorage(storage, path),
            decode: None,
            merge,
        });
    }
    validateSchema(&owner)?;
    for entry in &mut entries {
        if let Some(decode) = owner
            .stores
            .iter()
            .find(|store| store.path == path)
            .and_then(|store| store.decode)
        {
            decode(entry).map_err(|error| error.to_string())?;
        }
    }
    Ok((owner, entries))
}

/// Resolves only explicitly recorded alias decisions, never inferring identity from content.
fn declaredAliases(
    schema: &PreferenceMergeSchema,
    entries: &[PreferencesSyncedEntry],
) -> Result<BTreeMap<String, String>, String> {
    let mut aliases = BTreeMap::new();
    for entry in entries {
        let Some((source, target)) = entry.identityAlias() else {
            continue;
        };
        let (store, catalog) = schema
            .catalogs()
            .find(|(store, _)| store.path == entry.storagePath())
            .ok_or("Identity alias requires a declared catalog")?;
        let source = catalogIdentity(source, catalog)?;
        let target = catalogIdentity(target, catalog)?;
        let canonical = resolveIdentity(&store.path, target, &aliases)?;
        if source != canonical {
            aliases.insert(referenceKey(&store.path, source), canonical);
        }
    }
    for key in aliases.keys() {
        let (path, id) = key
            .split_once('\u{001f}')
            .ok_or("Invalid alias namespace")?;
        resolveIdentity(path, id, &aliases)?;
    }
    Ok(aliases)
}

/// Replays one dataset and commits its business state, original identities and archive intent together.
fn applyDeclaredOperations<F>(
    stores: &mut PreferenceStores<'_>,
    path: &str,
    incoming: &[SyncOperation],
    externalAliases: &BTreeMap<String, String>,
    retain: F,
) -> Result<BTreeMap<String, String>, String>
where
    F: Fn(&str, &str) -> bool,
{
    let schema = stores.schema;
    validateSchema(&PreferenceMergeSchema {
        stores: schema.stores.iter().filter(|store| store.path == path).cloned().collect(),
    })?;
    let encrypted = match incoming.first() {
        Some(operation) => PreferencesSyncedEntry::fromOperation(operation)
            .map_err(|error| error.to_string())?.encrypted(),
        None => match schema.stores.iter().find(|store| store.path == path) {
            Some(store) if stores.storage.exists(path).map_err(|error| error.to_string())? => store.encrypted,
            _ => return Ok(BTreeMap::new()),
        },
    };
    let store = stores.open(path, encrypted)?;
    let version = store.syncSchemaVersion().map_err(|error| error.to_string())?;
    store.materializeSyncedDataset(version, incoming,
        |preferences, originals, replay| {
        // A committed dataset only needs declarations and alias decisions for reference projection.
        let metadata;
        let source = if replay { originals } else {
            metadata = originals.iter().filter(|operation| isMergeMetadata(operation)).cloned().collect::<Vec<_>>();
            &metadata
        };
        let (owner, entries) = prepareOwner(schema, path, source).map_err(PreferencesDataStoreError::Message)?;
        let schema = &owner;
        if entries.iter().any(|entry| entry.storagePath() != path || entry.encrypted() != encrypted)
            || schema.stores.iter().any(|store| store.path != path || store.encrypted != encrypted) {
            return Err(PreferencesDataStoreError::Message("Conflicting preference dataset or encryption policy".into()));
        }
        let mut aliases = externalAliases.clone();
        aliases.extend(declaredAliases(schema, &entries).map_err(PreferencesDataStoreError::Message)?);
        let mut entries = entries.into_iter().filter(|entry| entry.identityAlias().is_none()).collect::<Vec<_>>();
        for entry in &mut entries {
            if let Some((store, catalog)) = schema.catalogs().next() {
                entry.rewriteRootEntries(|key, value| {
                    if !keyMatches(key, catalog) { return Ok(()); }
                    let id = catalogIdentity(key, catalog).map_err(PreferencesDataStoreError::Message)?;
                    let canonical = resolveIdentity(&store.path, id, &aliases).map_err(PreferencesDataStoreError::Message)?;
                    if let Some(value) = value {
                        if value.get(&catalog.identity_field).and_then(Value::as_str) != Some(id) {
                            return Err(PreferencesDataStoreError::Message(format!(
                                "Entity identity differs from its key: path={} key={key} identityField={} expectedId={id}", store.path, catalog.identity_field)));
                        }
                        value[&catalog.identity_field] = Value::String(canonical.clone());
                    }
                    *key = format!("{}{}{}", catalog.key_prefix, canonical, catalog.key_suffix);
                    Ok(())
                })?;
            }
        }
        if replay {
            PreferencesSyncedEntry::replay(&entries, preferences)?;
        }
        for (store, catalog) in schema.catalogs() {
            let namespace = format!("{}\u{001f}", store.path);
            for source in aliases.keys().filter_map(|key| key.strip_prefix(&namespace)) {
                preferences.remove(&stringPreferencesKey(&format!(
                    "{}{}{}", catalog.key_prefix, source, catalog.key_suffix
                )));
            }
        }
        let retired = preferences.iterEntries().filter_map(|(key, _)| {
            (!retain(&path, key) || schema.catalogs().any(|(_, catalog)| catalog.retired_keys.iter().any(|retired| retired == key)))
                .then(|| key.to_owned())
        }).collect::<Vec<_>>();
        for key in retired { preferences.remove(&stringPreferencesKey(&key)); }
        let mut snapshots = BTreeMap::from([(path.to_owned(), preferences.clone())]);
        rewriteReferences(&mut snapshots, schema, &aliases)?;
        *preferences = snapshots.remove(path).expect("dataset snapshot");
        for (_, catalog) in schema.catalogs() {
            let records = currentRecords(preferences, catalog).map_err(PreferencesDataStoreError::Message)?;
            let records = nameRecords(records, catalog).map_err(PreferencesDataStoreError::Message)?;
            writeRecords(preferences, catalog, &records)?;
        }
        for store in &schema.stores {
            let Some(projection) = &store.merge.projection else { continue; };
            let catalog = store.merge.catalog.as_ref().expect("validated projection catalog");
            match preferences.get(&stringPreferencesKey(&projection.selection_key)).cloned() {
                None | Some(Value::Null) => continue,
                Some(Value::String(id)) => {
                    let key = format!("{}{}{}", catalog.key_prefix, id, catalog.key_suffix);
                    let source = preferences.get(&stringPreferencesKey(&key))
                        .and_then(|record| record.get(&projection.source_field)).and_then(Value::as_object).cloned()
                        .ok_or_else(|| PreferencesDataStoreError::Message("Selected entity projection source is missing".into()))?;
                    for (field, key) in &projection.fields {
                        match source.get(field) {
                            None | Some(Value::Null) => { preferences.remove(&stringPreferencesKey(key)); }
                            Some(value) if value.is_boolean() || value.is_number() || value.is_string() => {
                                preferences.set(&stringPreferencesKey(key), value.clone());
                            }
                            _ => return Err(PreferencesDataStoreError::Message("Projection requires scalar fields".into())),
                        }
                    }
                    for key in &projection.retired_keys { preferences.remove(&stringPreferencesKey(key)); }
                }
                _ => return Err(PreferencesDataStoreError::Message("Projection selection must be an identity or null".into())),
            }
        }
        Ok(aliases.clone())
    }).map_err(|error| error.to_string())
}

/// Rewrites declared references using the executor's canonical identity map.
fn rewriteReferences(
    staged: &mut BTreeMap<String, Preferences>,
    schema: &PreferenceMergeSchema,
    referenceMap: &BTreeMap<String, String>,
) -> Result<(), PreferencesDataStoreError> {
    for store in &schema.stores {
        for reference in &store.merge.references {
            let preferences = staged.get_mut(&store.path).ok_or_else(|| {
                PreferencesDataStoreError::Message("Declared preference reference store is missing".into())
            })?;
            let keys = preferences.iterEntries().filter(|(key, _)| reference.key.matches(key))
                .map(|(key, _)| stringPreferencesKey(key)).collect::<Vec<_>>();
            for key in keys {
                let mut value = preferences.get(&key).cloned().expect("selected preference exists");
                rewriteReferenceValue(&mut value, &reference.path, &reference.target, referenceMap)
                    .map_err(PreferencesDataStoreError::Message)?;
                preferences.set(&key, value);
            }
        }
    }
    Ok(())
}

/// Reads all currently materialized records belonging to one declared configuration catalog.
fn currentRecords(
    preferences: &Preferences,
    descriptor: &StoragePreferencesMergeCatalog,
) -> Result<Vec<ConfigRecord>, String> {
    preferences
        .iterEntries()
        .filter_map(|(key, value)| {
            if !keyMatches(key, descriptor) {
                return None;
            }
            Some((key.to_string(), value.clone()))
        })
        .map(|(key, value)| {
            let id = value
                .get(&descriptor.identity_field)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("configuration {key} has no string id"))?;
            let expectedId = key
                .strip_prefix(descriptor.key_prefix.as_str())
                .and_then(|id| id.strip_suffix(descriptor.key_suffix.as_str()))
                .ok_or_else(|| {
                    format!("configuration key is outside its declared catalog: {key}")
                })?;
            if id != expectedId {
                return Err(format!("configuration identity does not match key: {key}"));
            }
            Ok(ConfigRecord {
                id: id.to_string(),
                key,
                value,
            })
        })
        .collect()
}

/// Projects deterministic display names while preserving every stable entity identity.
fn nameRecords(
    records: Vec<ConfigRecord>,
    descriptor: &StoragePreferencesMergeCatalog,
) -> Result<Vec<ConfigRecord>, String> {
    let names = records
        .iter()
        .map(|record| {
            let name = record
                .value
                .get(&descriptor.name_field)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    format!("Configuration {} has no declared string name", record.id)
                })?;
            Ok((record.id.clone(), name.to_owned()))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let names = uniqueNames(&names);
    records
        .into_iter()
        .map(|mut record| {
            record.value[&descriptor.name_field] = Value::String(names[&record.id].clone());
            Ok(record)
        })
        .collect()
}

/// Extracts an identity only from a validated catalog root key.
fn catalogIdentity<'a>(key: &'a str, catalog: &StoragePreferencesMergeCatalog) -> Result<&'a str, String> {
    if !keyMatches(key, catalog) {
        return Err("Identity alias does not target a declared entity".into());
    }
    key.strip_prefix(catalog.key_prefix.as_str())
        .and_then(|id| id.strip_suffix(catalog.key_suffix.as_str()))
        .ok_or_else(|| "Invalid catalog identity key".into())
}

/// Matches only the declared entity key pattern, excluding retired membership keys.
fn keyMatches(key: &str, descriptor: &StoragePreferencesMergeCatalog) -> bool {
    if descriptor.retired_keys.iter().any(|retired| retired == key) {
        return false;
    }
    key.strip_prefix(descriptor.key_prefix.as_str())
        .and_then(|rest| rest.strip_suffix(descriptor.key_suffix.as_str()))
        .is_some_and(|identity| !identity.is_empty())
}

/// Writes canonical records without modifying unrelated preferences or emitting new operations.
fn writeRecords(
    preferences: &mut Preferences,
    descriptor: &StoragePreferencesMergeCatalog,
    records: &[ConfigRecord],
) -> Result<(), PreferencesDataStoreError> {
    let keys = preferences
        .iterEntries()
        .filter_map(|(key, _)| keyMatches(key, descriptor).then(|| key.to_string()))
        .collect::<Vec<_>>();
    for key in keys {
        preferences.remove(&stringPreferencesKey(&key));
    }
    for key in &descriptor.retired_keys {
        preferences.remove(&stringPreferencesKey(key));
    }
    for record in records {
        preferences.set(&stringPreferencesKey(&record.key), record.value.clone());
    }
    Ok(())
}

/// Follows a bounded explicit selector instead of guessing business fields in arbitrary JSON.
fn rewriteReferenceValue(
    value: &mut Value,
    path: &[StoragePreferencesMergePathSegment],
    target: &str,
    referenceMap: &BTreeMap<String, String>,
) -> Result<(), String> {
    let Some((segment, remaining)) = path.split_first() else {
        match value {
            Value::Null => {}
            Value::String(id) => {
                if let Some(canonicalId) = referenceMap.get(&referenceKey(target, id)) {
                    *id = canonicalId.clone();
                }
            }
            _ => return Err("Declared preference reference must be a string or null".to_string()),
        }
        return Ok(());
    };
    match segment {
        StoragePreferencesMergePathSegment::Field { name: field } => {
            let object = value.as_object_mut().ok_or("Declared preference reference selector requires an object")?;
            let child = object.get_mut(field).ok_or_else(|| {
                format!("Declared preference reference field is missing: {field}")
            })?;
            rewriteReferenceValue(child, remaining, target, referenceMap)?;
        }
        StoragePreferencesMergePathSegment::ObjectValues => {
            let object = value.as_object_mut().ok_or("Declared preference reference selector requires an object")?;
            for child in object.values_mut() {
                rewriteReferenceValue(child, remaining, target, referenceMap)?;
            }
        }
        StoragePreferencesMergePathSegment::ArrayValues => {
            let array = value.as_array_mut().ok_or("Declared preference reference selector requires an array")?;
            for child in array { rewriteReferenceValue(child, remaining, target, referenceMap)?; }
        }
        StoragePreferencesMergePathSegment::Variant { name } => {
            if value.is_null() { return Ok(()); }
            let object = value.as_object_mut().ok_or("Declared preference variant selector requires an object")?;
            if object.len() != 1 { return Err("Declared preference variant requires one discriminant".into()); }
            if let Some(child) = object.get_mut(name) {
                rewriteReferenceValue(child, remaining, target, referenceMap)?;
            }
        }
        StoragePreferencesMergePathSegment::Identity { prefix, suffix } => {
            if !remaining.is_empty() { return Err("Preference identity selector must be terminal".into()); }
            if value.is_null() { return Ok(()); }
            let text = value.as_str().ok_or("Declared preference identity requires a string")?;
            if let Some(id) = text.strip_prefix(prefix.as_str()).and_then(|text| text.strip_suffix(suffix.as_str())) {
                if let Some(mapped) = referenceMap.get(&referenceKey(target, id)) {
                    *value = Value::String(format!("{prefix}{mapped}{suffix}"));
                }
            }
        }
    }
    Ok(())
}

/// Resolves historical aliases transitively and rejects malformed cycles.
fn resolveIdentity(
    path: &str,
    id: &str,
    aliases: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut current = id.to_owned();
    let mut seen = BTreeSet::new();
    while let Some(next) = aliases.get(&referenceKey(path, &current)) {
        if !seen.insert(current.clone()) {
            return Err("Cyclic preference identity mapping".into());
        }
        current = next.clone();
    }
    Ok(current)
}

/// Builds a namespace-scoped key for one canonical identity mapping.
fn referenceKey(path: &str, id: &str) -> String {
    format!("{path}\u{001f}{id}")
}
