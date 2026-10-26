use super::*;
use operit_store::CoreSpaceStore::CoreSpaceLinkAdvertisement;
use operit_store::NetworkControlStore::NetworkControlIdentityAssignment;

/// Creates a cyclic graph with a single last-hop bridge to the fourth member.
fn routedMesh(prefix: &str) -> (Vec<CoreNodeRouter>, Vec<Arc<ApprovalMeshPeer>>) {
    let routers = (0..4)
        .map(|index| approvalService(&format!("{prefix}-{index}")).0)
        .collect::<Vec<_>>();
    mergedChainFixture(&routers.iter().collect::<Vec<_>>());
    let peers = routers
        .iter()
        .map(|router| ApprovalMeshPeer::new(router.localNodeId()))
        .collect::<Vec<_>>();
    for (a, b) in [(0, 1), (1, 2), (2, 0), (2, 3)] {
        peers[a].link(&routers[b]);
        peers[b].link(&routers[a]);
    }
    for (router, peer) in routers.iter().zip(&peers) {
        router
            .installNodeServices(NodeServices::new(peer.clone()))
            .unwrap();
    }
    publishMesh(&routers, &peers);
    (routers, peers)
}

/// Publishes only active direct links and replicates measured topology through the production store.
fn publishMesh(routers: &[CoreNodeRouter], peers: &[Arc<ApprovalMeshPeer>]) {
    let now = operit_host_api::TimeUtils::currentTimeMillis();
    for (router, peer) in routers.iter().zip(peers) {
        let adjacent = peer.activePeerNodeIds().unwrap();
        router
            .spaceStore
            .setDirectPeers(adjacent.iter().cloned().collect())
            .unwrap();
        for target in adjacent {
            router
                .spaceStore
                .publishLocalLinkAdvertisement(CoreSpaceLinkAdvertisement {
                    targetNodeId: target,
                    channelEpoch: "contract".into(),
                    sequence: 1,
                    smoothedRttMs: 1,
                    lossPermille: 0,
                    congestionPermille: 0,
                    measuredAt: now,
                    expiresAt: now + 600_000,
                })
                .unwrap();
        }
    }
    let revision = routers
        .iter()
        .flat_map(|router| router.spaceStore.topologyRecords().unwrap().into_values())
        .map(|record| {
            serde_json::to_value(record).unwrap()["updatedAt"]
                .as_i64()
                .unwrap()
        })
        .max()
        .unwrap()
        + 1;
    let topology = routers
        .iter()
        .map(|router| {
            let record = router
                .spaceStore
                .topologyRecords()
                .unwrap()
                .remove(&router.localNodeId())
                .unwrap();
            let mut encoded = serde_json::to_value(record).unwrap();
            encoded["updatedAt"] = serde_json::json!(revision);
            serde_json::from_value(encoded).unwrap()
        })
        .collect::<Vec<_>>();
    for router in routers {
        router
            .spaceStore
            .importTopologyRecords(topology.clone())
            .unwrap();
    }
}

/// Routes an actual approval lookup instead of merely asserting a calculated next-hop string.
async fn routedRead(source: &CoreNodeRouter, target: &CoreNodeRouter) -> CoreCallResponse {
    source
        .callNode(
            target.localNodeId(),
            CoreCallRequest::new(
                "mesh-read",
                crate::RuntimeRemoteLinkService::NODE_SPACE_APPROVAL_TARGET,
                "assigned",
                CoreValue::Null,
            ),
        )
        .await
}

/// Verifies cycle termination, partition visibility and reconnection without changing membership.
#[tokio::test]
async fn multi_hop_cycle_partition_and_reconnect_preserve_space_membership() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, peers) = routedMesh("route-partition");
    let before = routers[0].spaceStore.space().unwrap();
    assert!(routers[0]
        .nodeIsReachable(&routers[3].localNodeId())
        .unwrap());
    assert!(
        tokio::time::timeout(Duration::from_secs(5), routedRead(&routers[0], &routers[3]))
            .await
            .unwrap()
            .result
            .is_ok()
    );
    assert!(!peers[0]
        .activePeerNodeIds()
        .unwrap()
        .contains(&routers[3].localNodeId()));
    peers[2]
        .disconnectPeer(&routers[3].localNodeId())
        .await
        .unwrap();
    peers[3]
        .disconnectPeer(&routers[2].localNodeId())
        .await
        .unwrap();
    publishMesh(&routers, &peers);
    assert!(!routers[0]
        .nodeIsReachable(&routers[3].localNodeId())
        .unwrap());
    assert!(
        tokio::time::timeout(Duration::from_secs(5), routedRead(&routers[0], &routers[3]))
            .await
            .unwrap()
            .result
            .is_err()
    );
    assert_eq!(routers[0].spaceStore.space().unwrap(), before);
    peers[2].link(&routers[3]);
    peers[3].link(&routers[2]);
    publishMesh(&routers, &peers);
    assert!(routedRead(&routers[0], &routers[3]).await.result.is_ok());
    assert_eq!(routers[0].spaceStore.space().unwrap(), before);
}

