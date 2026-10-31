//! The source representative A must satisfy m(A) = the actual replay baseline.
use super::*;
use crate::SyncOperationStore::SyncOperationStoreError;
use crate::Transaction::{migration::Context, Target, Transaction};
use operit_host_api::RuntimeSqliteTransaction;

fn create_sources(tx: &mut dyn RuntimeSqliteTransaction) -> Result<(), SyncOperationStoreError> {
    tx.execute(
        "CREATE TABLE IF NOT EXISTS preference_migration_sources(
        owner TEXT NOT NULL, fingerprint TEXT NOT NULL, source TEXT NOT NULL,
        PRIMARY KEY(owner, fingerprint))",
        vec![],
    )?;
    Ok(())
}

pub(super) fn save_source(
    tx: &mut dyn RuntimeSqliteTransaction,
    source: &Data,
) -> Result<(), SyncOperationStoreError> {
    create_sources(tx)?;
    tx.execute(
        "INSERT OR IGNORE INTO preference_migration_sources VALUES(?1,?2,?3)",
        vec![
            crate::Transaction::text(&source.owner.path),
            crate::Transaction::text(
                &crate::Transaction::digest(source).map_err(SyncOperationStoreError::Message)?,
            ),
            crate::Transaction::text(&serde_json::to_string(source)?),
        ],
    )?;
    Ok(())
}

fn migrate(store: &PreferencesDataStore, to: u32, context: &mut Context) -> Result<(), Error> {
    for from in context.after.owner.format..to {
        context.destination.format = from + 1;
        store
            .schema
            .as_ref()
            .ok_or(Error::MissingMigration { from, to: from + 1 })?
            .migrate
            .apply(from, context)?;
        context.before.owner = context.destination.clone();
        context.after.owner = context.destination.clone();
    }
    Ok(())
}

fn upgrade(store: &PreferencesDataStore, data: Data, to: u32) -> Result<Data, Error> {
    let mut context = Context::state(data);
    migrate(store, to, &mut context)?;
    Ok(context.after)
}

fn source(
    store: &PreferencesDataStore,
    from: u32,
    current: &Data,
    retained: &[Data],
    history: &[Transaction],
    cached: Option<&Data>,
) -> Result<Data, Error> {
    if from == current.owner.format {
        return Ok(current.clone());
    }
    let matches = |data: &Data| {
        upgrade(store, data.clone(), current.owner.format).is_ok_and(|data| data == *current)
    };
    if let Some(data) = cached.filter(|data| matches(data)) {
        return Ok(data.clone());
    }
    let retained = retained
        .iter()
        .filter(|data| data.owner.format <= from)
        .find_map(|baseline| {
            let mut data = upgrade(store, baseline.clone(), from).ok()?;
            for transaction in history {
                let mut transaction = transaction.clone();
                transaction.target = Target::Data(data.owner.clone());
                data.apply(&transaction).ok()?;
            }
            matches(&data).then_some(data)
        });
    if let Some(data) = retained {
        return Ok(data);
    }
    // Generated portable codecs have a payload-local source map. Opaque callbacks
    // use the retained source above; a canonical image cannot recover erased context.
    if *current != Data::empty(current.owner.clone())
        && !store.schema.as_ref().is_some_and(|schema| schema.migrate.independent) {
        return Err(Error::Message(format!(
            "Missing source baseline for preference migration: {} format {from}", store.storagePath
        )));
    }
    let mut data = current.clone();
    data.owner.format = from;
    if upgrade(store, data.clone(), current.owner.format)? != *current {
        return Err(Error::Message(format!(
            "Missing source baseline for preference migration: {} format {from}",
            store.storagePath
        )));
    }
    Ok(data)
}

// Convert the structural prefix directly, then give codecs their actual context.
// map_preferences marks a step opaque, so an empty probe can never certify a codec.
fn route_rules(store: &PreferencesDataStore, original: &Transaction, version: u32) -> Transaction {
    let mut transaction = original.clone();
    let Target::Data(owner) = &original.target else {
        return transaction;
    };
    for from in owner.format..version {
        let Target::Data(owner) = &transaction.target else {
            break;
        };
        let mut context = Context::state(Data::empty(owner.clone()));
        if context.after.apply(&transaction).is_err() {
            break;
        }
        context.destination.format = from + 1;
        context.original = Some(original.clone());
        context.transactions = vec![transaction.clone()];
        let Some(schema) = &store.schema else {
            break;
        };
        if schema.migrate.apply(from, &mut context).is_err()
            || context.rules.as_ref().is_none_or(Vec::is_empty)
        {
            break;
        }
        transaction = context.transactions.remove(0);
    }
    transaction
}

pub(crate) fn migrate_entries(
    store: &PreferencesDataStore,
    version: u32,
    baseline: &Preferences,
    entries: &mut Vec<PreferencesSyncedEntry>,
) -> Result<(), Error> {
    if !entries.iter().any(|entry| {
        entry.identityAlias().is_none() && entry.schemaVersion().unwrap_or(0) < version
    }) {
        return Ok(());
    }
    let mut current = data(store, baseline)?;
    let retained: Vec<Data> = journal(store)?.withJournal(|j| j.transaction(|tx| {
        create_sources(tx)?;
        tx.query("SELECT source FROM preference_migration_sources WHERE owner=?1 ORDER BY fingerprint",
            vec![crate::Transaction::text(&store.storagePath)])?.into_iter()
            .map(|row| Ok(serde_json::from_str(&row.valueAt(0)?.asString()?)?)).collect()
    }))?;
    let mut sources = BTreeMap::new();
    let mut history = Vec::new();
    let mut staged = entries.clone();
    for entry in &mut staged {
        if entry.identityAlias().is_some() {
            continue;
        }
        entry.requireSchemaVersion(version)?;
        let original = entry.transaction()?;
        let routed = route_rules(store, &original, version);
        let Target::Data(owner) = &routed.target else {
            unreachable!("preference transaction")
        };
        let from = owner.format;
        let (converted, after) = if from == version {
            (routed, None)
        } else {
            let before = source(
                store,
                from,
                &current,
                &retained,
                &history,
                sources.get(&from),
            )?;
            let mut after = before.clone();
            after.apply(&routed).map_err(Error::Message)?;
            let mut context = Context::state(before);
            context.after = after.clone();
            context.original = Some(original.clone());
            context.transactions = vec![routed];
            migrate(store, version, &mut context)?;
            (context.transactions.remove(0), Some(after))
        };
        current.apply(&converted).map_err(Error::Message)?;
        sources.retain(|format, data| {
            let mut effect = converted.clone();
            effect.target = Target::Data(data.owner.clone());
            *format == from || data.apply(&effect).is_ok()
        });
        match after {
            Some(after) => {
                sources.insert(from, after);
            }
            None => {
                sources.remove(&from);
            }
        }
        history.push(converted.clone());
        *entry = PreferencesSyncedEntry::fromTransaction(converted, store.encryption.is_some())?;
    }
    *entries = staged;
    Ok(())
}
