use super::*;
use crate::NodeSpaceService::space_reconcile;
use operit_store::NetworkControlStore::{NetworkControlIdentityAssignment, NetworkControlState};
use operit_store::SyncOperationStore::SyncOperation;
use std::collections::BTreeMap;

/// Deterministic xorshift generator: the property suite must be reproducible
/// from its seeds, not randomly flaky.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// Full local view: the control command set plus the replayed policy state.
fn commandSnapshot(router: &CoreNodeRouter) -> (BTreeMap<String, SyncOperation>, NetworkControlState) {
    let operations = router.networkControlStore.currentSpaceOperations().unwrap();
    let state = router.networkControlStore.currentState().unwrap();
    (
        operations.into_iter().map(|operation| (operation.opId.clone(), operation)).collect(),
        state,
    )
}

/// Two nodes sharing one Space identity with diverging journals, linked by a
/// mesh so the reconciliation wire calls route between real routers.
fn divergentPair(seed: u64, rng: &mut Rng) -> ((CoreNodeRouter, RuntimeRemoteLinkService), (CoreNodeRouter, RuntimeRemoteLinkService)) {
    let (aRouter, aService) = approvalService(&format!("conv-a-{seed}"));
    let (bRouter, bService) = approvalService(&format!("conv-b-{seed}"));
    // One shared identity: a admits b through real policy, both sides adopt
    // the aligned member records, then b bootstraps a's control log.
    // Collect each side's own profile first: once the member records merge,
    // the counterpart's profile must already be importable.
    let profiles: Vec<_> = [&aRouter, &bRouter]
        .iter()
        .flat_map(|router| router.spaceStore.deviceProfilesForCurrentSpace().unwrap())
        .collect();
    aRouter.networkControlStore.admitMember(bRouter.localNodeId()).unwrap();
    let mut space = aRouter.spaceStore.initialize().unwrap();
    space.members.push(bRouter.localNodeId());
    space.members.sort();
    // Adopting a different identity requires a strictly higher revision.
    space.spaceRevision += 1;
    aRouter.spaceStore.adopt(space.clone()).unwrap();
    bRouter.spaceStore.adopt(space).unwrap();
    let operations = aRouter.networkControlStore.currentSpaceOperations().unwrap();
    for operation in &operations {
        bRouter.networkControlStore.applyBootstrapOperation(operation).unwrap();
    }
    for router in [&aRouter, &bRouter] {
        router.spaceStore.importDeviceProfiles(profiles.clone()).unwrap();
    }

    // Divergence: each side issues its own random control commands over the
    // shared policy. Validation failures are ignored: only appended commands
    // are part of the history the invariant reasons about.
    let subjects = ["conv-p1", "conv-p2", "conv-p3"];
    let commands = 4 + rng.below(10);
    for _ in 0..commands {
        let router = if rng.below(2) == 0 { &aRouter } else { &bRouter };
        let subject = subjects[rng.below(subjects.len() as u64) as usize].to_string();
        let outcome = match rng.below(5) {
            0 => router.networkControlStore.admitMember(subject.clone()),
            1 => router.networkControlStore.removeMember(subject.clone()),
            2 => router.networkControlStore.disconnectNode(subject.clone()),
            3 => router.networkControlStore.setIdentity(NetworkControlIdentityAssignment {
                nodeId: subject.clone(),
                roleId: "admin".into(),
            }),
            _ => router.networkControlStore.updatePolicy(
                format!("conv-policy-{}", rng.below(3)),
                format!("v{}", rng.below(5)),
            ),
        };
        let _ = outcome;
    }

    // Arbitrary partial overlap before reconciliation: each side has already
    // merged a random subset of the other's operations, in random order.
    for (mine, theirs) in [(&aRouter, &bRouter), (&bRouter, &aRouter)] {
        let mut foreign: Vec<SyncOperation> = theirs
            .networkControlStore
            .currentSpaceOperations()
            .unwrap()
            .into_iter()
            .filter(|operation| operation.originDeviceId == theirs.localNodeId())
            .collect();
        foreign.reverse();
        for operation in foreign {
            if rng.below(2) == 0 {
                let _ = mine.networkControlStore.applySyncedOperation(&operation);
            }
        }
    }

    let peers: Vec<Arc<ApprovalMeshPeer>> = [&aRouter, &bRouter]
        .iter()
        .map(|router| {
            let peer = ApprovalMeshPeer::new(router.localNodeId());
            peer.link(router);
            peer
        })
        .collect();
    // Cross-link so each side's pairedPeers (and routing) reaches the other.
    peers[0].link(&bRouter);
    peers[1].link(&aRouter);
    for (router, peer) in [&aRouter, &bRouter].iter().zip(&peers) {
        router.installNodeServices(NodeServices::new(peer.clone())).unwrap();
    }
    ((aRouter, aService), (bRouter, bService))
}

/// P1 convergence and P2 monotonicity over random histories, arrival orders
/// and split states, with both nodes taking the initiator role.
#[tokio::test]
async fn pairwise_reconciliation_converges_for_random_histories() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    for seed in 0..16u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1));
        let ((aRouter, aService), (bRouter, bService)) = divergentPair(seed, &mut rng);
        let (aBefore, bBefore) = (commandSnapshot(&aRouter), commandSnapshot(&bRouter));
        let alreadyConverged = aBefore == bBefore;

        let result = if seed % 2 == 0 {
            space_reconcile::reconcile(&aService, &bRouter.localNodeId()).await.unwrap()
        } else {
            space_reconcile::reconcile(&bService, &aRouter.localNodeId()).await.unwrap()
        };
        let (aAfter, bAfter) = (commandSnapshot(&aRouter), commandSnapshot(&bRouter));

        // P1: both replicas replay one identical merged log into one state.
        assert_eq!(aAfter, bAfter, "seed {seed}: replicas diverge after reconciliation");
        assert!(
            aAfter.1.memberNodeIds == result.members.iter().cloned().collect::<BTreeSet<_>>(),
            "seed {seed}: reported membership disagrees with replayed policy"
        );
        if !alreadyConverged {
            assert!(!aAfter.0.is_empty(), "seed {seed}: merged log lost every command");
        }

        // P2: merging only adds accepted history; no pre-existing command
        // disappears from either replica.
        let mut expectedUnion: std::collections::BTreeSet<String> = aBefore.0.keys().cloned().collect();
        expectedUnion.extend(bBefore.0.keys().cloned());
        for opId in &expectedUnion {
            assert!(aAfter.0.contains_key(opId), "seed {seed}: replica a lost {opId}");
            assert!(bAfter.0.contains_key(opId), "seed {seed}: replica b lost {opId}");
        }
    }
}
