use super::*;
use crate::NodeSpaceService::space_reconcile;
use crate::NodeSpaceService::NodeSpaceContext;
use operit_link::protocol::{fromCoreValue, toCoreValue};
use crate::PeerStateStore::PeerStateStore;
use operit_store::SyncOperationStore::{SyncClock, SyncOperation};

const CAPS_PATH: &str = "runtime/link_access/space_reconcile_caps.preferences.json";

/// Two nodes sharing one Space identity with separate journals and a mesh link.
fn reconcilablePair(tag: &str) -> ((CoreNodeRouter, RuntimeRemoteLinkService), (CoreNodeRouter, RuntimeRemoteLinkService)) {
    let (aRouter, aService) = approvalService(&format!("{tag}-a"));
    let (bRouter, bService) = approvalService(&format!("{tag}-b"));
    let profiles: Vec<_> = [&aRouter, &bRouter]
        .iter()
        .flat_map(|router| router.spaceStore.deviceProfilesForCurrentSpace().unwrap())
        .collect();
    aRouter.networkControlStore.admitMember(bRouter.localNodeId()).unwrap();
    let mut space = aRouter.spaceStore.initialize().unwrap();
    space.members.push(bRouter.localNodeId());
    space.members.sort();
    space.spaceRevision += 1;
    aRouter.spaceStore.adopt(space.clone()).unwrap();
    bRouter.spaceStore.adopt(space).unwrap();
    for operation in aRouter.networkControlStore.currentSpaceOperations().unwrap() {
        bRouter.networkControlStore.applyBootstrapOperation(&operation).unwrap();
    }
    for router in [&aRouter, &bRouter] {
        router.spaceStore.importDeviceProfiles(profiles.clone()).unwrap();
    }
    let peers: Vec<Arc<ApprovalMeshPeer>> = [&aRouter, &bRouter]
        .iter()
        .map(|router| {
            let peer = ApprovalMeshPeer::new(router.localNodeId());
            peer.link(router);
            peer
        })
        .collect();
    peers[0].link(&bRouter);
    peers[1].link(&aRouter);
    for (router, peer) in [&aRouter, &bRouter].iter().zip(&peers) {
        router.installNodeServices(NodeServices::new(peer.clone())).unwrap();
    }
    ((aRouter, aService), (bRouter, bService))
}

fn commandSet(router: &CoreNodeRouter) -> std::collections::BTreeSet<String> {
    router
        .networkControlStore
        .currentSpaceOperations()
        .unwrap()
        .into_iter()
        .map(|operation| operation.opId)
        .collect()
}

/// A known-v2 peer with more history than one page converges over multiple
/// rounds and records the exchange in the doctor counters.
#[tokio::test]
async fn paged_reconciliation_converges_over_multiple_rounds() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let ((aRouter, aService), (bRouter, _)) = reconcilablePair("page");
    // Seed the capability cache so the initiator pages immediately.
    PeerStateStore::new(aService.storage())
        .putRecord(CAPS_PATH, &bRouter.localNodeId(), &2u32)
        .unwrap();
    // One page is 6KB; ~120 distinct policy commands need several pages.
    for index in 0..120 {
        aRouter
            .networkControlStore
            .updatePolicy(format!("paging-policy-{index}"), format!("v{index}"))
            .unwrap();
    }
    let aBefore = commandSet(&aRouter);
    let result = space_reconcile::reconcile(&aService, &bRouter.localNodeId()).await.unwrap();
    assert!(result.converged, "paged exchange must terminate as converged");
    assert_eq!(commandSet(&aRouter), aBefore, "initiator must not lose history");
    assert_eq!(commandSet(&aRouter), commandSet(&bRouter), "replicas must hold the same commands");
    let counters = crate::NodeSpaceService::space_doctor::readCounters(&aService);
    assert!(counters.reconcileExchanges >= 1);
    assert!(counters.reconcileRounds >= 2, "120 commands over 6KB pages need multiple rounds");
}

