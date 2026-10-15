use super::*;

/// Verifies a member forgets a removed device at the transition it observes,
/// and keeps pairing with devices that were never removed.
#[tokio::test]
async fn members_forget_a_removed_device_after_applying_the_removal() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) =
        revocationMesh("revoke-member", 4, &[(0, 1), (0, 2), (0, 3), (1, 2), (2, 3)]);
    let removed = routers[3].localNodeId();

    access[0]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();

    // The observer has not synchronized yet: it still admits the removed
    // device and still holds its pairing.
    assert!(routers[2]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert!(peers[2]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == removed));

    // One control exchange with the remover applies the removal: the policy
    // drops the device, the observer's member table shrinks with it, and the
    // deferred sweep drops the pairing while its other pairings stay.
    let sync = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
        routers[2].localCore.clone(),
        routers[2].clone(),
        routers[2].spaceStore.clone(),
    );
    sync.synchronizeReachablePeer(routers[0].localNodeId(), 512, false)
        .await
        .unwrap();
    assert!(!routers[2]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert!(!routers[2].spaceStore.contains(removed.clone()).unwrap());
    assert!(!routers[2]
        .spaceStore
        .space()
        .unwrap()
        .members
        .contains(&removed));
    assert!(!peers[2]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == removed));
    // Pairings with devices that were never removed stay.
    assert!(peers[2]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == routers[0].localNodeId()));
    assert!(peers[2]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == routers[1].localNodeId()));
}

/// Verifies the notice stays best-effort: an offline target never blocks the
/// removal and never changes what the remover forgets.
#[tokio::test]
async fn removal_notice_is_best_effort_when_the_target_is_offline() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = revocationMesh("revoke-offline", 2, &[(0, 1)]);
    let removed = routers[1].localNodeId();
    peers[0].disconnectPeer(&removed).await.unwrap();

    access[0]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();

    assert!(!routers[0].spaceStore.contains(removed.clone()).unwrap());
    assert!(!peers[0]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == removed));
    // The offline device keeps its stale view: delivery is best-effort, and a
    // partitioned device converges when a path returns.
    assert!(routers[1]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
}

/// Verifies a member cannot relay another issuer's authority command: only
/// the command's own origin (or a wildcard authority) may introduce it, so a
/// removed device can never broadcast a removal it did not receive.
#[tokio::test]
async fn control_exchange_rejects_relayed_foreign_authority_commands() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, _access, _peers) =
        revocationMesh("revoke-relay", 3, &[(0, 1), (0, 2), (1, 2)]);
    let courier = routers[1].localNodeId();
    let observer = RuntimeRemoteLinkService::newWithRouter(
        (*routers[2].localCore).clone(),
        routers[2].clone(),
    );

    // The courier holds an authority command the observer has never seen and
    // tries to introduce it as its own.
    routers[0]
        .networkControlStore
        .admitMember("revoke-relay-outsider".into())
        .unwrap();
    let foreignOperations = routers[0]
        .networkControlStore
        .currentSpaceOperations()
        .unwrap();
    assert!(!foreignOperations.is_empty());
    let request = operit_link::CoreCallRequest::new(
        "relayed-authority".to_string(),
        crate::NodeSpaceService::NODE_SPACE_CONTROL_TARGET,
        "exchange",
        operit_link::toCoreValue(foreignOperations).unwrap(),
    );
    let error = crate::NodeSpaceService::acceptSpaceControlCall(&observer, &courier, request)
        .expect_err("a member must not introduce another issuer's authority command");
    assert!(error.contains("cannot introduce another issuer"), "{error}");
}

