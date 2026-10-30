use super::*;

pub(crate) fn local_mutation(
        store: &PreferencesDataStore,
        previous: &Preferences,
        preferences: &Preferences,
    ) -> Result<Option<NewSyncOperation>, PreferencesDataStoreError> {
        let Some(descriptor) = &store.syncDescriptor else {
            return Ok(None);
        };
        let previousEntries = previous.entries().into_iter().collect::<BTreeMap<_, _>>();
        let nextEntries = preferences
            .entries()
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        let mut keys = previousEntries
            .keys()
            .chain(nextEntries.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        keys.remove(PREFERENCES_SCHEMA_VERSION_KEY_NAME);
        keys.remove(PREFERENCES_REPLICATION_KEY);
        let schemaVersion = PreferencesDataStore::readSchemaVersion(preferences)?;
        let schemaChanged = PreferencesDataStore::readSchemaVersion(previous)? != schemaVersion;
        let changed = keys
            .into_iter()
            .filter(|key| schemaChanged || previousEntries.get(key) != nextEntries.get(key))
            .collect::<Vec<_>>();
        if changed.is_empty() {
            return Ok(None);
        }
        let entries = changed
            .iter()
            .map(|key| entry_mutation(key, preferences))
            .collect::<Result<Vec<_>, _>>()?;
        let mut mutation = if entries.len() == 1 {
            let entry = entries
                .into_iter()
                .next()
                .expect("one changed preference entry");
            let semantics = if matches!(entry.operation.as_str(), "increment" | "patch_object") {
                SyncOperationSemantics::Transaction
            } else {
                SyncOperationSemantics::EntityState
            };
            NewSyncOperation {
                domain: descriptor.domain.clone(),
                entityType: descriptor.entityType.clone(),
                entityId: preferenceMutationEntityId(&descriptor.storagePath, &entry.key, &[])?,
                operation: entry.operation,
                semantics,
                payload: serde_json::to_value(PreferencesSyncEntryPayload {
                    storagePath: descriptor.storagePath.clone(),
                    key: entry.key,
                    value: entry.value,
                    encrypted: store.encryption.is_some(),
                    formatVersion: 2,
                    schemaVersion: Some(schemaVersion),
                    migrationSnapshot: false,
                    importCoverage: BTreeSet::new(),
                    jsonPath: Vec::new(),
                    jsonMutation: None,
                })?,
            }
        } else {
            NewSyncOperation {
                domain: descriptor.domain.clone(),
                entityType: descriptor.entityType.clone(),
                entityId: serde_json::to_string(&(descriptor.storagePath.as_str(), &changed))?,
                operation: "transaction".into(),
                semantics: SyncOperationSemantics::Transaction,
                payload: serde_json::to_value(PreferencesSyncTransactionPayload {
                    contractVersion: Some(1),
                    logicalTime: None,
                    storagePath: descriptor.storagePath.clone(),
                    encrypted: store.encryption.is_some(),
                    formatVersion: 2,
                    schemaVersion: Some(schemaVersion),
                    migrationSnapshot: false,
                    importCoverage: BTreeSet::new(),
                    entries,
                })?,
            }
        };
        mutation.payload["contractVersion"] = Value::from(1);
        Ok(Some(mutation))
    }

pub(super) fn entry_mutation(
        key: &str,
        preferences: &Preferences,
    ) -> Result<PreferencesTransactionMutation, PreferencesDataStoreError> {
        if let Some(mutation) = preferences.pendingMutations.get(key) {
            if mutation.operation != "set" {
                return Ok(mutation.clone());
            }
        }
        let value = preferences.get(&stringPreferencesKey(key));
        Ok(PreferencesTransactionMutation {
            key: key.to_owned(),
            operation: if value.is_some() { "set" } else { "delete" }.into(),
            value: match value {
                Some(value) => value.clone(),
                None => Value::Null,
            },
        })
    }