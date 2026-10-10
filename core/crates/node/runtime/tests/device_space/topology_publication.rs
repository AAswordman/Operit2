use super::*;

/// Publishes every device's adjacency through the production helper, then carries the
/// records the way the Space snapshot exchange does.
fn publishTree(routers: &[CoreNodeRouter], peers: &[Arc<ApprovalMeshPeer>]) {
    for (router, peer) in routers.iter().zip(peers) {
        let activePeers = peer.activePeerNodeIds().unwrap();
        crate::NodeSpaceService::space_topology::publishLocalTopology(
            &router.spaceStore,
            &router.networkControlStore,
            &router.localNodeId(),
            &activePeers,
        )
        .expect("production adjacency publication must succeed");
    }
    propagateTopology(routers);
}

/// Carries every published record to every other device, exactly as snapshots do.
fn propagateTopology(routers: &[CoreNodeRouter]) {
    let topology = routers
        .iter()
        .map(|router| {
            router
                .spaceStore
                .topologyRecords()
                .unwrap()
                .remove(&router.localNodeId())
                .expect("every device publishes its own adjacency")
        })
        .collect::<Vec<_>>();
    for router in routers {
        router
            .spaceStore
            .importTopologyRecords(topology.clone())
            .unwrap();
    }
}

/// Verifies a tree resolves member-to-member routing from published adjacency alone.
#[tokio::test]
async fn tree_topology_publication_routes_between_leaves() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let services = (0..3)
        .map(|index| approvalService(&format!("tree-{index}")))
        .collect::<Vec<_>>();
    let routers = services
        .iter()
        .map(|(router, _)| router.clone())
        .collect::<Vec<_>>();
    let access = services
        .iter()
        .map(|(_, service)| service.clone())
        .collect::<Vec<_>>();
    mergedChainFixture(&routers.iter().collect::<Vec<_>>());
    let peers = routers
        .iter()
        .map(|router| ApprovalMeshPeer::new(router.localNodeId()))
        .collect::<Vec<_>>();
    // A tree: leaf A - core C - leaf B.
    for (a, b) in [(0, 1), (1, 2)] {
        peers[a].link(&routers[b]);
        peers[b].link(&routers[a]);
    }
    for (router, peer) in routers.iter().zip(&peers) {
        router
            .installNodeServices(NodeServices::new(peer.clone()))
            .unwrap();
    }

    publishTree(&routers, &peers);

    let middleRecord = serde_json::to_value(
        routers[1]
            .spaceStore
            .topologyRecords()
            .unwrap()
            .remove(&routers[1].localNodeId())
            .expect("the core publishes its own adjacency"),
    )
    .unwrap();
    assert_eq!(middleRecord["peers"].as_array().unwrap().len(), 2);
    assert_eq!(middleRecord["links"].as_array().unwrap().len(), 2);

    let leafToLeaf = routers[0]
        .nodeRoutePlan(&routers[2].localNodeId())
        .expect("route plan lookup must succeed")
        .expect("one leaf must reach its sibling through the core");
    assert_eq!(
        leafToLeaf.path,
        vec![routers[1].localNodeId(), routers[2].localNodeId()]
    );
    assert_eq!(leafToLeaf.hops(), 2);
    assert!(routers[0]
        .nodeIsReachable(&routers[2].localNodeId())
        .unwrap());
    let reverse = routers[2]
        .nodeRoutePlan(&routers[0].localNodeId())
        .unwrap()
        .expect("the reverse tree path must resolve");
    assert_eq!(
        reverse.path,
        vec![routers[1].localNodeId(), routers[0].localNodeId()]
    );
    let direct = routers[0]
        .nodeRoutePlan(&routers[1].localNodeId())
        .unwrap()
        .expect("the direct peer must stay reachable");
    assert_eq!(direct.hops(), 1);

    let topology = access[0]
        .deviceSpaceTopology()
        .expect("the device-space projection must read");
    let sibling = topology
        .devices
        .iter()
        .find(|device| device.deviceId == routers[2].localNodeId())
        .expect("the sibling leaf must appear in the projection");
    assert!(sibling.online, "a relayed member must report online");
    assert_eq!(sibling.relayHops, Some(2));
    assert_eq!(
        sibling.relayPath,
        Some(vec![routers[1].localNodeId(), routers[2].localNodeId()])
    );
    let core = topology
        .devices
        .iter()
        .find(|device| device.deviceId == routers[1].localNodeId())
        .expect("the core must appear in the projection");
    assert!(core.online);
    assert_eq!(core.relayHops, None, "a direct peer is not relayed");
    let crossEdge = topology
        .connections
        .iter()
        .find(|connection| {
            (connection.firstDeviceId == routers[1].localNodeId()
                && connection.secondDeviceId == routers[2].localNodeId())
                || (connection.firstDeviceId == routers[2].localNodeId()
                    && connection.secondDeviceId == routers[1].localNodeId())
        })
        .expect("the core announces its sibling link");
    assert_eq!(
        crossEdge.status,
        crate::RuntimeRemoteLinkService::RuntimeDeviceSpaceConnectionStatus::Announced
    );
    let localEdge = topology
        .connections
        .iter()
        .find(|connection| {
            (connection.firstDeviceId == routers[0].localNodeId()
                && connection.secondDeviceId == routers[1].localNodeId())
                || (connection.firstDeviceId == routers[1].localNodeId()
                    && connection.secondDeviceId == routers[0].localNodeId())
        })
        .expect("the local device announces its direct link");
    assert_eq!(
        localEdge.status,
        crate::RuntimeRemoteLinkService::RuntimeDeviceSpaceConnectionStatus::Online
    );
}