/// Verifies a removed device can pair again and submit a fresh join request:
/// its own retirement leaves it as an independent Space, which is exactly the
/// state the join flow expects from an applicant.
#[tokio::test]
async fn removed_device_rejoins_after_pairing_again() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = revocationMesh(
        "revoke-rejoin",
        4,
        &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
    );
    let target = 0;
    let removed = routers[3].localNodeId();

    access[target]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();

    // The removed device retired into its own Space and forgot its pairings
    // in the same step.
    assert_eq!(
        routers[3].spaceStore.space().unwrap().members,
        vec![removed.clone()]
    );
    assert!(peers[3].pairedPeers().unwrap().is_empty());

    // Pairing again (the fixture models the handshake by linking) and applying.
    peers[3].link(&routers[target]);
    peers[target].link(&routers[3]);
    let request = access[3]
        .requestDeviceSpaceJoin(routers[target].localNodeId())
        .await
        .expect("a re-paired removed device must be able to submit a join request");
    assert_eq!(request.targetDeviceId, routers[target].localNodeId());
    assert!(access[target]
        .incomingDeviceSpaceJoins()
        .await
        .unwrap()
        .iter()
        .any(|pending| pending.requestId == request.requestId));
}

/// Verifies a device that never learned about its removal — it was offline
/// when it happened and no sync ran since — heals on its own when it applies
/// again: the join flow reconciles the split, proves the removal from the
/// merged log, retires it into its own Space and submits the request instead
/// of failing with a bare "membership split" error.
#[tokio::test]
async fn stale_removed_device_heals_into_its_own_space_when_reapplying() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = revocationMesh(
        "revoke-stale",
        4,
        &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
    );
    let target = 0;
    let removed = routers[3].localNodeId();

    // The device is offline for the removal on every link: it neither
    // receives the notice (not even through a relay) nor runs a
    // synchronization round, so it keeps its stale member view.
    for index in 0..4 {
        peers[index].disconnectPeer(&removed).await.unwrap();
    }
    access[target]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();
    assert!(routers[3]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert_eq!(
        routers[3].spaceStore.space().unwrap().spaceId,
        routers[target].spaceStore.space().unwrap().spaceId
    );

    // Pairing again (the fixture models the handshake by linking) and applying.
    peers[target].link(&routers[3]);
    peers[3].link(&routers[target]);
    let request = access[3]
        .requestDeviceSpaceJoin(routers[target].localNodeId())
        .await
        .expect("a stale removed device must heal and submit its request");
    assert_eq!(request.targetDeviceId, routers[target].localNodeId());
    assert!(access[target]
        .incomingDeviceSpaceJoins()
        .await
        .unwrap()
        .iter()
        .any(|pending| pending.requestId == request.requestId));
    // The applicant ended up as its own Space, ready for the admission.
    assert_eq!(
        routers[3].spaceStore.space().unwrap().members,
        vec![removed]
    );
}

/// Verifies re-reading the shared history after a re-join cannot retire the
/// device again: the removal stays in the log forever, but the replayed state
/// admits the device, so the retirement check never fires.
#[tokio::test]
async fn readmitted_device_survives_its_own_removal_record_in_history() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = revocationMesh("revoke-readmit", 3, &[(0, 1), (0, 2), (1, 2)]);
    let target = 0;
    let removed = routers[2].localNodeId();

    // Remove, then admit the device again through the ordinary policy path.
    access[target]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();
    assert_eq!(
        routers[2].spaceStore.space().unwrap().members,
        vec![removed.clone()]
    );
    routers[target]
        .networkControlStore
        .admitMember(removed.clone())
        .unwrap();
    assert!(routers[target]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));

    // The shared Space history still carries the removal, and the device
    // recovers the whole log while re-joining.
    let operations = routers[target]
        .networkControlStore
        .currentSpaceOperations()
        .unwrap();
    assert!(operations.iter().any(|operation| {
        serde_json::to_string(operation).unwrap().contains("remove_member")
    }));
    for operation in &operations {
        routers[2]
            .networkControlStore
            .applyBootstrapOperation(operation)
            .unwrap();
    }
    let mut sharedSpace = routers[target].spaceStore.space().unwrap();
    sharedSpace.spaceRevision = routers[2].spaceStore.space().unwrap().spaceRevision + 10;
    routers[2].spaceStore.adopt(sharedSpace).unwrap();

    // The retirement check sees a member and a history with its removal; the
    // device must stay in the Space.
    let service = RuntimeRemoteLinkService::newWithRouter(
        (*routers[2].localCore).clone(),
        routers[2].clone(),
    );
    assert!(!service.retireRemovedDevice().await.unwrap());
    assert!(routers[2]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert!(routers[2]
        .spaceStore
        .space()
        .unwrap()
        .members
        .contains(&removed));
    // Re-pairing for the re-admission survives every later sweep: the
    // retirement marker was consumed once and cannot fire again.
    peers[2].link(&routers[1]);
    peers[1].link(&routers[2]);
    assert!(!service.retireRemovedDevice().await.unwrap());
    assert!(peers[2]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == routers[1].localNodeId()));
}

