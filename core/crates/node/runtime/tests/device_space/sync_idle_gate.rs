use super::*;

/// Counts delivered calls for one synchronization method.
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
        "snapshotHeader",
        "snapshotControlPage",
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

/// Creates an observer over the fixture's authoritative stores.
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
    pendingFailure: Arc<std::sync::atomic::AtomicBool>,
}

impl GateTestApplicationClient {
    /// Reports a recoverable plugin failure independently of the control journal.
    fn gateErrors(&self) -> Vec<&'static str> {
        if self
            .pendingFailure
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            vec!["plugin database is unavailable"]
        } else {
            Vec::new()
        }
    }

    /// Returns exact receipt intervals for the gate fixture.
    fn gateClock(&self) -> Result<operit_store::SyncOperationStore::SyncReceipt, String> {
        let mut clock = operit_store::SyncOperationStore::SyncReceipt::default();
        for operation in self.control.currentSpaceOperations()? {
            clock.observe(&operation.originDeviceId, operation.sequence);
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
                    serde_json::json!({"ranges":self.gateClock()?.ranges, "protocolVersion":1, "pending":0, "pendingBlobs":[], "errors":self.gateErrors()}),
                )
                .map_err(|error| error.to_string()),
                "syncClockDigest" => operit_link::toCoreValue(serde_json::json!({
                    "protocolVersion": 1,
                    // The fixture models one remote process per router, so the
                    // digest follows the remote's own receipt. Production adds
                    // the peer-local mutation revision, which cannot be shared
                    // across two routers living in one test process.
                    "digest": format!(
                        "ranges={}#errors={:?}",
                        serde_json::to_string(&self.gateClock()?.ranges).unwrap_or_default(),
                        self.gateErrors(),
                    ),
                }))
                .map_err(|error| error.to_string()),
                "syncOperationsSince" => {
                    let args: serde_json::Value = operit_link::fromCoreValue(request.args)
                        .map_err(|error| error.to_string())?;
                    let clock: operit_store::SyncOperationStore::SyncReceipt =
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
                            !operit_store::SyncOperationStore::SyncOperationKnowledge::containsSequence(
                                &clock, &operation.originDeviceId, operation.sequence,
                            )
                        })
                        .take(limit)
                        .collect();
                    operit_link::toCoreValue(
                        serde_json::json!({"operations":operations, "errors":self.gateErrors(), "failedDomains":if self.gateErrors().is_empty() { vec![] } else { vec!["plugin_storage"] }}),
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
                        serde_json::json!({ "applied": applied, "pending":0, "errors":self.gateErrors() }),
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
            result: result.map_err(|error| {
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

/// Installs the control journal application used by fixtures that exchange replication calls.
pub(super) fn installControlSyncTestApplication(router: &mut CoreNodeRouter) {
    let control = router.networkControlStore.clone();
    router.installTestApplicationClient(Arc::new(GateTestApplicationClient {
        control,
        pendingFailure: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    }));
}

/// Builds a two-node authorized Space whose routers carry the minimal gate
/// application, so full passes run their business leg against real stores.
fn gateMesh(
    prefix: &str,
) -> (
    Vec<CoreNodeRouter>,
    Vec<RuntimeRemoteLinkService>,
    Vec<Arc<ApprovalMeshPeer>>,
    Arc<std::sync::atomic::AtomicBool>,
) {
    let services = (0..2)
        .map(|index| approvalService(&format!("{prefix}-{index}")))
        .collect::<Vec<_>>();
    let mut routers = services
        .iter()
        .map(|(router, _)| router.clone())
        .collect::<Vec<_>>();
    let pendingFailure = Arc::new(std::sync::atomic::AtomicBool::new(false));
    for (index, router) in routers.iter_mut().enumerate() {
        if index == 0 {
            let control = router.networkControlStore.clone();
            router.installTestApplicationClient(Arc::new(GateTestApplicationClient {
                control,
                pendingFailure: pendingFailure.clone(),
            }));
        } else {
            installControlSyncTestApplication(router);
        }
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
    (routers, access, peers, pendingFailure)
}

/// Verifies the change-gated synchronization floor: after one converging pass,
/// idle cycles only probe the remote clock, one remote authority change costs
/// exactly one more full pass, and the following cycles idle again.
#[tokio::test]
async fn idle_cycles_probe_clocks_without_repeating_full_exchanges() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers, _pendingFailure) = gateMesh("sync-idle");
    let observer = idleObserver(&routers[1]);

    // The first pass converges with the remote and seeds the clock baseline.
    // A bootstrap pass may still leave one detectable ripple on the remote
    // (replayed operations land through dedup), so one settling cycle absorbs
    // it before the idling baseline is taken.
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    let markersAfterFirstPass = fullPassMarkers(&peers[1]);
    let probesAfterFirstPass = synchronizedMethodCount(&peers[1], "syncClockDigest");
    let fullClockCallsAfterFirstPass = synchronizedMethodCount(&peers[1], "syncClock");
    assert!(
        markersAfterFirstPass > 0,
        "the first pass must run the full exchanges"
    );
    assert!(
        probesAfterFirstPass > 0,
        "the settled gate must probe with the compact digest"
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
        synchronizedMethodCount(&peers[1], "syncClockDigest") > probesAfterFirstPass,
        "idle cycles keep probing the remote clock"
    );
    assert_eq!(
        synchronizedMethodCount(&peers[1], "syncClock"),
        fullClockCallsAfterFirstPass,
        "idle probes must not move the complete per-entity clock"
    );

    // Reconstructing a facade must not construct a second worker or discard
    // the settled gate. Frequent small syncs stay silent at the proxy boundary.
    let reopened = idleObserver(&routers[1]);
    reopened.synchronizeOnce().await.unwrap();
    assert_eq!(fullPassMarkers(&peers[1]), markersAfterFirstPass);
    assert!(access[1].spaceSyncProgressFlow().value().is_empty());

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

    // Delaying a small incremental exchange does not make it substantial.
    // Both the delay and a real backlog are required for visible progress.
    let reopenedFacade = RuntimeRemoteLinkService::newWithRouter(
        (*routers[1].localCore).clone(), routers[1].clone());
    let progress = reopenedFacade.spaceSyncProgressFlow();
    access[0].updateDeviceSpacePolicy("sync.large.incremental".into(), "v1".into()).unwrap();
    let (arrived, resume) = peers[1].pauseResponse("syncApplyOperations");
    let (result, ()) = tokio::join!(observer.synchronizeOnce(), async {
        arrived.await.unwrap();
        assert!(progress.value().is_empty(), "a new incremental exchange starts silently");
        tokio::time::sleep(Duration::from_millis(3_100)).await;
        assert!(progress.value().is_empty(), "a slow small exchange stays silent");
        resume.send(()).unwrap();
    });
    result.unwrap();
    assert!(progress.value().is_empty(), "small completion stays silent too");
}

/// Verifies a failed pass arms the exponential backoff: the immediate retry is
/// suppressed, a pass whose backoff elapsed retries and converges, and the
/// Space returns to idling afterwards.
#[tokio::test]
async fn a_failed_pass_backs_off_before_retrying_the_full_exchange() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, _access, peers, _pendingFailure) = gateMesh("sync-backoff");
    let observer = idleObserver(&routers[1]);

    // One-shot fault: the authority page is lost during the member pass.
    // The cycle must fail and arm backoff without switching protocols.
    peers[1]
        .faults
        .lock()
        .unwrap()
        .insert("snapshotControlPage".into(), CallFault::BeforeDelivery);
    assert!(
        observer.synchronizeOnce().await.is_err(),
        "the failed authority page must fail the cycle"
    );
    let markersAfterFailure = fullPassMarkers(&peers[1]);
    assert_eq!(synchronizedMethodCount(&peers[1], "syncApplyOperations"), 0,
        "failed authority validation must stop business replication in the same cycle");

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

/// A lost clock probe must arm the failure backoff instead of forcing a full
/// journal export against a peer whose transport cannot answer probes.
#[tokio::test]
async fn a_lost_clock_probe_backs_off_instead_of_forcing_a_full_exchange() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, _access, peers, _pendingFailure) = gateMesh("sync-probe-loss");
    let observer = idleObserver(&routers[1]);
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    let markersBefore = fullPassMarkers(&peers[1]);

    // Every clock probe is lost: the cycles must idle. The old behavior fell
    // back to a full exchange per cycle, re-exporting the entire journal
    // against an unreachable peer indefinitely.
    for _ in 0..3 {
        {
            let mut faults = peers[1].faults.lock().unwrap();
            faults.insert("syncClockDigest".into(), CallFault::BeforeDelivery);
            faults.insert("syncClock".into(), CallFault::BeforeDelivery);
        }
        observer.synchronizeOnce().await.unwrap();
        assert_eq!(
            fullPassMarkers(&peers[1]),
            markersBefore,
            "a lost probe must not trigger a full exchange"
        );
    }

    // The armed backoff keeps idling even on probe-free cycles.
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(
        fullPassMarkers(&peers[1]),
        markersBefore,
        "the probe-failure backoff must suppress the retry"
    );

    // The probe recovers: once the backoff elapses, the gate reopens and
    // converges without further manual expiry.
    peers[1].faults.lock().unwrap().clear();
    observer.testExpirePeerSyncBackoff(&routers[0].localNodeId());
    observer.synchronizeOnce().await.unwrap();
    assert!(
        fullPassMarkers(&peers[1]) > markersBefore,
        "the recovered probe must reopen the gate"
    );
    let settled = fullPassMarkers(&peers[1]);
    peers[1]
        .faults
        .lock()
        .unwrap()
        .insert("syncClockDigest".into(), CallFault::BeforeDelivery);
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(
        fullPassMarkers(&peers[1]), settled,
        "the settled gate must stay settled"
    );
}

