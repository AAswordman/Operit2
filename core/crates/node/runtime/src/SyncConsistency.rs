//! Final content comparison at a stable exchange boundary, excluding incomplete transfers.
use super::*;
use operit_store::Transaction::{compare_manifests, digest, Manifest, Source};
use operit_store::RuntimeFileSyncStore::FileCorrectionReference;
use operit_store::SyncOperationStore::SyncReceipt;

#[derive(Deserialize)]
struct Report {
    manifests: Vec<Manifest>,
    boundary: String,
    #[serde(default)]
    errors: Vec<String>,
    #[serde(default)]
    pendingOwners: BTreeSet<String>,
    #[serde(default)]
    skipped: BTreeMap<String, BTreeSet<String>>,
}

/// Exchanging every known original does not prove equal contents on an older peer.
pub(super) fn requireConsistencySupport(local: &Value, remote: &Value) -> Result<(), String> {
    for (side, clock) in [("Local", local), ("Remote", remote)] {
        if clock.get("consistencyVersion").and_then(Value::as_u64) != Some(7) {
            return Err(format!(
                "{side} peer cannot verify synchronized contents; a compatible version is required"
            ));
        }
    }
    Ok(())
}

impl SpacePersistenceSyncService {
    async fn consistencyReports(&self, target: &str, deferred: &BTreeSet<String>) -> Result<(Report, Report), String> {
        let (remote, local) = tokio::join!(
            biased;
            callRemote(
                &self.state.nodeRouter,
                target,
                PeerSyncMethod::SyncConsistency,
                Value::Null
            ),
            self.callLocal("syncConsistency", Value::Null),
        );
        let (mut local, mut remote): (Report, Report) = (local?, remote?);
        for report in [&mut local, &mut remote] { report.pendingOwners.extend(deferred.iter().cloned()); }
        Ok((local, remote))
    }

