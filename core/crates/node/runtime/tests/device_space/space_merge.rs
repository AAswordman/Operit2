use super::*;

/// Exercises a real group approval and propagates migration through B-C-D-(E,F)-G only.
#[tokio::test]
async fn source_space_chain_merges_all_members_profiles_policy_and_routes() {
    use operit_store::CoreSpaceStore::CoreSpaceLinkAdvertisement;
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let nodes = [
        "merge-a", "merge-b", "merge-c", "merge-d", "merge-e", "merge-f", "merge-g",
    ]
    .map(approvalService);
    let routers = nodes.iter().map(|(router, _)| router).collect::<Vec<_>>();
    let services = nodes.iter().map(|(_, service)| service).collect::<Vec<_>>();
    mergedChainFixture(&[routers[1], routers[0]]);
    mergedChainFixture(&routers[2..]);
    let sourceId = services[2].deviceSpace().unwrap().spaceId;
    let peers = routers
        .iter()
        .map(|router| ApprovalMeshPeer::new(router.localNodeId()))
        .collect::<Vec<_>>();
    let edges = [(0, 1), (1, 2), (2, 3), (3, 4), (3, 5), (5, 6)];
    for &(a, b) in &edges {
        peers[a].link(routers[b]);
        peers[b].link(routers[a]);
    }
    for (router, peer) in routers.iter().zip(&peers) {
        router
            .installNodeServices(NodeServices::new(peer.clone()))
            .unwrap();
        let adjacent = peer
            .activePeerNodeIds()
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        router.spaceStore.setDirectPeers(adjacent.clone()).unwrap();
        let now = operit_host_api::TimeUtils::currentTimeMillis();
        for node in adjacent {
            router
                .spaceStore
                .publishLocalLinkAdvertisement(CoreSpaceLinkAdvertisement {
                    targetNodeId: node,
                    channelEpoch: "merge-test".into(),
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
    // Each source has only its actual directed edges, replicated as normal topology metadata.
    for group in [&routers[..2], &routers[2..]] {
        let topology = group
            .iter()
            .flat_map(|router| router.spaceStore.topologyRecords().unwrap().into_values())
            .collect::<Vec<_>>();
        for router in group {
            router
                .spaceStore
                .importTopologyRecords(topology.clone())
                .unwrap();
        }
    }
    let request = services[2]
        .requestDeviceSpaceJoin(routers[1].localNodeId())
        .await
        .unwrap();
    let incoming = services[1].incomingDeviceSpaceJoins().await.unwrap();
    assert_eq!(incoming.len(), 1);
    assert_eq!(
        incoming[0].applicantName,
        "merge-c, merge-d, merge-e, merge-f, merge-g"
    );
    services[1]
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .unwrap();
    // B can render every approved member before C completes its own join.
    assert_eq!(services[1].deviceSpaceTopology().unwrap().devices.len(), 7);
    assert_eq!(
        services[2]
            .refreshDeviceSpaceJoin(request.requestId)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Joined
    );
    // The migrated C can still observe an old-space D without moving back to the source Space.
    let mergedId = services[2].deviceSpace().unwrap().spaceId;
    services[2]
        .observePeerSpaceSnapshot(
            &routers[3].localNodeId(),
            services[3].peerSpaceSnapshot().unwrap(),
        )
        .unwrap();
    assert_eq!(services[2].deviceSpace().unwrap().spaceId, mergedId);
    for &(from, to) in &[(1, 0), (2, 3), (3, 4), (3, 5), (5, 6)] {
        let sync = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
            routers[from].localCore.clone(),
            routers[from].clone(),
            routers[from].spaceStore.clone(),
        );
        assert!(sync
            .exchangePairedDeviceSpaceProjection(&routers[to].localNodeId())
            .await
            .unwrap());
    }
    for service in &services {
        let space = service.deviceSpace().unwrap();
        assert_eq!(space.spaceId, mergedId);
        assert_ne!(space.spaceId, sourceId);
        assert_eq!(space.members.len(), 7);
        assert_eq!(service.deviceSpaceTopology().unwrap().devices.len(), 7);
        assert_eq!(service.deviceSpaceControl().unwrap().memberNodeIds.len(), 7);
        let router = routers
            .iter()
            .find(|router| {
                router.localNodeId() == service.deviceSpaceTopology().unwrap().currentDeviceId
            })
            .unwrap();
        router.networkControlStore.audit().unwrap();
    }
    // Transit permission is preserved by normal user admission, without creating B-D shortcuts.
    assert!(routers[1]
        .nodeIsReachable(&routers[6].localNodeId())
        .unwrap());
    assert!(!peers[1]
        .activePeerNodeIds()
        .unwrap()
        .contains(&routers[3].localNodeId()));
    assert_eq!(
        services[1].deviceSpaceTopology().unwrap().connections.len(),
        edges.len() * 2
    );
}

/// Verifies that one incomplete incoming projection cannot publish dangling member references.
#[tokio::test]
async fn incomplete_space_merge_snapshot_does_not_change_membership() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (sourceRouter, source) = approvalService("incomplete-source");
    let (targetRouter, target) = approvalService("incomplete-target");
    let before = source.deviceSpace().unwrap();
    targetRouter
        .networkControlStore
        .admitSpace(
            before.spaceId.clone(),
            before.members.iter().map(|node| (node.clone(), "user".into())).collect(),
        )
        .unwrap();
    let mut snapshot = target.peerSpaceSnapshot().unwrap();
    snapshot.space.members.push(sourceRouter.localNodeId());
    let error = source
        .observePeerSpaceSnapshot(&targetRouter.localNodeId(), snapshot)
        .unwrap_err();
    assert_eq!(
        error,
        "Space snapshot is missing a member profile: incomplete-source"
    );
    assert_eq!(source.deviceSpace().unwrap(), before);
}
