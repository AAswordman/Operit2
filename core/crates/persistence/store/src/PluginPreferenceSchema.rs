//! Portable plugin format rules share native migration and deterministic contract resolution.
use operit_host_api::PluginStorage::{StoragePreferencesSchema, StoragePreferencesMigrationRule as Step};
use crate::{PreferencesDataStore::{PreferencesDataStore, PreferencesDataStoreError as Error, stringPreferencesKey}, Transaction::{Rule, Form}};

pub(crate) const KEY: &str = "__operit_preferences_schema";

fn failure(message: impl Into<String>) -> Error { Error::Message(message.into()) }

pub(crate) fn reserved(key: &str) -> bool { key.starts_with("__operit_preferences_") }

pub(crate) fn validate(schema: &StoragePreferencesSchema) -> Result<(), Error> {
    if schema.version as usize != schema.steps.len() || schema.steps.len() > 256
        || schema.steps.iter().any(|step| step.len() > 256) {
        return Err(failure("A schema requires one bounded migration step per version"));
    }
    let key = |key: &str| if key.is_empty() || reserved(key) { Err(failure("Invalid migration key")) } else { Ok(()) };
    for step in schema.steps.iter().flatten() {
        match step {
            Step::RenameKey { from, to } => { key(from)?; key(to)?; }
            Step::SplitObject { key: source, fields } => { key(source)?; for destination in fields.values() { key(destination)?; } }
            Step::SplitRecords { key: source, prefix, identity } => { key(source)?; key(prefix)?; if identity.is_empty() { return Err(failure("Record identity is empty")); } }
            Step::RenameField { key: source, from, to } => { key(source)?; if from.is_empty() || to.is_empty() { return Err(failure("Migration field is empty")); } }
            Step::SetObjectField { key: source, field, .. } => { key(source)?; if field.is_empty() { return Err(failure("Migration field is empty")); } }
            Step::DropKey { key: source } | Step::DecodeJson { key: source } => key(source)?,
        }
    }
    Ok(())
}

/// Prefer the highest format, then a canonical ordering of the complete contract.
/// This total order is associative, commutative and idempotent: accidental edits to
/// released steps cannot block an owner or make peers choose by arrival order.
/// Keep one complete migration chain; splicing conflicting steps could invent a
/// chain that no plugin published. Already applied versions are never rerun.
pub(crate) fn combine(a: &StoragePreferencesSchema, b: &StoragePreferencesSchema) -> Result<StoragePreferencesSchema, Error> {
    validate(a)?; validate(b)?;
    if a.version != b.version { return Ok(if a.version > b.version { a } else { b }.clone()); }
    if a == b { return Ok(a.clone()); }
    let canonical = |schema: &StoragePreferencesSchema| -> Result<Vec<u8>, Error> {
        let mut value = serde_json::to_value(schema)?;
        value.sort_all_objects();
        Ok(serde_json::to_vec(&value)?)
    };
    Ok(if canonical(a)? >= canonical(b)? { a } else { b }.clone())
}

pub(crate) fn read(store: &PreferencesDataStore) -> Result<Option<StoragePreferencesSchema>, Error> {
    store.data()?.get(&stringPreferencesKey(KEY)).cloned().map(serde_json::from_value).transpose().map_err(Into::into)
}

pub(crate) fn configure(store: PreferencesDataStore, schema: StoragePreferencesSchema) -> Result<PreferencesDataStore, Error> {
    validate(&schema)?;
    let mut store = store.withMigration(schema.version, move |from, context| {
        let rules = schema.steps.get(from as usize).ok_or(Error::MissingMigration { from, to: from+1 })?;
        for step in rules {
            let rule = match step {
                Step::RenameKey { from,to } => Some(Rule::RenameKey { from:from.clone(),to:to.clone() }),
                Step::RenameField { key,from,to } => Some(Rule::RenameField { key:key.clone(),from:from.clone(),to:to.clone() }),
                Step::SplitObject { key,fields } => Some(Rule::SplitObject { key:key.clone(),fields:fields.clone() }),
                Step::SplitRecords { key,prefix,identity } => Some(Rule::SplitRecords { key:key.clone(),prefix:prefix.clone(),identity:identity.clone() }),
                Step::DropKey { key } => Some(Rule::DropKey { key:key.clone() }),
                Step::SetObjectField { .. } | Step::DecodeJson { .. } => None,
            };
            if let Some(rule) = rule { context.apply_rule(&rule).map_err(failure)?; }
            else {
                context.map_preferences(from, &|_, preferences| {
                    let key = match step { Step::SetObjectField { key,.. } | Step::DecodeJson { key } => key, _ => unreachable!() };
                    let key = stringPreferencesKey(key);
                    let Some(mut value) = preferences.get(&key).cloned() else { return Ok(()); };
                    match step {
                        Step::SetObjectField { field,value:field_value,.. } => {
                            value.as_object_mut().ok_or_else(||failure("Migration requires an object"))?.insert(field.clone(),field_value.clone());
                        }
                        Step::DecodeJson { .. } => { if let Some(encoded) = value.as_str() { value = serde_json::from_str(encoded)?; } }
                        _ => unreachable!(),
                    }
                    preferences.set(&key,value);
                    Ok(())
                })?;
            }
        }
        Ok(())
    });
    // The portable rule compiler owns this property; plugin authors declare rules only.
    std::sync::Arc::make_mut(&mut store.schema.as_mut().expect("configured schema").migrate).independent = true;
    Ok(store)
}

