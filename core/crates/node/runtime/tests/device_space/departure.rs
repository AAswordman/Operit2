use super::*;
use crate::NodeSpaceService::{space_departure, peerSpaceSnapshotForPeer};

/// Exit authorization belongs to the device itself; relays carry the resulting
/// control fact, and stale snapshots cannot restore departed membership.
#[tokio::test]
async fn departure_is_self_authored_and_relayed_without_moving_other_members() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, services, peers) = revocationMesh("self-exit", 4, &[(0,1),(1,2),(2,3)]);
    let c = routers[2].localNodeId();
    let b = routers[1].localNodeId();
    let d = routers[3].localNodeId();
    let original = routers[0].spaceStore.space().unwrap();
    let stale = services[2].peerSpaceSnapshot().unwrap();
    let pairings = peers[1].pairedPeers().unwrap();
    let singleton = services[2].leaveDeviceSpace().unwrap();
    assert_eq!(singleton.members, vec![c.clone()]);
    assert_ne!(singleton.spaceId, original.spaceId);
    assert_eq!(routers[2].networkControlStore.currentState().unwrap().deviceIdentityIds.get(&c).map(String::as_str), Some("admin"));
    let receipt = peerSpaceSnapshotForPeer(&services[2], &b).unwrap();
    assert_eq!(receipt.departures.len(), 1);
    // A different paired sender cannot use the departure channel to impersonate C.
    assert!(space_departure::observe(&services[1], &d, &receipt.departures, &[]).is_err());
    assert_eq!(routers[1].spaceStore.space().unwrap(), original);
    services[1].observePeerSpaceSnapshot(&c, receipt).unwrap();
    services[3].observePeerSpaceSnapshot(&c, peerSpaceSnapshotForPeer(&services[2], &d).unwrap()).unwrap();
    services[0].observePeerSpaceSnapshot(&b, services[1].peerSpaceSnapshot().unwrap()).unwrap();
    for index in [0,1,3] {
        let space = routers[index].spaceStore.space().unwrap();
        assert_eq!(space.spaceId, original.spaceId);
        assert_eq!(space.members, original.members.iter().filter(|node| **node != c).cloned().collect::<Vec<_>>());
        assert!(!routers[index].networkControlStore.currentState().unwrap().memberNodeIds.contains(&c));
        assert!(routers[index].spaceStore.deviceConnections().unwrap().iter()
            .all(|edge| edge.firstDeviceId != c && edge.secondDeviceId != c));
    }
    assert_eq!(peers[1].pairedPeers().unwrap(), pairings, "voluntary exit must retain the delivery pairing");
    services[1].observePeerSpaceSnapshot(&c, stale).unwrap();
    assert!(!routers[1].spaceStore.space().unwrap().members.contains(&c));
}

/// Receipt cleanup requires the neighbor to return the exact operation in its
/// durable log. Reopening the service or receiving an ordinary reply cannot ack it.
#[tokio::test]
async fn departure_survives_reopen_until_the_neighbor_has_the_exact_receipt() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, services, _) = revocationMesh("exit-retry", 2, &[(0,1)]);
    let a = routers[0].localNodeId();
    let c = routers[1].localNodeId();
    let old = services[0].peerSpaceSnapshot().unwrap();
    services[1].leaveDeviceSpace().unwrap();
    let receipt = space_departure::pending(&services[1], &a).unwrap();
    assert_eq!(receipt.len(), 1);
    let reopened = RuntimeRemoteLinkService::newWithRouter((*routers[1].localCore).clone(), routers[1].clone());
    assert_eq!(space_departure::pending(&reopened, &a).unwrap(), receipt);
    space_departure::observe(&reopened, &a, &old.departures, &old.controlOperations).unwrap();
    assert_eq!(space_departure::pending(&reopened, &a).unwrap(), receipt);
    services[0].observePeerSpaceSnapshot(&c, peerSpaceSnapshotForPeer(&reopened, &a).unwrap()).unwrap();
    let ack = services[0].peerSpaceSnapshot().unwrap();
    assert!(ack.controlOperations.contains(&receipt[0]));
    reopened.observePeerSpaceSnapshot(&a, ack).unwrap();
    assert!(space_departure::pending(&reopened, &a).unwrap().is_empty());
}

/// Even the last administrator can leave locally. The old Space is not given
/// a replacement administrator or silently migrated to another device.
#[tokio::test]
async fn departure_of_the_last_admin_does_not_promote_or_move_other_devices() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, services, _) = revocationMesh("admin-exit", 2, &[(0,1)]);
    let admin = routers[0].localNodeId();
    let other = routers[1].localNodeId();
    let original = routers[1].spaceStore.space().unwrap().spaceId;
    services[0].leaveDeviceSpace().unwrap();
    services[1].observePeerSpaceSnapshot(&admin, peerSpaceSnapshotForPeer(&services[0], &other).unwrap()).unwrap();
    let state = routers[1].networkControlStore.currentState().unwrap();
    assert_eq!(state.spaceId, original);
    assert_eq!(state.memberNodeIds, BTreeSet::from([other.clone()]));
    assert_eq!(state.deviceIdentityIds.get(&other).map(String::as_str), Some("user"));
}

/// Persisting the old-Space command and switching the local member record are
/// separate writes. Recovery finishes a crash between them exactly once.
#[tokio::test]
async fn departure_recovers_a_crash_before_the_local_space_switch() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, services, _) = revocationMesh("exit-interrupted", 2, &[(0,1)]);
    let a = routers[0].localNodeId();
    let original = services[1].deviceSpace().unwrap();
    let operation = routers[1].networkControlStore.leaveCurrentSpace().unwrap().unwrap();
    assert_eq!(services[1].deviceSpace().unwrap(), original);
    let reopened = RuntimeRemoteLinkService::newWithRouter((*routers[1].localCore).clone(), routers[1].clone());
    crate::NodeSpaceService::space_join::recoverLocalDeparture(&reopened).unwrap();
    let recovered = reopened.deviceSpace().unwrap();
    assert_ne!(recovered.spaceId, original.spaceId);
    assert_eq!(space_departure::pending(&reopened, &a).unwrap(), vec![operation]);
    crate::NodeSpaceService::space_join::recoverLocalDeparture(&reopened).unwrap();
    assert_eq!(reopened.deviceSpace().unwrap(), recovered);
}