/// First contact without a capability cache still converges (the v1-shaped
/// full offer discovers the v2 gateway and continues incrementally).
#[tokio::test]
async fn first_contact_full_offer_discovers_v2_and_converges() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let ((aRouter, aService), (bRouter, bService)) = reconcilablePair("first");
    for index in 0..10 {
        aRouter
            .networkControlStore
            .updatePolicy(format!("first-policy-{index}"), "v1".into())
            .unwrap();
    }
    let result = space_reconcile::reconcile(&aService, &bRouter.localNodeId()).await.unwrap();
    assert!(result.converged);
    assert_eq!(commandSet(&aRouter), commandSet(&bRouter));
    // The capability cache now remembers the peer speaks v2.
    let caps = PeerStateStore::new(aService.storage()).records::<u32>(CAPS_PATH).unwrap();
    assert_eq!(caps.get(&bRouter.localNodeId()), Some(&2));
    let _ = bService; // keep both services alive through the assertions
}

/// A v1-shaped offer (no protocol fields) gets today's full-list reply and no
/// paging metadata confusion.
#[tokio::test]
async fn legacy_offer_receives_full_reply() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let ((aRouter, _), (bRouter, bService)) = reconcilablePair("legacy");
    // History from either side ends up in the gateway's merged reply.
    for index in 0..5 {
        aRouter
            .networkControlStore
            .updatePolicy(format!("legacy-policy-{index}"), "v1".into())
            .unwrap();
    }
    // Exactly the fields a v1 binary serializes: nothing else.
    let offer = serde_json::json!({
        "spaceId": aRouter.spaceStore.initialize().unwrap().spaceId,
        "operations": [],
        "deviceProfiles": [],
    });
    let request = CoreCallRequest::new(
        "legacy-test".to_string(),
        NODE_SPACE_TARGET,
        "reconcileSharedSpace",
        toCoreValue(&offer).unwrap(),
    );
    let reply = space_reconcile::receive(&bService, &aRouter.localNodeId(), request).unwrap();
    let outcome: serde_json::Value = fromCoreValue(reply).unwrap();
    // The v1 reply carries the merged log and members; paging fields stay unset.
    assert!(outcome["more"].is_null());
    assert!(outcome["haveOpIds"].is_null());
    let operations = outcome["operations"].as_array().unwrap();
    // B's merged view holds every command it knows (bootstrap + admission).
    assert!(!operations.is_empty(), "legacy reply carries the full merged list");
}

/// Offers exceeding the operation ceiling are refused whole and counted.
#[tokio::test]
async fn oversize_offer_is_refused_and_counted() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let ((aRouter, _), (bRouter, bService)) = reconcilablePair("oversize");
    let mut flood: Vec<SyncOperation> = Vec::new();
    for index in 0..5000i64 {
        flood.push(SyncOperation {
            opId: format!("flood:{index}"),
            originDeviceId: aRouter.localNodeId(),
            sequence: index,
            domain: "network_control".into(),
            entityType: "command".into(),
            entityId: format!("flood-{index}"),
            operation: "apply".into(),
            semantics: operit_store::SyncOperationStore::SyncOperationSemantics::Transaction,
            payload: serde_json::json!({}),
            createdAt: index,
            schemaVersion: 1,
        });
    }
    let before = crate::NodeSpaceService::space_doctor::readCounters(&bService)
        .oversizeOffersRefused;
    let offer = serde_json::json!({
        "spaceId": bRouter.spaceStore.initialize().unwrap().spaceId,
        "operations": flood,
        "deviceProfiles": [],
    });
    let request = CoreCallRequest::new(
        "oversize-test".to_string(),
        NODE_SPACE_TARGET,
        "reconcileSharedSpace",
        toCoreValue(&offer).unwrap(),
    );
    let error = space_reconcile::receive(&bService, &aRouter.localNodeId(), request).unwrap_err();
    assert!(error.contains("ceiling"), "unexpected error: {error}");
    let after = crate::NodeSpaceService::space_doctor::readCounters(&bService).oversizeOffersRefused;
    assert_eq!(after, before + 1);
    let _ = SyncClock::empty();
}