/// Verifies a rejected digest probe arms backoff without requesting a full clock.
#[tokio::test]
async fn rejected_digest_probes_back_off_without_reading_full_clocks() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, _access, peers, _pendingFailure) = gateMesh("sync-rejected-probe");
    let observer = idleObserver(&routers[1]);
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    let markersBefore = fullPassMarkers(&peers[1]);
    let fullClockCallsBefore = synchronizedMethodCount(&peers[1], "syncClock");

    peers[1].faults.lock().unwrap()
        .insert("syncClockDigest".into(), CallFault::MethodDenied);
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(fullPassMarkers(&peers[1]), markersBefore);
    assert_eq!(synchronizedMethodCount(&peers[1], "syncClock"), fullClockCallsBefore);

    let probesAfterFailure = synchronizedMethodCount(&peers[1], "syncClockDigest");
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(synchronizedMethodCount(&peers[1], "syncClockDigest"), probesAfterFailure,
        "the rejected probe must arm backoff");
    assert_eq!(fullPassMarkers(&peers[1]), markersBefore);

    observer.testExpirePeerSyncBackoff(&routers[0].localNodeId());
    observer.synchronizeOnce().await.unwrap();
    assert!(fullPassMarkers(&peers[1]) > markersBefore,
        "a recovered digest probe must reopen the gate after backoff");
}

