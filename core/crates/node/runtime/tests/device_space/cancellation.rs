use super::*;

/// Verifies two independent administrators assign the receiver locally without reading unrelated topology.
#[tokio::test]
async fn local_admin_assignment_and_cancellation_do_not_require_topology() {
    use crate::PeerStateStore::PeerStateStore;
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (applicantRouter, applicant) = approvalService("local-review-applicant");
    let (receiverRouter, receiver) = approvalService("local-review-receiver");
    let _link = installTestPeer(
        &applicantRouter,
        receiverRouter.localNodeId(),
        TestCoreNodeRouterEndpoint::new(receiverRouter.clone()),
    )
    .unwrap();
    let request = applicant
        .requestDeviceSpaceJoin(receiverRouter.localNodeId())
        .await
        .unwrap();
    assert_eq!(
        request.reviewerDeviceId.as_deref(),
        Some("local-review-receiver")
    );
    let store = PeerStateStore::new(receiverRouter.localCore.runtimeStorageHost());
    let path = "runtime/link_access/space_merge_inbound.preferences.json";
    let mut saved = store
        .records::<serde_json::Value>(path)
        .unwrap()
        .remove(&request.requestId)
        .unwrap();
    saved["request"]["reviewerDeviceId"] = serde_json::Value::Null;
    saved["request"]["reviewerName"] = serde_json::Value::Null;
    saved["request"]["reviewerHops"] = serde_json::Value::Null;
    store.putRecord(path, &request.requestId, &saved).unwrap();
    receiverRouter
        .localCore
        .runtimeStorageHost()
        .writeBytes(
            "runtime/space/topology/unrelated.preferences.json",
            b"not-json",
        )
        .unwrap();
    let refreshed = applicant
        .refreshDeviceSpaceJoin(request.requestId.clone())
        .await
        .unwrap();
    assert_eq!(
        refreshed.reviewerDeviceId.as_deref(),
        Some("local-review-receiver")
    );
    assert_eq!(refreshed.reviewerHops, Some(1));
    // Force an unassigned pending record again; cancel must not execute discovery.
    store.putRecord(path, &request.requestId, &saved).unwrap();
    let cancelled = applicant
        .cancelDeviceSpaceJoin(request.requestId.clone())
        .await
        .unwrap();
    assert_eq!(cancelled.status, SpaceJoinStatus::Cancelled);
    assert!(receiver
        .incomingDeviceSpaceJoins()
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        applicant
            .refreshDeviceSpaceJoin(request.requestId)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Cancelled
    );
}

/// Verifies cancellation can precede receipt of a submission and prevents delayed submission resurrection.
#[tokio::test]
async fn cancellation_records_a_tombstone_before_delayed_submission_arrives() {
    use crate::PeerStateStore::PeerStateStore;
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (applicantRouter, applicant) = approvalService("cancel-first-applicant");
    let (receiverRouter, receiver) = approvalService("cancel-first-receiver");
    let _link = installTestPeer(
        &applicantRouter,
        receiverRouter.localNodeId(),
        TestCoreNodeRouterEndpoint::new(receiverRouter.clone()),
    )
    .unwrap();
    let request = applicant
        .requestDeviceSpaceJoin(receiverRouter.localNodeId())
        .await
        .unwrap();
    let inbound = PeerStateStore::new(receiverRouter.localCore.runtimeStorageHost());
    inbound
        .deleteRecord(
            "runtime/link_access/space_merge_inbound.preferences.json",
            &request.requestId,
        )
        .unwrap();
    let outbound = PeerStateStore::new(applicantRouter.localCore.runtimeStorageHost());
    let saved = outbound
        .records::<serde_json::Value>("runtime/link_access/space_merge_outbound.preferences.json")
        .unwrap()
        .remove(&request.requestId)
        .unwrap();
    let cancelled = applicant
        .cancelDeviceSpaceJoin(request.requestId.clone())
        .await
        .unwrap();
    assert_eq!(cancelled.status, SpaceJoinStatus::Cancelled);
    let delayed = serde_json::json!({
        "requestId": request.requestId,
        "sourceSpaceId": saved["sourceSpaceId"], "sourceRevision": saved["sourceRevision"],
        "targetSpaceId": saved["targetSpaceId"], "profile": saved["profile"], "source": saved["source"],
    });
    let response = receiver
        .acceptPeerSpaceCall(
            &applicantRouter.localNodeId(),
            CoreCallRequest::new(
                "delayed-submission",
                crate::RuntimeRemoteLinkService::NODE_SPACE_TARGET,
                "requestJoin",
                operit_link::toCoreValue(delayed).unwrap(),
            ),
        )
        .unwrap();
    let record: serde_json::Value = operit_link::fromCoreValue(response).unwrap();
    assert_eq!(record["request"]["status"], "cancelled");
    assert!(receiver
        .incomingDeviceSpaceJoins()
        .await
        .unwrap()
        .is_empty());
}

