use super::*;
use operit_store::NetworkControlStore::NetworkControlIdentityAssignment;

/// Verifies policy replay is independent of arrival order and duplicate transport delivery.
#[tokio::test]
async fn policy_replay_converges_with_duplicate_reversed_and_rotated_delivery() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (owner, _) = approvalService("policy-owner");
    owner
        .networkControlStore
        .admitMember("policy-member".into())
        .unwrap();
    owner
        .networkControlStore
        .setIdentity(NetworkControlIdentityAssignment {
            nodeId: "policy-member".into(),
            roleId: "admin".into(),
        })
        .unwrap();
    owner
        .networkControlStore
        .clearIdentity("policy-member".into())
        .unwrap();
    owner
        .networkControlStore
        .removeMember("policy-member".into())
        .unwrap();
    let space = owner.spaceStore.space().unwrap();
    let expected = owner.networkControlStore.currentState().unwrap();
    let operations = owner.networkControlStore.currentSpaceOperations().unwrap();
    for reverse in [false, true] {
        for rotation in 0..operations.len() {
            let mut delivered = operations.clone();
            delivered.rotate_left(rotation);
            if reverse {
                delivered.reverse();
            }
            let (replica, _) = approvalService(&format!("policy-replica-{reverse}-{rotation}"));
            for operation in delivered.iter().chain(delivered.iter()) {
                replica
                    .networkControlStore
                    .applySyncedOperation(operation)
                    .unwrap();
            }
            // Replay the received source policy, not the replica's unrelated
            // singleton Space policy (no membership migration occurs here).
            let received = controlOperationsForSpace(&replica, &space.spaceId);
            assert_eq!(
                replica
                    .networkControlStore
                    .validateSpacePolicy(&space.spaceId, &received)
                    .unwrap(),
                expected
            );
            assert_eq!(
                controlOperationsForSpace(&replica, &space.spaceId).len(),
                operations.len()
            );
        }
    }
}

/// Verifies issuer spoofing cannot grant administrator authority even when its log envelope is well formed.
#[tokio::test]
async fn forged_policy_issuer_never_grants_capability() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (owner, _) = approvalService("policy-provenance");
    owner
        .networkControlStore
        .admitMember("ordinary-member".into())
        .unwrap();
    let before = owner.networkControlStore.currentState().unwrap();
    let granted = owner
        .networkControlStore
        .setIdentity(NetworkControlIdentityAssignment {
            nodeId: "ordinary-member".into(),
            roleId: "admin".into(),
        })
        .unwrap();
    let mut operations = owner.networkControlStore.currentSpaceOperations().unwrap();
    let forged = operations
        .iter_mut()
        .find(|operation| operation.opId == granted.opId)
        .unwrap();
    forged.originDeviceId = "ordinary-member".into();
    let state = owner
        .networkControlStore
        .validateSpacePolicy(&before.spaceId, &operations)
        .unwrap();
    assert_eq!(state, before);
}

/// Verifies removing the final administrator fails without changing policy records.
#[tokio::test]
async fn clearing_the_last_administrator_is_rejected_without_a_policy_write() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (owner, _) = approvalService("last-admin");
    let before = owner.networkControlStore.currentSpaceOperations().unwrap();
    assert!(owner
        .networkControlStore
        .clearIdentity(owner.localNodeId())
        .is_err());
    assert_eq!(
        owner.networkControlStore.currentSpaceOperations().unwrap(),
        before
    );
    assert!(owner
        .networkControlStore
        .nodeHasCapability(&owner.localNodeId(), "network.approval", None)
        .unwrap());
}

/// Verifies an administrator who loses approval authority cannot use a previously displayed request.
#[tokio::test]
async fn permission_revocation_invalidates_a_stale_reviewer_action() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let pair = IndependentPair::new("reviewer-revoked");
    let (other, _) = approvalService("reviewer-new-admin");
    mergedChainFixture(&[&pair.b, &other]);
    pair.bPeer.link(&other);
    let otherPeer = ApprovalMeshPeer::new(other.localNodeId());
    otherPeer.link(&pair.b);
    other
        .installNodeServices(NodeServices::new(otherPeer))
        .unwrap();
    pair.b
        .networkControlStore
        .setIdentity(NetworkControlIdentityAssignment {
            nodeId: other.localNodeId(),
            roleId: "admin".into(),
        })
        .unwrap();
    let request = pair.request().await;
    pair.receiver.incomingDeviceSpaceJoins().await.unwrap();
    pair.b
        .networkControlStore
        .clearIdentity(pair.b.localNodeId())
        .unwrap();
    let before = pair.b.networkControlStore.currentSpaceOperations().unwrap();
    assert!(pair
        .receiver
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .is_err());
    assert_eq!(
        pair.b.networkControlStore.currentSpaceOperations().unwrap(),
        before
    );
    assert!(!pair
        .b
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&pair.a.localNodeId()));
    assert_eq!(
        pair.applicant
            .cancelDeviceSpaceJoin(request.requestId)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Cancelled
    );
}
