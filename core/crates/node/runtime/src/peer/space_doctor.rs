//! Read-mostly diagnostics for the shared-Space replication plane. The doctor
//! compares every layer of local truth (pairing, member records, policy
//! replay, join review state) against the raw operation journals, so existing
//! pollution becomes visible instead of poisoning whole features silently.
//! `--repair` only ever quarantines damaged journal lines through the store's
//! own locked entry point, after a timestamped backup, verified by a shadow
//! reconstruction of the control command set.

use super::*;
use crate::PeerStateStore::PeerStateStore;
use operit_store::NetworkControlStore::{NetworkControlCommandRecord, NETWORK_CONTROL_SYNC_DOMAIN};
use operit_store::PreferencesDataStore::CoreNodeStateStore;
use operit_store::SyncOperationStore::{
    OperationLogLineFinding, SyncOperation, SyncOperationStore,
};
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

pub(crate) const CONTROL_PROJECTION_PATH: &str = "runtime/link_access/space_policy.preferences.json";
const COUNTERS_PATH: &str = "runtime/diagnostics/space_doctor_counters.preferences.json";
const COUNTERS_KEY: &str = "counters";
static COUNTERS_MUTATION: Mutex<()> = Mutex::new(());

/// One observation the doctor wants a human or caller to see.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceDoctorFinding {
    pub severity: String,
    pub check: String,
    pub summary: String,
}

/// Pairing-layer health for one paired peer (D2/D8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairHealth {
    pub deviceId: String,
    pub pairedInbound: bool,
    pub pairedOutbound: bool,
    pub member: bool,
    pub online: bool,
}

/// Join review state judged through the shared commit predicate (D3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinHealth {
    pub requestId: String,
    pub applicantDeviceId: String,
    pub status: SpaceJoinStatus,
    pub reviewerDeviceId: Option<String>,
    pub assignmentVersion: u64,
    pub approvedDecision: Option<bool>,
    /// The admission reached the control log; any replica can verify this.
    pub committed: bool,
    /// Approving + committed + replay still admits: reconcile() completes it.
    pub recoverable: bool,
    /// Approving + uncommitted + reviewer gone past the grace window: the
    /// next review pass reassigns the record instead of leaving it stuck.
    pub reassignable: bool,
}

/// Raw journal classification for one origin (D4/D5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JournalHealth {
    pub deviceId: String,
    pub exists: bool,
    pub totalLines: u64,
    pub decodedLines: u64,
    pub highestSequence: i64,
    pub findings: Vec<OperationLogLineFinding>,
    /// Control-domain operations whose payload does not decode into a command.
    pub corruptPayloads: Vec<String>,
}

/// Dual-copy comparison between journals and the control projection (D6).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionHealth {
    pub projectionEntries: usize,
    pub journalOnlyOperationIds: Vec<String>,
    pub projectionOnlyOperationIds: Vec<String>,
}

/// Counters persisted for trend observability across doctor invocations.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceDoctorCounters {
    #[serde(default)]
    pub divergencesDetected: u64,
    #[serde(default)]
    pub linesQuarantined: u64,
    #[serde(default)]
    pub repairsRun: u64,
    #[serde(default)]
    pub repairsFailed: u64,
    #[serde(default)]
    pub reconcileExchanges: u64,
    #[serde(default)]
    pub reconcileRounds: u64,
    #[serde(default)]
    pub reconcileIncomplete: u64,
    #[serde(default)]
    pub oversizeOffersRefused: u64,
}

/// What `--repair` did, including the selfcheck verdict.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepairReport {
    pub backupPaths: Vec<String>,
    pub linesQuarantined: u64,
    pub selfcheckPassed: bool,
    pub restoredFromBackup: bool,
    pub detail: String,
}

/// The full diagnosis. Read-only unless `repair` is Some.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpaceHealthReport {
    pub localNodeId: String,
    pub spaceId: String,
    pub spaceName: String,
    pub spaceRevision: i64,
    pub policyReadable: bool,
    pub policyError: Option<String>,
    pub recordMembers: Vec<String>,
    pub replayMembers: Vec<String>,
    pub pairs: Vec<PairHealth>,
    pub joins: Vec<JoinHealth>,
    pub journals: Vec<JournalHealth>,
    pub projection: ProjectionHealth,
    pub clockAnomalies: Vec<String>,
    pub findings: Vec<SpaceDoctorFinding>,
    pub counters: SpaceDoctorCounters,
    pub repair: Option<RepairReport>,
}

