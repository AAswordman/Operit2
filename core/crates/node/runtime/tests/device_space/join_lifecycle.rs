use super::*;

/// Exercises independent space join is pending then only explicit approval admits through isolated runtime stores.
#[tokio::test]
async fn independent_space_join_is_pending_then_only_explicit_approval_admits() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (macRouter, mac) = approvalService("approval-mac");
    let (iosRouter, ios) = approvalService("approval-ios");
    let _link = installTestPeer(
        &macRouter,
        iosRouter.localNodeId(),
        TestCoreNodeRouterEndpoint::new(iosRouter.clone()),
    )
    .unwrap();
    let beforeMac = mac.deviceSpace().unwrap();
    let beforeIos = ios.deviceSpace().unwrap();
    let request = mac
        .requestDeviceSpaceJoin(iosRouter.localNodeId())
        .await
        .unwrap();
    assert_eq!(request.status, SpaceJoinStatus::Pending);
    assert_eq!(request.reviewerDeviceId.as_deref(), Some("approval-ios"));
    assert_eq!(request.reviewerHops, Some(1));
    assert_eq!(mac.deviceSpace().unwrap(), beforeMac);
    assert_eq!(ios.deviceSpace().unwrap(), beforeIos);
    assert!(!iosRouter
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains("approval-mac"));
    assert!(mac.incomingDeviceSpaceJoins().await.unwrap().is_empty());
    let incoming = ios.incomingDeviceSpaceJoins().await.unwrap();
    assert_eq!(incoming.len(), 1);
    assert!(incoming[0].canApprove);
    // Restarting the facade neither loses the request nor generates a new one.
    let restoredMac =
        RuntimeRemoteLinkService::newWithRouter((*macRouter.localCore).clone(), macRouter.clone());
    assert_eq!(
        restoredMac
            .requestDeviceSpaceJoin("approval-ios".into())
            .await
            .unwrap()
            .requestId,
        request.requestId
    );
    let approved = ios
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .unwrap();
    assert_eq!(approved.status, SpaceJoinStatus::Approved);
    // A duplicate decision returns the original outcome, without a second admission.
    let revision = ios.deviceSpace().unwrap().spaceRevision;
    ios.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .unwrap();
    assert_eq!(ios.deviceSpace().unwrap().spaceRevision, revision);
    let joined = restoredMac
        .refreshDeviceSpaceJoin(request.requestId)
        .await
        .unwrap();
    assert_eq!(joined.status, SpaceJoinStatus::Joined);
    assert_eq!(mac.deviceSpace().unwrap(), ios.deviceSpace().unwrap());
    assert!(!macRouter
        .networkControlStore
        .nodeHasCapability("approval-mac", "network.members.join", None)
        .unwrap());
    assert!(ios.incomingDeviceSpaceJoins().await.unwrap().is_empty());
}

/// Exercises rejection cancel and stale assignment do not admit or change applicant space through isolated runtime stores.
#[tokio::test]
async fn rejection_cancel_and_stale_assignment_do_not_admit_or_change_applicant_space() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (macRouter, mac) = approvalService("reject-mac");
    let (iosRouter, ios) = approvalService("reject-ios");
    let _link = installTestPeer(
        &macRouter,
        iosRouter.localNodeId(),
        TestCoreNodeRouterEndpoint::new(iosRouter.clone()),
    )
    .unwrap();
    let before = mac.deviceSpace().unwrap();
    let request = mac
        .requestDeviceSpaceJoin("reject-ios".into())
        .await
        .unwrap();
    ios.incomingDeviceSpaceJoins().await.unwrap();
    assert!(ios
        .decideDeviceSpaceJoin(
            request.requestId.clone(),
            request.assignmentVersion + 1,
            true
        )
        .await
        .is_err());
    assert_eq!(
        ios.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, false)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Rejected
    );
    assert_eq!(
        mac.refreshDeviceSpaceJoin(request.requestId)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Rejected
    );
    assert_eq!(mac.deviceSpace().unwrap(), before);
    assert!(!iosRouter
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains("reject-mac"));
    let retry = mac
        .requestDeviceSpaceJoin("reject-ios".into())
        .await
        .unwrap();
    assert_eq!(
        mac.cancelDeviceSpaceJoin(retry.requestId.clone())
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Cancelled
    );
    assert!(ios.incomingDeviceSpaceJoins().await.unwrap().is_empty());
    assert_eq!(mac.deviceSpace().unwrap(), before);
    assert!(!iosRouter.spaceStore.contains("reject-mac".into()).unwrap());
}