/// Proves withdrawal changes request bookkeeping only, preserves files and never grants membership.
#[tokio::test]
async fn withdrawal_preserves_both_spaces_files_permissions_and_identity_after_restart() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("withdraw-contract");
    pair.a
        .localCore
        .runtimeStorageHost()
        .writeBytes("runtime/data/user_assets/a.bin", b"private-a")
        .unwrap();
    pair.b
        .localCore
        .runtimeStorageHost()
        .writeBytes("runtime/data/user_assets/b.bin", b"private-b")
        .unwrap();
    let beforeA = durableFiles(&pair.a);
    let beforeB = durableFiles(&pair.b);
    let policyA = pair.a.networkControlStore.currentSpaceOperations().unwrap();
    let policyB = pair.b.networkControlStore.currentSpaceOperations().unwrap();
    let request = pair.request().await;
    assert_eq!(
        pair.receiver
            .incomingDeviceSpaceJoins()
            .await
            .unwrap()
            .len(),
        1
    );
    let cancelled = pair
        .applicant
        .cancelDeviceSpaceJoin(request.requestId.clone())
        .await
        .unwrap();
    assert_eq!(cancelled.status, SpaceJoinStatus::Cancelled);
    assertRecordStatus(
        &pair.a,
        OUTBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Cancelled,
    );
    assertRecordStatus(
        &pair.b,
        INBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Cancelled,
    );
    assert!(pair
        .receiver
        .incomingDeviceSpaceJoins()
        .await
        .unwrap()
        .is_empty());
    assert!(pair
        .receiver
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .is_err());
    assert!(protocolRecords(&pair.b, RESULT_RECORDS).is_empty());
    assert_eq!(durableFiles(&pair.a), beforeA);
    assert_eq!(durableFiles(&pair.b), beforeB);
    assert_eq!(
        pair.a.networkControlStore.currentSpaceOperations().unwrap(),
        policyA
    );
    assert_eq!(
        pair.b.networkControlStore.currentSpaceOperations().unwrap(),
        policyB
    );
    assert!(!pair
        .b
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&pair.a.localNodeId()));
    let restarted = pair.restartApplicant();
    assert_eq!(
        restarted
            .refreshDeviceSpaceJoin(request.requestId.clone())
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Cancelled
    );
    assert_eq!(
        restarted
            .cancelDeviceSpaceJoin(request.requestId)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Cancelled
    );
    assert_eq!(durableFiles(&pair.a), beforeA);
    assert_eq!(durableFiles(&pair.b), beforeB);
}

/// Verifies a receiver-committed cancellation survives a lost acknowledgment and applicant restart.
#[tokio::test]
async fn lost_cancellation_ack_is_reconciled_without_resubmitting_a_new_request() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("cancel-ack");
    let request = pair.request().await;
    let beforeA = durableFiles(&pair.a);
    let beforeB = durableFiles(&pair.b);
    pair.aPeer.failNext("cancelJoin", true);
    assert!(pair
        .applicant
        .cancelDeviceSpaceJoin(request.requestId.clone())
        .await
        .is_err());
    assertRecordStatus(
        &pair.a,
        OUTBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Pending,
    );
    assertRecordStatus(
        &pair.b,
        INBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Cancelled,
    );
    assert!(pair
        .receiver
        .incomingDeviceSpaceJoins()
        .await
        .unwrap()
        .is_empty());
    let refreshed = pair
        .restartApplicant()
        .refreshDeviceSpaceJoin(request.requestId.clone())
        .await
        .unwrap();
    assert_eq!(refreshed.status, SpaceJoinStatus::Cancelled);
    assertRecordStatus(
        &pair.a,
        OUTBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Cancelled,
    );
    assert_eq!(protocolRecords(&pair.b, INBOUND_RECORDS).len(), 1);
    assert_eq!(durableFiles(&pair.a), beforeA);
    assert_eq!(durableFiles(&pair.b), beforeB);
}

