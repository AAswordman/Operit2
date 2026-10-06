use super::*;

/// Verifies authenticated peers cannot rewrite request identity, source membership or target Space.
#[tokio::test]
async fn malformed_submission_matrix_preserves_receiver_records_and_files() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("forged-submission");
    let request = pair.request().await;
    let original: serde_json::Value =
        operit_link::fromCoreValue(submissionArgs(&pair.a, &request.requestId)).unwrap();
    let beforeFiles = durableFiles(&pair.b);
    let beforeRecords = protocolRecords(&pair.b, INBOUND_RECORDS);
    for case in 0..6 {
        let mut args = original.clone();
        match case {
            0 => args["requestId"] = serde_json::json!("invalid-uuid"),
            1 => args["profile"]["nodeId"] = serde_json::json!("impostor"),
            2 => args["targetSpaceId"] = serde_json::json!("wrong-space"),
            3 => args["sourceRevision"] = serde_json::json!(0),
            4 => args["source"]["deviceProfiles"] = serde_json::json!([]),
            5 => args["source"]["space"]["members"] = serde_json::json!(["impostor"]),
            _ => unreachable!(),
        }
        let reply = directCommand(
            &pair.a,
            &pair.b.localNodeId(),
            "requestJoin",
            operit_link::toCoreValue(args).unwrap(),
        )
        .await;
        assert!(
            reply.result.is_err(),
            "mutation {case} unexpectedly succeeded"
        );
        assert_eq!(protocolRecords(&pair.b, INBOUND_RECORDS), beforeRecords);
        assert_eq!(durableFiles(&pair.b), beforeFiles);
    }
}

/// Verifies a different directly paired administrator cannot cancel another applicant's request.
#[tokio::test]
async fn paired_third_party_cannot_withdraw_another_devices_request() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("foreign-cancel");
    let request = pair.request().await;
    let (third, _) = approvalService("foreign-cancel-third");
    let thirdPeer = ApprovalMeshPeer::new(third.localNodeId());
    thirdPeer.link(&pair.b);
    pair.bPeer.link(&third);
    third
        .installNodeServices(NodeServices::new(thirdPeer))
        .unwrap();
    let before = protocolRecords(&pair.b, INBOUND_RECORDS);
    let reply = directCommand(
        &third,
        &pair.b.localNodeId(),
        "cancelJoin",
        submissionArgs(&pair.a, &request.requestId),
    )
    .await;
    assert!(reply.result.is_err());
    assert_eq!(protocolRecords(&pair.b, INBOUND_RECORDS), before);
    assertRecordStatus(
        &pair.a,
        OUTBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Pending,
    );
    assert_eq!(
        pair.receiver
            .incomingDeviceSpaceJoins()
            .await
            .unwrap()
            .len(),
        1
    );
}

/// Verifies an independent applicant's administrator identity grants no target-Space approval authority.
#[tokio::test]
async fn source_administrator_cannot_self_approve_target_membership() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("no-self-approval");
    let request = pair.request().await;
    assert!(pair
        .a
        .networkControlStore
        .nodeHasCapability(&pair.a.localNodeId(), "network.approval", None)
        .unwrap());
    let before = durableFiles(&pair.b);
    let forged = pair
        .a
        .callNode(
            pair.b.localNodeId(),
            CoreCallRequest::new(
                "self-approval",
                crate::RuntimeRemoteLinkService::NODE_SPACE_APPROVAL_TARGET,
                "claim",
                operit_link::toCoreValue(serde_json::json!({"requestId": request.requestId,
            "assignmentVersion": request.assignmentVersion, "approve": true}))
                .unwrap(),
            ),
        )
        .await;
    assert!(forged.result.is_err());
    assert!(pair
        .applicant
        .incomingDeviceSpaceJoins()
        .await
        .unwrap()
        .is_empty());
    assert_eq!(durableFiles(&pair.b), before);
    assertRecordStatus(
        &pair.b,
        INBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Pending,
    );
}

/// Verifies expiry becomes durable on both ends without changing Space membership or business files.
#[tokio::test]
async fn expired_pending_request_cannot_be_approved_or_mutate_either_space() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("request-expiry");
    let request = pair.request().await;
    pair.receiver.incomingDeviceSpaceJoins().await.unwrap();
    let beforeA = durableFiles(&pair.a);
    let beforeB = durableFiles(&pair.b);
    let mut record = protocolRecords(&pair.b, INBOUND_RECORDS)
        .remove(&request.requestId)
        .unwrap();
    record["request"]["expiresAt"] = serde_json::json!(1);
    crate::PeerStateStore::PeerStateStore::new(pair.b.localCore.runtimeStorageHost())
        .putRecord(INBOUND_RECORDS, &request.requestId, &record)
        .unwrap();
    assert_eq!(
        pair.applicant
            .refreshDeviceSpaceJoin(request.requestId.clone())
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Expired
    );
    assert!(pair
        .receiver
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .is_err());
    assertRecordStatus(
        &pair.b,
        INBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Expired,
    );
    assert!(pair
        .receiver
        .incomingDeviceSpaceJoins()
        .await
        .unwrap()
        .is_empty());
    assert_eq!(durableFiles(&pair.a), beforeA);
    assert_eq!(durableFiles(&pair.b), beforeB);
}

/// Verifies cancellation remains possible after the target has explicitly changed its Space.
#[tokio::test]
async fn target_space_change_does_not_block_withdrawal_of_an_old_request() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("cancel-old-space");
    let request = pair.request().await;
    let oldSpace = pair.receiver.deviceSpace().unwrap();
    pair.receiver.leaveDeviceSpace().unwrap();
    let newSpace = pair.receiver.deviceSpace().unwrap();
    assert_ne!(newSpace.spaceId, oldSpace.spaceId);
    let before = durableFiles(&pair.b);
    assert_eq!(
        pair.applicant
            .cancelDeviceSpaceJoin(request.requestId.clone())
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Cancelled
    );
    assertRecordStatus(
        &pair.b,
        INBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Cancelled,
    );
    assert_eq!(durableFiles(&pair.b), before);
    assert_eq!(pair.receiver.deviceSpace().unwrap(), newSpace);
}

/// Verifies a duplicate decision cannot change an already claimed approve decision into rejection.
#[tokio::test]
async fn conflicting_reviewer_decisions_cannot_change_a_claimed_outcome() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("decision-collision");
    let request = pair.request().await;
    pair.receiver.incomingDeviceSpaceJoins().await.unwrap();
    let args = serde_json::json!({"requestId": request.requestId, "assignmentVersion": request.assignmentVersion, "approve": true});
    assert!(reviewerCommand(&pair, "claim", args).await.result.is_ok());
    assert!(pair
        .receiver
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, false)
        .await
        .is_err());
    assertRecordStatus(
        &pair.b,
        INBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Approving,
    );
    assert!(protocolRecords(&pair.b, RESULT_RECORDS).is_empty());
    pair.receiver
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .unwrap();
    let operations = pair.b.networkControlStore.currentSpaceOperations().unwrap();
    pair.receiver
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .unwrap();
    assert_eq!(
        pair.b.networkControlStore.currentSpaceOperations().unwrap(),
        operations
    );
    // A local gateway recovers from its stable admission log; it must not
    // duplicate that outcome into a second durable result record.
    assert!(protocolRecords(&pair.b, RESULT_RECORDS).is_empty());
}
