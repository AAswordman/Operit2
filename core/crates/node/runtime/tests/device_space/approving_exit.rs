use super::*;
use crate::NodeServices::NodeServices;
use crate::NodeSpaceService::space_join as join;
use crate::NodeSpaceService::{PeerSpaceSnapshot, SpaceJoinRequest, SpaceJoinStatus};
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceDeviceProfile};
use operit_store::NetworkControlStore::NetworkControlIdentityAssignment;

/// Builds one gateway-side record whose reviewer already claimed an approval.
fn reviewingRecord(reviewer: &str, targetSpaceId: &str, version: u64) -> join::Record {
    join::Record {
        request: SpaceJoinRequest {
            requestId: format!("gx-request-{version}"),
            targetDeviceId: reviewer.into(),
            applicantDeviceId: "gx-applicant".into(),
            applicantName: "Applicant".into(),
            spaceName: "Target".into(),
            status: SpaceJoinStatus::Approving,
            createdAt: 1,
            expiresAt: currentTimeMillis() + 60_000,
            canApprove: false,
            reviewerDeviceId: Some(reviewer.into()),
            reviewerName: None,
            reviewerHops: None,
            assignmentVersion: version,
            decisionApprove: Some(true),
        },
        sourceSpaceId: "gx-source".into(),
        sourceRevision: 1,
        targetSpaceId: targetSpaceId.into(),
        profile: CoreSpaceDeviceProfile {
            nodeId: "gx-applicant".into(),
            displayName: "Applicant".into(),
            userName: String::new(),
            platform: "test".into(),
            model: "test".into(),
            coreVersion: None,
            updatedAt: 1,
        },
        source: PeerSpaceSnapshot {
            space: CoreSpace {
                spaceId: "gx-source".into(),
                spaceName: "Source".into(),
                spaceRevision: 1,
                members: vec!["gx-applicant".into()],
            },
            deviceProfiles: vec![CoreSpaceDeviceProfile {
                nodeId: "gx-applicant".into(),
                displayName: "Applicant".into(),
                userName: String::new(),
                platform: "test".into(),
                model: "test".into(),
                coreVersion: None,
                updatedAt: 1,
            }],
            controlOperations: Vec::new(),
            topology: Vec::new(),
        },
        accepted: None,
        unavailableSince: None,
        approvedDecision: Some(true),
        decisionRevision: Some(2),
        cancelRequested: false,
    }
}

/// Installs node services so unreachable reviewers resolve instead of erroring.
fn selfMesh(router: &CoreNodeRouter) {
    let peer = ApprovalMeshPeer::new(router.localNodeId());
    peer.link(router);
    router.installNodeServices(NodeServices::new(peer)).unwrap();
}

/// A committed decision never moves reviewers and reconcile completes it locally.
#[tokio::test]
async fn committed_approving_decision_is_pinned_and_locally_recovered() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, service) = approvalService("gx-pin");
    selfMesh(&router);
    let space = router.spaceStore.initialize().unwrap();
    let mut record = reviewingRecord(router.localNodeId().as_str(), &space.spaceId, 3);
    // The reviewer crashed after its admission reached the control log but
    // before completeDecision could write the Approved record.
    router
        .networkControlStore
        .admitSpaceForReview(
            "gx-source".into(),
            record.source.space.members.iter().cloned().collect(),
            &format!("{}-3", record.request.requestId),
        )
        .unwrap();
    join::saveRecord(&service, join::INBOUND, &record.request.requestId, &record).unwrap();

    // Committed: pinned even once the grace window has long passed.
    join::assign(&service, &mut record, currentTimeMillis() + 10 * 60_000).unwrap();
    assert_eq!(record.request.status, SpaceJoinStatus::Approving);
    assert_eq!(record.request.reviewerDeviceId.as_deref(), Some(router.localNodeId().as_str()));
    assert_eq!(record.request.assignmentVersion, 3);

    // Any later reconcile rebuilds the projection and the applicant receipt.
    join::reconcile(&service, &mut record).unwrap();
    assert_eq!(record.request.status, SpaceJoinStatus::Approved);
    assert!(record.accepted.is_some());
    assert!(router.spaceStore.initialize().unwrap().members.contains(&"gx-applicant".to_string()));
    assert!(router.networkControlStore.currentState().unwrap().memberNodeIds.contains("gx-applicant"));
}

/// An uncommitted decision reassigns only after the offline grace window.
#[tokio::test]
async fn uncommitted_approving_decision_reassigns_after_grace() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, service) = approvalService("gx-reassign");
    selfMesh(&router);
    let space = router.spaceStore.initialize().unwrap();
    // A capable reviewer that is a member with admin rights, but never reachable.
    router.networkControlStore.admitMember("gx-away".into()).unwrap();
    router
        .networkControlStore
        .setIdentity(NetworkControlIdentityAssignment { nodeId: "gx-away".into(), roleId: "admin".into() })
        .unwrap();
    // Policy commands do not rewrite the member-record projection; align it.
    let mut target = router.spaceStore.initialize().unwrap();
    target.members.push("gx-away".into());
    target.members.sort();
    router.spaceStore.adopt(target).unwrap();
    let mut record = reviewingRecord("gx-away", &space.spaceId, 1);

    // Inside the grace window the claim is stable; no reassignment happens.
    join::assign(&service, &mut record, 1_000).unwrap();
    assert_eq!(record.request.status, SpaceJoinStatus::Approving);
    assert_eq!(record.request.reviewerDeviceId.as_deref(), Some("gx-away"));
    assert_eq!(record.request.assignmentVersion, 1);
    assert!(record.unavailableSince.is_some());

    // Past the grace window the gateway claims the uncommitted decision itself.
    join::assign(&service, &mut record, 1_000 + 31_000).unwrap();
    assert_eq!(record.request.reviewerDeviceId.as_deref(), Some(router.localNodeId().as_str()));
    assert_eq!(record.request.assignmentVersion, 2);
    // Reassignment moves the review, never the decision already recorded.
    assert_eq!(record.request.status, SpaceJoinStatus::Approving);
    assert_eq!(record.approvedDecision, Some(true));
}

/// A committed admission that later policy excludes is reported, not forged.
#[tokio::test]
async fn committed_but_excluded_decision_is_not_forged_into_approval() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, service) = approvalService("gx-excluded");
    selfMesh(&router);
    let space = router.spaceStore.initialize().unwrap();
    let mut record = reviewingRecord(router.localNodeId().as_str(), &space.spaceId, 5);
    router
        .networkControlStore
        .admitSpaceForReview(
            "gx-source".into(),
            record.source.space.members.iter().cloned().collect(),
            &format!("{}-5", record.request.requestId),
        )
        .unwrap();
    router.networkControlStore.removeMember("gx-applicant".into()).unwrap();

    join::reconcile(&service, &mut record).unwrap();
    assert_eq!(record.request.status, SpaceJoinStatus::Approving);
    assert!(record.accepted.is_none());
}
