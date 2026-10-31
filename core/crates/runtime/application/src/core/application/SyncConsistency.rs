use super::OperitApplication::{OperitApplication, synchronizationDataset};
use crate::data::preferences::PreferenceSpaceMerge;
use operit_store::sync::SqlChatSyncStore::{SqlChatSyncStore, CHAT_SYNC_DOMAIN};
use operit_store::CoreNodeBindingStore::BINDING_SYNC_DOMAIN;
use operit_store::NetworkControlStore::NETWORK_CONTROL_SYNC_DOMAIN;
use operit_store::PluginStorage::PLUGIN_STORAGE_SYNC_DOMAIN;
use operit_store::SyncOperationStore::{SyncOperation, DEFAULT_SYNC_PAGE_BYTES};
use std::collections::BTreeMap;
use operit_store::PreferenceMerge::{self, PreferenceMergeSchema};
use operit_store::Transaction::{Owner, Snapshot, kernel::{Registry, Participant, Source}};
use operit_store::RuntimeFileSyncStore::{FileCorrectionReference, FileCoverage};
use operit_store::PluginStorage;
use serde_json::{json, Value};

/// File contents and their durable replay boundary travel together.
#[derive(serde::Serialize, serde::Deserialize)]
struct CorrectionFile {
    snapshot: Snapshot,
    content: Option<String>,
    definition: Option<Value>,
}

