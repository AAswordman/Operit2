//! One production lifecycle for receipt, routing, replay, owner commits and reconciliation.
use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use crate::SyncOperationStore::{SyncOperation, SyncOperationStore, SyncReceipt};
use super::{Snapshot, Owner, Revision};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnerState {
    /// One-time compatibility input; replay starts from an exported initialization transaction.
    #[serde(default, rename = "baseline", skip_serializing)]
    pub legacy_baseline: Option<Snapshot>,
    pub current: Snapshot,
    pub format_progress: u32,
    pub layout_progress: u32,
}

/// Only the unfinished cross-resource commit is embedded in a file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileCommit {
    pub id: String,
    pub originals: Vec<SyncOperation>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub retired: BTreeSet<String>,
    pub completed: Vec<String>,
    pub covered: BTreeSet<String>,
    pub aliases: BTreeMap<String, String>,
    pub deleted: BTreeSet<String>,
    pub revision: Option<Revision>,
    #[serde(default)]
    pub versions: BTreeMap<String, Revision>,
    #[serde(default)]
    pub additions: BTreeMap<String, Vec<super::Addition>>,
    pub generation: u64,
    #[serde(default, rename = "reset_baseline", skip_serializing_if = "std::ops::Not::not")]
    pub legacy_reset: bool,
    /// Exact plaintext file to publish after the SQL commit; encrypted locally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Default)]
pub struct ApplyReport {
    pub applied: Vec<SyncOperation>,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub aliases: BTreeMap<String, String>,
    pub pending: Vec<String>,
    pub outcomes: BTreeMap<String, Result<BTreeMap<String,String>,String>>,
}

/// Unknown failures stay retryable. Only recognized data/capability errors abandon incremental apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureKind { Temporary, Capability }

impl FailureKind {
    fn classify(error: &str) -> Self {
        let error = error.to_ascii_lowercase();
        if ["locked", "busy", "timeout", "timed out", "temporar", "interrupted", "connection",
            "i/o", "io error", "disk", "permission denied", "unavailable", "not registered for persistent sync",
            "snapshot changed", "file changed", "os error"].iter().any(|reason| error.contains(reason)) {
            return Self::Temporary;
        }
        if ["unknown", "unsupported", "not supported", "not registered", "missing", "invalid", "malformed",
            "migration", "schema", "format", "deserialize", "expected", "constraint", "no such table",
            "no such column", "identity mismatch", "scope", "panicked", "requires", "cannot decode"]
            .iter().any(|reason| error.contains(reason)) { Self::Capability } else { Self::Temporary }
    }
}

pub struct Participant<'a> {
    pub path: String,
    pub prepare: Box<dyn Fn() -> Result<(), String> + 'a>,
    pub validate: Option<Box<dyn Fn(&SyncOperation) -> Result<(), String> + 'a>>,
    pub apply: Box<dyn Fn(&[SyncOperation], &BTreeMap<String, String>) -> Result<BTreeMap<String, String>, String> + 'a>,
    pub project: Option<Box<dyn Fn(&BTreeMap<String,String>) -> Result<(),String> + 'a>>,
    pub snapshot: Option<Box<dyn Fn() -> Result<Snapshot, String> + 'a>>,
    pub coverage: Box<dyn Fn() -> Result<BTreeSet<String>, String> + 'a>,
    /// Owner-defined business equality; snapshot tokens and transfer contents remain complete.
    pub content_digest: fn(&Snapshot) -> Result<String, String>,
    pub repair: Option<Box<dyn Fn(&str, &Snapshot, Option<&str>) -> Result<bool, String> + 'a>>,
}

/// Existing SQL business databases expose their originals through the same source contract.
/// Transport pagination and blobs remain outside the owner lifecycle.
pub struct Source<'a> {
    pub receipt: Box<dyn Fn() -> Result<(SyncReceipt, Vec<String>), String> + 'a>,
    pub export: Box<dyn Fn(&SyncReceipt, &[String], usize) -> Result<(Vec<SyncOperation>, Vec<String>), String> + 'a>,
    pub inventory: Box<dyn Fn(&SyncReceipt, &[String]) -> Result<(BTreeSet<String>, Vec<String>), String> + 'a>,
}

pub struct Registry<'a> {
    pub journal: SyncOperationStore,
    pub owners: BTreeMap<String, Participant<'a>>,
    pub sources: Vec<Source<'a>>,
    pub routes: BTreeMap<String, String>,
    pub retired: BTreeSet<String>,
    pub locate: Box<dyn Fn(&SyncOperation) -> Result<String, String> + 'a>,
}