/// A preference-only edit must reopen a settled storage-provider gate without
/// creating business operations, then return to idling after convergence.
#[tokio::test]
async fn preference_edits_reopen_a_settled_storage_provider_gate() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers, _pendingFailure) = gateMesh("sync-device-preferences");
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
                pendingFailure: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
                control: router.networkControlStore.clone(),
                pendingFailure: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }
            .gateClock()
            .unwrap(),
            before
        );
    }
}

/// Continues healthy replication and retries partial business failures without arming transport backoff.
#[tokio::test]
async fn partial_plugin_failure_keeps_healthy_replication_moving() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers, pendingFailure) = gateMesh("sync-partial-plugin");
    let observer = idleObserver(&routers[1]);
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    pendingFailure.store(true, std::sync::atomic::Ordering::SeqCst);
    let before = routers[1]
        .networkControlStore
        .currentSpaceOperations()
        .unwrap()
        .len();
    access[0]
        .updateDeviceSpacePolicy("sync.partial.plugin".into(), "v1".into())
        .unwrap();
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(
        routers[1]
            .networkControlStore
            .currentSpaceOperations()
            .unwrap()
            .len(),
        before + 1
    );
    let markers = fullPassMarkers(&peers[1]);
    observer.synchronizeOnce().await.unwrap();
    assert!(
        fullPassMarkers(&peers[1]) > markers,
        "partial failures must remain eligible for retry"
    );
    access[0]
        .updateDeviceSpacePolicy("sync.partial.plugin".into(), "v2".into())
        .unwrap();
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(
        routers[1]
            .networkControlStore
            .currentSpaceOperations()
            .unwrap()
            .len(),
        before + 2
    );
    pendingFailure.store(false, std::sync::atomic::Ordering::SeqCst);
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    let settled = fullPassMarkers(&peers[1]);
    observer.synchronizeOnce().await.unwrap();
    assert_eq!(fullPassMarkers(&peers[1]), settled);
}

/// Verifies progress this device only received from peers does not reopen a
/// settled gate: the local-state summary counts work this device still owes
/// its peers, not every origin whose ranges advanced when remote operations
/// applied here.
#[tokio::test]
async fn applied_remote_progress_does_not_reopen_a_settled_gate() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, _access, peers, _pendingFailure) = gateMesh("sync-remote-progress");
    let observer = idleObserver(&routers[1]);
    observer.synchronizeOnce().await.unwrap();
    observer.synchronizeOnce().await.unwrap();
    let settled = fullPassMarkers(&peers[1]);

    // Model an applied peer operation: the clock gains a remote origin this
    // device never authored anything under.
    let store = operit_store::SyncOperationStore::SyncOperationStore::new(
        routers[1].localCore.runtimeStorageHost(),
        operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH.to_string(),
    );
    let mut clock = store.currentLocalClock().unwrap();
    clock.setSequence("core-remote-applied-peer", 9);
    store.writeLocalClock(&clock).unwrap();

    observer.synchronizeOnce().await.unwrap();
    assert_eq!(
        fullPassMarkers(&peers[1]),
        settled,
        "received progress must not start a pass this device owes nothing to"
    );
}