/// Checks both possible submission-delivery failures remain cancellable with one durable request id.
#[tokio::test]
async fn cancellation_handles_submission_failure_before_and_after_receiver_commit() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    for after in [false, true] {
        let pair = IndependentPair::new(&format!("submit-fault-{after}"));
        let beforeA = durableFiles(&pair.a);
        let beforeB = durableFiles(&pair.b);
        pair.aPeer.failNext("requestJoin", after);
        assert!(pair
            .applicant
            .requestDeviceSpaceJoin(pair.b.localNodeId())
            .await
            .is_err());
        let outgoing = pair.applicant.outgoingDeviceSpaceJoins().unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(
            protocolRecords(&pair.b, INBOUND_RECORDS).len(),
            usize::from(after)
        );
        let id = outgoing[0].requestId.clone();
        // Replay the original submission, not the compact terminal receipt.
        let originalSubmission = submissionArgs(&pair.a, &id);
        assert_eq!(
            pair.applicant
                .cancelDeviceSpaceJoin(id.clone())
                .await
                .unwrap()
                .status,
            SpaceJoinStatus::Cancelled
        );
        assertRecordStatus(&pair.a, OUTBOUND_RECORDS, &id, SpaceJoinStatus::Cancelled);
        assertRecordStatus(&pair.b, INBOUND_RECORDS, &id, SpaceJoinStatus::Cancelled);
        let delayed = directCommand(
            &pair.a,
            &pair.b.localNodeId(),
            "requestJoin",
            originalSubmission,
        )
        .await;
        assert!(delayed.result.is_ok());
        assertRecordStatus(&pair.b, INBOUND_RECORDS, &id, SpaceJoinStatus::Cancelled);
        assert_eq!(durableFiles(&pair.a), beforeA);
        assert_eq!(durableFiles(&pair.b), beforeB);
    }
}

/// Uses an explicit response gate to reproduce a late poll overwriting an acknowledged withdrawal.
#[tokio::test]
async fn cancelled_request_cannot_be_resurrected_by_an_in_flight_poll() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("cancel-race");
    let request = pair.request().await;
    let (arrived, release) = pair.aPeer.pauseResponse("requestJoin");
    let id = request.requestId;
    let raced = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(pair.applicant.refreshDeviceSpaceJoin(id.clone()), async {
            arrived.await.unwrap();
            let cancelled = pair
                .applicant
                .cancelDeviceSpaceJoin(id.clone())
                .await
                .unwrap();
            assert_eq!(cancelled.status, SpaceJoinStatus::Cancelled);
            release.send(()).unwrap();
        })
    })
    .await
    .expect("controlled response race must terminate");
    assert_eq!(raced.0.unwrap().status, SpaceJoinStatus::Cancelled);
    assertRecordStatus(&pair.a, OUTBOUND_RECORDS, &id, SpaceJoinStatus::Cancelled);
    assertRecordStatus(&pair.b, INBOUND_RECORDS, &id, SpaceJoinStatus::Cancelled);
    assert!(pair
        .receiver
        .incomingDeviceSpaceJoins()
        .await
        .unwrap()
        .is_empty());
}

