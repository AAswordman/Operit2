//! DataStore adapter: SQL owns replay/coverage; a file carries only unfinished commit intent.
use super::{
    kernel::{FileCommit, OwnerState},
    Data, Journal, Revision, Snapshot,
};
use crate::PreferencesDataStore::{
    self as file, stringPreferencesKey, Preferences, PreferencesDataStore,
    PreferencesDataStoreError as Error, PreferencesSyncedEntry,
};
use crate::SyncOperationStore::SyncOperation;
use std::collections::{BTreeMap, BTreeSet};

fn journal(
    store: &PreferencesDataStore,
) -> Result<&crate::SyncOperationStore::SyncOperationStore, Error> {
    store
        .syncOperationStore
        .as_ref()
        .ok_or_else(|| Error::Message("A node-local file has no transaction owner".into()))
}
fn owner_paths(store: &PreferencesDataStore) -> Result<Vec<String>,Error> {
    let routes = crate::PreferenceLayout::routes(store.storageHost.clone()).map_err(Error::Message)?;
    Ok(std::iter::once(store.storagePath.clone()).chain(routes.into_iter()
        .filter(|(_,target)|target == &store.storagePath).map(|(source,_)|source)).collect())
}

fn data(store: &PreferencesDataStore, preferences: &Preferences) -> Result<Data, Error> {
    let version = PreferencesDataStore::readSchemaVersion(preferences)?;
    let mut result = preferences
        .view
        .clone()
        .unwrap_or_else(|| Data::empty(file::preferenceOwner(&store.storagePath, version)));
    result.owner = file::preferenceOwner(&store.storagePath, version);
    result.values = preferences
        .iterEntries()
        .filter(|(key, _)| *key != "__operit_preferences_schema_version")
        .map(|(key, value)| (key.to_owned(), value.clone()))
        .collect();
    Ok(result)
}
fn preferences(snapshot: &Snapshot) -> Preferences {
    let mut result = file::emptyPreferences();
    for (key, value) in &snapshot.data.values {
        result.set(&stringPreferencesKey(key), value.clone());
    }
    result.set(
        &PreferencesDataStore::schemaVersionKey(),
        snapshot.data.owner.format,
    );
    result.view = Some(snapshot.data.clone());
    result
}
fn state(store: &PreferencesDataStore) -> Result<Option<OwnerState>, Error> {
    let mut owner = journal(store)?.withJournal(|j| j.owner_state(&store.storagePath))?;
    if let Some(owner) = &mut owner {
        if let Some(baseline) = owner.legacy_baseline.take() {
            if !baseline.data.values.is_empty() || !baseline.covered.is_empty() || !baseline.data.deleted.is_empty() {
                let operation = super::compaction::Initialization::from_snapshot(&baseline).operation("preferences",store.encryption.is_some()).map_err(Error::Message)?;
                journal(store)?.appendOperations(&[operation])?;
            }
            journal(store)?.withJournal(|j|j.transaction(|tx|Journal::save_owner(tx,owner)))?;
        }
    }
    Ok(owner)
}

fn replay_start(store: &PreferencesDataStore, history: &[SyncOperation], format: u32) -> Result<Snapshot,Error> {
    let mut selected: Option<(&SyncOperation,super::compaction::Initialization)> = None;
    for operation in history {
        let Some(initialization) = super::compaction::Initialization::decode(operation).map_err(Error::Message)? else { continue; };
        if selected.as_ref().is_none_or(|(previous,data)| initialization.covered.contains(&previous.opId)
            || (!data.covered.contains(&operation.opId) && (&initialization.data.revision,&operation.opId) > (&data.data.revision,&previous.opId))) {
            selected = Some((operation,initialization));
        }
    }
    let Some((original,initialization)) = selected else {
        return Ok(Snapshot { node:journal(store)?.localDeviceId()?,data:Data::empty(file::preferenceOwner(&store.storagePath,format)),covered:BTreeSet::new(),generation:0 });
    };
    let mut snapshot = initialization.snapshot(&journal(store)?.localDeviceId()?,0);
    snapshot.covered.insert(original.opId.clone());
    snapshot.data.relocate(&store.storagePath).map_err(Error::Message)?;
    let source = snapshot.data.clone();
    let mut values = preferences(&snapshot);
    store.migratePreferencesToCurrentSchema(&mut values)?;
    snapshot.data = data(store,&values)?;
    if source.owner.format != snapshot.data.owner.format {
        journal(store)?.withJournal(|j|j.transaction(|tx|format::save_source(tx,&source)))?;
    }
    Ok(snapshot)
}