/// Verifies a member without relay authority cannot carry a later hop despite complete topology.
#[tokio::test]
async fn removing_relay_capability_blocks_transit_but_not_direct_access() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, _) = routedMesh("route-permission");
    assert!(routers[0]
        .nodeIsReachable(&routers[3].localNodeId())
        .unwrap());
    let operation = routers[0]
        .networkControlStore
        .setIdentity(NetworkControlIdentityAssignment {
            nodeId: routers[2].localNodeId(),
            roleId: "edge".into(),
        })
        .unwrap();
    for router in routers.iter().skip(1) {
        router
            .networkControlStore
            .applySyncedOperation(&operation)
            .unwrap();
    }
    assert!(!routers[0]
        .nodeIsReachable(&routers[3].localNodeId())
        .unwrap());
    assert!(routers[0]
        .nodeIsReachable(&routers[2].localNodeId())
        .unwrap());
    assert!(routedRead(&routers[0], &routers[3]).await.result.is_err());
    assert_eq!(routers[0].spaceStore.space().unwrap().members.len(), 4);
}

/// Verifies an exhausted TTL is rejected before forwarding into a cycle.
#[tokio::test]
async fn exhausted_route_ttl_is_rejected_without_forwarding() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, peers) = routedMesh("route-ttl");
    let beforeCalls = peers[1].calls.lock().unwrap().len();
    let response = peers[0]
        .call(
            &routers[1].localNodeId(),
            RoutedCoreRequest {
                spaceId: routers[0].spaceStore.space().unwrap().spaceId,
                originNodeId: routers[0].localNodeId(),
                targetNodeId: routers[3].localNodeId(),
                ttl: 0,
                routeKind: RoutedCoreRequestKind::Target,
                payload: CoreCallRequest::new(
                    "expired-ttl",
                    crate::RuntimeRemoteLinkService::NODE_SPACE_APPROVAL_TARGET,
                    "assigned",
                    CoreValue::Null,
                ),
            },
        )
        .await;
    assert_eq!(
        response.result.unwrap_err().code,
        "CORE_NODE_ROUTE_TTL_EXHAUSTED"
    );
    assert_eq!(peers[1].calls.lock().unwrap().len(), beforeCalls);
}

/// Verifies an ejected target cannot be reached through still connected peers.
#[tokio::test]
async fn removed_target_cannot_be_reached_through_still_connected_peers() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, peers) = routedMesh("route-revocation");
    let removed = routers[3].localNodeId();
    let operation = routers[0]
        .networkControlStore
        .removeMember(removed.clone())
        .unwrap();
    for router in routers.iter() {
        if router.localNodeId() != removed {
            router
                .spaceStore
                .removeRemoteMember(removed.clone())
                .unwrap();
        }
    }
    for router in routers.iter().skip(1) {
        router
            .networkControlStore
            .applySyncedOperation(&operation)
            .unwrap();
    }
    assert!(peers[2]
        .activePeerNodeIds()
        .unwrap()
        .contains(&removed));
    assert!(!routers[0]
        .nodeIsReachable(&removed)
        .unwrap());
    let response = routedRead(&routers[0], &routers[3]).await;
    assert_eq!(
        response.result.unwrap_err().code,
        "CORE_NODE_NOT_IN_SPACE"
    );
}

/// Verifies ownership movement is replicated once and stale compare-and-set cannot overwrite it.
#[tokio::test]
async fn binding_owner_changes_replicate_and_reject_stale_writers() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, _) = routedMesh("binding-move");
    let stores = routers
        .iter()
        .map(|router| {
            operit_store::CoreNodeBindingStore::CoreNodeBindingStore::new(
                router.localCore.runtimeStorageHost(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let key = "space-contract-chat";
    let initial = stores[0].create(key, &routers[0].localNodeId()).unwrap();
    for store in stores.iter().skip(1) {
        store.applySyncedOperation(&initial.operation).unwrap();
    }
    let moved = stores[0]
        .compareAndSet(key, &routers[0].localNodeId(), &routers[3].localNodeId())
        .unwrap();
    assert_eq!(moved.binding.generation, 2);
    assert!(stores[0]
        .compareAndSet(key, &routers[0].localNodeId(), &routers[1].localNodeId())
        .is_err());
    for (router, store) in routers.iter().zip(&stores) {
        store.applySyncedOperation(&moved.operation).unwrap();
        store.applySyncedOperation(&initial.operation).unwrap();
        store.applySyncedOperation(&moved.operation).unwrap();
        assert_eq!(store.binding(key).unwrap(), moved.binding);
        assert_eq!(router.bindingStore.binding(key).unwrap(), moved.binding);
    }
}