/// Exercises leaving joined space initializes admin and rejoining requires approval through isolated runtime stores.
#[tokio::test]
async fn leaving_joined_space_initializes_admin_and_rejoining_requires_approval() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (macRouter, mac) = approvalService("leave-mac");
    let (iosRouter, ios) = approvalService("leave-ios");
    let _link = installTestPeer(
        &macRouter,
        iosRouter.localNodeId(),
        TestCoreNodeRouterEndpoint::new(iosRouter.clone()),
    )
    .unwrap();
    let first = mac
        .requestDeviceSpaceJoin("leave-ios".into())
        .await
        .unwrap();
    ios.incomingDeviceSpaceJoins().await.unwrap();
    ios.decideDeviceSpaceJoin(first.requestId.clone(), first.assignmentVersion, true)
        .await
        .unwrap();
    mac.refreshDeviceSpaceJoin(first.requestId).await.unwrap();
    let joined = mac.deviceSpace().unwrap();
    // A normal admitted member remains unable to approve in the joined Space.
    assert!(mac.incomingDeviceSpaceJoins().await.unwrap().is_empty());
    assert!(!macRouter
        .networkControlStore
        .nodeHasCapability("leave-mac", "network.approval", None)
        .unwrap());
    let macSpace = mac.leaveDeviceSpace().unwrap();
    let iosSpace = ios.leaveDeviceSpace().unwrap();
    assert_ne!(macSpace.spaceId, joined.spaceId);
    assert_ne!(iosSpace.spaceId, joined.spaceId);
    assert_ne!(macSpace.spaceId, iosSpace.spaceId);
    for (router, space) in [(&macRouter, &macSpace), (&iosRouter, &iosSpace)] {
        assert_eq!(space.members, vec![router.localNodeId()]);
        let state = router.networkControlStore.currentState().unwrap();
        assert!(state.initialized);
        assert_eq!(
            state
                .deviceIdentityIds
                .get(&router.localNodeId())
                .map(String::as_str),
            Some("admin")
        );
        assert!(router
            .networkControlStore
            .nodeHasCapability(&router.localNodeId(), "network.members.join", None)
            .unwrap());
        assert!(router
            .networkControlStore
            .nodeHasCapability(&router.localNodeId(), "network.approval", None)
            .unwrap());
        assert_eq!(controlOperationsForSpace(router, &space.spaceId).len(), 1);
    }
    // Existing pairing survives leaving, but membership still needs approval.
    let second = mac
        .requestDeviceSpaceJoin("leave-ios".into())
        .await
        .unwrap();
    assert_eq!(second.status, SpaceJoinStatus::Pending);
    assert_eq!(second.reviewerDeviceId.as_deref(), Some("leave-ios"));
    let incoming = ios.incomingDeviceSpaceJoins().await.unwrap();
    assert_eq!(incoming.len(), 1);
    assert!(incoming[0].canApprove);
    assert_eq!(ios.incomingDeviceSpaceJoins().await.unwrap(), incoming);
    assert_eq!(
        controlOperationsForSpace(&iosRouter, &iosSpace.spaceId).len(),
        1
    );
    assert_eq!(mac.deviceSpace().unwrap(), macSpace);
    assert!(!iosRouter
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains("leave-mac"));
    ios.decideDeviceSpaceJoin(second.requestId.clone(), second.assignmentVersion, true)
        .await
        .unwrap();
    assert_eq!(
        mac.refreshDeviceSpaceJoin(second.requestId)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Joined
    );
    assert_eq!(mac.deviceSpace().unwrap(), ios.deviceSpace().unwrap());
    assert!(!macRouter
        .networkControlStore
        .nodeHasCapability("leave-mac", "network.approval", None)
        .unwrap());
}

/// Exercises repeated leave initializes each new space once without changing old policy through isolated runtime stores.
#[tokio::test]
async fn repeated_leave_initializes_each_new_space_once_without_changing_old_policy() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, service) = approvalService("repeat-leave");
    let old = service.deviceSpace().unwrap();
    let oldOperations = controlOperationsForSpace(&router, &old.spaceId);
    let first = service.leaveDeviceSpace().unwrap();
    assert_ne!(first.spaceId, old.spaceId);
    let firstOperations = controlOperationsForSpace(&router, &first.spaceId);
    assert_eq!(firstOperations.len(), 1);
    let second = service.leaveDeviceSpace().unwrap();
    assert_ne!(second.spaceId, first.spaceId);
    assert_eq!(controlOperationsForSpace(&router, &second.spaceId).len(), 1);
    assert!(router
        .networkControlStore
        .nodeHasCapability("repeat-leave", "network.approval", None)
        .unwrap());
    // Inspect only the isolated test store: all older Space policy operations survive.
    assert_eq!(
        controlOperationsForSpace(&router, &first.spaceId),
        firstOperations
    );
    assert_eq!(
        controlOperationsForSpace(&router, &old.spaceId),
        oldOperations
    );
    assert_eq!(service.deviceSpace().unwrap(), second);
}

