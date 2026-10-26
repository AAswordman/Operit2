#[derive(Default)]
struct RecordingDeviceToolService {
    requests: StdMutex<Vec<(String, CoreWatchRequest)>>,
    closed: Arc<AtomicBool>,
}
impl DeviceToolService for RecordingDeviceToolService {
    fn open(
        &self,
        request: CoreWatchRequest,
        origin: String,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.requests
            .lock()
            .unwrap()
            .push((origin, request.clone()));
        let (_sender, stream) = CoreEventStream::channel();
        let closed = self.closed.clone();
        Ok(stream.withOnClose(move || {
            closed.store(true, Ordering::SeqCst);
        }))
    }
}
fn deviceToolWatch(id: &str) -> CoreWatchRequest {
    CoreWatchRequest {
        requestId: operit_link::CoreRequestId(id.into()),
        target: DEVICE_TOOL_TARGET.into(),
        propertyName: "execute".into(),
        args: operit_link::toCoreValue(serde_json::json!({
            "tool_type":"default", "tool_name":"read_file_full", "parameters":{"path":"/target.txt"}
        }))
        .unwrap(),
    }
}

#[tokio::test]
async fn device_tools_route_between_real_routers_preserves_origin_and_closes_stream() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let joined = CoreSpace {
        spaceId: "device-tools-space".into(),
        spaceName: "Device tools".into(),
        spaceRevision: 2,
        members: vec!["device-a".into(), "device-b".into()],
    };
    let (source, _) = testCoreNodeRouterInJoinedSpace(
        "device-a",
        "device-b",
        "device-binding",
        "device-b",
        joined.clone(),
    );
    let (target, _) = testCoreNodeRouterInJoinedSpace(
        "device-b",
        "device-a",
        "device-binding",
        "device-b",
        joined,
    );
    let service = Arc::new(RecordingDeviceToolService::default());
    target.bindDeviceToolService(service.clone()).unwrap();
    let link = installTestPeer(
        &source,
        "device-b".into(),
        TestCoreNodeRouterEndpoint::new(target),
    )
    .unwrap();
    let stream = source
        .watchNode("device-b".into(), deviceToolWatch("device-route"))
        .await
        .unwrap();
    let recorded = service.requests.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].0, "device-a");
    assert_eq!(recorded[0].1, deviceToolWatch("device-route"));
    drop(recorded);
    drop(stream);
    assert!(service.closed.load(Ordering::SeqCst));
    link.close();
}

#[tokio::test]
async fn device_tools_accept_non_executor_members_and_reject_binding_route_bypasses() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let router = testCoreNodeRouter("tools-target", "tools-origin", "tools-binding");
    let service = Arc::new(RecordingDeviceToolService::default());
    router.bindDeviceToolService(service.clone()).unwrap();
    router
        .networkControlStore
        .setIdentity(
            operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
                nodeId: "tools-origin".into(),
                roleId: "relay".into(),
            },
        )
        .unwrap();
    let stream = router.openDeviceToolWatch(
        deviceToolWatch("member"), "tools-origin".into(), RoutedCoreRequestKind::Target,
    ).expect("a relay member may send device requests without hosting Core execution");
    drop(stream);
    let error = router.openDeviceToolWatch(
        deviceToolWatch("stranger"), "stranger".into(), RoutedCoreRequestKind::Target,
    ).err().unwrap();
    assert_eq!(error.code, "ORIGIN_CORE_NODE_NOT_IN_SPACE");
    router.networkControlStore.disconnectNode("tools-origin".into()).unwrap();
    let error = router.openDeviceToolWatch(
        deviceToolWatch("revoked"), "tools-origin".into(), RoutedCoreRequestKind::Target,
    ).err().unwrap();
    assert_eq!(error.code, "ORIGIN_CORE_NODE_REVOKED");
    let error = router
        .openDeviceToolWatch(
            deviceToolWatch("binding"),
            "tools-target".into(),
            RoutedCoreRequestKind::SpaceBinding,
        )
        .err()
        .unwrap();
    assert_eq!(error.code, "DEVICE_TOOL_ROUTE_INVALID");
    let response = router
        .callNode(
            "tools-origin".into(),
            CoreCallRequest::new("unknown", DEVICE_TOOL_TARGET, "execute", CoreValue::Null),
        )
        .await;
    assert!(response.result.is_err());
    let response = router
        .executeLocalCall(CoreCallRequest::new(
            "call-bypass",
            DEVICE_TOOL_TARGET,
            "execute",
            CoreValue::Null,
        ))
        .await;
    assert_eq!(
        response.result.unwrap_err().code,
        "DEVICE_TOOL_MODE_INVALID"
    );
    assert_eq!(service.requests.lock().unwrap().len(), 1, "rejected routes must not reach the tool service");
}