/// Verifies offline cancellation reports failure without silently admitting or changing either Space.
#[tokio::test]
async fn offline_cancellation_can_be_retried_after_reconnection() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("cancel-offline");
    let request = pair.request().await;
    let beforeA = durableFiles(&pair.a);
    let beforeB = durableFiles(&pair.b);
    pair.aPeer
        .disconnectPeer(&pair.b.localNodeId())
        .await
        .unwrap();
    assert!(pair
        .applicant
        .cancelDeviceSpaceJoin(request.requestId.clone())
        .await
        .is_err());
    assertRecordStatus(
        &pair.a,
        OUTBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Pending,
    );
    assertRecordStatus(
        &pair.b,
        INBOUND_RECORDS,
        &request.requestId,
        SpaceJoinStatus::Pending,
    );
    pair.aPeer.link(&pair.b);
    assert_eq!(
        pair.restartApplicant()
            .cancelDeviceSpaceJoin(request.requestId.clone())
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Cancelled
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

/// Verifies the first serialized decision is authoritative and neither side pretends approval was cancelled.
#[tokio::test]
async fn cancellation_and_reviewer_claim_have_explicit_ordered_outcomes() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    for cancelFirst in [true, false] {
        let pair = IndependentPair::new(&format!("claim-order-{cancelFirst}"));
        let request = pair.request().await;
        pair.receiver.incomingDeviceSpaceJoins().await.unwrap();
        let args = serde_json::json!({"requestId": request.requestId, "assignmentVersion": request.assignmentVersion, "approve": true});
        if cancelFirst {
            assert_eq!(
                pair.applicant
                    .cancelDeviceSpaceJoin(request.requestId.clone())
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Cancelled
            );
            assert!(reviewerCommand(&pair, "claim", args).await.result.is_err());
            assert!(!pair.b.spaceStore.contains(pair.a.localNodeId()).unwrap());
        } else {
            assert!(reviewerCommand(&pair, "claim", args).await.result.is_ok());
            assert_eq!(
                pair.applicant
                    .cancelDeviceSpaceJoin(request.requestId.clone())
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Approving
            );
            assertRecordStatus(
                &pair.b,
                INBOUND_RECORDS,
                &request.requestId,
                SpaceJoinStatus::Approving,
            );
            pair.receiver
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await
                .unwrap();
            assert_eq!(
                pair.applicant
                    .refreshDeviceSpaceJoin(request.requestId.clone())
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Joined
            );
            assert_eq!(
                pair.applicant.deviceSpace().unwrap(),
                pair.receiver.deviceSpace().unwrap()
            );
        }
    }
}

/// Repeatedly interleaves duplicate submissions, polling and withdrawal without accumulating live requests.
#[tokio::test]
async fn repeated_cancel_reapply_cycles_preserve_files_and_keep_only_one_active_request() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("cancel-cycles");
    let beforeA = durableFiles(&pair.a);
    let beforeB = durableFiles(&pair.b);
    let mut ids = BTreeSet::new();
    for _ in 0..12 {
        let request = pair.request().await;
        assert!(ids.insert(request.requestId.clone()));
        assert_eq!(pair.request().await.requestId, request.requestId);
        assert_eq!(
            pair.receiver
                .incomingDeviceSpaceJoins()
                .await
                .unwrap()
                .len(),
            1
        );
        pair.applicant
            .refreshDeviceSpaceJoin(request.requestId.clone())
            .await
            .unwrap();
        pair.applicant
            .cancelDeviceSpaceJoin(request.requestId.clone())
            .await
            .unwrap();
        pair.applicant
            .cancelDeviceSpaceJoin(request.requestId.clone())
            .await
            .unwrap();
        assert!(pair
            .receiver
            .incomingDeviceSpaceJoins()
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            pair.applicant
                .outgoingDeviceSpaceJoins()
                .unwrap()
                .iter()
                .filter(|r| r.status == SpaceJoinStatus::Pending)
                .count(),
            0
        );
        assert_eq!(durableFiles(&pair.a), beforeA);
        assert_eq!(durableFiles(&pair.b), beforeB);
    }
    assert_eq!(protocolRecords(&pair.a, OUTBOUND_RECORDS).len(), ids.len());
    assert_eq!(protocolRecords(&pair.b, INBOUND_RECORDS).len(), ids.len());
}