fn initialization(store: &PreferencesDataStore, snapshot: &Snapshot) -> Result<SyncOperation,Error> {
    super::compaction::Initialization::from_snapshot(snapshot).operation("preferences",store.encryption.is_some()).map_err(Error::Message)
}

/// Complete a file/SQL handoff before reading its replay state or accepting another edit.
pub(crate) fn recover(store: &PreferencesDataStore, values: &mut Preferences) -> Result<(), Error> {
    let Some(commit) = values.commit.clone() else {
        if values.view.is_none() {
            values.view = state(store)?.map(|owner| owner.current.data);
        }
        return Ok(());
    };
    let mut current = Snapshot {
        node: journal(store)?.localDeviceId()?,
        data: data(store, values)?,
        covered: commit.covered.clone(),
        generation: commit.generation,
    };
    current.data.aliases = commit.aliases;
    current.data.deleted = commit.deleted;
    current.data.revision = commit.revision;
    current.data.versions = commit.versions;
    current.data.additions = commit.additions;
    let mut originals = commit.originals.clone();
    if commit.legacy_reset && !originals.iter().any(|op|op.operation == "initialize") {
        originals.push(initialization(store,&current)?);
    }
    journal(store)?.appendOperations(&originals)?;
    current.covered.extend(history_initialization_ids(&originals));
    let owner = OwnerState {
        legacy_baseline: None,
        current: current.clone(),
        format_progress: current.data.owner.format,
        layout_progress: current.data.owner.layout,
    };
    let mut sealed = BTreeSet::new();
    for original in originals.iter().filter(|op|op.operation == "initialize") {
        if let Some(initialization) = super::compaction::Initialization::decode(original).map_err(Error::Message)? {
            sealed.extend(initialization.covered);
        }
    }
    sealed.retain(|id| !originals.iter().any(|op|&op.opId == id));
    let paths = owner_paths(store)?;
    journal(store)?.withJournal(|j| {
        j.transaction(|tx| {
            Journal::save_owner(tx, &owner)?;
            Journal::mark_complete(tx, &commit.completed)?;
            if originals.iter().any(|op|op.operation == "initialize") {
                Journal::retire_covered(tx,&paths,&sealed)?;
            }
            Journal::retire_ids(tx, commit.retired.iter().cloned())?;
            tx.execute(
                "INSERT OR IGNORE INTO transaction_commits(id) VALUES(?1)",
                vec![super::text(&commit.id)],
            )?;
            Ok(())
        })
    })?;
    values.commit = None;
    values.legacy = None;
    values.view = Some(current.data);
    // Failure to remove a finished intent is safe: the next open repeats the same SQL commit.
    match commit.file {
        Some(file) => store.writePlaintextFile(&file)?,
        None => store.writeStoredPreferencesUnlocked(values)?,
    }
    store.publishCommitted(values);
    Ok(())
}

fn save(
    store: &PreferencesDataStore,
    values: &mut Preferences,
    snapshot: Snapshot,
    originals: Vec<SyncOperation>,
    completed: Vec<String>,
    reset: bool,
) -> Result<(), Error> {
    save_file(store, values, snapshot, originals, completed, reset, None, BTreeSet::new())
}

fn save_file(
    store: &PreferencesDataStore, values: &mut Preferences, snapshot: Snapshot,
    originals: Vec<SyncOperation>, completed: Vec<String>, reset: bool, file: Option<String>,
    retired: BTreeSet<String>,
) -> Result<(), Error> {
    let mut originals = originals;
    if reset { originals.push(initialization(store,&snapshot)?); }
    values.view = Some(snapshot.data.clone());
    values.commit = Some(FileCommit {
        id: uuid::Uuid::new_v4().to_string(),
        originals,
        retired,
        completed,
        covered: snapshot.covered,
        aliases: snapshot.data.aliases,
        deleted: snapshot.data.deleted,
        revision: snapshot.data.revision,
        versions: snapshot.data.versions,
        additions: snapshot.data.additions,
        generation: snapshot.generation,
        legacy_reset: false,
        file,
    });
    values.legacy = None;
    store.writeStoredPreferencesUnlocked(values)?;
    store.publishCommitted(values);
    recover(store, values)
}