#[tokio::test]
async fn device_tools_unreachable_device_never_executes_local_service() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let source = testCoreNodeRouter("tools-local", "tools-offline", "tools-offline-binding");
    let service = Arc::new(RecordingDeviceToolService::default());
    source.bindDeviceToolService(service.clone()).unwrap();
    let link = installTestPeer(
        &source,
        "tools-offline".into(),
        Arc::new(TestClientEndpoint),
    )
    .unwrap();
    link.close();
    assert!(source
        .watchNode("tools-offline".into(), deviceToolWatch("offline"))
        .await
        .is_err());
    assert!(service.requests.lock().unwrap().is_empty());
}

/// Connects A -> B -> C through real routers, with only the next hop paired at each source.
fn deviceToolsRelayRoute() -> (
    CoreNodeRouter,
    CoreNodeRouter,
    CoreNodeRouter,
    Arc<TestPeerService>,
    Arc<TestPeerService>,
) {
    let space = CoreSpace {
        spaceId: "tools-multihop".into(),
        spaceName: "Tool relay".into(),
        spaceRevision: 2,
        members: vec!["tools-a".into(), "tools-b".into(), "tools-c".into()],
    };
    let (source, _) = testCoreNodeRouterInJoinedSpace(
        "tools-a",
        "tools-b",
        "tools-binding",
        "tools-c",
        space.clone(),
    );
    let (relay, _) = testCoreNodeRouterInJoinedSpace(
        "tools-b",
        "tools-c",
        "tools-binding",
        "tools-c",
        space.clone(),
    );
    let (target, _) =
        testCoreNodeRouterInJoinedSpace("tools-c", "tools-b", "tools-binding", "tools-c", space);
    for router in [&source, &relay, &target] {
        router
            .networkControlStore
            .setIdentity(
                operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
                    nodeId: "tools-a".into(),
                    roleId: "admin".into(),
                },
            )
            .unwrap();
        router
            .networkControlStore
            .setIdentity(
                operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
                    nodeId: "tools-b".into(),
                    roleId: "relay".into(),
                },
            )
            .unwrap();
    }
    relay
        .spaceStore
        .publishLocalAdjacency(vec!["tools-c".into()], 60_000, 1000)
        .unwrap();
    // Copy the synchronized B adjacency announcement into A's topology view.
    let path = format!(
        "{}/tools-b.preferences.json",
        operit_util::RuntimeStorageLayout::RUNTIME_SPACE_TOPOLOGY_DIR_PATH
    );
    let adjacency = relay
        .localCore
        .runtimeStorageHost()
        .readBytes(&path)
        .unwrap();
    source
        .localCore
        .runtimeStorageHost()
        .writeBytes(&path, &adjacency)
        .unwrap();
    let bc = TestPeerService::new(
        "tools-b".into(),
        "tools-c".into(),
        TestCoreNodeRouterEndpoint::new(target.clone()),
    );
    relay
        .installNodeServices(NodeServices::new(bc.clone()))
        .unwrap();
    let ab = TestPeerService::new(
        "tools-a".into(),
        "tools-b".into(),
        TestCoreNodeRouterEndpoint::new(relay.clone()),
    );
    source
        .installNodeServices(NodeServices::new(ab.clone()))
        .unwrap();
    (source, relay, target, ab, bc)
}

