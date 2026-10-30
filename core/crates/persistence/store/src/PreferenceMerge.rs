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
    pub merge: StoragePreferencesMergeDescriptor,
}

impl PreferenceMergeStore {
    /// Declares an owner of ordinary typed values without catalog or reference side effects.
    pub fn values(
        path: impl Into<String>,
        encrypted: bool,
        open: fn(Arc<dyn RuntimeStorageHost>, &str) -> PreferencesDataStore,
    ) -> Self {
        Self {
            path: path.into(),
            encrypted,
            open,
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
    /// Resolves native declarations, dynamic Space records and Space-owned plugin preferences.
    /// An unrelated file is not an implicit preference owner.
    pub fn owner(&self, path: &str) -> Result<Option<PreferenceMergeStore>, String> {
        if let Some(owner) = self.stores.iter().find(|store| store.path == path) {
            return Ok(Some(owner.clone()));
        }
        if let Some(owner) = crate::CoreSpaceStore::CoreSpaceStore::preferenceSyncStore(path) {
            return Ok(Some(owner));
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
    /// Layout explicitly retired this target; originals remain in the journal.
    pub skipped: bool,
    pub result: Result<BTreeMap<String, String>, String>,
}

/// Applies each dataset independently; stable references do not join commit or failure boundaries.
pub fn applyIndependentOperations<F>(
    storage: Arc<dyn RuntimeStorageHost>, schema: &PreferenceMergeSchema,
    history: &[SyncOperation], incoming: &[SyncOperation], retain: F,
) -> Vec<PreferenceMergeOutcome>
where F: Fn(&str, &str) -> bool + Copy {
    let result = (|| {
        let mut registry = registry(storage.clone(), schema, history.iter().chain(incoming), retain)?;
        registry.journal.appendOperations(history).map_err(|e|e.to_string())?;
        let mut report = registry.receive(incoming)?;
        if incoming.is_empty() {
            for (path, owner) in &registry.owners {
                if storage.exists(path).map_err(|e|e.to_string())? {
                    report.outcomes.insert(path.clone(),(owner.apply)(&[],&report.aliases));
                }
            }
        }
        Ok::<_,String>(report.outcomes.into_iter().map(|(path,result)| {
            let skipped = registry.is_retired(&path);
            PreferenceMergeOutcome {
                operations: incoming.iter().filter(|op| registry.target(op).as_ref() == Ok(&path)).cloned().collect(),
                paths: BTreeSet::from([path]), skipped, result,
            }
        }).collect::<Vec<_>>())
    })();
    result.unwrap_or_else(|error| vec![PreferenceMergeOutcome { paths: BTreeSet::new(),operations: incoming.to_vec(),skipped: false,result: Err(error) }])
}

/// The owner registry is shared by ordinary synchronization and final comparison/correction.
pub fn registry<'a, 'op, F>(storage: Arc<dyn RuntimeStorageHost>, schema: &'a PreferenceMergeSchema,
    operations: impl IntoIterator<Item=&'op SyncOperation>, retain: F) -> Result<crate::Transaction::kernel::Registry<'a>,String>
where F: Fn(&str,&str)->bool + Copy + 'a {
    use crate::Transaction::kernel::{Registry,Participant};
    validateSchema(schema)?;
    let routes = crate::PreferenceLayout::routes(storage.clone())?;
    let journal = crate::SyncOperationStore::SyncOperationStore::new(storage.clone(),operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH);
    let mut paths = schema.stores.iter().map(|owner|owner.path.clone()).collect::<BTreeSet<_>>();
    paths.extend(journal.preferenceDatasets().map_err(|e|e.to_string())?.into_keys());
    for path in journal.preferenceOwnerPaths().map_err(|e|e.to_string())? {
        paths.insert(crate::Transaction::Migrations::resolve_path(&routes,&path)?);
    }
    for op in operations {
        if op.domain == "preferences" {
            let path = PreferencesSyncedEntry::operationPath(op).map_err(|e|e.to_string())?;
            paths.insert(crate::Transaction::Migrations::resolve_path(&routes,path)?);
        }
    }
    let retired = crate::PreferenceLayout::retirements(storage.clone())?;
    let mut registry = Registry { journal,owners: BTreeMap::new(),sources: vec![],routes,retired,
        locate: Box::new(|op| PreferencesSyncedEntry::operationPath(op).map(str::to_owned).map_err(|e|e.to_string())) };
    for path in paths {
        if registry.is_retired(&path) { continue; }
        let Some(owner) = schema.owner(&path)? else { continue; };
        let store = (owner.open)(storage.clone(),&path);
        let preparation = store.clone();
        let validation = store.clone();
        let application = store.clone();
        let projection = store.clone();
        let snapshot = store.clone();
        let repair = store;
        registry.register(Participant { validate: Some(Box::new(move |operation| {
                let entry = PreferencesSyncedEntry::fromOperation(operation).map_err(|e|e.to_string())?;
                if entry.encrypted() != owner.encrypted { return Err("Conflicting preference encryption policy".into()); }
                entry.requireSchemaVersion(validation.syncSchemaVersion().map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                if owner.merge.catalog.is_some() { declaredAliases(schema, std::slice::from_ref(&entry)).map(|_| ()) }
                else { Ok(()) }
            })), path: path.clone(),
            prepare: Box::new(move || preparation.syncSchemaVersion().map(|_|()).map_err(|e|e.to_string())),
            apply: Box::new(move |operations, aliases| applyDeclaredOperations(&application,schema,operations,aliases,retain)),
            project: Some( Box::new(move |aliases: &BTreeMap<String,String>| {
                applyDeclaredOperations(&projection,schema,&[],aliases,retain).map(|_|())
            }) as Box<dyn Fn(&BTreeMap<String,String>)->Result<(),String> + 'a>),
            snapshot: Some(Box::new(move || snapshot.syncSnapshot().map_err(|e|e.to_string()))),
            repair: Some(Box::new(move |expected, source| repair.repairSyncedSnapshot(expected,source).map_err(|e|e.to_string()))),
        })?;
    }
    Ok(registry)
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
    let mut entries = crate::Transaction::ordered(history.iter().cloned(), |op| &op.opId, crate::Transaction::Revision::fromOperation)?
        .iter().map(PreferencesSyncedEntry::fromOperation).collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    for entry in &mut entries { entry.routeTo(path); }
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
        if let Some(existing) = owner.stores.iter().find(|owner|owner.path == path) {
            if existing.encrypted || existing.merge != merge { return Err("Conflicting preference merge declarations".into()); }
            continue;
        }
        owner.stores.push(PreferenceMergeStore {
            path,
            encrypted: false,
            open: |storage, path| PreferencesDataStore::newWithStorage(storage, path),
            merge,
        });
    }
    validateSchema(&owner)?;
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
    store: &PreferencesDataStore, schema: &PreferenceMergeSchema,
    incoming: &[SyncOperation], externalAliases: &BTreeMap<String,String>, retain: F,
) -> Result<BTreeMap<String,String>,String>
where F: Fn(&str,&str)->bool {
    let path = store.path().to_str().ok_or("Invalid preference path")?;
    let owner = schema.owner(path)?.ok_or("Preference owner is not registered")?;
    let encrypted = owner.encrypted;
    let version = store.syncSchemaVersion().map_err(|e|e.to_string())?;
    for operation in incoming {
        let entry = PreferencesSyncedEntry::fromOperation(operation).map_err(|e|e.to_string())?;
        entry.requireSchemaVersion(version).map_err(|e|e.to_string())?;
        if entry.encrypted() != encrypted { return Err("Conflicting preference encryption policy".into()); }
    }
    crate::Transaction::preference::apply(store, incoming,
        |preferences, originals, replay| {
        // A committed dataset only needs declarations and alias decisions for reference projection.
        let metadata;
        let source = if replay { originals } else {
            metadata = originals.iter().filter(|operation| isMergeMetadata(operation)).cloned().collect::<Vec<_>>();
            &metadata
        };
        let (mut owner, mut entries) = prepareOwner(schema, path, source).map_err(PreferencesDataStoreError::Message)?;
        if !owner.stores.iter().any(|store| store.path == path) {
            if let Some(value) = preferences.get(&stringPreferencesKey(PLUGIN_MERGE_DECLARATION_KEY)) {
                let merge: StoragePreferencesMergeDescriptor = serde_json::from_value(value.clone())?;
                validatePluginDeclaration(path, &merge).map_err(PreferencesDataStoreError::Message)?;
                owner.stores.push(PreferenceMergeStore {
                    path: path.into(), encrypted, open: |storage, path| PreferencesDataStore::newWithStorage(storage, path), merge,
                });
            }
        }
        for entry in &mut entries { entry.routeTo(path); }
        for owner in &mut owner.stores { owner.path = path.into(); }
        store.migrateSyncedEntries(version, &mut entries)?;
        let schema = &owner;
        if entries.iter().any(|entry| entry.storagePath() != path || entry.encrypted() != encrypted)
            || schema.stores.iter().any(|store| store.path != path || store.encrypted != encrypted) {
            return Err(PreferencesDataStoreError::Message("Conflicting preference dataset or encryption policy".into()));
        }
        let mut aliases = preferences.mergeAliases().clone();
        aliases.extend(externalAliases.clone());
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
        // Content identity is declared by the owner catalog; ignored fields are not business identity.
        for (store, catalog) in schema.catalogs() {
            let mut contents = BTreeMap::<String, String>::new();
            let mut records = currentRecords(preferences, catalog).map_err(PreferencesDataStoreError::Message)?;
            records.sort_by(|left, right| left.id.cmp(&right.id));
            for record in records {
                let mut content = record.value.clone();
                let object = content.as_object_mut().expect("validated catalog object");
                object.remove(&catalog.identity_field);
                for field in &catalog.ignored_fields { object.remove(field); }
                let fingerprint = serde_json::to_string(&content)?;
                if let Some(canonical) = contents.get(&fingerprint) {
                    aliases.insert(referenceKey(&store.path, &record.id), canonical.clone());
                } else {
                    contents.insert(fingerprint, record.id);
                }
            }
        }
        preferences.setMergeAliases(aliases.clone());
        for (store, catalog) in schema.catalogs() {
            let namespace = format!("{}\u{001f}", store.path);
            for source in aliases.keys().filter_map(|key| key.strip_prefix(&namespace)) {
                preferences.remove(&stringPreferencesKey(&format!(
                    "{}{}{}", catalog.key_prefix, source, catalog.key_suffix
                )));
            }
        }
        let retired = preferences.iterEntries().filter_map(|(key, _)| {
            (!retain(path, key) || schema.catalogs().any(|(_, catalog)| catalog.retired_keys.iter().any(|retired| retired == key)))
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
