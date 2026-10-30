//! Final materialized-content comparison after the ordinary history exchange is empty.
use super::*;
use operit_store::Transaction::{compare_manifests, Manifest, Source};
use operit_store::RuntimeFileSyncStore::FileCorrectionReference;

#[derive(Deserialize)]
struct Report {
    manifests: Vec<Manifest>,
    #[serde(default)]
    errors: Vec<String>,
    #[serde(default)]
    pendingOwners: BTreeSet<String>,
}

/// Exchanging every known original does not prove equal contents on an older peer.
pub(super) fn requireConsistencySupport(local: &Value, remote: &Value) -> Result<(), String> {
    for (side, clock) in [("Local", local), ("Remote", remote)] {
        if clock.get("consistencyVersion").and_then(Value::as_u64) != Some(2) {
            return Err(format!(
                "{side} peer cannot verify synchronized contents; a compatible version is required"
            ));
        }
    }
    Ok(())
}

impl SpacePersistenceSyncService {
    async fn consistencyReports(&self, target: &str) -> Result<(Report, Report), String> {
        let (local, remote) = tokio::join!(
            self.callLocal("syncConsistency", Value::Null),
            callRemote(
                &self.state.nodeRouter,
                target,
                PeerSyncMethod::SyncConsistency,
                Value::Null
            )
        );
        Ok((local?, remote?))
    }

    pub(super) async fn synchronizeConsistency(&self, target: &str) -> Result<bool, String> {
        let (local, remote) = self.consistencyReports(target).await?;
        let plans = correctionPlans(&local, &remote);
        let mut changed = BTreeSet::new();
        if plans.is_empty() { return verifyReports(&local, &remote, &changed); }
        let mut transfers = Vec::new();
        for (a, b, choice) in plans {
            let result = async {
                let source = if choice == Source::Local { a } else { b }.expect("source exists");
                let source_file = self.transferSnapshot(target, source, choice == Source::Local, false).await?;
                let other = if choice == Source::Local { b } else { a };
                let superseded = match other {
                    Some(manifest) => Some(self.transferSnapshot(target, manifest, choice != Source::Local, true).await?),
                    None => None,
                };
                Ok::<_, String>(FileCorrectionReference { source: source_file, superseded })
            }.await;
            match result {
                Ok(reference) => transfers.push((a, b, choice, reference)),
                Err(error) if snapshotChanged(&error) => { changed.insert(a.or(b).expect("file exists").owner.path.clone()); },
                Err(error) => self.recordPendingWork(target, "consistency transfer", &json!({"errors":[error],"pending":1})),
            }
        }
        if !transfers.is_empty() {
            // One batch check; a changing file cannot prevent independent files from converging.
            let (current_a, current_b) = self.consistencyReports(target).await?;
            for (a, b, choice, reference) in transfers {
                let result = async {
                    if unchangedPair(&current_a, &current_b, a, b).is_err() { return Ok(false); }
                    let source = if choice == Source::Local { a } else { b }.expect("source exists");
                    for local in [choice != Source::Local, choice == Source::Local] {
                        if !self.storageReplicationAllowed(target)? { return Err("Storage replication authorization changed".into()); }
                        let destination = if local { a } else { b };
                        let args = json!({"owner":source.owner,"expected":destination.map_or("", |m|m.token.as_str()),"reference":reference});
                        let repaired: bool = if local { self.callLocal("syncRepairSnapshot",args).await? }
                            else { callRemote(&self.state.nodeRouter,target,PeerSyncMethod::SyncRepairSnapshot,args).await? };
                        if !repaired { return Ok(false); }
                    }
                    Ok::<_, String>(true)
                }.await;
                match result {
                    Ok(true) => {},
                    Ok(false) => { changed.insert(a.or(b).expect("file exists").owner.path.clone()); },
                    Err(error) if snapshotChanged(&error) => { changed.insert(a.or(b).expect("file exists").owner.path.clone()); },
                    Err(error) => self.recordPendingWork(target, "consistency correction", &json!({"errors":[error],"pending":1})),
                }
            }
        }
        let (latest_local, latest_remote) = self.consistencyReports(target).await?;
        // Edits can also arrive after a successful CAS and before this final comparison.
        changed.extend(changedOwners(&local, &latest_local));
        changed.extend(changedOwners(&remote, &latest_remote));
        verifyReports(&latest_local, &latest_remote, &changed)
    }