impl OperitApplication {
    /// All synchronization entry points use this same owner/source inventory.
    pub(super) fn transactionRegistry<'a>(&'a self, schema: &'a PreferenceMergeSchema, operations: &[SyncOperation]) -> Result<Registry<'a>,String> {
        let storage = self.hostManager.runtimeStorageHost.clone().ok_or("Runtime storage is unavailable")?;
        let skipped = self.runtimeSyncOperationStore()?.skippedOperations().map_err(|e|e.to_string())?;
        let operations = operations.iter().chain(&skipped).collect::<Vec<_>>();
        let mut registry = PreferenceMerge::registry(storage.clone(), schema, operations.iter().copied(), |_,_|true)?;
        registry.locate = Box::new(|op| match op.domain.as_str() {
            "preferences" => op.payload["storagePath"].as_str().map(str::to_owned).ok_or("Preference path is missing".into()),
            CHAT_SYNC_DOMAIN => Ok(operit_util::RuntimeStorageLayout::SQLITE_DATABASE_PATH.into()),
            PLUGIN_STORAGE_SYNC_DOMAIN => PluginStorage::syncDatasetPath(op).map_err(|e|e.to_string()),
            _ => Ok(format!("{}:{}",op.domain,synchronizationDataset(op)?)),
        });
        let chat = SqlChatSyncStore::default().map_err(|e|e.to_string());
        let (prepare,apply,snapshot,repair,receipt,export,inventory) = (chat.clone(),chat.clone(),chat.clone(),chat.clone(),chat.clone(),chat.clone(),chat);
        registry.register(Participant { validate: None,
            content_digest: SqlChatSyncStore::snapshotContentDigest,
            path: operit_util::RuntimeStorageLayout::SQLITE_DATABASE_PATH.into(),
            prepare: Box::new(move || prepare.as_ref().map(|_|()).map_err(Clone::clone)),
            apply: Box::new(move |operations,_| {
                let store = apply.as_ref().map_err(Clone::clone)?;
                for operation in operations { store.applyOperation(operation).map_err(|e|e.to_string())?; }
                Ok(BTreeMap::new())
            }), project: None,
            snapshot: Some(Box::new(move || snapshot.as_ref().map_err(Clone::clone)?.syncSnapshot().map_err(|e|e.to_string()))),
            repair: Some(Box::new(move |expected,source,_| repair.as_ref().map_err(Clone::clone)?.repairSyncedSnapshot(expected,source).map_err(|e|e.to_string()))),
        })?;
        registry.sources.push(Source {
            receipt: Box::new(move || receipt.as_ref().map_err(Clone::clone)?.receipt().map(|r|(r,vec![])).map_err(|e|e.to_string())),
            export: Box::new(move |receipt,domains,limit| export.as_ref().map_err(Clone::clone)?.operationsSince(receipt,domains,limit).map(|ops|(ops,vec![])).map_err(|e|e.to_string())),
            inventory: Box::new(move |receipt,domains| inventory.as_ref().map_err(Clone::clone)?.operationIdsSince(receipt,domains).map(|ids|(ids,vec![])).map_err(|e|e.to_string())),
        });
        let plugin = self.pluginSqliteHost();
        let (receipt,export,inventory) = (plugin.clone(),plugin.clone(),plugin.clone());
        registry.sources.push(Source {
            receipt: Box::new(move || PluginStorage::receipt(receipt.clone()?).map_err(|e|e.to_string())),
            export: Box::new(move |receipt,domains,limit| PluginStorage::exportWithBudget(export.clone()?,receipt,domains,limit,DEFAULT_SYNC_PAGE_BYTES).map_err(|e|e.to_string())),
            inventory: Box::new(move |receipt,domains| {
                if !domains.is_empty() && !domains.iter().any(|domain|domain==PLUGIN_STORAGE_SYNC_DOMAIN) { return Ok((Default::default(),vec![])); }
                PluginStorage::operationIdsSince(inventory.clone()?,receipt).map_err(|e|e.to_string())
            }),
        });
        let mut paths = plugin.clone().and_then(|host|PluginStorage::syncDatasetPaths(host).map_err(|e|e.to_string())).unwrap_or_default().into_iter().collect::<std::collections::BTreeSet<_>>();
        for operation in operations.iter().filter(|op|op.domain==PLUGIN_STORAGE_SYNC_DOMAIN) { paths.insert(PluginStorage::syncDatasetPath(operation).map_err(|e|e.to_string())?); }
        let node = registry.journal.localDeviceId().map_err(|e|e.to_string())?;
        for path in paths {
            let (apply,snapshot,repair,project) = (plugin.clone(),plugin.clone(),plugin.clone(),plugin.clone());
            let (snapshot_path,node_snapshot,node_repair,storage) = (path.clone(),node.clone(),node.clone(),storage.clone());
            registry.register(Participant { validate: None, path,
                content_digest: PluginStorage::snapshotContentDigest,
                prepare: Box::new(||Ok(())),
                apply: Box::new(move |operations,aliases| {
                    for operation in operations { PluginStorage::applyOperation(apply.clone()?,storage.clone(),operation,aliases).map_err(|e|e.to_string())?; }
                    Ok(BTreeMap::new())
                }),
                project: Some(Box::new(move |aliases|PluginStorage::reconcilePreferenceReferences(project.clone()?,aliases).map_err(|e|e.to_string()))),
                snapshot: Some(Box::new(move ||PluginStorage::syncSnapshot(snapshot.clone()?,&snapshot_path,&node_snapshot))),
                repair: Some(Box::new(move |expected,source,_|PluginStorage::repairSyncedSnapshot(repair.clone()?,expected,source,&node_repair))),
            })?;
        }
        for operation in operations.iter().filter(|op|!matches!(op.domain.as_str(),"preferences"|CHAT_SYNC_DOMAIN|PLUGIN_STORAGE_SYNC_DOMAIN)) {
            if operation.domain == operit_store::RuntimeFileSyncStore::RUNTIME_FILE_SYNC_DOMAIN
                && operit_util::RuntimeStorageLayout::runtimeStorageOwnership(&operation.entityId) != Ok(operit_util::RuntimeStorageLayout::RuntimeStorageOwnership::Space) {
                registry.journal.ignoreFileOperations(&operation.domain, &operation.entityId).map_err(|e|e.to_string())?;
                continue;
            }
            let path = registry.target(operation)?;
            if registry.owners.contains_key(&path) { continue; }
            registry.register(Participant { validate: None, path, prepare: Box::new(||Ok(())), project: None, snapshot: None, content_digest: operit_store::Transaction::Snapshot::content_digest, repair: None,
                apply: Box::new(move |operations,_| {
                    for operation in operations {
                        match operation.domain.as_str() {
                            BINDING_SYNC_DOMAIN => self.runtimeBindingStore()?.applySyncedOperation(operation)?,
                            NETWORK_CONTROL_SYNC_DOMAIN => self.runtimeNetworkControlStore()?.applySyncedOperation(operation)?,
                            _ => {
                                let journal = self.runtimeSyncOperationStore()?;
                                if journal.shouldApplyOperation(operation).map_err(|e|e.to_string())? {
                                    if operit_store::WorkspaceFileSyncStore::WorkspaceFileSyncStore::owns(operation) {
                                        self.runtimeWorkspaceFileSyncStore()?.materializeOperation(operation,false)?;
                                    } else { self.applyNonPreferenceSyncOperation(operation)?; }
                                    journal.recordAppliedOperation(operation).map_err(|e|e.to_string())?;
                                }
                                journal.appendOperation(operation).map_err(|e|e.to_string())?;
                            }
                        }
                    }
                    Ok(BTreeMap::new())
                }),
            })?;
        }
        Ok(registry)
    }