impl<'a> Registry<'a> {
    pub fn register(&mut self, owner: Participant<'a>) -> Result<(), String> {
        if self.is_retired(&owner.path) { return Ok(()); }
        if self.owners.insert(owner.path.clone(), owner).is_some() { return Err("Duplicate transaction owner".into()); }
        Ok(())
    }

    pub fn target(&self, operation: &SyncOperation) -> Result<String, String> {
        super::Migrations::resolve_path(&self.routes, &(self.locate)(operation)?)
    }

    /// Directory declarations include descendants, while file declarations match exactly.
    pub fn is_retired(&self, path: &str) -> bool {
        self.retired.iter().any(|source| path == source || source.ends_with('/')
            && (path.starts_with(source) || path == source.trim_end_matches('/')))
    }

    pub fn prepare(&self) -> Vec<String> {
        let skipped = match self.skipped() { Ok(skipped) => skipped, Err(error) => return vec![error] };
        self.owners.values().filter(|owner| !skipped.contains_key(&owner.path))
            .filter_map(|owner| (owner.prepare)().err().map(|e| format!("{}: {e}", owner.path))).collect()
    }

    /// Coverage, rather than the reception status, determines what still needs application.
    fn discard_covered(&self, path: &str, operations: &mut Vec<SyncOperation>) -> Result<(),String> {
        if operations.is_empty() { return Ok(()); }
        let covered = match self.owners.get(path) { Some(owner) => (owner.coverage)()?, None => BTreeSet::new() };
        let retired = operations.iter().filter(|op|covered.contains(&op.opId)).cloned().collect::<Vec<_>>();
        if !retired.is_empty() { self.journal.retireCoveredOperations(&retired).map_err(|e|e.to_string())?; }
        operations.retain(|op| !covered.contains(&op.opId));
        Ok(())
    }

    pub fn pending_owners(&self) -> Result<BTreeSet<String>, String> {
        let mut groups = BTreeMap::<String,Vec<SyncOperation>>::new();
        for operation in self.journal.pendingOperations().map_err(|e|e.to_string())? {
            groups.entry(self.target(&operation)?).or_default().push(operation);
        }
        let mut pending = BTreeSet::new();
        for (path,mut operations) in groups {
            self.discard_covered(&path,&mut operations)?;
            if !operations.is_empty() { pending.insert(path); }
        }
        Ok(pending)
    }

    pub fn skipped(&self) -> Result<BTreeMap<String, Vec<SyncOperation>>, String> {
        let mut groups = BTreeMap::<String, Vec<SyncOperation>>::new();
        for operation in self.journal.skippedOperations().map_err(|e|e.to_string())? {
            // Preference-only registries also share the SQL journal; only their routable originals belong here.
            if let Ok(path) = self.target(&operation) { groups.entry(path).or_default().push(operation); }
        }
        Ok(groups)
    }

    fn failure(&self, path: &str, operations: &[SyncOperation], error: String, report: &mut ApplyReport) -> Result<(), String> {
        match FailureKind::classify(&error) {
            FailureKind::Temporary => report.errors.push(format!("{path}: {error}")),
            FailureKind::Capability => {
                self.journal.skipOperations(operations).map_err(|e|e.to_string())?;
                report.skipped += operations.len();
                operit_util::AppLogger::AppLogger::w("TransactionSync", &format!(
                    "sync_apply.skipped owner={path} kind=capability count={} reason={error}", operations.len()));
            }
        }
        report.outcomes.insert(path.to_owned(), Err(error));
        Ok(())
    }

    /// Receives originals once before applying their durable pending work.
    pub fn receive(&self, incoming: &[SyncOperation]) -> Result<ApplyReport, String> {
        self.journal.receiveOperations(incoming).map_err(|e| e.to_string())?;
        let mut report = self.apply_pending()?;
        for operation in incoming.iter().filter(|op| !report.pending.contains(&op.opId)) {
            if let Ok(path) = self.target(operation) { report.outcomes.entry(path).or_insert_with(|| Ok(BTreeMap::new())); }
        }
        Ok(report)
    }

