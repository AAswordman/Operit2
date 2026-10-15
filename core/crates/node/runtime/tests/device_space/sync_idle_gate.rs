use super::*;

fn synchronizedMethodCount(peer: &ApprovalMeshPeer, method: &str) -> usize {
    peer.calls
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, name)| name == method)
        .count()
}

/// Every RPC that only a full synchronization pass issues. Cheap clock probes
/// are excluded: idle cycles keep probing, but must not repeat any exchange.
fn fullPassMarkers(peer: &ApprovalMeshPeer) -> usize {
    [
        "snapshot",
        "observeSpaceSnapshot",
        "exchange",
        "coreVersion",
        "deviceSpace",
        "syncOperationsSince",
        "syncApplyOperations",
    ]
    .iter()
    .map(|method| synchronizedMethodCount(peer, method))
    .sum()
}

fn idleObserver(
    router: &CoreNodeRouter,
) -> crate::SpacePersistenceSyncService::SpacePersistenceSyncService {
    crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
        router.localCore.clone(),
        router.clone(),
        router.spaceStore.clone(),
    )
}

/// A minimal application for gate tests: serves the synchronization methods
/// from the node's real Space stores so both legs of a business pass run
/// against genuine state, while everything else stays unsupported.
struct GateTestApplicationClient {
    control: operit_store::NetworkControlStore::NetworkControlStore,
}

impl GateTestApplicationClient {
    fn gateClock(
        &self,
    ) -> Result<operit_store::SyncOperationStore::SyncClock, String> {
        let mut clock = operit_store::SyncOperationStore::SyncClock::empty();
        for operation in self.control.currentSpaceOperations()? {
            if operation.sequence > clock.sequenceFor(&operation.originDeviceId) {
                clock.setSequence(operation.originDeviceId, operation.sequence);
            }
        }
        Ok(clock)
    }
}

#[async_trait(?Send)]
impl operit_link::CoreLinkSharedClient for GateTestApplicationClient {
    async fn call(&self, request: operit_link::CoreCallRequest) -> operit_link::CoreCallResponse {
        let result = (|| -> Result<operit_link::CoreValue, String> {
            match request.methodName.as_str() {
                "coreVersion" => operit_link::toCoreValue(
                    serde_json::json!(env!("CARGO_PKG_VERSION")),
                )
                .map_err(|error| error.to_string()),
                "syncClock" => operit_link::toCoreValue(
                    serde_json::to_value(self.gateClock()?).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string()),
                "syncOperationsSince" => {
                    let args: serde_json::Value = operit_link::fromCoreValue(request.args)
                        .map_err(|error| error.to_string())?;
                    let clock: operit_store::SyncOperationStore::SyncClock =
                        serde_json::from_value(args.get("clock").cloned().unwrap_or_default())
                            .map_err(|error| error.to_string())?;
                    let limit = args
                        .get("limit")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(512) as usize;
                    let operations: Vec<
                        operit_store::SyncOperationStore::SyncOperation,
                    > = self
                        .control
                        .currentSpaceOperations()?
                        .into_iter()
                        .filter(|operation| {
                            operation.sequence
                                > clock
                                    .sequences
                                    .get(&operation.originDeviceId)
                                    .copied()
                                    .unwrap_or(0)
                        })
                        .take(limit)
                        .collect();
                    operit_link::toCoreValue(
                        serde_json::to_value(operations).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())
                }
                "syncApplyOperations" => {
                    // The generated dispatch passes the named `operations`
                    // parameter, exactly like the real application receives it.
                    let args: serde_json::Value = operit_link::fromCoreValue(request.args)
                        .map_err(|error| error.to_string())?;
                    let parameter = args.get("operations").cloned().unwrap_or_default();
                    let forceApply = parameter
                        .get("forceApply")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    let operationsValue = if forceApply {
                        parameter
                            .get("operations")
                            .cloned()
                            .ok_or_else(|| "bootstrap sync request is missing operations".to_string())?
                    } else {
                        parameter
                    };
                    let operations: Vec<
                        operit_store::SyncOperationStore::SyncOperation,
                    > = serde_json::from_value(operationsValue).map_err(|error| {
                        format!(
                            "operations shape {}: {error}",
                            serde_json::to_string(&args).unwrap_or_default()
                        )
                    })?;
                    let mut applied = 0_usize;
                    for operation in &operations {
                        self.control.applySyncedOperation(operation)?;
                        applied += 1;
                    }
                    operit_link::toCoreValue(
                        serde_json::json!({ "applied": applied }),
                    )
                    .map_err(|error| error.to_string())
                }
                other => Err(format!(
                    "gate test application does not implement {other}"
                )),
            }
        })();
        let servedMethod = request.methodName.clone();
        operit_link::CoreCallResponse {
            requestId: request.requestId,
            result: result
                .map_err(|error| {
                    operit_link::CoreLinkError::internal(format!("{servedMethod}: {error}"))
                }),
        }
    }