/// Verifies the pairing a device makes for re-admission survives every later
/// replay of the historical removal. The old `RemoveMember` stays in the shared
/// history forever, so a synchronization round that re-delivers it must not
/// delete the fresh pairing — otherwise the re-applicant's own core refuses the
/// submission ("Join request requires a current directional pairing") while
/// the paired peer never sees a request.
#[tokio::test]
async fn historical_removal_replay_keeps_the_fresh_pairing_of_a_reapplicant() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) =
        revocationMesh("revoke-keep-pairing", 3, &[(0, 1), (0, 2), (1, 2)]);
    let remover = 0;
    let member = 1;
    let removed = routers[2].localNodeId();

    access[remover]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();
    assert!(peers[2].pairedPeers().unwrap().is_empty());

    // The member applies the removal in an ordinary synchronization round and
    // forgets the pairing it held when the removal took effect.
    let memberSync = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
        routers[member].localCore.clone(),
        routers[member].clone(),
        routers[member].spaceStore.clone(),
    );
    memberSync
        .synchronizeReachablePeer(routers[remover].localNodeId(), 512, false)
        .await
        .unwrap();
    assert!(!routers[member]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert!(!peers[member]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == removed));

    // The device pairs again for re-admission and applies. The application is
    // pending on the member.
    peers[2].link(&routers[member]);
    peers[member].link(&routers[2]);
    let request = access[2]
        .requestDeviceSpaceJoin(routers[member].localNodeId())
        .await
        .expect("a re-paired device applies");
    assert_eq!(request.status, SpaceJoinStatus::Pending);

    // The next synchronization window replays the same history. The removal
    // took effect before this pairing existed, so the pairing must survive.
    memberSync
        .synchronizeReachablePeer(routers[remover].localNodeId(), 512, false)
        .await
        .unwrap();
    assert!(
        peers[member]
            .pairedPeers()
            .unwrap()
            .iter()
            .any(|peer| peer.nodeId == removed),
        "a replayed historical removal must not forget the pairing made for re-admission"
    );
    // The application is still usable: applying again refreshes the pending
    // request instead of failing locally.
    let refreshed = access[2]
        .requestDeviceSpaceJoin(routers[member].localNodeId())
        .await
        .expect("the fresh pairing still authorizes the application");
    assert_eq!(refreshed.requestId, request.requestId);
}