    async fn transferSnapshot(&self, target: &str, source: &Manifest, local: bool, coverage_only: bool) -> Result<RuntimeFileSyncReference, String> {
        let (name, method) = if coverage_only { ("syncReadCoverage", PeerSyncMethod::SyncReadCoverage) }
            else { ("syncReadSnapshot", PeerSyncMethod::SyncReadSnapshot) };
        let args = json!({"owner":source.owner,"token":source.token});
        let reference: RuntimeFileSyncReference = if local { self.callLocal(name,args).await? }
            else { callRemote(&self.state.nodeRouter,target,method,args).await? };
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

/// The union of inventories includes files that exist on only one endpoint.
fn correctionPlans<'a>(local: &'a Report, remote: &'a Report) -> Vec<(Option<&'a Manifest>, Option<&'a Manifest>, Source)> {
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
        (choice != Source::Equal).then_some((a,b,choice))
    }).collect()
}

/// Absence is also a version: a newly created destination invalidates the captured plan.
fn unchangedPair(local: &Report, remote: &Report, a: Option<&Manifest>, b: Option<&Manifest>) -> Result<(), String> {
    let owner = &a.or(b).ok_or("Correction has no file")?.owner;
    for (report,expected) in [(local,a),(remote,b)] {
        let current = report.manifests.iter().find(|m|m.owner.path == owner.path && m.owner.backend == owner.backend);
        if report.pendingOwners.contains(&owner.path) || current != expected {
            return Err(format!("File changed during transfer; synchronize again: {}",owner.path));
        }
    }
    Ok(())
}

fn changedOwners(before: &Report, after: &Report) -> BTreeSet<String> {
    let mut owners = BTreeMap::new();
    for manifest in &before.manifests { owners.entry((&manifest.owner.path, &manifest.owner.backend)).or_insert((None, None)).0 = Some(manifest); }
    for manifest in &after.manifests { owners.entry((&manifest.owner.path, &manifest.owner.backend)).or_insert((None, None)).1 = Some(manifest); }
    owners.into_iter().filter_map(|((path, _), (before, after))| (before != after).then(|| path.clone()))
        .chain(before.pendingOwners.symmetric_difference(&after.pendingOwners).cloned()).collect()
}