    async fn watchSnapshot(
        &self,
        _request: operit_link::CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, operit_link::CoreLinkError> {
        Err(operit_link::CoreLinkError::internal(
            "gate test application has no watch",
        ))
    }

    async fn watch(
        &self,
        _request: operit_link::CoreWatchRequest,
    ) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {
        Err(operit_link::CoreLinkError::internal(
            "gate test application has no watch",
        ))
    }

    /// Rejects inputs because the synchronization gate fixture serves only calls.
    async fn openPush(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn operit_link::CoreLinkPushSession>, operit_link::CoreLinkError> {
        Err(operit_link::CoreLinkError::methodNotFound(&request.methodName))
    }
}

/// Builds a two-node authorized Space whose routers carry the minimal gate
/// application, so full passes run their business leg against real stores.
fn gateMesh(
    prefix: &str,
) -> (
    Vec<CoreNodeRouter>,
    Vec<RuntimeRemoteLinkService>,
    Vec<Arc<ApprovalMeshPeer>>,
) {
    let services = (0..2)
        .map(|index| approvalService(&format!("{prefix}-{index}")))
        .collect::<Vec<_>>();
    let mut routers = services
        .iter()
        .map(|(router, _)| router.clone())
        .collect::<Vec<_>>();
    for router in routers.iter_mut() {
        let control = router.networkControlStore.clone();
        router.installTestApplicationClient(Arc::new(GateTestApplicationClient { control }));
    }
    let access = services
        .iter()
        .map(|(_, service)| service.clone())
        .collect::<Vec<_>>();
    mergedChainFixture(&routers.iter().collect::<Vec<_>>());
    let peers = routers
        .iter()
        .map(|router| ApprovalMeshPeer::new(router.localNodeId()))
        .collect::<Vec<_>>();
    for (a, b) in [(0_usize, 1_usize)] {
        peers[a].link(&routers[b]);
        peers[b].link(&routers[a]);
    }
    for (router, peer) in routers.iter().zip(&peers) {
        router
            .installNodeServices(NodeServices::new(peer.clone()))
            .unwrap();
    }
    publishRevocationFixtureAdjacency(&routers, &peers);
    (routers, access, peers)
}

/// The joining member needs a custom storage capability role: clock probes are
/// storage-replication calls, and the idle gate only skips passes whose probe
/// succeeds. The owning administrator already satisfies the check through its
/// wildcard capability, and reassigning it would drop the administrator.
fn grantStorageRole(routers: &[CoreNodeRouter], access: &[RuntimeRemoteLinkService]) {
    access[0].defineDeviceSpaceRole(operit_store::NetworkControlStore::NetworkControlRole {
        roleId: "replica".into(), displayName: "Test replica".into(),
        capabilities: std::collections::BTreeSet::from(["storage.provide".into()]),
    }).unwrap();
    access[0]
        .setDeviceSpaceIdentity(
            operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
                nodeId: routers[1].localNodeId(),
                roleId: "replica".into(),
            },
        )
        .unwrap();
}

/// Verifies the change-gated synchronization floor: after one converging pass,
/// idle cycles only probe the remote clock, one remote authority change costs
/// exactly one more full pass, and the following cycles idle again.
#[tokio::test]
async fn idle_cycles_probe_clocks_without_repeating_full_exchanges() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = gateMesh("sync-idle");
    grantStorageRole(&routers, &access);
    let observer = idleObserver(&routers[1]);

    // The first pass converges with the remote and seeds the clock baseline.
    // A bootstrap pass may still leave one detectable ripple on the remote
    // (replayed operations land through dedup), so one settling cycle absorbs
    // it before the idling baseline is taken.
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    let markersAfterFirstPass = fullPassMarkers(&peers[1]);
    let probesAfterFirstPass = synchronizedMethodCount(&peers[1], "syncClock");
    assert!(
        markersAfterFirstPass > 0,
        "the first pass must run the full exchanges"
    );

    // Idle cycles only probe: no full-exchange RPC is repeated.
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(
        fullPassMarkers(&peers[1]),
        markersAfterFirstPass,
        "idle cycles must not repeat full-exchange calls"
    );
    assert!(
        synchronizedMethodCount(&peers[1], "syncClock") > probesAfterFirstPass,
        "idle cycles keep probing the remote clock"
    );

    // A remote authority change moves the remote clock: the next cycle runs a
    // full pass and the operation is applied locally.
    let operationsBefore = routers[1]
        .networkControlStore
        .currentSpaceOperations()
        .unwrap()
        .len();
    access[0]
        .updateDeviceSpacePolicy("sync.idle.probe".into(), "v2".into())
        .unwrap();
    observer.synchronizeOnce().await.unwrap();
    assert!(
        fullPassMarkers(&peers[1]) > markersAfterFirstPass,
        "a detected remote change must run a full pass"
    );
    assert_eq!(
        routers[1]
            .networkControlStore
            .currentSpaceOperations()
            .unwrap()
            .len(),
        operationsBefore + 1,
        "the changed authority operation must arrive exactly once"
    );