    /// Structural damage selects the complete peer; every correction is verified before completion.
    pub(super) async fn synchronizeConsistency(&self, target: &str, local_clock: &Value, remote_clock: &Value, deferred: &BTreeSet<String>, force: bool) -> Result<bool, String> {
        let (local, remote) = self.consistencyReports(target, deferred).await?;
        let local_receipt = serde_json::from_value(local_clock.clone()).map_err(|e|e.to_string())?;
        let remote_receipt = serde_json::from_value(remote_clock.clone()).map_err(|e|e.to_string())?;
        if !reportsAtExchange(&local, &remote, &local_receipt, &remote_receipt)? { return Ok(false); }
        let plans = correctionPlans(&local, &remote, force);
        let correctionCount = plans.len();
        let startedAt = currentTimeMillis();
        for (a, b, choice) in plans {
            let source = if choice == Source::Local { a } else { b }.expect("source exists");
            operit_util::AppLogger::AppLogger::i("CoreSyncTrace", &format!(
                "sync_consistency.correct target={target} mode=full_snapshot owner={} source={choice:?} damaged={} localRevision={:?} remoteRevision={:?}",
                source.owner.path, a.is_some_and(|m|m.damaged) || b.is_some_and(|m|m.damaged), a.and_then(|m|m.revision.as_ref()), b.and_then(|m|m.revision.as_ref())));
            let correction: Result<bool, String> = async {
                let other = if choice == Source::Local { b } else { a };
                let superseded = match other {
                    Some(manifest) => Some(self.transferSnapshot(target, manifest, choice != Source::Local, true).await?),
                    None => None,
                };
                let reference = FileCorrectionReference {
                    source: self.transferSnapshot(target, source, choice == Source::Local, false).await?, superseded,
                    skipped: local.skipped.get(&source.owner.path).into_iter().chain(remote.skipped.get(&source.owner.path))
                        .flat_map(|ids|ids.iter().cloned()).collect(),
                };
                for local in [choice != Source::Local, choice == Source::Local] {
                    if !self.storageReplicationAllowed(target)? { return Err("Storage replication authorization changed".into()); }
                    let destination = if local { a } else { b };
                    let args = json!({"owner":source.owner,"expected":destination.map_or("", |m|m.token.as_str()),"reference":reference});
                    let repaired: bool = if local { self.callLocal("syncRepairSnapshot",args).await? }
                        else { callRemote(&self.state.nodeRouter,target,PeerSyncMethod::SyncRepairSnapshot,args).await? };
                    if !repaired { return Ok(false); }
                }
                Ok(true)
            }.await;
            match correction {
                Ok(true) => { operit_util::AppLogger::AppLogger::i("CoreSyncTrace", &format!(
                    "sync_consistency.applied target={target} mode=full_snapshot owner={} source={choice:?}", source.owner.path)); },
                Ok(false) => return Ok(false),
                Err(error) if snapshotChanged(&error) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        let (latest_local, latest_remote) = self.consistencyReports(target, deferred).await?;
        if !reportsAtExchange(&latest_local, &latest_remote, &local_receipt, &remote_receipt)? { return Ok(false); }
        let complete = verifyReports(&latest_local, &latest_remote)?;
        operit_util::AppLogger::AppLogger::i("CoreSyncTrace", &format!(
            "sync_consistency.done target={target} mode=full_snapshot corrections={correctionCount} complete={complete} elapsedMs={}", currentTimeMillis() - startedAt));
        Ok(complete)
    }
    async fn transferSnapshot(&self, target: &str, source: &Manifest, local: bool, coverage_only: bool) -> Result<RuntimeFileSyncReference, String> {
        let (name, method) = if coverage_only { ("syncReadCoverage", PeerSyncMethod::SyncReadCoverage) }
            else { ("syncReadSnapshot", PeerSyncMethod::SyncReadSnapshot) };
        let args = json!({"owner":source.owner,"token":source.token});
        let reference: RuntimeFileSyncReference = if local { self.callLocal(name,args).await? }
            else { callRemote(&self.state.nodeRouter,target,method,args).await? };
        operit_util::AppLogger::AppLogger::i("CoreSyncTrace", &format!(
            "sync_consistency.snapshot target={target} owner={} source={} coverageOnly={coverage_only} bytes={} hash={}",
            source.owner.path, if local { "local" } else { "remote" }, reference.size, reference.contentHash));
        self.synchronizeBlobReferences(target, vec![reference.clone()]).await?;
        Ok(reference)
    }

}

/// Application errors cross the RPC boundary as strings; only these exact retry reasons are quiet.
fn snapshotChanged(error: &str) -> bool {
    let reason = error.lines().next().unwrap_or_default();
    ["Snapshot changed; synchronize again", "File changed before export"].iter()
        .any(|message| reason == *message || reason.ends_with(&format!(": {message}")))
}

/// Each peer must still be at its own empty-export boundary. Their histories may differ.
fn reportsAtExchange(local: &Report, remote: &Report, local_receipt: &SyncReceipt, remote_receipt: &SyncReceipt) -> Result<bool, String> {
    Ok(local.boundary == digest(local_receipt)? && remote.boundary == digest(remote_receipt)?)
}

/// The union of inventories includes files that exist on only one endpoint.
fn correctionPlans<'a>(local: &'a Report, remote: &'a Report, force: bool) -> Vec<(Option<&'a Manifest>, Option<&'a Manifest>, Source)> {
    let mut files = BTreeMap::new();
    for manifest in &local.manifests { files.entry((&manifest.owner.path,&manifest.owner.backend)).or_insert((None,None)).0 = Some(manifest); }
    for manifest in &remote.manifests { files.entry((&manifest.owner.path,&manifest.owner.backend)).or_insert((None,None)).1 = Some(manifest); }
    files.into_iter().filter_map(|((path,_),(a,b))| {
        if local.pendingOwners.contains(path) || remote.pendingOwners.contains(path) { return None; }
        let choice = match (a,b) {
            (Some(a),Some(b)) => compare_manifests(a,b).ok()?,
            (Some(_),None) => Source::Local,
            (None,Some(_)) => Source::Remote,
            _ => unreachable!(),
        };
        let choice = if choice == Source::Equal && (force || local.skipped.contains_key(path) || remote.skipped.contains_key(path)) {
            let (a,b) = (a?,b?);
            if (&a.revision,&a.node) >= (&b.revision,&b.node) { Source::Local } else { Source::Remote }
        } else { choice };
        (choice != Source::Equal).then_some((a,b,choice))
    }).collect()
}

/// Completion requires equal synchronized contents; data failures are never deferred.
fn verifyReports(local: &Report, remote: &Report) -> Result<bool, String> {
    if !local.errors.is_empty() || !remote.errors.is_empty() {
        return Err(format!("Consistency check has owner errors: local={:?}; remote={:?}", local.errors, remote.errors));
    }
    if !local.pendingOwners.is_empty() || !remote.pendingOwners.is_empty() {
        return Err("Consistency check has pending owners".into());
    }
    let complete = local.pendingOwners.is_empty() && remote.pendingOwners.is_empty()
        && local.skipped.is_empty() && remote.skipped.is_empty();
    for (a,b) in [(local,remote),(remote,local)] {
        for manifest in &a.manifests {
            let comparison = match b.manifests.iter().find(|other|other.owner.path == manifest.owner.path && other.owner.backend == manifest.owner.backend) {
                Some(other) => compare_manifests(manifest,other),
                None => Err(format!("Missing synchronized file: {}",manifest.owner.path)),
            };
            if matches!(comparison, Ok(Source::Equal)) { continue; }
            comparison?;
            return Err(format!("Synchronized file differs after correction: {}",manifest.owner.path));
        }
    }
    Ok(complete)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_store::Transaction::{Backend, Data, Owner, Snapshot};

    /// Constructs one complete file inventory independently of its replay state.
    fn report(node: &str, path: &str, value: u64) -> Report {
        let mut data = Data::empty(Owner { backend: Backend::DataStore, path: path.into(), format: 1, layout: 1 });
        data.values.insert("value".into(), json!(value));
        let snapshot = Snapshot { damage: None, node: node.into(), data, covered: BTreeSet::new(), generation: 0 };
        Report { manifests: vec![snapshot.manifest().unwrap()], boundary: Default::default(), errors: vec![],
            pendingOwners: BTreeSet::new(), skipped: BTreeMap::new() }
    }

    /// Damaged state retains a CAS token but cannot become the authority, even with a newer revision.
    #[test]
    fn damaged_files_are_corrected_from_the_complete_peer() {
        let healthy = report("a", "database", 1);
        let mut damaged = report("z", "database", 2);
        damaged.manifests[0].damaged = true;
        damaged.manifests[0].revision = Some(operit_store::Transaction::Revision {
            createdAt: 999, originDeviceId: "z".into(), sequence: 1 });
        for (a,b,choice) in [(&healthy,&damaged,Source::Local),(&damaged,&healthy,Source::Remote)] {
            let plans = correctionPlans(a,b,false);
            assert_eq!(plans.len(),1);
            assert_eq!(plans[0].2,choice);
            assert!(verifyReports(a,b).is_err());
        }
        assert!(verifyReports(&damaged,&damaged).is_err());
        assert!(verifyReports(&healthy,&healthy).unwrap());
    }

    /// Missing files, unequal contents and skipped originals all require complete-file correction.
    #[test]
    fn correction_covers_the_union_of_files_and_seals_skipped_history() {
        let mut local = report("a", "local", 1);
        let mut remote = report("b", "remote", 2);
        assert_eq!(correctionPlans(&local,&remote,false).len(),2);
        assert!(verifyReports(&local,&remote).is_err());
        remote.manifests = local.manifests.clone();
        assert!(correctionPlans(&local,&remote,false).is_empty());
        assert_eq!(correctionPlans(&local,&remote,true).len(),1,"export/apply failures must seal a complete baseline even for equal contents");
        local.manifests[0].token = "different replay state".into();
        assert!(verifyReports(&local,&remote).unwrap());
        local.skipped.insert("local".into(), BTreeSet::from(["broken:1".into()]));
        assert_eq!(correctionPlans(&local,&remote,false).len(),1);
        assert!(!verifyReports(&local,&remote).unwrap());
        local.skipped.clear();
        remote.pendingOwners.insert("local".into());
        assert!(correctionPlans(&local,&remote,false).is_empty());
        assert!(verifyReports(&local,&remote).is_err());
        remote.pendingOwners.clear();
        remote.errors.push("unreadable storage".into());
        assert!(verifyReports(&local,&remote).unwrap_err().contains("unreadable storage"));
    }

    /// Concurrent writes resume incremental exchange in the current pass before choosing an authority.
    #[test]
    fn comparison_uses_each_peers_own_export_boundary() {
        let mut local = report("a", "file", 1);
        let mut remote = report("b", "file", 1);
        let mut a = SyncReceipt::default(); a.observe("a",17);
        let mut b = SyncReceipt::default(); b.observe("b",29);
        local.boundary = digest(&a).unwrap(); remote.boundary = digest(&b).unwrap();
        assert!(reportsAtExchange(&local,&remote,&a,&b).unwrap());
        a.observe("new",1);
        assert!(!reportsAtExchange(&local,&remote,&a,&b).unwrap());
    }

    /// Only exact snapshot races restart the current exchange; corruption remains an explicit failure.
    #[test]
    fn snapshot_races_are_distinct_from_storage_errors() {
        for reason in ["Snapshot changed; synchronize again", "File changed before export"] {
            assert!(snapshotChanged(reason));
            assert!(snapshotChanged(&format!("remote: COMMAND_ERROR: {reason}\nRust error location: test.rs:1")));
        }
        for error in ["database is corrupt", "permission denied", "Snapshot verification failed after replacement"] {
            assert!(!snapshotChanged(error));
        }
    }

    /// Old peers cannot interpret damaged-file inventories or the complete synchronized schema.
    #[test]
    fn consistency_requires_the_current_protocol() {
        let current = json!({"consistencyVersion":7});
        assert!(requireConsistencySupport(&current,&current).is_ok());
        for previous in (0..7).map(|version|json!({"consistencyVersion":version})).chain([Value::Null,json!({})]) {
            assert!(requireConsistencySupport(&previous,&current).is_err());
            assert!(requireConsistencySupport(&current,&previous).is_err());
        }
    }
}