fn initialize(store: &PreferencesDataStore, values: &mut Preferences) -> Result<OwnerState, Error> {
    recover(store, values)?;
    if let Some(mut existing) = state(store)? {
        if existing.current.data.owner.format != PreferencesDataStore::readSchemaVersion(values)? {
            let history = journal(store)?.archivedPreferenceHistory(&store.storagePath)?;
            let source = replay_start(store,&history,existing.current.data.owner.format)?.data;
            let mut converted = preferences(&existing.current);
            store.migratePreferencesToCurrentSchema(&mut converted)?;
            existing.current.data = data(store,&converted)?;
            existing.format_progress = existing.current.data.owner.format;
            journal(store)?.withJournal(|j| j.transaction(|tx| {
                format::save_source(tx, &source)?;
                Journal::save_owner(tx, &existing)
            }))?;
        }
        values.view = Some(existing.current.data.clone());
        return Ok(existing);
    }
    let history = journal(store)?.archivedPreferenceHistory(&store.storagePath)?;
    let imported = data(store, values)?;
    let mut baseline = Snapshot {
        node: journal(store)?.localDeviceId()?,
        data: imported.clone(),
        covered: if imported.values.is_empty() {
            BTreeSet::new()
        } else {
            history.iter().map(|op| op.opId.clone()).collect()
        },
        generation: 0,
    };
    for revision in history.iter().map(Revision::fromOperation) {
        if baseline
            .data
            .revision
            .as_ref()
            .is_none_or(|previous| super::newer(&revision, previous))
        {
            baseline.data.revision = Some(revision);
        }
    }
    if !imported.values.is_empty() {
        // One immutable import carries a legacy file to an empty peer. Its business
        // time comes from the imported history, never from this device's migration time.
        use sha2::{Digest, Sha256};
        let mutation = file::local_mutation(store, &file::emptyPreferences(), values)?
            .ok_or_else(|| Error::Message("Nonempty import has no mutation".into()))?;
        let identity = format!(
            "~import:{:x}",
            Sha256::digest(serde_json::to_vec(&(
                &imported.owner,
                &imported.values,
                &baseline.covered
            ))?)
        );
        let mut payload = mutation.payload;
        payload["importCoverage"] = serde_json::to_value(&baseline.covered)?;
        payload["contractVersion"] = serde_json::json!(1);
        payload["logicalTime"] =
            serde_json::json!(baseline.data.revision.as_ref().map_or(1, |r| r.createdAt));
        let original = SyncOperation {
            opId: format!("{identity}:1"),
            originDeviceId: identity,
            sequence: 1,
            domain: mutation.domain,
            entityType: mutation.entityType,
            entityId: mutation.entityId,
            operation: mutation.operation,
            semantics: mutation.semantics,
            payload,
            createdAt: baseline.data.revision.as_ref().map_or(1, |r| r.createdAt),
            schemaVersion: 1,
        };
        baseline.covered.insert(original.opId.clone());
        baseline
            .data
            .apply(&PreferencesSyncedEntry::fromOperation(&original)?.transaction()?)
            .map_err(Error::Message)?;
        let owner = OwnerState {
            legacy_baseline: None,
            current: baseline.clone(),
            format_progress: baseline.data.owner.format,
            layout_progress: 1,
        };
        // Publish the import through the same recoverable file/SQL handoff as edits.
        journal(store)?.withJournal(|j| j.transaction(|tx| Journal::save_owner(tx, &owner)))?;
        save(store, values, baseline, vec![original], vec![], false)?;
        return Ok(owner);
    }
    let owner = OwnerState {
        legacy_baseline: None,
        current: baseline,
        format_progress: PreferencesDataStore::readSchemaVersion(values)?,
        layout_progress: 1,
    };
    journal(store)?.withJournal(|j| j.transaction(|tx| Journal::save_owner(tx, &owner)))?;
    values.view = Some(owner.current.data.clone());
    Ok(owner)
}