    // And the Space returns to idling afterwards.
    let markersAfterChangePass = fullPassMarkers(&peers[1]);
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(fullPassMarkers(&peers[1]), markersAfterChangePass);
}

/// Verifies a failed pass arms the exponential backoff: the immediate retry is
/// suppressed, a pass whose backoff elapsed retries and converges, and the
/// Space returns to idling afterwards.
#[tokio::test]
async fn a_failed_pass_backs_off_before_retrying_the_full_exchange() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = gateMesh("sync-backoff");
    grantStorageRole(&routers, &access);
    let observer = idleObserver(&routers[1]);

    // One-shot fault: the control exchange with the remote is lost. The
    // projection snapshot still succeeds, so the failure comes from the
    // member synchronization leg, which is what arms the backoff.
    peers[1]
        .faults
        .lock()
        .unwrap()
        .insert("exchange".into(), CallFault::BeforeDelivery);
    assert!(
        observer.synchronizeOnce().await.is_err(),
        "the failed control exchange must fail the cycle"
    );
    let markersAfterFailure = fullPassMarkers(&peers[1]);

    // The immediate retry is suppressed by the failure backoff.
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(
        fullPassMarkers(&peers[1]),
        markersAfterFailure,
        "the backoff must suppress the next full pass"
    );

    // Once the backoff elapses the pass retries and converges; one settling
    // cycle absorbs the bootstrap ripple before the idling baseline.
    observer.testExpirePeerSyncBackoff(&routers[0].localNodeId());
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    assert!(
        fullPassMarkers(&peers[1]) > markersAfterFailure,
        "the expired backoff must allow the retry"
    );

    // Converged again: cycles idle.
    let markersAfterRetry = fullPassMarkers(&peers[1]);
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(fullPassMarkers(&peers[1]), markersAfterRetry);
}

/// A preference-only edit must reopen a settled storage-provider gate without
/// creating business operations, then return to idling after convergence.
#[tokio::test]
async fn preference_edits_reopen_a_settled_storage_provider_gate() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = gateMesh("sync-device-preferences");
    grantStorageRole(&routers, &access);
    let observer = idleObserver(&routers[1]);
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    let markersBefore = fullPassMarkers(&peers[1]);
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(fullPassMarkers(&peers[1]), markersBefore);

    let space = routers[1].spaceStore.space().unwrap();
    let target = routers[0].localNodeId();
    let source = operit_store::DevicePreferencesStore::DevicePreferencesStore::new(
        routers[1].localCore.runtimeStorageHost(),
    );
    let destination = operit_store::DevicePreferencesStore::DevicePreferencesStore::new(
        routers[0].localCore.runtimeStorageHost(),
    );
    let clocksBefore: Vec<_> = routers
        .iter()
        .map(|router| {
            GateTestApplicationClient {
                control: router.networkControlStore.clone(),
            }
            .gateClock()
            .unwrap()
        })
        .collect();
    let notifications = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = notifications.clone();
    let callbackSource = source.clone();
    let callbackSpace = space.spaceId.clone();
    let callbackTarget = target.clone();
    let _subscription = operit_store::SyncOperationStore::subscribeSyncMutations(move || {
        // A subscriber may immediately read back the committed preferences.
        callbackSource
            .read(&callbackSpace, &callbackTarget)
            .unwrap();
        count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });
    access[1]
        .setDevicePreference(target.clone(), "volume".into(), Some(serde_json::json!(42)))
        .unwrap();
    assert!(notifications.load(std::sync::atomic::Ordering::Relaxed) > 0);
    observer.synchronizeOnce().await.unwrap();
    assert!(fullPassMarkers(&peers[1]) > markersBefore);
    assert_eq!(
        destination.read(&space.spaceId, &target).unwrap().values()["volume"],
        serde_json::json!(42)
    );
    let notificationsAfterSync = notifications.load(std::sync::atomic::Ordering::Relaxed);
    let state = source.read(&space.spaceId, &target).unwrap();
    source.merge(&state).unwrap();
    assert_eq!(
        notifications.load(std::sync::atomic::Ordering::Relaxed),
        notificationsAfterSync,
        "idempotent preference exchange must not keep rescheduling synchronization"
    );
    let markersAfterSync = fullPassMarkers(&peers[1]);
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(fullPassMarkers(&peers[1]), markersAfterSync);
    let local = routers[1].localNodeId();
    access[1]
        .setDevicePreference(local.clone(), "volume".into(), Some(serde_json::json!(7)))
        .unwrap();
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(
        destination.read(&space.spaceId, &local).unwrap().values()["volume"],
        serde_json::json!(7),
        "a local device setting must propagate through the same settled gate"
    );
    let markersAfterLocalEdit = fullPassMarkers(&peers[1]);
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(fullPassMarkers(&peers[1]), markersAfterLocalEdit);
    for (router, before) in routers.iter().zip(clocksBefore) {
        assert_eq!(
            GateTestApplicationClient {
                control: router.networkControlStore.clone()
            }
            .gateClock()
            .unwrap(),
            before
        );
    }
}
