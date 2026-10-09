use super::*;
use crate::NodeSpaceService::NodeSpaceContext;
use crate::NodeSpaceService::space_doctor;
use operit_store::NetworkControlStore::NetworkControlIdentityAssignment;
use operit_store::SyncOperationStore::SyncOperationStore;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;

/// Writes garbage over the last control-domain journal line (the identity
/// grant in this fixture) so the doctor has pollution to find. Journal length
/// is timing-dependent (binding writes land asynchronously), so the line is
/// located by content, never by index.
fn corruptLastControlLine(service: &RuntimeRemoteLinkService, deviceId: &str) {
    let sync = SyncOperationStore::new(service.storage(), RUNTIME_SYNC_DIR_PATH);
    let path = sync.operationsPath(deviceId);
    let content = service.storage().readBytes(&path).unwrap();
    let lines: Vec<&[u8]> = content.split_inclusive(|byte| *byte == b'\n').collect();
    let mut target = None;
    for (index, line) in lines.iter().enumerate() {
        let parsed = std::str::from_utf8(line)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok());
        if parsed.as_ref().and_then(|value| value.get("domain"))
            .and_then(serde_json::Value::as_str) == Some("network_control")
        {
            target = Some(index);
        }
    }
    let index = target.expect("journal carries control lines");
    let mut corrupted = Vec::new();
    for (lineIndex, line) in lines.iter().enumerate() {
        if lineIndex == index {
            corrupted.extend_from_slice(b"###corrupted by doctor test###\n");
        } else {
            corrupted.extend_from_slice(line);
        }
    }
    service.storage().writeBytes(&path, &corrupted).unwrap();
}

/// Damaged journal lines are diagnosed, repaired reversibly and selfchecked.
#[tokio::test]
async fn doctor_detects_quarantines_and_selfchecks_damaged_lines() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, service) = approvalService("doc-local");
    let local = router.localNodeId();
    router.networkControlStore.admitMember("doc-away".into()).unwrap();
    router
        .networkControlStore
        .setIdentity(NetworkControlIdentityAssignment { nodeId: "doc-away".into(), roleId: "admin".into() })
        .unwrap();
    // Corrupt the identity grant (the last control line): the admission
    // survives, so repair must lose exactly one operation while the member
    // stays admitted by the surviving control log.
    corruptLastControlLine(&service, &local);

    let report = space_doctor::diagnose(&service, false).unwrap();
    assert!(report.policyReadable, "resilient reads survive one damaged line");
    let journal = report.journals.iter().find(|journal| journal.deviceId == local).expect("local journal");
    assert_eq!(journal.findings.len(), 1);
    assert!(report.findings.iter().any(|finding| finding.check == "D4_raw_line_scan"));
    assert!(report.counters.divergencesDetected >= 1);

    let repaired = space_doctor::diagnose(&service, true).unwrap();
    let repair = repaired.repair.expect("repair report");
    assert!(repair.selfcheckPassed, "selfcheck detail: {}", repair.detail);
    assert_eq!(repair.linesQuarantined, 1);
    assert_eq!(repair.backupPaths.len(), 1);
    assert!(!repair.restoredFromBackup);
    // The rescanned journals in the repaired report are clean.
    assert!(repaired.journals.iter().all(|journal| journal.findings.is_empty()));
    // The surviving control policy keeps everything except the quarantined
    // operation: the admission survives, so replay still admits the member.
    assert!(repaired.policyReadable);
    assert!(repaired.replayMembers.contains(&"doc-away".to_string()));
    assert!(repaired.counters.linesQuarantined >= 1);

    // The backup preserved the original file bytes, including the damage.
    let backup = repair.backupPaths[0].clone();
    let bytes = service.storage().readBytes(&backup).unwrap();
    assert!(String::from_utf8(bytes).unwrap().contains("###corrupted by doctor test###"));
}

/// The exact divergence class from the original incident is reported.
#[tokio::test]
async fn doctor_reports_member_record_vs_replay_divergence() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, service) = approvalService("doc-diverge");
    // A projection write without any policy operation: the record side claims
    // a member the replayed control log never admitted.
    let mut target = router.spaceStore.initialize().unwrap();
    target.members.push("ghost-member".into());
    router.spaceStore.adopt(target).unwrap();

    let report = space_doctor::diagnose(&service, false).unwrap();
    assert!(report.policyReadable);
    assert!(report.recordMembers.contains(&"ghost-member".to_string()));
    assert!(!report.replayMembers.contains(&"ghost-member".to_string()));
    let divergence = report
        .findings
        .iter()
        .find(|finding| finding.check == "D1_member_record_vs_replay")
        .expect("divergence finding");
    assert_eq!(divergence.severity, "error");
    assert!(divergence.summary.contains("ghost-member"));
}