/// Publishes a portable format contract as migrated state, preserving business order and coverage.
pub(crate) fn declare_format(store: &PreferencesDataStore, key: &str, value: serde_json::Value) -> Result<(), Error> {
    let _lock = store.lockTransaction();
    store.loadUnlocked()?;
    let mut values = store.loadedPreferences().expect("loaded file");
    if let Some(existing) = values.get(&stringPreferencesKey(key)) {
        crate::PluginPreferenceSchema::combine(&serde_json::from_value(existing.clone())?,&serde_json::from_value(value.clone())?)?;
    }
    let migrated = store.migratePreferencesToCurrentSchema(&mut values)?;
    let mut current = initialize(store,&mut values)?.current;
    if !migrated && values.get(&stringPreferencesKey(key)) == Some(&value)
        && current.data.owner.format == PreferencesDataStore::readSchemaVersion(&values)? { return Ok(()); }
    values.set(&stringPreferencesKey(key),value);
    current.data = data(store,&values)?;
    current.generation += 1;
    save(store,&mut values,current,vec![],vec![],true)?;
    store.notifyChanged();
    Ok(())
}

/// A layout move commits the migrated snapshot and exact coverage together.
/// It never reimports moved values as a newer business assignment.
pub(crate) fn move_to(
    source: &PreferencesDataStore,
    destination: &PreferencesDataStore,
) -> Result<(), Error> {
    let mut current = snapshot(source)?;
    let _lock = destination.lockTransaction();
    let existing = if destination.storageHost.exists(&destination.storagePath)? {
        destination.loadUnlocked()?;
        let mut values = destination.loadedPreferences().expect("loaded destination");
        destination.migratePreferencesToCurrentSchema(&mut values)?;
        let mut current = initialize(destination, &mut values)?.current;
        current.data.values = data(destination, &values)?.values;
        Some(current)
    } else {
        None
    };
    current
        .data
        .relocate(&destination.storagePath)
        .map_err(Error::Message)?;
    let mut values = preferences(&current);
    destination.migratePreferencesToCurrentSchema(&mut values)?;
    current.data = data(destination, &values)?;
    if let Some(mut existing) = existing {
        existing.data.merge(&current.data).map_err(Error::Message)?;
        existing.covered.extend(current.covered);
        current = existing;
        values = preferences(&current);
    }
    current.generation = current
        .generation
        .checked_add(1)
        .ok_or_else(|| Error::Message("Snapshot generation overflow".into()))?;
    journal(destination)?
        .registerPreferenceOutbox(&destination.storagePath, destination.encryption.is_some())?;
    save(destination, &mut values, current, vec![], vec![], true)
}

pub fn is_prepared(store: &PreferencesDataStore) -> Result<bool, Error> {
    let Some(owner) = state(store)? else { return Ok(false); };
    Ok(owner.current.data.owner.format == store.syncSchemaVersion()?)
}
pub fn prepare(store: &PreferencesDataStore) -> Result<(), Error> {
    if store.syncDescriptor.is_none() {
        return Ok(());
    }
    let _lock = store.lockTransaction();
    snapshot_unlocked(store).map(|_| ())
}

/// Read and correct the same materialized view under the dataset lock. Missing
/// projection files recover from SQL; an existing file remains visible to CAS.
fn snapshot_unlocked(store: &PreferencesDataStore) -> Result<Snapshot, Error> {
    let exists = store.storageHost.exists(&store.storagePath)?;
    let mut values = store.readFromStorage()?;
    let mut restored = false;
    if !exists {
        if let Some(owner) = state(store)? {
            values = preferences(&owner.current);
            restored = true;
        }
    }
    if values.commit.is_some() {
        recover(store, &mut values)?;
    }
    let migrated = store.migratePreferencesToCurrentSchema(&mut values)?;
    let mut current = initialize(store, &mut values)?.current;
    current.data = data(store, &values)?;
    // An absent, empty owner needs no placeholder. A real business deletion
    // lives in the SQL checkpoint and must not resurrect the previous record.
    if (restored || migrated) && (exists || !current.data.values.is_empty()) {
        store.writeStoredPreferencesUnlocked(&values)?;
    }
    store.publishCommitted(&values);
    Ok(current)
}

