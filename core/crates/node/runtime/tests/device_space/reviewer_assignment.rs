use super::*;

/// Exercises nearest authorized reviewer only then offline transfer and remote decision through isolated runtime stores.
#[tokio::test]
async fn nearest_authorized_reviewer_only_then_offline_transfer_and_remote_decision() {
    use crate::PeerStateStore::PeerStateStore;
    use operit_store::CoreSpaceStore::CoreSpaceLinkAdvertisement;
    use operit_store::NetworkControlStore::NetworkControlIdentityAssignment;
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (applicantRouter, applicant) = approvalService("mesh-applicant");
    let (gatewayRouter, gateway) = approvalService("mesh-gateway");
    let (nearRouter, near) = approvalService("mesh-near");
    let (relayRouter, _) = approvalService("mesh-relay");
    let (farRouter, far) = approvalService("mesh-far");
    let nodes = [&gatewayRouter, &nearRouter, &relayRouter, &farRouter];
    let profiles: Vec<_> = nodes
        .iter()
        .flat_map(|r| r.spaceStore.deviceProfilesForCurrentSpace().unwrap())
        .collect();
    // The near reviewer bootstraps this target Space, explicitly admitting the
    // gateway as a normal member and the other reviewer as admin. No auto-admin.
    for router in [&gatewayRouter, &relayRouter, &farRouter] {
        nearRouter
            .networkControlStore
            .admitMember(router.localNodeId())
            .unwrap();
    }
    let mut target = near.deviceSpace().unwrap();
    target.spaceRevision = 10;
    target.members = nodes.iter().map(|r| r.localNodeId()).collect();
    nearRouter.spaceStore.adopt(target.clone()).unwrap();
    nearRouter
        .networkControlStore
        .setIdentity(NetworkControlIdentityAssignment {
            nodeId: "mesh-far".into(),
            roleId: "admin".into(),
        })
        .unwrap();
    let operations = nearRouter
        .networkControlStore
        .currentSpaceOperations()
        .unwrap();
    for router in nodes {
        if router.localNodeId() != "mesh-near" {
            router.spaceStore.adopt(target.clone()).unwrap();
        }
        for operation in &operations {
            router
                .networkControlStore
                .applyBootstrapOperation(operation)
                .unwrap();
        }
        router
            .spaceStore
            .importDeviceProfiles(profiles.clone())
            .unwrap();
    }
    let all = [
        &applicantRouter,
        &gatewayRouter,
        &nearRouter,
        &relayRouter,
        &farRouter,
    ];
    let peers: Vec<_> = all
        .iter()
        .map(|r| ApprovalMeshPeer::new(r.localNodeId()))
        .collect();
    for (a, b) in [(0, 1), (1, 2), (1, 3), (3, 4)] {
        peers[a].link(all[b]);
        peers[b].link(all[a]);
    }
    for (router, peer) in all.iter().zip(&peers) {
        router
            .installNodeServices(NodeServices::new(peer.clone()))
            .unwrap();
    }
    // Publish real directed topology and current link measurements. These are
    // the exact graph APIs used by production routing and distance selection.
    let now = operit_host_api::TimeUtils::currentTimeMillis();
    for (router, peer) in all.iter().zip(&peers).skip(1) {
        let active = peer
            .activePeerNodeIds()
            .unwrap()
            .into_iter()
            .filter(|id| target.members.contains(id))
            .collect::<Vec<_>>();
        router.spaceStore.setDirectPeers(active.clone()).unwrap();
        for node in active {
            router
                .spaceStore
                .publishLocalLinkAdvertisement(CoreSpaceLinkAdvertisement {
                    targetNodeId: node,
                    channelEpoch: "test-epoch".into(),
                    sequence: 1,
                    measuredAt: now,
                    expiresAt: now + 60_000,
                    smoothedRttMs: 1,
                    lossPermille: 0,
                    congestionPermille: 0,
                })
                .unwrap();
        }
    }
    // Replicate topology records through the existing host fixture only.
    let topologyPath = operit_util::RuntimeStorageLayout::RUNTIME_SPACE_TOPOLOGY_DIR_PATH;
    for source in nodes {
        let storage = source.localCore.runtimeStorageHost();
        for entry in storage.list(topologyPath).unwrap() {
            if entry.isDirectory {
                continue;
            }
            let bytes = storage.readBytes(&entry.path).unwrap();
            for dest in nodes {
                dest.localCore
                    .runtimeStorageHost()
                    .writeBytes(&entry.path, &bytes)
                    .unwrap();
            }
        }
    }
    assert!(!gatewayRouter
        .networkControlStore
        .nodeHasCapability("mesh-gateway", "network.members.join", None)
        .unwrap());
    let request = applicant
        .requestDeviceSpaceJoin("mesh-gateway".into())
        .await
        .unwrap();
    assert_eq!(request.reviewerDeviceId.as_deref(), Some("mesh-near"));
    assert_eq!(request.reviewerHops, Some(2));
    assert_eq!(near.incomingDeviceSpaceJoins().await.unwrap().len(), 1);
    let inbox = protocolRecords(&nearRouter, REVIEW_RECORDS);
    let projection = &inbox[&request.requestId];
    assert!(projection.get("profile").is_none(), "review list copied applicant profile evidence");
    assert!(projection["source"].get("controlOperations").is_none(), "review list copied authority history");
    assert!(projection["source"].get("deviceProfiles").is_none(), "review list copied applicant profiles");
    assert!(projection["source"].get("topology").is_none(), "review list copied applicant topology");
    assert_eq!(projection["source"]["space"]["members"], serde_json::json!(["mesh-applicant"]));
    assert!(far.incomingDeviceSpaceJoins().await.unwrap().is_empty());
    assert!(gateway.incomingDeviceSpaceJoins().await.unwrap().is_empty());
    peers[1].disconnectPeer("mesh-near").await.unwrap();
    let stillWaiting = applicant
        .refreshDeviceSpaceJoin(request.requestId.clone())
        .await
        .unwrap();
    assert_eq!(stillWaiting.reviewerDeviceId.as_deref(), Some("mesh-near")); // Grace period.
                                                                             // Advance just the record's unavailable timestamp; no real 30-second sleep.
    let records = PeerStateStore::new(gatewayRouter.localCore.runtimeStorageHost());
    let path = "runtime/link_access/space_merge_inbound.preferences.json";
    let mut record = records
        .records::<serde_json::Value>(path)
        .unwrap()
        .remove(&request.requestId)
        .unwrap();
    record["unavailableSince"] = serde_json::json!(now - 31_000);
    records
        .putRecord(path, &request.requestId, &record)
        .unwrap();
    let transferred = applicant
        .refreshDeviceSpaceJoin(request.requestId.clone())
        .await
        .unwrap();
    assert_eq!(transferred.reviewerDeviceId.as_deref(), Some("mesh-far"));
    assert_eq!(transferred.reviewerHops, Some(3));
    assert!(transferred.assignmentVersion > request.assignmentVersion);
    assert!(near
        .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
        .await
        .is_err());
    assert_eq!(far.incomingDeviceSpaceJoins().await.unwrap().len(), 1);
    far.decideDeviceSpaceJoin(
        request.requestId.clone(),
        transferred.assignmentVersion,
        true,
    )
    .await
    .unwrap();
    assert_eq!(
        applicant
            .refreshDeviceSpaceJoin(request.requestId)
            .await
            .unwrap()
            .status,
        SpaceJoinStatus::Joined
    );
    assert_eq!(
        applicant.deviceSpace().unwrap(),
        gateway.deviceSpace().unwrap()
    );
    assert!(!gatewayRouter
        .networkControlStore
        .nodeHasCapability("mesh-gateway", "network.members.join", None)
        .unwrap());
}