    /// Retries originals already received before owner discovery and layout preparation.
    pub fn apply_pending(&self) -> Result<ApplyReport, String> {
        let pending = self.journal.pendingOperations().map_err(|e| e.to_string())?;
        let pending = super::ordered(pending, |op| &op.opId, Revision::fromOperation)?;
        let mut report = ApplyReport::default();
        let mut skipped = self.skipped()?;
        for (path, operations) in &skipped {
            if self.is_retired(path) {
                self.journal.completeOperations(operations).map_err(|e|e.to_string())?;
                report.skipped += operations.len();
            }
        }
        skipped.retain(|path,_| !self.is_retired(path));
        let mut groups = BTreeMap::<String, Vec<SyncOperation>>::new();
        for operation in pending {
            match self.target(&operation) {
                Ok(path) => groups.entry(path).or_default().push(operation),
                Err(error) => report.errors.push(format!("{}: {error}", operation.opId)),
            }
        }
        // Reference owners consume only alias decisions from committed owners.
        let mut groups = groups.into_iter().collect::<Vec<_>>();
        groups.sort_by_key(|(path, operations)| (operations[0].domain != "preferences", path.clone()));
        for (path, mut operations) in groups {
            if self.is_retired(&path) {
                self.journal.completeOperations(&operations).map_err(|e| e.to_string())?;
                report.skipped += operations.len();
                report.outcomes.insert(path, Ok(BTreeMap::new()));
                continue;
            }
            let first = &operations[0];
            if first.domain == "preferences" && !self.owners.contains_key(&path)
                || first.domain == crate::RuntimeFileSyncStore::RUNTIME_FILE_SYNC_DOMAIN
                    && operit_util::RuntimeStorageLayout::runtimeStorageOwnership(&first.entityId) != Ok(operit_util::RuntimeStorageLayout::RuntimeStorageOwnership::Space) {
                self.journal.ignoreFileOperations(&first.domain, if first.domain == "preferences" { &path } else { &first.entityId }).map_err(|e|e.to_string())?;
                report.skipped += operations.len();
                continue;
            }
            if let Err(error) = self.discard_covered(&path,&mut operations) {
                self.failure(&path, &operations, error, &mut report)?;
                continue;
            }
            if operations.is_empty() { continue; }
            if skipped.contains_key(&path) && !operations.iter().any(|operation|
                operation.operation == "initialize" && operation.entityType == "initialization") {
                self.journal.skipOperations(&operations).map_err(|e|e.to_string())?;
                report.skipped += operations.len();
                report.outcomes.insert(path, Err("Incremental apply skipped until file correction".into()));
                continue;
            }
            let Some(owner) = self.owners.get(&path) else {
                self.failure(&path, &operations, format!("Unknown transaction owner: {path}"), &mut report)?;
                continue;
            };
            if let Err(error) = (owner.prepare)() {
                self.failure(&path, &operations, error, &mut report)?;
                continue;
            }
            // Validate only up to the first failure. Apply the valid prefix once, never retry a failed batch one by one.
            let invalid = operations.iter().enumerate().find_map(|(index, operation)|
                owner.validate.as_ref()?.as_ref()(operation).err().map(|error|(index,error)));
            let count = invalid.as_ref().map_or(operations.len(), |(index,_)|*index);
            if count > 0 {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(||
                    (owner.apply)(&operations[..count], &report.aliases)))
                    .unwrap_or_else(|_| Err(format!("Owner panicked: {path}")));
                match result {
                    Ok(aliases) => {
                        self.journal.completeOperations(&operations[..count]).map_err(|e|e.to_string())?;
                        report.applied.extend_from_slice(&operations[..count]);
                        report.aliases.extend(aliases.clone());
                        report.outcomes.insert(path.clone(), Ok(aliases));
                    }
                    Err(error) => {
                        self.failure(&path, &operations, error, &mut report)?;
                        continue;
                    }
                }
            }
            if let Some((index,error)) = invalid {
                self.failure(&path, &operations[index..], error, &mut report)?;
            }
        }
        if !report.aliases.is_empty() {
            let skipped = self.skipped()?;
            for (path, owner) in &self.owners {
                if skipped.contains_key(path) || report.outcomes.get(path).is_some_and(Result::is_err) { continue; }
                if let Some(project) = &owner.project {
                    if let Err(error) = project(&report.aliases) { self.failure(path, &[], error, &mut report)?; }
                }
            }
        }
        report.pending = self.journal.pendingOperationIds().map_err(|e| e.to_string())?;
        Ok(report)
    }

    pub fn receipt(&self) -> Result<(SyncReceipt, Vec<String>), String> {
        let mut receipt = self.journal.receipt().map_err(|e| e.to_string())?;
        let mut errors = self.prepare();
        for source in &self.sources {
            match (source.receipt)() { Ok((other, failures)) => { receipt.merge(other); errors.extend(failures); }, Err(error) => errors.push(error) }
        }
        Ok((receipt, errors))
    }

    pub fn export(&self, receipt: &SyncReceipt, domains: &[String], limit: usize) -> Result<(Vec<SyncOperation>, Vec<String>), String> {
        let mut operations = self.journal.operationsSince(receipt, domains, limit).map_err(|e| e.to_string())?;
        let mut errors = self.prepare();
        for source in &self.sources {
            match (source.export)(receipt, domains, limit) { Ok((page, failures)) => { operations.extend(page); errors.extend(failures); }, Err(error) => errors.push(error) }
        }
        let operations = super::ordered(operations, |op| &op.opId, Revision::fromOperation)?;
        Ok((operations.into_iter().take(limit).collect(), errors))
    }

    pub fn inventory(&self, receipt: &SyncReceipt, domains: &[String]) -> Result<(BTreeSet<String>,Vec<String>),String> {
        let mut ids = self.journal.operationIdsSince(receipt,domains).map_err(|e|e.to_string())?;
        let mut errors = Vec::new();
        for source in &self.sources {
            match (source.inventory)(receipt,domains) {
                Ok((more,failures)) => { ids.extend(more); errors.extend(failures); },
                Err(error) => errors.push(error),
            }
        }
        Ok((ids,errors))
    }

    pub fn snapshots(&self) -> Result<(Vec<Snapshot>, BTreeSet<String>, Vec<String>), String> {
        let pending = self.pending_owners()?;
        let mut errors = self.prepare();
        let mut snapshots = Vec::new();
        for (path, owner) in &self.owners {
            if pending.contains(path) { continue; }
            if let Some(snapshot) = &owner.snapshot {
                match snapshot() { Ok(value) => snapshots.push(value), Err(error) => errors.push(format!("{path}: {error}")) }
            }
        }
        Ok((snapshots, pending, errors))
    }

    /// Only materialized contents decide whether a dataset needs correction.
    pub fn manifests(&self) -> Result<(Vec<super::Manifest>, BTreeSet<String>, Vec<String>), String> {
        let (snapshots, pending, mut errors) = self.snapshots()?;
        let mut manifests = Vec::new();
        for snapshot in snapshots {
            let manifest = self.owners.get(&snapshot.data.owner.path)
                .ok_or_else(|| "Unknown transaction owner".to_string())
                .and_then(|owner| (owner.content_digest)(&snapshot))
                .and_then(|digest| snapshot.manifest_with_digest(digest));
            match manifest {
                Ok(manifest) => manifests.push(manifest),
                Err(error) => errors.push(format!("{}: {error}", snapshot.data.owner.path)),
            }
        }
        Ok((manifests,pending,errors))
    }

    pub fn snapshot(&self, owner: &Owner) -> Result<Snapshot, String> {
        if self.pending_owners()?.contains(&owner.path) { return Err("Dataset has pending transactions".into()); }
        let participant = self.owners.get(&owner.path).ok_or("Unknown transaction owner")?;
        if !self.skipped()?.contains_key(&owner.path) { (participant.prepare)()?; }
        let snapshot = (participant.snapshot.as_ref().ok_or("Owner has no snapshot capability")?)()?;
        if &snapshot.data.owner != owner { return Err("Owner format/layout is not ready".into()); }
        Ok(snapshot)
    }

    /// Authority, version checks, CAS and verification are shared by every backend.
    pub fn repair(&self, owner: &Owner, expected: &str, source: &Snapshot) -> Result<bool, String> {
        self.repair_file(owner, expected, source, None)
    }

    pub fn repair_file(&self, owner: &Owner, expected: &str, source: &Snapshot, file: Option<&str>) -> Result<bool, String> {
        if &source.data.owner != owner { return Err("Snapshot owner differs".into()); }
        let participant = self.owners.get(&owner.path).ok_or("Unknown transaction owner")?;
        if !self.skipped()?.contains_key(&owner.path) { (participant.prepare)()?; }
        let mut pending = self.journal.pendingOperations().map_err(|e|e.to_string())?.into_iter()
            .filter(|operation|self.target(operation).as_ref() == Ok(&owner.path)).collect::<Vec<_>>();
        self.discard_covered(&owner.path,&mut pending)?;
        if pending.iter().any(|operation|!source.covered.contains(&operation.opId)) {
            return Err("Dataset has uncovered pending transactions".into());
        }
        let capture = || (participant.snapshot.as_ref().ok_or("Owner has no snapshot capability")?)();
        if !expected.is_empty() {
            let current = capture()?;
            if current.token()? != expected { return Ok(false); }
            if (&current.data.revision, &current.node) > (&source.data.revision, &source.node) {
                return Err("Snapshot source is older than the local authority".into());
            }
        }
        let repair = participant.repair.as_ref().ok_or("Owner cannot replace files")?;
        if !repair(expected, source, file)? { return Ok(false); }
        let verified = capture()?;
        if verified.data != source.data { return Err("Snapshot verification failed after replacement".into()); }
        self.journal.retireCoveredOperations(&pending).map_err(|e|e.to_string())?;
        let skipped = self.skipped()?.remove(&owner.path).unwrap_or_default();
        let covered = skipped.into_iter().filter(|operation|source.covered.contains(&operation.opId)).collect::<Vec<_>>();
        self.journal.retireCoveredOperations(&covered).map_err(|e|e.to_string())?;
        Ok(true)
    }
}