pub(crate) fn commit_local(
    store: &PreferencesDataStore,
    previous: &Preferences,
    values: &mut Preferences,
) -> Result<(), Error> {
    journal(store)?.registerPreferenceOutbox(&store.storagePath, store.encryption.is_some())?;
    let mut previous = previous.clone();
    let mut owner = initialize(store, &mut previous)?;
    let format = PreferencesDataStore::readSchemaVersion(values)?;
    if owner.current.data.owner.format != format {
        owner.current.data.owner.format = format;
        owner.format_progress = format;
        journal(store)?.withJournal(|j| j.transaction(|tx| Journal::save_owner(tx, &owner)))?;
    }
    let Some(mutation) = file::local_mutation(store, &previous, values)? else {
        return store.writeStoredPreferencesUnlocked(values);
    };
    let origin = format!("{}:preferences", journal(store)?.localDeviceId()?);
    let original = journal(store)?.withJournal(|j| j.reserve(&origin, mutation))?;
    let entry = PreferencesSyncedEntry::fromOperation(&original)?;
    let retired = {
        let mut history = journal(store)?.preferenceCompactionHistory(&store.storagePath, format)?;
        history.push(original.clone());
        let transactions = history.iter().filter(|operation| !crate::PreferenceMerge::isMergeMetadata(operation)
            && !operation.originDeviceId.starts_with("~import:"))
            .map(|operation| PreferencesSyncedEntry::fromOperation(operation)?.transaction()).collect::<Result<Vec<_>, _>>()?;
        super::compaction::superseded(&transactions).map_err(Error::Message)?
    };
    let mut current = owner.current;
    current
        .data
        .apply(&entry.transaction()?)
        .map_err(Error::Message)?;
    current.data.values = data(store, values)?.values;
    current.covered.insert(original.opId.clone());
    current.generation += 1;
    save_file(store, values, current, vec![original], vec![], false, None, retired)
}

pub fn snapshot(store: &PreferencesDataStore) -> Result<Snapshot, Error> {
    let _lock = store.lockTransaction();
    snapshot_unlocked(store)
}
/// Export a complete file and its checkpoint under the same dataset lock.
pub fn file_snapshot(store: &PreferencesDataStore, expected: &str) -> Result<String, Error> {
    let _lock = store.lockTransaction();
    let current = snapshot_unlocked(store)?;
    if current.token().map_err(Error::Message)? != expected {
        return Err(Error::Message("File changed before export".into()));
    }
    if store.storageHost.exists(&store.storagePath)? {
        store.readPlaintextFile()
    } else {
        let format = current.data.owner.format;
        let mut entries = current.data.values;
        entries.insert("__operit_preferences_schema_version".into(), serde_json::json!(format));
        Ok(serde_json::to_string_pretty(&entries)?)
    }
}

pub fn repair(store: &PreferencesDataStore, expected: &str, source: &Snapshot) -> Result<bool, Error> {
    repair_file(store, expected, source, None)
}