/// Data upgrades and publication of their portable contract share the file/SQL commit boundary.
pub(crate) fn declare(store: &PreferencesDataStore, schema: StoragePreferencesSchema) -> Result<(), Error> {
    validate(&schema)?;
    let schema = match read(store)? { Some(existing) => combine(&existing,&schema)?, None => schema };
    let store = configure(store.clone(),schema.clone())?;
    let value = serde_json::to_value(&schema)?;
    if store.syncOperationStore.is_some() {
        crate::PreferenceMerge::applyDeclaredOperations(&store, &Default::default(), &[], &Default::default(), |_, _| true)?;
        crate::Transaction::preference::declare_format(&store,KEY,value)
    } else {
        store.edit(|preferences| preferences.set(&stringPreferencesKey(KEY),value.clone()))?;
        Ok(())
    }
}

/// A peer can prepare an unopened plugin owner from the contract carried with its originals.
pub(crate) fn contract<'a>(operations: impl IntoIterator<Item=&'a crate::SyncOperationStore::SyncOperation>)
    -> Result<Option<StoragePreferencesSchema>,Error> {
    use crate::PreferencesDataStore::PreferencesSyncedEntry;
    let mut result = None;
    for operation in operations {
        if operation.domain != "preferences" { continue; }
        let mut found = None;
        if let Some(initialization) = crate::Transaction::compaction::Initialization::decode(operation).map_err(failure)? {
            found = initialization.data.values.get(KEY).cloned();
        } else {
            let mut entry = PreferencesSyncedEntry::fromOperation(operation)?;
            if !entry.keys().contains(&KEY) { continue; }
            entry.rewriteRootEntries(|key,value| {
                if key == KEY { found = Some(value.cloned().ok_or_else(||failure("A migration contract cannot be deleted"))?); }
                Ok(())
            })?;
            if found.is_none() { return Err(failure("Migration contracts require complete immutable records")); }
        }
        if let Some(value) = found {
            let schema: StoragePreferencesSchema = serde_json::from_value(value)?;
            validate(&schema)?;
            result = Some(match result.as_ref() { Some(existing) => combine(existing,&schema)?,None => schema });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn contract_resolution_converges_for_every_order_and_grouping() {
        let contracts = [
            json!({"version":0,"steps":[]}),
            json!({"version":1,"steps":[[]]}),
            json!({"version":1,"steps":[[{"op":"rename_key","from":"a","to":"b"}]]}),
            json!({"version":1,"steps":[[{"op":"drop_key","key":"a"}]]}),
            json!({"version":2,"steps":[[],[{"op":"rename_key","from":"x","to":"y"}]]}),
            json!({"version":2,"steps":[[{"op":"drop_key","key":"b"}],[]]}),
        ].map(|value| serde_json::from_value::<StoragePreferencesSchema>(value).unwrap());
        for a in &contracts {
            assert_eq!(combine(a,a).unwrap(), *a);
            for b in &contracts {
                let resolved = combine(a,b).unwrap();
                assert_eq!(resolved, combine(b,a).unwrap());
                assert_eq!(resolved.version, a.version.max(b.version));
                assert!(resolved == *a || resolved == *b, "Never splice migration chains");
                for c in &contracts {
                    assert_eq!(combine(&resolved,c).unwrap(), combine(a,&combine(b,c).unwrap()).unwrap());
                }
            }
        }
        let invalid = StoragePreferencesSchema { version:1, steps:vec![] };
        assert!(combine(&contracts[0],&invalid).is_err());
    }
}