    pub(super) fn consistencyManifest(&self) -> Result<Value,String> {
        let journal = self.runtimeSyncOperationStore()?;
        let pending = journal.pendingOperations().map_err(|e|e.to_string())?;
        let schema = PreferenceSpaceMerge::schema();
        let registry = self.transactionRegistry(&schema,&pending)?;
        let (manifests,pending,mut errors) = registry.manifests()?;
        // Captured after the snapshots: a new commit since the empty export invalidates this comparison.
        let (receipt, failures) = registry.receipt()?;
        errors.extend(failures);
        let skipped = registry.skipped()?.into_iter().map(|(path,operations)|
            (path,operations.into_iter().map(|operation|operation.opId).collect::<std::collections::BTreeSet<_>>()))
            .collect::<BTreeMap<_,_>>();
        Ok(json!({"manifests":manifests,"pendingOwners":pending,"skipped":skipped,"errors":errors,"boundary":operit_store::Transaction::digest(&receipt)?}))
    }
    pub(super) fn exportConsistencySnapshot(&self, owner: Owner, token: &str) -> Result<Value,String> {
        let pending = self.runtimeSyncOperationStore()?.pendingOperations().map_err(|e|e.to_string())?;
        let schema = PreferenceSpaceMerge::schema();
        let snapshot = self.transactionRegistry(&schema,&pending)?.snapshot(&owner)?;
        if snapshot.token()? != token { return Err("Snapshot changed; synchronize again".into()); }
        let content = if owner.backend == operit_store::Transaction::Backend::DataStore {
            let storage = self.hostManager.runtimeStorageHost.clone().ok_or("Runtime storage is unavailable")?;
            let declaration = schema.owner(&owner.path)?.ok_or("Unknown preference file")?;
            Some(operit_store::Transaction::preference::file_snapshot(&(declaration.open)(storage,&owner.path),token).map_err(|e|e.to_string())?)
        } else { None };
        let definition = if owner.path == operit_util::RuntimeStorageLayout::SQLITE_DATABASE_PATH || owner.backend != operit_store::Transaction::Backend::Sql {
            None
        } else { Some(PluginStorage::snapshotDefinition(self.pluginSqliteHost()?, &owner.path).map_err(|e|e.to_string())?) };
        let bytes = serde_json::to_vec(&CorrectionFile { snapshot, content, definition }).map_err(|e|e.to_string())?;
        serde_json::to_value(self.runtimeFileSyncStore()?.storeBlob(&bytes)?).map_err(|e|e.to_string())
    }
    pub(super) fn exportConsistencyCoverage(&self, owner: Owner, token: &str) -> Result<Value,String> {
        let pending = self.runtimeSyncOperationStore()?.pendingOperations().map_err(|e|e.to_string())?;
        let schema = PreferenceSpaceMerge::schema();
        let snapshot = self.transactionRegistry(&schema,&pending)?.snapshot(&owner)?;
        if snapshot.token()? != token { return Err("Snapshot changed; synchronize again".into()); }
        let bytes = serde_json::to_vec(&FileCoverage { owner, covered: snapshot.covered }).map_err(|e|e.to_string())?;
        serde_json::to_value(self.runtimeFileSyncStore()?.storeBlob(&bytes)?).map_err(|e|e.to_string())
    }
    pub(super) fn repairConsistencySnapshot(&self, owner: Owner, expected: &str, reference: FileCorrectionReference) -> Result<bool,String> {
        let journal = self.runtimeSyncOperationStore()?;
        let _application = journal.lockApplication().map_err(|e|e.to_string())?;
        let files = self.runtimeFileSyncStore()?;
        let mut source: CorrectionFile = serde_json::from_slice(&files.readBlob(&reference.source)?).map_err(|e|e.to_string())?;
        if source.snapshot.data.owner != owner { return Err("Correction file owner differs".into()); }
        source.snapshot.covered.extend(reference.skipped);
        if let Some(reference) = reference.superseded {
            let superseded: FileCoverage = serde_json::from_slice(&files.readBlob(&reference)?).map_err(|e|e.to_string())?;
            if superseded.owner != owner { return Err("Superseded file owner differs".into()); }
            source.snapshot.covered.extend(superseded.covered);
        }
        let pending = journal.pendingOperations().map_err(|e|e.to_string())?;
        let mut schema = PreferenceSpaceMerge::schema();
        let registry = self.transactionRegistry(&schema,&pending)?;
        let missing = !registry.owners.contains_key(&owner.path);
        if expected.is_empty() != missing { return Ok(false); }
        drop(registry);
        // A missing inventory entry is provisioned by the complete file, then committed as a baseline.
        if missing {
            match owner.backend {
                operit_store::Transaction::Backend::DataStore => {
                    let declaration = schema.owner(&owner.path)?.ok_or("Unknown shared preference file")?;
                    schema.stores.push(declaration);
                },
                operit_store::Transaction::Backend::Sql => {
                    let storage = self.hostManager.runtimeStorageHost.clone().ok_or("Runtime storage is unavailable")?;
                    PluginStorage::installSnapshotDefinition(self.pluginSqliteHost()?,storage,&owner.path,
                        source.definition.clone().ok_or("Missing database definition")?).map_err(|e|e.to_string())?;
                },
            }
        }
        let registry = self.transactionRegistry(&schema,&pending)?;
        registry.repair_file(&owner,expected,&source.snapshot,source.content.as_deref())
    }
}
