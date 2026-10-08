use super::*;
/// Builds a pending protocol fixture without executing membership changes.
fn record(status: SpaceJoinStatus) -> Record {
    Record {
        request: SpaceJoinRequest {
            requestId: "request".into(),
            targetDeviceId: "gateway".into(),
            applicantDeviceId: "applicant".into(),
            applicantName: "Applicant".into(),
            spaceName: "Space".into(),
            status,
            createdAt: 1,
            expiresAt: 100,
            canApprove: false,
            reviewerDeviceId: None,
            reviewerName: None,
            reviewerHops: None,
            assignmentVersion: 0,
            decisionApprove: None,
        },
        sourceSpaceId: "source".into(),
        sourceRevision: 1,
        targetSpaceId: "target".into(),
        profile: CoreSpaceDeviceProfile {
            nodeId: "applicant".into(),
            displayName: "Applicant".into(),
            userName: String::new(),
            platform: "test".into(),
            model: "test".into(),
            coreVersion: None,
            updatedAt: 1,
        },
        source: PeerSpaceSnapshot {
            space: CoreSpace {
                spaceId: "source".into(),
                spaceName: "Source".into(),
                spaceRevision: 1,
                members: vec!["applicant".into()],
            },
            deviceProfiles: Vec::new(),
            controlOperations: Vec::new(),
            topology: Vec::new(),
        },
        accepted: None,
        unavailableSince: None,
        approvedDecision: None,
        decisionRevision: None,
        cancelRequested: false,
    }
}
/// Verifies late polling responses cannot undo cancellation or an in-progress decision.
#[test]
fn outgoing_responses_do_not_resurrect_or_regress_requests() {
    assert!(preserveOutgoingStatus(
        &SpaceJoinStatus::Cancelled,
        &SpaceJoinStatus::Pending
    ));
    assert!(preserveOutgoingStatus(
        &SpaceJoinStatus::Cancelled,
        &SpaceJoinStatus::Approved
    ));
    assert!(preserveOutgoingStatus(
        &SpaceJoinStatus::Joined,
        &SpaceJoinStatus::Pending
    ));
    assert!(preserveOutgoingStatus(
        &SpaceJoinStatus::Approving,
        &SpaceJoinStatus::Pending
    ));
    assert!(preserveOutgoingStatus(
        &SpaceJoinStatus::Approved,
        &SpaceJoinStatus::Approving
    ));
    assert!(!preserveOutgoingStatus(
        &SpaceJoinStatus::Pending,
        &SpaceJoinStatus::Cancelled
    ));
    assert!(!preserveOutgoingStatus(
        &SpaceJoinStatus::Approved,
        &SpaceJoinStatus::Joined
    ));
}

/// Exercises only pending requests expire not committed or claimed decisions through the device-space contract fixture.
#[test]
fn only_pending_requests_expire_not_committed_or_claimed_decisions() {
    for status in [
        SpaceJoinStatus::Approving,
        SpaceJoinStatus::Approved,
        SpaceJoinStatus::Joined,
        SpaceJoinStatus::Rejected,
    ] {
        let mut r = record(status.clone());
        expire(&mut r, 101, "target");
        assert_eq!(r.request.status, status);
    }
    let mut r = record(SpaceJoinStatus::Pending);
    expire(&mut r, 101, "target");
    assert_eq!(r.request.status, SpaceJoinStatus::Expired);
    let mut r = record(SpaceJoinStatus::Pending);
    expire(&mut r, 1, "another");
    assert_eq!(r.request.status, SpaceJoinStatus::Cancelled);
}
/// Exercises hop distances use directed edges and do not transit ordinary members through the device-space contract fixture.
#[test]
fn hop_distances_use_directed_edges_and_do_not_transit_ordinary_members() {
    let edge = |a: &str, b: &str| CoreSpaceDeviceConnection {
        firstDeviceId: a.into(),
        secondDeviceId: b.into(),
    };
    let edges = [
        edge("gateway", "near"),
        edge("gateway", "relay"),
        edge("relay", "far"),
        edge("near", "not_reachable"),
        edge("relay", "gateway"),
    ];
    let distances = hopDistances("gateway", &edges, &BTreeSet::from(["relay".into()]));
    assert_eq!(distances["gateway"], 0);
    assert_eq!(distances["near"], 1);
    assert_eq!(distances["far"], 2);
    assert!(!distances.contains_key("not_reachable"));
    assert_eq!(hopDistances("far", &edges, &BTreeSet::new()).len(), 1);
}

/// Covers every terminal-state/late-response combination rather than one representative cancellation.
#[test]
fn every_terminal_request_resists_every_late_response_status() {
    let all = [
        SpaceJoinStatus::Pending,
        SpaceJoinStatus::Approving,
        SpaceJoinStatus::Approved,
        SpaceJoinStatus::Rejected,
        SpaceJoinStatus::Cancelled,
        SpaceJoinStatus::Expired,
        SpaceJoinStatus::Joined,
    ];
    for terminal in [
        SpaceJoinStatus::Rejected,
        SpaceJoinStatus::Cancelled,
        SpaceJoinStatus::Expired,
        SpaceJoinStatus::Joined,
    ] {
        for response in &all {
            assert!(preserveOutgoingStatus(&terminal, response));
        }
    }
}

/// Verifies the exact expiry boundary and that claimed decisions are never silently expired.
#[test]
fn expiry_boundary_is_inclusive_and_claimed_approval_survives_space_change() {
    let mut pending = record(SpaceJoinStatus::Pending);
    expire(&mut pending, 99, "target");
    assert_eq!(pending.request.status, SpaceJoinStatus::Pending);
    expire(&mut pending, 100, "target");
    assert_eq!(pending.request.status, SpaceJoinStatus::Expired);
    for status in [SpaceJoinStatus::Approving, SpaceJoinStatus::Approved] {
        let mut claimed = record(status.clone());
        expire(&mut claimed, i64::MAX, "another-space");
        assert_eq!(claimed.request.status, status);
    }
}