/// Changed owners retry quietly; completion still requires every file and baseline to match.
fn verifyReports(local: &Report, remote: &Report, changed: &BTreeSet<String>) -> Result<bool, String> {
    if !local.errors.is_empty() || !remote.errors.is_empty() {
        return Err("Consistency check has owner errors".into());
    }
    if local.pendingOwners.union(&remote.pendingOwners).any(|path| !changed.contains(path)) {
        return Err("Consistency check has pending owners".into());
    }
    let mut complete = local.pendingOwners.is_empty() && remote.pendingOwners.is_empty();
    for (a,b) in [(local,remote),(remote,local)] {
        for manifest in &a.manifests {
            let comparison = match b.manifests.iter().find(|other|other.owner.path == manifest.owner.path && other.owner.backend == manifest.owner.backend) {
                Some(other) => compare_manifests(manifest,other),
                None => Err(format!("Missing synchronized file: {}",manifest.owner.path)),
            };
            if matches!(comparison, Ok(Source::Equal)) { continue; }
            if changed.contains(&manifest.owner.path) {
                complete = false;
                continue;
            }
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

    fn verifyReports(local: &Report, remote: &Report) -> Result<bool, String> {
        super::verifyReports(local, remote, &BTreeSet::new())
    }

    fn report(snapshot: &Snapshot) -> Report {
        Report {
            manifests: vec![snapshot.manifest().unwrap()],
            errors: vec![],
            pendingOwners: BTreeSet::new(),
        }
    }

    #[test]
    fn batch_version_check_is_per_owner_and_final_verification_checks_every_owner() {
        let snapshot = |path: &str| Snapshot {
            node: "local".into(),
            data: Data::empty(Owner {
                backend: Backend::DataStore,
                path: path.into(),
                format: 1,
                layout: 1,
            }),
            covered: BTreeSet::new(),
            generation: 0,
        };
        let a = snapshot("a");
        let b = snapshot("b");
        let expected = a.manifest().unwrap();
        let mut local = report(&a);
        let mut remote = report(&a);
        local.manifests.push(b.manifest().unwrap());
        remote.manifests.push(b.manifest().unwrap());
        assert!(unchangedPair(&local, &remote, Some(&expected), Some(&expected)).is_ok());
        // A different dataset changing must not cancel this independent correction.
        remote.manifests[1].token = "changed-other-owner".into();
        assert!(unchangedPair(&local, &remote, Some(&expected), Some(&expected)).is_ok());
        remote.manifests[0].token = "changed-source".into();
        assert!(unchangedPair(&local, &remote, Some(&expected), Some(&expected)).is_err());
        remote.manifests[0] = expected.clone();
        local.manifests[0].token = "changed-destination".into();
        assert!(unchangedPair(&local, &remote, Some(&expected), Some(&expected)).is_err());
        local.manifests[0] = expected.clone();
        remote.pendingOwners.insert("a".into());
        assert!(unchangedPair(&local, &remote, Some(&expected), Some(&expected)).is_err());
        remote.pendingOwners.clear();
        remote.manifests.pop();
        assert!(unchangedPair(&local, &remote, Some(&expected), Some(&expected)).is_ok());
        assert!(verifyReports(&local, &remote).is_err());
    }

    #[test]
    fn equal_contents_with_different_baselines_are_not_complete() {
        let snapshot = Snapshot { node: "a".into(), data: Data::empty(Owner {
            backend: Backend::DataStore,path: "file".into(),format: 0,layout: 1,
        }),covered: BTreeSet::new(),generation: 0 };
        let mut a = report(&snapshot);
        let mut b = report(&snapshot);
        a.manifests[0].baseline = Some("before correction".into());
        b.manifests[0].baseline = Some("copied file".into());
        assert_eq!(correctionPlans(&a,&b).len(),1);
        assert!(verifyReports(&a,&b).is_err());
        a.manifests[0].baseline = b.manifests[0].baseline.clone();
        assert!(verifyReports(&a,&b).is_ok());
    }

    #[test]
    fn absent_files_are_correction_targets_and_creation_invalidates_the_plan() {
        let snapshot = Snapshot { node: "a".into(), data: Data::empty(Owner {
            backend: Backend::DataStore, path: "file.preferences.json".into(), format: 0, layout: 1,
        }), covered: BTreeSet::new(), generation: 0 };
        let present = report(&snapshot);
        let empty = Report { manifests: vec![], errors: vec![], pendingOwners: BTreeSet::new() };
        for (a,b,choice) in [(&present,&empty,Source::Local),(&empty,&present,Source::Remote)] {
            let plans = correctionPlans(a,b);
            assert_eq!(plans.len(),1);
            assert_eq!(plans[0].2,choice);
            assert!(unchangedPair(a,b,plans[0].0,plans[0].1).is_ok());
            assert!(unchangedPair(&present,&present,plans[0].0,plans[0].1).is_err());
        }
        assert!(correctionPlans(&present,&present).is_empty());
    }

    #[test]
    fn unavailable_owners_do_not_block_plans_for_independent_ready_files() {
        let snapshot = |path: &str| Snapshot { node: "a".into(), data: Data::empty(Owner {
            backend: Backend::DataStore, path: path.into(), format: 1, layout: 1,
        }), covered: BTreeSet::new(), generation: 0 };
        let ready = snapshot("ready");
        let mut stale = ready.clone();
        stale.node = "b".into();
        stale.data.values.insert("old".into(), json!(true));
        let mut local = report(&ready);
        let mut remote = report(&stale);
        local.pendingOwners.insert("pending".into());
        remote.errors.push("broken: unavailable file".into());
        let unsupported = snapshot("future");
        let mut previous = unsupported.clone();
        previous.data.owner.format = 0;
        local.manifests.push(unsupported.manifest().unwrap());
        remote.manifests.push(previous.manifest().unwrap());
        let plans = correctionPlans(&local, &remote);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].0.unwrap().owner.path, "ready");
        assert!(verifyReports(&local, &remote).is_err());
        local.pendingOwners.insert("ready".into());
        assert!(correctionPlans(&local, &remote).is_empty());
        assert!(verifyReports(&local, &remote).is_err());
        local.pendingOwners.clear();
        remote.errors.clear();
        remote.manifests[0] = local.manifests[0].clone();
        // Even equal contents cannot hide a different, unsupported format in another file.
        assert!(verifyReports(&local, &remote).is_err());
        remote.manifests[1] = local.manifests[1].clone();
        assert!(verifyReports(&local, &remote).is_ok());
    }

    #[test]
    fn only_known_snapshot_races_are_quiet_across_rpc_wrappers() {
        for reason in ["Snapshot changed; synchronize again", "File changed before export"] {
            for error in [reason.to_owned(), format!("local syncReadSnapshot: COMMAND_ERROR: {reason}"),
                format!("remote peer syncReadCoverage: COMMAND_ERROR: {reason}\nRust error location: test.rs:1")] {
                assert!(snapshotChanged(&error), "{error}");
            }
        }
        for error in ["database is corrupt", "permission denied", "Snapshot verification failed after replacement",
            "Snapshot changed; synchronize again: database is corrupt"] {
            assert!(!snapshotChanged(error), "{error}");
        }
    }

    #[test]
    fn concurrent_changes_retry_quietly_without_hiding_other_failures_or_claiming_completion() {
        let snapshot = |path: &str| Snapshot { node: "a".into(), data: Data::empty(Owner {
            backend: Backend::DataStore, path: path.into(), format: 1, layout: 1,
        }), covered: BTreeSet::new(), generation: 0 };
        let original = snapshot("changing");
        let mut changed = original.clone();
        changed.data.values.insert("new".into(), json!(true));
        let local = report(&original);
        let mut remote = report(&changed);
        let retry = BTreeSet::from(["changing".into()]);
        assert_eq!(super::verifyReports(&local, &remote, &retry), Ok(false));
        assert!(verifyReports(&local, &remote).is_err());
        remote.manifests.clear();
        assert_eq!(super::verifyReports(&local, &remote, &retry), Ok(false));
        remote.pendingOwners.insert("changing".into());
        assert_eq!(super::verifyReports(&local, &remote, &retry), Ok(false));
        remote.errors.push("database is corrupt".into());
        assert!(super::verifyReports(&local, &remote, &retry).is_err());
        remote.errors.clear();
        remote.pendingOwners.insert("unrelated".into());
        assert!(super::verifyReports(&local, &remote, &retry).is_err());
        remote.pendingOwners.clear();
        remote.manifests.push(snapshot("unrelated").manifest().unwrap());
        assert!(super::verifyReports(&local, &remote, &retry).is_err());
        remote.manifests = local.manifests.clone();
        assert_eq!(super::verifyReports(&local, &remote, &retry), Ok(true), "a concurrent correction may already have converged");
    }

    #[test]
    fn final_comparison_detects_edits_arriving_after_correction() {
        let original = Snapshot { node: "a".into(), data: Data::empty(Owner {
            backend: Backend::Sql, path: "memory.sqlite".into(), format: 1, layout: 1,
        }), covered: BTreeSet::new(), generation: 0 };
        let before = report(&original);
        let mut edited = original.clone();
        edited.data.values.insert("new".into(), json!(true));
        let after = report(&edited);
        let retry = changedOwners(&before, &after);
        assert_eq!(retry, BTreeSet::from(["memory.sqlite".into()]));
        assert_eq!(super::verifyReports(&before, &after, &retry), Ok(false));
        assert_eq!(super::verifyReports(&after, &after, &retry), Ok(true));
        let empty = Report { manifests: vec![], errors: vec![], pendingOwners: BTreeSet::new() };
        assert_eq!(changedOwners(&before, &empty), retry);
        assert_eq!(changedOwners(&empty, &after), retry);
        assert!(changedOwners(&before, &before).is_empty());
    }

    #[test]
    fn completion_requires_a_supported_consistency_protocol_on_both_peers() {
        let current = json!({"consistencyVersion":2});
        assert!(requireConsistencySupport(&current, &current).is_ok());
        for unsupported in [
            Value::Null,
            json!({}),
            json!({"consistencyVersion":0}),
            json!({"consistencyVersion":1}),
            json!({"consistencyVersion":3}),
        ] {
            assert!(requireConsistencySupport(&unsupported, &current).is_err());
            assert!(requireConsistencySupport(&current, &unsupported).is_err());
        }
    }

    #[test]
    fn completion_requires_equal_contents_and_complete_owner_sets() {
        for backend in [Backend::DataStore, Backend::Sql] {
            let mut data = Data::empty(Owner {
                backend,
                path: "business".into(),
                format: 2,
                layout: 1,
            });
            data.values.insert("x".into(), json!(1));
            let a = Snapshot {
                node: "a".into(),
                data,
                covered: BTreeSet::new(),
                generation: 0,
            };
            let mut b = a.clone();
            b.node = "b".into();
            b.data.values.insert("x".into(), json!(2));
            assert!(verifyReports(&report(&a), &report(&b)).is_err());
            let token = b.token().unwrap();
            assert!(b.replace(&token, &a).unwrap());
            assert!(verifyReports(&report(&a), &report(&b)).is_ok());
            let mut missing = report(&b);
            missing.manifests.clear();
            assert!(verifyReports(&report(&a), &missing).is_err());
            assert!(verifyReports(&missing, &report(&a)).is_err());
            let mut pending = report(&b);
            pending.pendingOwners.insert("business".into());
            assert!(verifyReports(&report(&a), &pending).is_err());
            let mut failed = report(&b);
            failed.errors.push("replacement failed".into());
            assert!(verifyReports(&report(&a), &failed).is_err());
        }
    }
}
