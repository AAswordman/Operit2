//! Final materialized-content comparison after the ordinary history exchange is empty.
use super::*;
use operit_store::Transaction::{compare_manifests, Manifest, Source};

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
        if clock.get("consistencyVersion").and_then(Value::as_u64) != Some(1) {
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

    pub(super) async fn synchronizeConsistency(&self, target: &str) -> Result<(), String> {
        let (local, remote) = self.consistencyReports(target).await?;
        for errors in [&local.errors, &remote.errors] {
            self.recordPendingWork(target, "consistency", &json!({"errors": errors}));
        }
        // Each dataset can complete while another awaits its owner or migration.
        for a in &local.manifests {
            let Some(b) = remote
                .manifests
                .iter()
                .find(|b| b.owner.path == a.owner.path && b.owner.backend == a.owner.backend)
            else {
                self.recordPendingWork(target, "consistency", &json!({"pending":1}));
                continue;
            };
            let result = self.correctDataset(target, a, b).await;
            if let Err(error) = result {
                self.recordPendingWork(
                    target,
                    "consistency",
                    &json!({"errors":[error], "pending":1}),
                );
            }
        }
        for b in &remote.manifests {
            if !local
                .manifests
                .iter()
                .any(|a| a.owner.path == b.owner.path && a.owner.backend == b.owner.backend)
            {
                self.recordPendingWork(target, "consistency", &json!({"pending":1}));
            }
        }
        // Recheck the whole set after all independent repairs. An error or a
        // dataset becoming pending keeps the round incomplete, even if others converged.
        let (local, remote) = self.consistencyReports(target).await?;
        verifyReports(&local, &remote)
    }

    async fn correctDataset(&self, target: &str, a: &Manifest, b: &Manifest) -> Result<(), String> {
        let choice = compare_manifests(a, b)?;
        if choice == Source::Equal {
            return Ok(());
        }
        let (source, destination) = if choice == Source::Local {
            (a, b)
        } else {
            (b, a)
        };
        let args = json!({"owner":source.owner, "token":source.token});
        let reference: RuntimeFileSyncReference = if choice == Source::Local {
            self.callLocal("syncReadSnapshot", args).await?
        } else {
            callRemote(
                &self.state.nodeRouter,
                target,
                PeerSyncMethod::SyncReadSnapshot,
                args,
            )
            .await?
        };
        self.synchronizeBlobReferences(target, vec![reference.clone()])
            .await?;
        let (currentA, currentB) = self.consistencyReports(target).await?;
        let unchanged =
            |report: &Report, expected: &Manifest| report.manifests.iter().any(|m| m == expected);
        if !unchanged(&currentA, a) || !unchanged(&currentB, b) {
            return Err("Snapshot changed during transfer; synchronize again".into());
        }
        if !self.storageReplicationAllowed(target)? {
            return Err("Storage replication authorization changed".into());
        }
        let args =
            json!({"owner":destination.owner, "expected":destination.token, "reference":reference});
        let repaired: bool = if choice == Source::Local {
            callRemote(
                &self.state.nodeRouter,
                target,
                PeerSyncMethod::SyncRepairSnapshot,
                args,
            )
            .await?
        } else {
            self.callLocal("syncRepairSnapshot", args).await?
        };
        if !repaired {
            return Err("Snapshot changed before correction; synchronize again".into());
        }
        let (finalA, finalB) = self.consistencyReports(target).await?;
        let a = finalA
            .manifests
            .iter()
            .find(|m| m.owner == a.owner)
            .ok_or("Local owner became pending")?;
        let b = finalB
            .manifests
            .iter()
            .find(|m| m.owner == b.owner)
            .ok_or("Remote owner became pending")?;
        if compare_manifests(a, b)? != Source::Equal {
            return Err("Materialized contents still differ after correction".into());
        }
        Ok(())
    }
}

/// Completion means equal materialized contents for every ready owner on both peers.
fn verifyReports(local: &Report, remote: &Report) -> Result<(), String> {
    if !local.errors.is_empty() || !remote.errors.is_empty() {
        return Err("Consistency check has owner errors".into());
    }
    if !local.pendingOwners.is_empty() || !remote.pendingOwners.is_empty() {
        return Err("Consistency check has pending owners".into());
    }
    for (a, b) in [(local, remote), (remote, local)] {
        for owner in &a.manifests {
            let other = b
                .manifests
                .iter()
                .find(|m| {
                    m.owner.path == owner.owner.path && m.owner.backend == owner.owner.backend
                })
                .ok_or("Consistency check is missing an owner")?;
            if compare_manifests(owner, other)? != Source::Equal {
                return Err("Materialized contents differ; synchronize again".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_store::Transaction::{Backend, Data, Owner, Snapshot};

    fn report(snapshot: &Snapshot) -> Report {
        Report {
            manifests: vec![snapshot.manifest().unwrap()],
            errors: vec![],
            pendingOwners: BTreeSet::new(),
        }
    }

    #[test]
    fn completion_requires_a_supported_consistency_protocol_on_both_peers() {
        let current = json!({"consistencyVersion":1});
        assert!(requireConsistencySupport(&current, &current).is_ok());
        for unsupported in [
            Value::Null,
            json!({}),
            json!({"consistencyVersion":0}),
            json!({"consistencyVersion":2}),
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