#[tokio::test]
async fn offline_cancellation_intent_is_durable_and_stops_submission_retries_until_ack() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("cancel-offline-intent");
    let before = pair.applicant.deviceSpace().unwrap();
    let request = pair.request().await;
    let submissions = pair.aPeer.calls.lock().unwrap().iter().filter(|(_, method)| method == "requestJoin").count();
    pair.aPeer.disconnectPeer(&pair.b.localNodeId()).await.unwrap();
    assert!(pair.applicant.cancelDeviceSpaceJoin(request.requestId.clone()).await.is_err());
    let restored = pair.restartApplicant();
    assert_eq!(restored.outgoingDeviceSpaceJoins().unwrap()[0].status, SpaceJoinStatus::Pending);
    assert!(restored.refreshDeviceSpaceJoin(request.requestId.clone()).await.is_err());
    assert!(restored.requestDeviceSpaceJoin(pair.b.localNodeId()).await.is_err());
    assert_eq!(pair.aPeer.calls.lock().unwrap().iter().filter(|(_, method)| method == "requestJoin").count(), submissions);
    let store = crate::PeerStateStore::PeerStateStore::new(pair.a.localCore.runtimeStorageHost());
    let saved = store.records::<serde_json::Value>(OUTBOUND_RECORDS).unwrap();
    assert_eq!(saved[&request.requestId]["cancelRequested"], true);
    assert_eq!(restored.deviceSpace().unwrap(), before);
    assert!(!pair.receiver.deviceSpace().unwrap().members.contains(&pair.a.localNodeId()));
    pair.aPeer.link(&pair.b);
    assert_eq!(restored.refreshDeviceSpaceJoin(request.requestId.clone()).await.unwrap().status, SpaceJoinStatus::Cancelled);
    assert_eq!(store.records::<serde_json::Value>(OUTBOUND_RECORDS).unwrap()[&request.requestId]["cancelRequested"], false);
}

#[tokio::test]
async fn committed_approval_wins_cancellation_and_returns_actual_joined_state() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (applicantRouter, applicant) = approvalService("cancel-approved-applicant");
    let (gatewayRouter, gateway) = approvalService("cancel-approved-gateway");
    let _link = installTestPeer(&applicantRouter, gatewayRouter.localNodeId(),
        TestCoreNodeRouterEndpoint::new(gatewayRouter.clone())).unwrap();
    let request = applicant.requestDeviceSpaceJoin(gatewayRouter.localNodeId()).await.unwrap();
    gateway.incomingDeviceSpaceJoins().await.unwrap();
    gateway.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await.unwrap();

    let result = applicant.cancelDeviceSpaceJoin(request.requestId).await.unwrap();
    assert_eq!(result.status, SpaceJoinStatus::Joined);
    assert_eq!(applicant.outgoingDeviceSpaceJoins().unwrap()[0].status, SpaceJoinStatus::Joined);
    assert_eq!(applicant.deviceSpace().unwrap(), gateway.deviceSpace().unwrap());
}

/// A delayed cancellation receipt must not roll Joined back or start another submission.
#[tokio::test]
async fn delayed_cancellation_response_preserves_joined_state_without_resubmission() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("cancel-late-approved-receipt");
    let request = pair.request().await;
    pair.receiver.incomingDeviceSpaceJoins().await.unwrap();
    pair.receiver.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await.unwrap();
    let id = request.requestId;
    let (arrived, release) = pair.aPeer.pauseResponse("cancelJoin");
    let (cancelled, ()) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(pair.applicant.cancelDeviceSpaceJoin(id.clone()), async {
            arrived.await.unwrap();
            // Another caller acknowledges the decision and completes the same join.
            for _ in 0..2 {
                if pair.applicant.refreshDeviceSpaceJoin(id.clone()).await.unwrap().status == SpaceJoinStatus::Joined {
                    break;
                }
            }
            assertRecordStatus(&pair.a, OUTBOUND_RECORDS, &id, SpaceJoinStatus::Joined);
            // Joined is authoritative even if a subsequent submission would fail.
            pair.aPeer.failNext("requestJoin", false);
            release.send(()).unwrap();
        })
    }).await.expect("controlled cancellation response race must terminate");
    assert_eq!(cancelled.unwrap().status, SpaceJoinStatus::Joined);
    assertRecordStatus(&pair.a, OUTBOUND_RECORDS, &id, SpaceJoinStatus::Joined);
    assert_eq!(pair.applicant.deviceSpace().unwrap(), pair.receiver.deviceSpace().unwrap());
    assert_eq!(protocolRecords(&pair.a, OUTBOUND_RECORDS)[&id]["cancelRequested"], false);
}