#[derive(Default)]
struct MultiHopDeviceTools {
    origins: StdMutex<Vec<String>>,
    cancelled: Arc<AtomicBool>,
    fail_open: AtomicBool,
}
impl DeviceToolService for MultiHopDeviceTools {
    fn open(
        &self,
        request: CoreWatchRequest,
        origin: String,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.origins.lock().unwrap().push(origin);
        if self.fail_open.load(Ordering::SeqCst) {
            return Err(CoreLinkError::new(
                "PEER_RESPONSE_CLOSED",
                "Reply lost after target submission",
            ));
        }
        let (sender, stream) = CoreEventStream::channel();
        if request.args == CoreValue::Null {
            let cancelled = self.cancelled.clone();
            defaultHostRuntimeTaskSchedulerHost()
                .scheduleHostRuntimeAsyncTask(
                    "test-device-cancel",
                    Box::new(move || {
                        Box::pin(async move {
                            sender.closed().await;
                            cancelled.store(true, Ordering::SeqCst);
                        })
                    }),
                )
                .unwrap();
        } else {
            sender
                .send(operit_link::CoreEvent {
                    requestId: Some(request.requestId),
                    target: DEVICE_TOOL_TARGET.into(),
                    propertyName: request.propertyName,
                    kind: CoreEventKind::Completed,
                    value: operit_link::toCoreValue(serde_json::json!({
                        "success": true,
                        "data": {"kind": "value", "value": {"changed": "/target.txt"}},
                        "error": null,
                    }))
                    .unwrap(),
                })
                .unwrap();
        }
        Ok(stream)
    }
}

#[tokio::test]
async fn device_tools_multihop_preserves_origin_result_cancellation_and_permissions() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (source, relay, target, ab, bc) = deviceToolsRelayRoute();
    let service = Arc::new(MultiHopDeviceTools::default());
    target.bindDeviceToolService(service.clone()).unwrap();
    let mut stream = source
        .watchNode("tools-c".into(), deviceToolWatch("multihop-change"))
        .await
        .unwrap();
    let event = stream.recv().await.unwrap();
    assert_eq!(event.kind, CoreEventKind::Completed);
    let result: serde_json::Value = operit_link::fromCoreValue(event.value).unwrap();
    assert_eq!(result["success"], true);
    assert_eq!(result["data"]["value"]["changed"], "/target.txt");
    assert_eq!(*service.origins.lock().unwrap(), vec!["tools-a"]);
    assert!(
        relay.deviceToolService.get().is_none(),
        "A relay does not execute Core tools"
    );
    let mut waiting = deviceToolWatch("multihop-cancel");
    waiting.args = CoreValue::Null;
    let stream = source.watchNode("tools-c".into(), waiting).await.unwrap();
    drop(stream);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !service.cancelled.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    target
        .networkControlStore
        .setIdentity(
            operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
                nodeId: "tools-a".into(),
                roleId: "relay".into(),
            },
        )
        .unwrap();
    let stream = source.watchNode("tools-c".into(), deviceToolWatch("non-executor-origin"))
        .await.expect("execution capacity is not a caller permission");
    drop(stream);
    assert_eq!(service.origins.lock().unwrap().len(), 3);
    target.networkControlStore.disconnectNode("tools-a".into()).unwrap();
    let error = source.watchNode("tools-c".into(), deviceToolWatch("revoked-origin"))
        .await.err().unwrap();
    assert_eq!(error.code, "ORIGIN_CORE_NODE_REVOKED");
    assert_eq!(service.origins.lock().unwrap().len(), 3);
    ab.close();
    bc.close();
}

#[tokio::test]
async fn device_tools_multihop_never_resubmits_after_an_uncertain_reply() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (source, _relay, target, ab, bc) = deviceToolsRelayRoute();
    let service = Arc::new(MultiHopDeviceTools::default());
    service.fail_open.store(true, Ordering::SeqCst);
    target.bindDeviceToolService(service.clone()).unwrap();
    let error = source
        .watchNode("tools-c".into(), deviceToolWatch("uncertain"))
        .await
        .err()
        .unwrap();
    assert_eq!(
        error.code, "PEER_RESPONSE_CLOSED",
        "Both source and relay must preserve the original uncertain outcome"
    );
    assert_eq!(service.origins.lock().unwrap().len(), 1);
    assert!(!super::route_forwarding::canRetryRouteTarget(
        "edge.scene"
    ));
    assert!(!super::route_forwarding::canRetryRouteTarget(
        "edge.deviceIo"
    ));
    assert!(super::route_forwarding::canRetryRouteTarget("chat"));
    ab.close();
    bc.close();
}

#[derive(Default)]
struct LostForwardingReply(std::sync::atomic::AtomicUsize);
impl LostForwardingReply {
    fn fail(&self) -> CoreLinkError {
        self.0.fetch_add(1, Ordering::SeqCst);
        CoreLinkError::new(
            "PEER_RESPONSE_CLOSED",
            "Operation ran but its reply was lost",
        )
    }
}
#[async_trait]
impl TestRouteTarget for LostForwardingReply {
    async fn routedCall(
        &self,
        _: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        CoreCallResponse::err(request.payload.requestId, self.fail())
    }
    async fn routedWatchSnapshot(
        &self,
        _: String,
        _: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        Err(self.fail())
    }
    async fn routedWatch(
        &self,
        _: String,
        _: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        Err(self.fail())
    }
    async fn routedOpenPush(
        &self,
        _: String,
        _: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Err(self.fail())
    }
}

