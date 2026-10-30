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
    Ok(journal(store)?.withJournal(|j| j.owner_state(&store.storagePath))?)
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
    let previous = state(store)?;
    let owner = OwnerState {
        baseline: if commit.reset_baseline {
            current.clone()
        } else {
            previous
                .as_ref()
                .map(|s| s.baseline.clone())
                .unwrap_or_else(|| current.clone())
        },
        current: current.clone(),
        format_progress: current.data.owner.format,
        layout_progress: current.data.owner.layout,
    };
    journal(store)?.appendOperations(&commit.originals)?;
    journal(store)?.withJournal(|j| {
        j.transaction(|tx| {
            Journal::save_owner(tx, &owner)?;
            Journal::mark_complete(tx, &commit.completed)?;
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
    save_file(store, values, snapshot, originals, completed, reset, None)
}

fn save_file(
    store: &PreferencesDataStore, values: &mut Preferences, snapshot: Snapshot,
    originals: Vec<SyncOperation>, completed: Vec<String>, reset: bool, file: Option<String>,
) -> Result<(), Error> {
    values.view = Some(snapshot.data.clone());
    values.commit = Some(FileCommit {
        id: uuid::Uuid::new_v4().to_string(),
        originals,
        completed,
        covered: snapshot.covered,
        aliases: snapshot.data.aliases,
        deleted: snapshot.data.deleted,
        revision: snapshot.data.revision,
        versions: snapshot.data.versions,
        additions: snapshot.data.additions,
        generation: snapshot.generation,
        reset_baseline: reset,
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
            // The same owner definition migrates both replay baseline and current file.
            for snapshot in [&mut existing.baseline, &mut existing.current] {
                let mut converted = preferences(snapshot);
                store.migratePreferencesToCurrentSchema(&mut converted)?;
                snapshot.data = data(store, &converted)?;
            }
            existing.format_progress = existing.current.data.owner.format;
            journal(store)?
                .withJournal(|j| j.transaction(|tx| Journal::save_owner(tx, &existing)))?;
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
        let mut empty = baseline.clone();
        empty.data = Data::empty(imported.owner);
        empty.covered.clear();
        let owner = OwnerState {
            baseline: empty,
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
        baseline: baseline.clone(),
        current: baseline,
        format_progress: PreferencesDataStore::readSchemaVersion(values)?,
        layout_progress: 1,
    };
    journal(store)?.withJournal(|j| j.transaction(|tx| Journal::save_owner(tx, &owner)))?;
    values.view = Some(owner.current.data.clone());
    Ok(owner)
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
        owner.baseline.data.owner.format = format;
        owner.format_progress = format;
        journal(store)?.withJournal(|j| j.transaction(|tx| Journal::save_owner(tx, &owner)))?;
    }
    let Some(mutation) = file::local_mutation(store, &previous, values)? else {
        return store.writeStoredPreferencesUnlocked(values);
    };
    let origin = format!("{}:preferences", journal(store)?.localDeviceId()?);
    let original = journal(store)?.withJournal(|j| j.reserve(&origin, mutation))?;
    let mut current = owner.current;
    let entry = PreferencesSyncedEntry::fromOperation(&original)?;
    current
        .data
        .apply(&entry.transaction()?)
        .map_err(Error::Message)?;
    current.data.values = data(store, values)?.values;
    current.covered.insert(original.opId.clone());
    current.generation += 1;
    save(store, values, current, vec![original], vec![], false)
}

pub fn snapshot(store: &PreferencesDataStore) -> Result<Snapshot, Error> {
    let _lock = store.lockTransaction();
    snapshot_unlocked(store)
}
pub fn baseline_digest(store: &PreferencesDataStore) -> Result<String, Error> {
    let _lock = store.lockTransaction();
    state(store)?.ok_or_else(|| Error::Message("Missing file baseline".into()))?.baseline.baseline_digest().map_err(Error::Message)
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
    if unchanged {
        // Only SQL changes when the correct file already exists. No file/SQL handoff is needed.
        values.view = Some(current.data.clone());
        let owner = OwnerState { baseline: current.clone(), format_progress: current.data.owner.format,
            layout_progress: current.data.owner.layout, current };
        journal(store)?.withJournal(|j| j.transaction(|tx| Journal::save_owner(tx, &owner)))?;
        store.publishCommitted(&values);
    } else {
        save_file(store, &mut values, current, vec![], vec![], true, file.map(str::to_owned))?;
        drop(lock);
        store.notifyChanged();
    }
    Ok(true)
}

/// One owner commit, with its business merge rule operating on a private replay view.
pub(crate) fn apply<F>(
    store: &PreferencesDataStore,
    originals: &[SyncOperation],
    rule: F,
) -> Result<BTreeMap<String, String>, Error>
where
    F: FnOnce(&mut Preferences, &[SyncOperation], bool) -> Result<BTreeMap<String, String>, Error>,
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
    values = preferences(&owner.baseline);
    values.setMergeAliases(owner.current.data.aliases.clone());
    let mut covered = owner.baseline.covered.clone();
    for operation in &history {
        if operation.originDeviceId.starts_with("~import:") {
            if let Some(ids) = operation.payload.get("importCoverage") {
                covered.extend(serde_json::from_value::<BTreeSet<String>>(ids.clone())?);
            }
        }
    }
    let uncovered = history
        .into_iter()
        .filter(|op| !covered.contains(&op.opId))
        .collect::<Vec<_>>();
    let aliases = rule(&mut values, &uncovered, true)?;
    let mut current = Snapshot {
        node: owner.current.node.clone(),
        data: data(store, &values)?,
        covered,
        generation: owner.current.generation + 1,
    };
    current
        .covered
        .extend(uncovered.iter().map(|op| op.opId.clone()));
    if current.data == owner.current.data && current.covered == owner.current.covered {
        journal(store)?.completeOperations(originals)?;
        return Ok(aliases);
    }
    save(
        store,
        &mut values,
        current,
        originals.to_vec(),
        originals.iter().map(|op| op.opId.clone()).collect(),
        false,
    )?;
    store.notifyChanged();
    Ok(aliases)
}

pub(crate) fn migrate_entries(
    store: &PreferencesDataStore,
    version: u32,
    entries: &mut Vec<PreferencesSyncedEntry>,
) -> Result<(), Error> {
    let mut sources = BTreeMap::<u32, Data>::new();
    for entry in entries.iter_mut() {
        if entry.identityAlias().is_some() {
            continue;
        }
        let from = entry.schemaVersion().unwrap_or(0);
        entry.requireSchemaVersion(version)?;
        if from == version {
            continue;
        }
        let Some(schema) = &store.schema else {
            return Err(Error::MissingMigration { from, to: version });
        };
        let original = entry.transaction()?;
        let before = sources
            .entry(from)
            .or_insert_with(|| Data::empty(file::preferenceOwner(&store.storagePath, from)))
            .clone();
        let mut after = before.clone();
        after.apply(&original).map_err(Error::Message)?;
        sources.insert(from, after.clone());
        let mut context = super::migration::Context::state(before.clone());
        context.after = after;
        context.original = Some(original.clone());
        context.transactions = vec![original];
        for step in from..version {
            context.destination.format = step + 1;
            schema.migrate.apply(step, &mut context)?;
            context.before.owner = context.destination.clone();
            context.after.owner = context.destination.clone();
            for transaction in &mut context.transactions {
                if let super::Target::Data(owner) = &mut transaction.target {
                    *owner = context.destination.clone();
                }
            }
        }
        if context.transactions.len() != 1 {
            return Err(Error::Message(
                "Preference migration expansion must retain one atomic transaction".into(),
            ));
        }
        *entry = PreferencesSyncedEntry::fromTransaction(
            context.transactions.remove(0),
            store.encryption.is_some(),
        )?;
    }
    Ok(())
}