pub fn repair_file(store: &PreferencesDataStore, expected: &str, source: &Snapshot, file: Option<&str>) -> Result<bool, Error> {
    let lock = store.lockTransaction();
    let absent = expected.is_empty() && state(store)?.is_none() && !store.storageHost.exists(&store.storagePath)?;
    if expected.is_empty() && !absent { return Ok(false); }
    let mut current = if absent {
        // A new generic file adopts the source format; it has no old format to migrate.
        if source.data.owner != file::preferenceOwner(&store.storagePath, source.data.owner.format)
            || store.schema.as_ref().is_some_and(|schema| schema.currentVersion != source.data.owner.format) {
            return Err(Error::Message("Source file format/layout is not supported".into()));
        }
        Snapshot { node: journal(store)?.localDeviceId()?, data: Data::empty(source.data.owner.clone()),
            covered: BTreeSet::new(), generation: 0 }
    } else { snapshot_unlocked(store)? };
    let expected = if absent { current.token().map_err(Error::Message)? } else { expected.to_owned() };
    let unchanged = !absent && current.data == source.data && match file {
        Some(file) => store.storageHost.exists(&store.storagePath)? && store.readPlaintextFile()? == file,
        None => true,
    };
    if !current.replace(&expected, source).map_err(Error::Message)? { return Ok(false); }
    let mut values = match file {
        Some(file) => {
            let values: Preferences = serde_json::from_str(file)?;
            if values.commit.is_some() || values.legacy.is_some() || data(store, &values)?.values != source.data.values
                || PreferencesDataStore::readSchemaVersion(&values)? != source.data.owner.format {
                return Err(Error::Message("File differs from its baseline".into()));
            }
            values
        },
        None => preferences(&current),
    };
    journal(store)?.registerPreferenceOutbox(&store.storagePath, store.encryption.is_some())?;
    if unchanged {
        // Only SQL changes when the correct file already exists. No file/SQL handoff is needed.
        values.view = Some(current.data.clone());
        let original = initialization(store,&current)?;
        journal(store)?.appendOperations(&[original.clone()])?;
        let sealed = current.covered.clone();
        current.covered.insert(original.opId);
        let owner = OwnerState { legacy_baseline: None, format_progress: current.data.owner.format,
            layout_progress: current.data.owner.layout, current };
        let paths = owner_paths(store)?;
        journal(store)?.withJournal(|j| j.transaction(|tx| {
            Journal::retire_covered(tx,&paths,&sealed)?;
            Journal::save_owner(tx,&owner)
        }))?;
        store.publishCommitted(&values);
    } else {
        save_file(store, &mut values, current, vec![], vec![], true, file.map(str::to_owned), BTreeSet::new())?;
        drop(lock);
        store.notifyChanged();
    }
    Ok(true)
}

fn history_initialization_ids(originals: &[SyncOperation]) -> impl Iterator<Item=String> + '_ {
    originals.iter().filter(|op|op.operation == "initialize").map(|op|op.opId.clone())
}

/// One owner commit, with its business merge rule operating on a private replay view.
pub(crate) fn apply<F>(
    store: &PreferencesDataStore,
    originals: &[SyncOperation],
    rule: F,
) -> Result<BTreeMap<String, String>, Error>
where
    F: FnOnce(&mut Preferences, &[SyncOperation]) -> Result<(BTreeMap<String, String>, BTreeSet<String>), Error>,
{
    let _lock = store.lockTransaction();
    store.loadUnlocked()?;
    let mut values = store.loadedPreferences().expect("loaded file");
    store.migratePreferencesToCurrentSchema(&mut values)?;
    let owner = initialize(store, &mut values)?;
    let history = super::ordered(
        journal(store)?
            .archivedPreferenceHistory(&store.storagePath)?
            .into_iter()
            .chain(originals.iter().cloned()),
        |op| &op.opId,
        Revision::fromOperation,
    )
    .map_err(Error::Message)?;
    let start = replay_start(store,&history,owner.current.data.owner.format)?;
    values = preferences(&start);
    if start.covered.iter().all(|id| !id.starts_with("~init:") || owner.current.covered.contains(id)) {
        values.setMergeAliases(owner.current.data.aliases.clone());
    }
    let mut covered = start.covered;
    for operation in &history {
        if operation.originDeviceId.starts_with("~import:") {
            if let Some(ids) = operation.payload.get("importCoverage") {
                covered.extend(serde_json::from_value::<BTreeSet<String>>(ids.clone())?);
            }
        }
    }
    let uncovered = history
        .into_iter()
        .filter(|op| op.operation != "initialize" && !covered.contains(&op.opId))
        .collect::<Vec<_>>();
    let (aliases, retired) = rule(&mut values, &uncovered)?;
    let mut current = Snapshot {
        node: owner.current.node.clone(),
        data: data(store, &values)?,
        covered,
        generation: owner.current.generation + 1,
    };
    current.covered.extend(history_initialization_ids(originals));
    current.covered.extend(uncovered.iter().map(|op| op.opId.clone()));
    if current.data == owner.current.data && current.covered == owner.current.covered {
        journal(store)?.withJournal(|j|j.transaction(|tx|Journal::retire_ids(tx, retired.iter().cloned())))?;
        journal(store)?.completeOperations(originals)?;
        return Ok(aliases);
    }
    save_file(store, &mut values, current, originals.to_vec(),
        originals.iter().map(|op| op.opId.clone()).collect(), false, None, retired)?;
    store.notifyChanged();
    Ok(aliases)
}

#[path = "PreferenceMigration.rs"]
mod format;
pub(crate) use format::migrate_entries;