/// Verifies two applicants coexist and withdrawing one leaves the other independently approvable.
#[tokio::test]
async fn two_applicants_withdraw_one_approve_other_without_cross_request_mutation() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("two-applicants");
    let (third, thirdService) = approvalService("two-applicants-c");
    let thirdPeer = ApprovalMeshPeer::new(third.localNodeId());
    thirdPeer.link(&pair.b);
    pair.bPeer.link(&third);
    third
        .installNodeServices(NodeServices::new(thirdPeer))
        .unwrap();
    let beforeA = durableFiles(&pair.a);
    let first = pair.request().await;
    let second = thirdService
        .requestDeviceSpaceJoin(pair.b.localNodeId())
        .await
        .unwrap();
    assert_ne!(first.requestId, second.requestId);
    assert_eq!(
        pair.receiver
            .incomingDeviceSpaceJoins()
            .await
            .unwrap()
            .len(),
        2
    );
    pair.applicant
        .cancelDeviceSpaceJoin(first.requestId.clone())
        .await
        .unwrap();
    let incoming = pair.receiver.incomingDeviceSpaceJoins().await.unwrap();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].requestId, second.requestId);
    pair.receiver
        .decideDeviceSpaceJoin(second.requestId.clone(), second.assignmentVersion, true)
        .await
        .unwrap();
    assert_eq!(
        thirdService
            .refreshDeviceSpaceJoin(second.requestId.clone())
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Joined
    );
    assertRecordStatus(
        &pair.b,
        INBOUND_RECORDS,
        &first.requestId,
        SpaceJoinStatus::Cancelled,
    );
    assertRecordStatus(
        &pair.b,
        INBOUND_RECORDS,
        &second.requestId,
        SpaceJoinStatus::Approved,
    );
    assert!(!pair.b.spaceStore.contains(pair.a.localNodeId()).unwrap());
    assert!(pair.b.spaceStore.contains(third.localNodeId()).unwrap());
    assert_eq!(durableFiles(&pair.a), beforeA);
}

/// Verifies a lost initial response reuses the exact recorded request after a facade restart.
#[tokio::test]
async fn lost_submission_reply_reuses_request_and_does_not_duplicate_receiver_inbox() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("lost-initial-response");
    pair.aPeer.failNext("requestJoin", true);
    assert!(pair
        .applicant
        .requestDeviceSpaceJoin(pair.b.localNodeId())
        .await
        .is_err());
    let outgoing = pair.applicant.outgoingDeviceSpaceJoins().unwrap();
    assert_eq!(outgoing.len(), 1);
    let recovered = pair
        .restartApplicant()
        .requestDeviceSpaceJoin(pair.b.localNodeId())
        .await
        .unwrap();
    assert_eq!(recovered.requestId, outgoing[0].requestId);
    assert_eq!(recovered.reviewerDeviceId, Some(pair.b.localNodeId()));
    assert_eq!(protocolRecords(&pair.b, INBOUND_RECORDS).len(), 1);
    assert_eq!(
        pair.receiver
            .incomingDeviceSpaceJoins()
            .await
            .unwrap()
            .len(),
        1
    );
}

/// Exercises a target advertising the applicant's own space identity without the
/// applicant as a member is a membership split, never a joinable target through
/// isolated runtime stores.
#[tokio::test]
async fn join_request_rejects_target_advertising_the_local_space_without_membership() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("split-join");
    // Diverge the member tables on one shared space identity: the applicant's
    // projection carries the receiver's space identity while the receiver's own
    // snapshot still lists only itself, so neither side admits the applicant.
    let receiverSpace = pair.b.spaceStore.space().unwrap();
    pair.a
        .spaceStore
        .adopt(operit_store::CoreSpaceStore::CoreSpace {
            spaceId: receiverSpace.spaceId.clone(),
            spaceName: receiverSpace.spaceName.clone(),
            spaceRevision: receiverSpace.spaceRevision + 1,
            members: vec![pair.a.localNodeId()],
        })
        .unwrap();
    let error = pair
        .applicant
        .requestDeviceSpaceJoin(pair.b.localNodeId())
        .await
        .unwrap_err();
    assert!(
        error.contains("reconcile the split membership"),
        "unexpected error: {error}"
    );
    // A same-identity target must not leave behind a durable join request.
    assert!(pair
        .applicant
        .outgoingDeviceSpaceJoins()
        .unwrap()
        .is_empty());
    assert!(protocolRecords(&pair.b, INBOUND_RECORDS).is_empty());
}