pub(crate) fn readCounters(service: &dyn NodeSpaceContext) -> SpaceDoctorCounters {
    PeerStateStore::new(service.storage())
        .records::<SpaceDoctorCounters>(COUNTERS_PATH)
        .ok()
        .and_then(|records| records.get(COUNTERS_KEY).cloned())
        .unwrap_or_default()
}

pub(crate) fn updateCounters(
    service: &dyn NodeSpaceContext,
    mutate: impl FnOnce(&mut SpaceDoctorCounters),
) -> Result<(), String> {
    let _lock = COUNTERS_MUTATION.lock().map_err(|error| error.to_string())?;
    let store = PeerStateStore::new(service.storage());
    let mut counters = store
        .records::<SpaceDoctorCounters>(COUNTERS_PATH)?
        .get(COUNTERS_KEY)
        .cloned()
        .unwrap_or_default();
    mutate(&mut counters);
    store.putRecord(COUNTERS_PATH, COUNTERS_KEY, &counters)
}

/// Decodes the command envelope of one control operation, if it still can be.
fn commandSpaceId(operation: &SyncOperation) -> Result<String, String> {
    NetworkControlCommandRecord::deserialize(&operation.payload)
        .map(|record| record.spaceId)
        .map_err(|error| error.to_string())
}

/// Rebuilds the shadow control command set for one Space: every decodable
/// control line in every origin journal plus every projection entry,
/// deduplicated by opId. Conflicts between the two copies are collected.
fn shadowControlCommands(
    operations: &[SyncOperation],
    spaceId: &str,
) -> (BTreeMap<String, SyncOperation>, Vec<String>) {
    let mut shadow: BTreeMap<String, SyncOperation> = BTreeMap::new();
    let mut conflicts = Vec::new();
    for operation in operations {
        if operation.domain != NETWORK_CONTROL_SYNC_DOMAIN {
            continue;
        }
        if commandSpaceId(operation).as_deref() != Ok(spaceId) {
            continue;
        }
        match shadow.get(&operation.opId) {
            Some(existing) if existing != operation => {
                conflicts.push(format!("{} differs between copies", operation.opId));
            }
            Some(_) => {}
            None => {
                shadow.insert(operation.opId.clone(), operation.clone());
            }
        }
    }
    (shadow, conflicts)
}