/// Verifies a device that was admitted again is not ejected by the historical
/// removal replayed in later exchanges: membership follows the replayed
/// current policy, never the mere presence of the old command in history.
#[tokio::test]
async fn readmitted_device_survives_replayed_history_on_every_replica() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) =
        revocationMesh("revoke-readmit-kept", 3, &[(0, 1), (0, 2), (1, 2)]);
    let remover = 0;
    let member = 1;
    let removed = routers[2].localNodeId();

    access[remover]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();

    // The member enforces the removal in an ordinary synchronization round.
    let memberSync = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
        routers[member].localCore.clone(),
        routers[member].clone(),
        routers[member].spaceStore.clone(),
    );
    memberSync
        .synchronizeReachablePeer(routers[remover].localNodeId(), 512, false)
        .await
        .unwrap();
    assert!(!routers[member]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));

    // The device pairs again with both devices and completes a full re-admission.
    peers[2].link(&routers[remover]);
    peers[remover].link(&routers[2]);
    peers[2].link(&routers[member]);
    peers[member].link(&routers[2]);
    let request = access[2]
        .requestDeviceSpaceJoin(routers[remover].localNodeId())
        .await
        .unwrap();
    access[remover]
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .unwrap();
    let joined = access[2]
        .refreshDeviceSpaceJoin(request.requestId)
        .await
        .unwrap();
    assert_eq!(joined.status, SpaceJoinStatus::Joined);
    assert!(routers[remover].spaceStore.contains(removed.clone()).unwrap());

    // The next synchronization window re-delivers the same history. The
    // re-admission is the current truth; the old removal stays inert.
    let removerSync = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
        routers[remover].localCore.clone(),
        routers[remover].clone(),
        routers[remover].spaceStore.clone(),
    );
    removerSync
        .synchronizeReachablePeer(routers[member].localNodeId(), 512, false)
        .await
        .unwrap();
    assert!(
        routers[remover].spaceStore.contains(removed.clone()).unwrap(),
        "a re-admitted member must survive its historical removal"
    );
    assert!(routers[remover]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert!(
        peers[remover]
            .pairedPeers()
            .unwrap()
            .iter()
            .any(|peer| peer.nodeId == removed),
        "re-admission keeps the pairing it was approved with"
    );

    // The member learns the re-admission and keeps it too.
    memberSync
        .synchronizeReachablePeer(routers[remover].localNodeId(), 512, false)
        .await
        .unwrap();
    assert!(routers[member]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert!(
        peers[member]
            .pairedPeers()
            .unwrap()
            .iter()
            .any(|peer| peer.nodeId == removed),
        "history replay must not forget a re-admitted member's pairing"
    );
}

/// Verifies the pairing generation closes the late-learner window: a member
/// that was offline during the removal pairs with the removed device's next
/// generation, and only then accepts the removal for the first time. The
/// removal revokes the generation it was issued against, so the pairing made
/// afterwards must survive that first acceptance, and the application stays
/// usable instead of failing locally on a vanished pairing.
#[tokio::test]
async fn a_late_learner_keeps_the_pairing_made_after_the_removal() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) =
        revocationMesh("revoke-generation", 3, &[(0, 1), (0, 2), (1, 2)]);
    let remover = 0;
    let member = 1;
    let removed = routers[2].localNodeId();

    access[remover]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();
    // The removed device retired into its own Space with a fresh generation...
    assert!(peers[2].pairedPeers().unwrap().is_empty());
    // ...while the member never learned about the removal.
    assert!(routers[member]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));

    // The device pairs again with the still-stale member and applies.
    peers[2].link(&routers[member]);
    peers[member].link(&routers[2]);
    let request = access[2]
        .requestDeviceSpaceJoin(routers[member].localNodeId())
        .await
        .expect("a re-paired device applies");

    // The member now accepts the removal for the first time, in the same order
    // an ordinary round uses: the paired Space projection is exchanged before
    // any control exchange. It revokes the generation the device had while it
    // was admitted, not the one it paired with afterwards.
    let memberSync = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
        routers[member].localCore.clone(),
        routers[member].clone(),
        routers[member].spaceStore.clone(),
    );
    memberSync
        .exchangePairedDeviceSpaceProjection(&routers[remover].localNodeId())
        .await
        .unwrap();
    memberSync
        .synchronizeReachablePeer(routers[remover].localNodeId(), 512, false)
        .await
        .unwrap();
    assert!(!routers[member]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    // The membership projection still shrank...
    assert!(!routers[member].spaceStore.contains(removed.clone()).unwrap());
    // ...but the pairing made for re-admission survives the removal.
    assert!(
        peers[member]
            .pairedPeers()
            .unwrap()
            .iter()
            .any(|peer| peer.nodeId == removed),
        "a removal revokes the generation it was issued against, never the pairing made after it"
    );
    let refreshed = access[2]
        .requestDeviceSpaceJoin(routers[member].localNodeId())
        .await
        .expect("the fresh pairing still authorizes the application");
    assert_eq!(refreshed.requestId, request.requestId);
}

