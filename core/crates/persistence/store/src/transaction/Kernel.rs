//! One production lifecycle for receipt, routing, replay, owner commits and reconciliation.
use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use crate::SyncOperationStore::{SyncOperation, SyncOperationStore, SyncReceipt};
use super::{Snapshot, Owner, Revision};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnerState {
    /// Baseline coverage is explicit. A late transaction is never discarded by its timestamp.
    pub baseline: Snapshot,
    pub current: Snapshot,
    pub format_progress: u32,
    pub layout_progress: u32,
}

/// Only the unfinished cross-resource commit is embedded in a file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileCommit {
    pub id: String,
    pub originals: Vec<SyncOperation>,
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
    pub reset_baseline: bool,
    /// Exact plaintext file to publish after the checkpoint commits; encrypted locally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Default)]
pub struct ApplyReport {
    pub received: usize,
    pub applied: Vec<SyncOperation>,
    pub retired: usize,
    pub errors: Vec<String>,
    pub aliases: BTreeMap<String, String>,
    pub pending: Vec<String>,
    pub outcomes: BTreeMap<String, Result<BTreeMap<String,String>,String>>,
}

pub struct Participant<'a> {
    pub path: String,
    pub prepare: Box<dyn Fn() -> Result<(), String> + 'a>,
    pub validate: Option<Box<dyn Fn(&SyncOperation) -> Result<(), String> + 'a>>,
    pub apply: Box<dyn Fn(&[SyncOperation], &BTreeMap<String, String>) -> Result<BTreeMap<String, String>, String> + 'a>,
    pub project: Option<Box<dyn Fn(&BTreeMap<String,String>) -> Result<(),String> + 'a>>,
    pub snapshot: Option<Box<dyn Fn() -> Result<Snapshot, String> + 'a>>,
    pub baseline: Option<Box<dyn Fn() -> Result<String, String> + 'a>>,
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
        self.owners.values().filter_map(|owner| (owner.prepare)().err().map(|e| format!("{}: {e}", owner.path))).collect()
    }

    pub fn pending_owners(&self) -> Result<BTreeSet<String>, String> {
        self.journal.pendingOperations().map_err(|e| e.to_string())?.iter().map(|op| self.target(op)).collect()
    }

    pub fn receive(&self, incoming: &[SyncOperation]) -> Result<ApplyReport, String> {
        self.journal.receiveOperations(incoming).map_err(|e| e.to_string())?;
        let pending = self.journal.pendingOperations().map_err(|e| e.to_string())?;
        let pending = super::ordered(pending, |op| &op.opId, Revision::fromOperation)?;
        let mut report = ApplyReport { received: incoming.len(), ..ApplyReport::default() };
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
        for (path, operations) in groups {
            if self.is_retired(&path) {
                self.journal.completeOperations(&operations).map_err(|e| e.to_string())?;
                report.retired += operations.len();
                report.outcomes.insert(path, Ok(BTreeMap::new()));
                continue;
            }
            let Some(owner) = self.owners.get(&path) else {
                let error = format!("Pending transaction owner: {path}");
                report.errors.push(error.clone());
                report.outcomes.insert(path,Err(error));
                continue;
            };
            if let Err(error) = (owner.prepare)() { report.errors.push(format!("{path}: {error}")); report.outcomes.insert(path,Err(error)); continue; }
            let operations = operations.into_iter().filter(|operation| {
                let result = owner.validate.as_ref().map_or(Ok(()), |validate| validate(operation));
                if let Err(error) = result {
                    report.errors.push(format!("{}: {error}", operation.opId));
                    report.outcomes.insert(path.clone(), Err(error));
                    false
                } else { true }
            }).collect::<Vec<_>>();
            if operations.is_empty() { continue; }
            let apply = |operations: &[SyncOperation], report: &mut ApplyReport| {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (owner.apply)(operations, &report.aliases)))
                    .unwrap_or_else(|_| Err(format!("Owner panicked: {path}")));
                let aliases = result?;
                self.journal.completeOperations(operations).map_err(|e|e.to_string())?;
                report.outcomes.entry(path.clone()).or_insert_with(|| Ok(aliases.clone()));
                report.applied.extend_from_slice(operations);
                report.aliases.extend(aliases);
                Ok::<_,String>(())
            };
            if let Err(error) = apply(&operations, &mut report) {
                // A failed atomic transaction stays pending. Other ready transactions
                // in the same owner still commit through the identical lifecycle.
                let failures = if operations.len() == 1 { vec![error] } else {
                    operations.iter().filter_map(|operation| apply(std::slice::from_ref(operation), &mut report).err()).collect()
                };
                for error in failures {
                    report.errors.push(format!("{path}: {error}"));
                    report.outcomes.insert(path.clone(), Err(error));
                }
            }
        }
        if !report.aliases.is_empty() {
            for (path, owner) in &self.owners {
                if report.outcomes.get(path).is_some_and(Result::is_err) { continue; }
                if let Some(project) = &owner.project {
                    if let Err(error) = project(&report.aliases) { report.errors.push(format!("{path}: {error}")); report.outcomes.insert(path.clone(),Err(error)); }
                }
            }
        }
        report.pending = self.journal.pendingOperationIds().map_err(|e| e.to_string())?;
        for operation in incoming.iter().filter(|op| !report.pending.contains(&op.opId)) {
            if let Ok(path) = self.target(operation) { report.outcomes.entry(path).or_insert_with(|| Ok(BTreeMap::new())); }
        }
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

    /// File equality also requires the same durable starting point for future replay.
    pub fn manifests(&self) -> Result<(Vec<super::Manifest>, BTreeSet<String>, Vec<String>), String> {
        let (snapshots, pending, mut errors) = self.snapshots()?;
        let mut manifests = Vec::new();
        for snapshot in snapshots {
            let mut manifest = snapshot.manifest()?;
            if let Some(baseline) = self.owners.get(&manifest.owner.path).and_then(|owner|owner.baseline.as_ref()) {
                match baseline() {
                    Ok(digest) => manifest.baseline = Some(digest),
                    Err(error) => { errors.push(format!("{}: {error}",manifest.owner.path)); continue; },
                }
            }
            manifests.push(manifest);
        }
        Ok((manifests,pending,errors))
    }

    pub fn snapshot(&self, owner: &Owner) -> Result<Snapshot, String> {
        if self.pending_owners()?.contains(&owner.path) { return Err("Dataset has pending transactions".into()); }
        let participant = self.owners.get(&owner.path).ok_or("Unknown transaction owner")?;
        (participant.prepare)()?;
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
        if self.pending_owners()?.contains(&owner.path) { return Err("Dataset has pending transactions".into()); }
        if !expected.is_empty() {
            let current = self.snapshot(owner)?;
            if current.token()? != expected { return Ok(false); }
            if super::compare_manifests(&current.manifest()?, &source.manifest()?)? == super::Source::Local {
                return Err("Snapshot source is older than the local authority".into());
            }
        }
        let repair = self.owners.get(&owner.path).and_then(|owner| owner.repair.as_ref()).ok_or("Owner cannot replace files")?;
        if !repair(expected, source, file)? { return Ok(false); }
        let verified = self.snapshot(owner)?;
        if verified.content_digest()? != source.content_digest()? { return Err("Snapshot verification failed after replacement".into()); }
        if let Some(baseline) = self.owners.get(&owner.path).and_then(|owner|owner.baseline.as_ref()) {
            if baseline()? != verified.baseline_digest()? { return Err("Replacement baseline was not committed".into()); }
        }
        Ok(true)
    }
}