#[tokio::test]
async fn forwarding_uses_one_replay_policy_for_calls_snapshots_watches_and_pushes() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (source, _relay, _target, ab, bc) = deviceToolsRelayRoute();
    let failed = Arc::new(LostForwardingReply::default());
    bc.attach(failed.clone());
    for target in [DEVICE_TOOL_TARGET, "edge.scene", "edge.deviceIo", "chat"] {
        let expected = if target == "chat" {
            "CORE_NODE_UNREACHABLE"
        } else {
            "PEER_RESPONSE_CLOSED"
        };
        let before = failed.0.load(Ordering::SeqCst);
        let call = CoreCallRequest::new("lost-call", target, "execute", CoreValue::emptyMap());
        assert_eq!(
            source
                .callNode("tools-c".into(), call)
                .await
                .result
                .unwrap_err()
                .code,
            expected
        );
        let watch = CoreWatchRequest::new("lost-watch", target, "execute", CoreValue::emptyMap());
        assert_eq!(
            source
                .watchNodeSnapshot("tools-c".into(), watch.clone())
                .await
                .err()
                .unwrap()
                .code,
            expected
        );
        assert_eq!(
            source
                .watchNode("tools-c".into(), watch.clone())
                .await
                .err()
                .unwrap()
                .code,
            expected
        );
        let push = CorePushRequest::new("lost-push", target, "execute");
        assert_eq!(
            source
                .openPushNode("tools-c".into(), push.clone())
                .await
                .err()
                .unwrap()
                .code,
            expected
        );
        // Space adapters use the same rule as explicit device adapters, at the source and relay.
        assert_eq!(
            source
                .watchNodeSnapshotSpaceWithOrigin("tools-c".into(), watch.clone(), "tools-a".into())
                .await
                .err()
                .unwrap()
                .code,
            expected
        );
        assert_eq!(
            source
                .watchNodeSpaceWithOrigin("tools-c".into(), watch, "tools-a".into())
                .await
                .err()
                .unwrap()
                .code,
            expected
        );
        assert_eq!(
            source
                .openPushNodeSpaceWithOrigin("tools-c".into(), push, "tools-a".into())
                .await
                .err()
                .unwrap()
                .code,
            expected
        );
        assert_eq!(
            failed.0.load(Ordering::SeqCst) - before,
            7,
            "each request reaches the target once"
        );
    }
    ab.close();
    bc.close();
}

#[tokio::test]
async fn route_status_and_forwarding_share_transit_and_revocation_policy() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (source, _relay, _target, ab, bc) = deviceToolsRelayRoute();
    let next = || {
        source.nextActiveHopAvoiding(
            "tools-c".into(),
            None,
            &BTreeSet::new(),
            &RoutingPolicy::load(&source.networkControlStore).unwrap(),
        )
    };
    assert_eq!(
        source.nodeRoutePlan("tools-c").unwrap().unwrap().firstHop(),
        "tools-b"
    );
    assert_eq!(next().unwrap(), "tools-b");
    source
        .networkControlStore
        .setIdentity(
            operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
                nodeId: "tools-b".into(),
                roleId: "edge".into(),
            },
        )
        .unwrap();
    assert!(source.nodeRoutePlan("tools-c").unwrap().is_none());
    assert_eq!(next().unwrap_err().code, "CORE_NODE_UNREACHABLE");
    // Removing transit permission must retain direct reachability.
    assert!(source.nodeIsReachable("tools-b").unwrap());
    source
        .networkControlStore
        .disconnectNode("tools-b".into())
        .unwrap();
    assert!(!source.nodeIsReachable("tools-b").unwrap());
    assert_eq!(
        source
            .watchNode("tools-b".into(), deviceToolWatch("revoked"))
            .await
            .err()
            .unwrap()
            .code,
        "CORE_NODE_REVOKED"
    );
    assert!(source.spaceChannelScope("tools-b").unwrap().is_none());
    ab.close();
    bc.close();
}