/// Runs the full diagnosis; with `repair` it quarantines damaged lines and
/// verifies the store view against a shadow reconstruction afterwards.
pub fn diagnose(service: &dyn NodeSpaceContext, repair: bool) -> Result<SpaceHealthReport, String> {
    let sync = SyncOperationStore::new(service.storage(), RUNTIME_SYNC_DIR_PATH);
    let localNodeId = service.localNodeId();
    let space = service.spaceStore().initialize()?;
    let spaceId = space.spaceId.clone();
    let mut findings: Vec<SpaceDoctorFinding> = Vec::new();
    let mut divergences = 0u64;

    // R4: raw journals, classified line by line (D4), payloads probed (D5).
    let scanJournals = |sync: &SyncOperationStore,
                        findings: &mut Vec<SpaceDoctorFinding>|
     -> Result<(Vec<JournalHealth>, Vec<SyncOperation>), String> {
        let mut journals = Vec::new();
        let mut operations = Vec::new();
        for deviceId in sync.devices().map_err(|error| error.to_string())? {
            let scan = sync.scanOperationLog(&deviceId).map_err(|error| error.to_string())?;
            let mut corruptPayloads = Vec::new();
            for operation in &scan.operations {
                if operation.domain == NETWORK_CONTROL_SYNC_DOMAIN {
                    if let Err(error) = commandSpaceId(operation) {
                        corruptPayloads.push(format!("{}: {error}", operation.opId));
                    }
                }
            }
            operations.extend(scan.operations.iter().cloned());
            journals.push(JournalHealth {
                deviceId,
                exists: scan.exists,
                totalLines: scan.totalLines,
                decodedLines: scan.decodedLines,
                highestSequence: scan.highestSequence,
                findings: scan.findings,
                corruptPayloads,
            });
        }
        Ok((journals, operations))
    };
    let (mut journals, journalOperations) = scanJournals(&sync, &mut findings)?;
    for journal in &journals {
        if !journal.findings.is_empty() {
            findings.push(SpaceDoctorFinding {
                severity: "error".into(),
                check: "D4_raw_line_scan".into(),
                summary: format!(
                    "origin {} journal carries {} corrupt line(s); first at line {}",
                    journal.deviceId,
                    journal.findings.len(),
                    journal.findings[0].lineNumber
                ),
            });
            divergences += 1;
        }
        for payload in &journal.corruptPayloads {
            findings.push(SpaceDoctorFinding {
                severity: "error".into(),
                check: "D5_payload_decode".into(),
                summary: format!("control payload does not decode: {payload}"),
            });
            divergences += 1;
        }
    }

    // D7: floors that point past the surviving journal content.
    let mut clockAnomalies = Vec::new();
    for journal in &journals {
        let floor = sync.exportFloorFor(&journal.deviceId).map_err(|error| error.to_string())?;
        if floor > journal.highestSequence {
            clockAnomalies.push(format!(
                "origin {} export floor {} exceeds highest journal sequence {}",
                journal.deviceId, floor, journal.highestSequence
            ));
        }
    }
    for anomaly in &clockAnomalies {
        findings.push(SpaceDoctorFinding {
            severity: "warning".into(),
            check: "D7_clock_floor_sanity".into(),
            summary: anomaly.clone(),
        });
    }

    // Repair (§3.1): quarantine moves damaged lines out after a backup and is
    // verified against the shadow reconstruction; the report rescans after it.
    let repairReport = if repair {
        Some(runRepair(service, &sync, &journals, &spaceId)?)
    } else {
        None
    };
    let mut journals = journals;
    if repairReport.is_some() {
        let (rescanned, _) = scanJournals(&sync, &mut Vec::new())?;
        journals = rescanned;
    }

    // L3: policy replay readability (resilient reads make this rare, but a
    // poisoned projection or conflicting copies can still fail it).
    let policy = service.networkControlStore().currentState();
    let (policyReadable, policyError, replayMembers) = match &policy {
        Ok(state) => (true, None, state.memberNodeIds.iter().cloned().collect::<Vec<_>>()),
        Err(error) => (false, Some(error.clone()), Vec::new()),
    };
    if !policyReadable {
        findings.push(SpaceDoctorFinding {
            severity: "error".into(),
            check: "D1_member_record_vs_replay".into(),
            summary: format!(
                "policy replay is unreadable: {}",
                policyError.clone().unwrap_or_default()
            ),
        });
        divergences += 1;
    }

    // D1: the record side against the replay side of the same membership.
    let mut recordMembers = space.members.clone();
    recordMembers.sort();
    if policyReadable {
        let record: BTreeSet<&String> = recordMembers.iter().collect();
        let replay: BTreeSet<&String> = replayMembers.iter().collect();
        let recordOnly: Vec<&String> = record.difference(&replay).cloned().collect();
        let replayOnly: Vec<&String> = replay.difference(&record).cloned().collect();
        if !recordOnly.is_empty() || !replayOnly.is_empty() {
            findings.push(SpaceDoctorFinding {
                severity: "error".into(),
                check: "D1_member_record_vs_replay".into(),
                summary: format!(
                    "member records and replayed policy diverge: records-only {:?}, replay-only {:?}",
                    recordOnly, replayOnly
                ),
            });
            divergences += 1;
        }
    }

    // D2/D8: pairing layer against membership and liveness.
    let mut pairs = Vec::new();
    let mut paired = Vec::new();
    let mut active = BTreeSet::new();
    match service.peers() {
        Ok(peers) => match (peers.pairedPeers(), peers.activePeerNodeIds()) {
            (Ok(pairedPeers), Ok(activePeers)) => {
                paired = pairedPeers;
                active = activePeers;
            }
            (error, _) => {
                let reason = error.err().map(|error| error.to_string()).unwrap_or_default();
                findings.push(SpaceDoctorFinding {
                    severity: "warning".into(),
                    check: "D8_availability_snapshot".into(),
                    summary: format!("peer liveness unavailable: {reason}"),
                });
            }
        },
        Err(error) => findings.push(SpaceDoctorFinding {
            severity: "warning".into(),
            check: "D8_availability_snapshot".into(),
            summary: format!("peer service unavailable: {error}"),
        }),
    }
    for peer in paired {
        let member = recordMembers.contains(&peer.nodeId);
        pairs.push(PairHealth {
            member,
            online: active.contains(&peer.nodeId),
            deviceId: peer.nodeId,
            pairedInbound: peer.inbound,
            pairedOutbound: peer.outbound,
        });
    }

    // D3: join review state through the shared commit predicate.
    let joins = space_join::doctorSummaries(service)?;
    for join in &joins {
        if join.status == SpaceJoinStatus::Approving && !join.committed && join.reassignable {
            findings.push(SpaceDoctorFinding {
                severity: "warning".into(),
                check: "D3_join_vs_log".into(),
                summary: format!(
                    "join {} is stuck in Approving with a vanished reviewer; the next review pass reassigns it",
                    join.requestId
                ),
            });
        }
    }

    // D6: dual-copy consistency between journals and the control projection.
    let projectionStore = CoreNodeStateStore::newWithStorage(service.storage(), CONTROL_PROJECTION_PATH);
    let projectionEntries = projectionStore.data().map_err(|error| error.to_string())?.entries();
    let mut projectionOperations = Vec::new();
    for (_, encoded) in &projectionEntries {
        match serde_json::from_str::<SyncOperation>(encoded) {
            Ok(operation) => projectionOperations.push(operation),
            Err(error) => findings.push(SpaceDoctorFinding {
                severity: "error".into(),
                check: "D6_dual_copy_consistency".into(),
                summary: format!("control projection entry does not parse: {error}"),
            }),
        }
    }
    let controlsCurrentSpace = |operation: &SyncOperation| {
        operation.domain == NETWORK_CONTROL_SYNC_DOMAIN && commandSpaceId(operation).as_deref() == Ok(spaceId.as_str())
    };
    let journalIds: BTreeSet<String> = journalOperations
        .iter()
        .filter(|operation| controlsCurrentSpace(operation))
        .map(|operation| operation.opId.clone())
        .collect();
    let projectionIds: BTreeSet<String> = projectionOperations
        .iter()
        .filter(|operation| controlsCurrentSpace(operation))
        .map(|operation| operation.opId.clone())
        .collect();
    let projectionOnly: Vec<String> = projectionIds.difference(&journalIds).cloned().collect();
    let journalOnly: Vec<String> = journalIds.difference(&projectionIds).cloned().collect();
    if !projectionOnly.is_empty() {
        findings.push(SpaceDoctorFinding {
            severity: "warning".into(),
            check: "D6_dual_copy_consistency".into(),
            summary: format!(
                "{} control operation(s) exist only in the projection copy",
                projectionOnly.len()
            ),
        });
    }
    let projection = ProjectionHealth {
        projectionEntries: projectionEntries.len(),
        journalOnlyOperationIds: journalOnly,
        projectionOnlyOperationIds: projectionOnly,
    };

    if divergences > 0 {
        let _ = updateCounters(service, |counters| counters.divergencesDetected += divergences);
    }
    let counters = readCounters(service);
    Ok(SpaceHealthReport {
        localNodeId,
        spaceId,
        spaceName: space.spaceName,
        spaceRevision: space.spaceRevision,
        policyReadable,
        policyError,
        recordMembers,
        replayMembers,
        pairs,
        joins,
        journals,
        projection,
        clockAnomalies,
        findings,
        counters,
        repair: repairReport,
    })
}