/// Exercises the gateway refuses a submission whose source Space equals its own
/// target Space instead of admitting an unreviewable request through isolated
/// runtime stores.
#[tokio::test]
async fn gateway_rejects_a_submission_whose_source_equals_the_target_space() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("self-merge");
    let targetSpaceId = pair.b.spaceStore.space().unwrap().spaceId;
    let applicant = pair.a.localNodeId();
    let profile = serde_json::json!({
        "nodeId": applicant, "displayName": "Applicant", "userName": "",
        "platform": "test", "model": "test", "coreVersion": null, "updatedAt": 1,
    });
    let args = operit_link::toCoreValue(serde_json::json!({
        "requestId": uuid::Uuid::new_v4().to_string(),
        "sourceSpaceId": targetSpaceId,
        "sourceRevision": 3,
        "targetSpaceId": targetSpaceId,
        "profile": profile,
        "source": {
            "space": {"spaceId": targetSpaceId, "spaceName": "Shared", "spaceRevision": 3,
                      "members": [applicant]},
            "deviceProfiles": [profile],
            "controlOperations": [],
            "topology": [],
        },
    }))
    .unwrap();
    let response = directCommand(&pair.a, &pair.b.localNodeId(), "requestJoin", args).await;
    let error = response.result.unwrap_err();
    assert!(
        error.message.contains("distinct source Space"),
        "unexpected error: {}",
        error.message
    );
    // The rejected submission leaves no durable inbound record behind.
    assert!(protocolRecords(&pair.b, INBOUND_RECORDS).is_empty());
}

/// Leaving is local, so a target that keeps hosting its Space still holds the
/// earlier admission. A re-application from the forked source Space must
/// retire that stale record and go through a fresh review instead of being
/// blocked forever.
#[tokio::test]
async fn rejoin_after_only_the_applicant_leaves_starts_a_fresh_review() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (macRouter, mac) = approvalService("rejoin-mac");
    let (iosRouter, ios) = approvalService("rejoin-ios");
    let _link = installTestPeer(
        &macRouter,
        iosRouter.localNodeId(),
        TestCoreNodeRouterEndpoint::new(iosRouter.clone()),
    )
    .unwrap();
    let first = mac
        .requestDeviceSpaceJoin("rejoin-ios".into())
        .await
        .unwrap();
    ios.incomingDeviceSpaceJoins().await.unwrap();
    ios.decideDeviceSpaceJoin(first.requestId.clone(), first.assignmentVersion, true)
        .await
        .unwrap();
    assert_eq!(
        mac.refreshDeviceSpaceJoin(first.requestId.clone())
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Joined
    );
    // Only the applicant leaves; the target keeps hosting the same Space.
    let targetSpace = ios.deviceSpace().unwrap();
    mac.leaveDeviceSpace().unwrap();
    assert_eq!(ios.deviceSpace().unwrap(), targetSpace);
    // The stale admission is retired, so the new application is accepted...
    let second = mac
        .requestDeviceSpaceJoin("rejoin-ios".into())
        .await
        .unwrap();
    assert_eq!(second.status, SpaceJoinStatus::Pending);
    assert_ne!(second.requestId, first.requestId);
    assert_eq!(second.reviewerDeviceId.as_deref(), Some("rejoin-ios"));
    // ...but the retired admission can no longer be decided.
    assert!(ios
        .decideDeviceSpaceJoin(
            first.requestId.clone(),
            first.assignmentVersion,
            true
        )
        .await
        .is_err());
    // Membership still needs an explicit approval of the fresh application.
    let incoming = ios.incomingDeviceSpaceJoins().await.unwrap();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].requestId, second.requestId);
    assert!(incoming[0].canApprove);
    ios.decideDeviceSpaceJoin(second.requestId.clone(), second.assignmentVersion, true)
        .await
        .unwrap();
    assert_eq!(
        mac.refreshDeviceSpaceJoin(second.requestId)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Joined
    );
    assert_eq!(mac.deviceSpace().unwrap().spaceId, targetSpace.spaceId);
}