/// Verifies a dropped direct link is pruned from the announcement and stops routing.
#[tokio::test]
async fn tree_topology_prunes_a_dropped_direct_link() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let routers = (0..3)
        .map(|index| approvalService(&format!("prune-{index}")).0)
        .collect::<Vec<_>>();
    mergedChainFixture(&routers.iter().collect::<Vec<_>>());
    let peers = routers
        .iter()
        .map(|router| ApprovalMeshPeer::new(router.localNodeId()))
        .collect::<Vec<_>>();
    for (a, b) in [(0, 1), (1, 2)] {
        peers[a].link(&routers[b]);
        peers[b].link(&routers[a]);
    }
    for (router, peer) in routers.iter().zip(&peers) {
        router
            .installNodeServices(NodeServices::new(peer.clone()))
            .unwrap();
    }

    publishTree(&routers, &peers);
    assert!(routers[0]
        .nodeIsReachable(&routers[2].localNodeId())
        .unwrap());

    // The leaf drops its only link; the live peer set stops proving its sibling reachable.
    peers[0]
        .disconnectPeer(&routers[1].localNodeId())
        .await
        .unwrap();
    assert!(!routers[0]
        .nodeIsReachable(&routers[2].localNodeId())
        .unwrap());

    // The next publication removes the peer and its link from the announcement.
    let wrote = crate::NodeSpaceService::space_topology::publishLocalTopology(
        &routers[0].spaceStore,
        &routers[0].networkControlStore,
        &routers[0].localNodeId(),
        &peers[0].activePeerNodeIds().unwrap(),
    )
    .unwrap();
    assert!(
        wrote,
        "dropping the only peer must rewrite the local announcement"
    );
    let record = serde_json::to_value(
        routers[0]
            .spaceStore
            .topologyRecords()
            .unwrap()
            .remove(&routers[0].localNodeId())
            .unwrap(),
    )
    .unwrap();
    assert!(record["peers"].as_array().unwrap().is_empty());
    assert!(record["links"].as_array().unwrap().is_empty());

    // An unchanged topology does not rewrite the record again.
    let wroteAgain = crate::NodeSpaceService::space_topology::publishLocalTopology(
        &routers[0].spaceStore,
        &routers[0].networkControlStore,
        &routers[0].localNodeId(),
        &peers[0].activePeerNodeIds().unwrap(),
    )
    .unwrap();
    assert!(!wroteAgain, "an unchanged adjacency must not rewrite the record");
}