/// Quarantines every damaged journal line after a timestamped backup, then
/// verifies the store's control view against the shadow reconstruction over
/// the surviving raw lines plus the projection copy. Any selfcheck failure
/// restores the backups and reports the repair as failed.
fn runRepair(
    service: &dyn NodeSpaceContext,
    sync: &SyncOperationStore,
    journals: &[JournalHealth],
    spaceId: &str,
) -> Result<RepairReport, String> {
    let storage = service.storage();
    let timestamp = currentTimeMillis();
    let mut backupPaths = Vec::new();
    let mut quarantineByDevice: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for journal in journals {
        if journal.findings.is_empty() { continue; }
        let offsets: Vec<u64> = journal.findings.iter().map(|finding| finding.byteOffset).collect();
        let path = sync.operationsPath(&journal.deviceId);
        if storage.exists(&path).map_err(|error| error.to_string())? {
            let content = storage.readBytes(&path).map_err(|error| error.to_string())?;
            let backup = format!("{path}.bak-{timestamp}");
            storage.writeBytes(&backup, &content).map_err(|error| error.to_string())?;
            backupPaths.push(backup);
        }
        quarantineByDevice.insert(journal.deviceId.clone(), offsets);
    }
    if quarantineByDevice.is_empty() {
        return Ok(RepairReport {
            backupPaths,
            linesQuarantined: 0,
            selfcheckPassed: true,
            restoredFromBackup: false,
            detail: "no damaged journal lines to quarantine".into(),
        });
    }

    let mut quarantined = 0u64;
    for (deviceId, offsets) in &quarantineByDevice {
        let moved = sync.quarantineOperationLines(deviceId, offsets).map_err(|error| error.to_string())?;
        quarantined += moved.len() as u64;
    }

    // Selfcheck: the store's control view must equal the shadow set rebuilt
    // from the surviving raw journal lines plus the projection copy.
    let mut shadowOperations = Vec::new();
    for deviceId in sync.devices().map_err(|error| error.to_string())? {
        let scan = sync.scanOperationLog(&deviceId).map_err(|error| error.to_string())?;
        shadowOperations.extend(scan.operations.iter().cloned());
    }
    let projectionStore = CoreNodeStateStore::newWithStorage(service.storage(), CONTROL_PROJECTION_PATH);
    for (_, encoded) in projectionStore.data().map_err(|error| error.to_string())?.entries() {
        if let Ok(operation) = serde_json::from_str::<SyncOperation>(&encoded) {
            shadowOperations.push(operation);
        }
    }
    let (shadow, conflicts) = shadowControlCommands(&shadowOperations, spaceId);
    let (selfcheckPassed, detail) = if !conflicts.is_empty() {
        (false, format!("shadow reconstruction found conflicts: {conflicts:?}"))
    } else {
        match service.networkControlStore().currentSpaceOperations() {
            Ok(storeOperations) => {
                let storeView: BTreeMap<String, SyncOperation> = storeOperations
                    .into_iter()
                    .map(|operation| (operation.opId.clone(), operation))
                    .collect();
                if storeView == shadow {
                    (true, format!("shadow reconstruction matches {} control operation(s)", shadow.len()))
                } else {
                    let missing: Vec<String> = shadow.keys().filter(|id| !storeView.contains_key(*id)).cloned().collect();
                    let extra: Vec<String> = storeView.keys().filter(|id| !shadow.contains_key(*id)).cloned().collect();
                    (false, format!("shadow mismatch: shadow-only {missing:?}, store-only {extra:?}"))
                }
            }
            Err(error) => (false, format!("store control read failed after repair: {error}")),
        }
    };

    let restored = if selfcheckPassed {
        false
    } else {
        for backup in &backupPaths {
            let original = backup.strip_suffix(&format!(".bak-{timestamp}")).unwrap_or(backup);
            if let Ok(content) = storage.readBytes(backup) {
                storage.writeBytes(original, &content).map_err(|error| error.to_string())?;
            }
        }
        true
    };
    let _ = updateCounters(service, |counters| {
        if selfcheckPassed {
            counters.repairsRun += 1;
            counters.linesQuarantined += quarantined;
        } else {
            counters.repairsFailed += 1;
        }
    });
    Ok(RepairReport {
        linesQuarantined: if restored { 0 } else { quarantined },
        selfcheckPassed,
        restoredFromBackup: restored,
        detail,
        backupPaths,
    })
}