/// Verifies a member forgets the removed device in an ordinary synchronization
/// round, where the paired Space projection is exchanged before any control
/// exchange. That snapshot path ingests the removal into the local log, so the
/// forget must act on the operation, not on whichever delivery carried it —
/// otherwise every reachable member keeps the removed device's pairing.
#[tokio::test]
async fn a_member_forgets_the_removed_device_when_the_snapshot_arrives_first() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = revocationMesh("revoke-round", 3, &[(0, 1), (0, 2), (1, 2)]);
    let remover = 0;
    let member = 1;
    let removed = routers[2].localNodeId();

    access[remover]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();
    // The member has not synchronized: it still holds the removed device.
    assert!(routers[member]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert!(peers[member]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == removed));

    let memberSync = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
        routers[member].localCore.clone(),
        routers[member].clone(),
        routers[member].spaceStore.clone(),
    );
    // An ordinary round exchanges the paired Space projection first, then the
    // control exchange.
    memberSync
        .exchangePairedDeviceSpaceProjection(&routers[remover].localNodeId())
        .await
        .unwrap();
    memberSync
        .synchronizeReachablePeer(routers[remover].localNodeId(), 512, false)
        .await
        .unwrap();

    assert!(!routers[member]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert!(!routers[member].spaceStore.contains(removed.clone()).unwrap());
    assert!(
        !peers[member]
            .pairedPeers()
            .unwrap()
            .iter()
            .any(|peer| peer.nodeId == removed),
        "mutual forgetting must fire in an ordinary round, whichever path carried the removal"
    );
    // Pairings with devices that were never removed stay.
    assert!(peers[member]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == routers[remover].localNodeId()));
}

/// Verifies a member that missed an entire remove -> re-join -> remove cycle
/// still forgets the pairing it made in between: the removals arrive in one
/// batch, and the pairing is revoked when any of them revoked the generation
/// it was made under. The older removal's generation must not shadow the newer.
#[tokio::test]
async fn a_member_that_missed_multiple_removals_forgets_the_latest_generation() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = revocationMesh("revoke-multi", 3, &[(0, 1), (0, 2), (1, 2)]);
    let remover = 0;
    let member = 1;
    let removed = routers[2].localNodeId();

    // First removal: the member never synchronizes, so it misses everything.
    access[remover]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();
    assert!(peers[2].pairedPeers().unwrap().is_empty());

    // The device pairs again with the remover and is fully re-admitted.
    peers[2].link(&routers[remover]);
    peers[remover].link(&routers[2]);
    let request = access[2]
        .requestDeviceSpaceJoin(routers[remover].localNodeId())
        .await
        .unwrap();
    access[remover]
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .unwrap();
    let joined = access[2]
        .refreshDeviceSpaceJoin(request.requestId)
        .await
        .unwrap();
    assert_eq!(joined.status, SpaceJoinStatus::Joined);

    // The still-stale member pairs with the re-admitted device...
    peers[2].link(&routers[member]);
    peers[member].link(&routers[2]);
    // ...and the device is removed a second time.
    access[remover]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();

    // The member synchronizes once and sees both removals in the same batch.
    let memberSync = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
        routers[member].localCore.clone(),
        routers[member].clone(),
        routers[member].spaceStore.clone(),
    );
    memberSync
        .exchangePairedDeviceSpaceProjection(&routers[remover].localNodeId())
        .await
        .unwrap();
    memberSync
        .synchronizeReachablePeer(routers[remover].localNodeId(), 512, false)
        .await
        .unwrap();

    assert!(!routers[member].spaceStore.contains(removed.clone()).unwrap());
    assert!(
        !peers[member]
            .pairedPeers()
            .unwrap()
            .iter()
            .any(|peer| peer.nodeId == removed),
        "the latest revoked generation must revoke the pairing made under it"
    );
}
