//! Real TCP, shared pairing and shared admission, with no application runtime.
#[path = "../../../../../apps/esp32/src/runtime_storage_pack.rs"]
mod runtime_storage_pack;
use runtime_storage_pack as pack;
#[path = "../../../../../apps/esp32/src/runtime_storage_codec.rs"]
mod codec;
use super::*;
use crate::PeerRouter::EdgePeerRouter;
use operit_host_api::{HostError, HostResult, RuntimeStorageHost, RuntimeStorageEntry};
use operit_host_native_common::{NativeTcpHost, NativeHostRuntimeTaskSchedulerHost};
use operit_node_runtime::HostRuntimePeerService::HostRuntimePeerService;
use operit_node_runtime::NodeSpaceService::{NodeSpaceService, NodeSpaceContext, SpaceJoinStatus, NODE_SPACE_TARGET, NODE_SPACE_APPROVAL_TARGET};
use operit_node_runtime::NodeServices::{PeerEndpoint, PeerTransport};
use operit_node_runtime::PeerStateStore::{PeerStateStore, PeerHostConfig, PeerHostPortMode};
use operit_node_runtime::RuntimePeerService::RuntimePeerService;
use operit_store::CoreSpaceStore::CoreSpaceDeviceProfile;
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Default)]
struct Storage(Mutex<BTreeMap<String, Vec<u8>>>, bool, Mutex<codec::tests::PhysicalNvs>, Mutex<Option<usize>>);
impl Storage {
    fn update(&self, change: impl FnOnce(&mut BTreeMap<String,Vec<u8>>)) -> HostResult<()> {
        let mut files = self.0.lock().unwrap();
        let mut next = files.clone(); change(&mut next);
        if next == *files { return Ok(()); }
        let mut failure = self.3.lock().unwrap();
        if let Some(remaining) = failure.as_mut() {
            if *remaining == 0 { *failure = None; return Err(HostError::new("injected approval persistence failure")); }
            *remaining -= 1;
        }
        drop(failure);
        if self.1 {
            let packed: codec::Files = next.iter().map(|(path, bytes)| pack::pack(bytes).map(|bytes| (path.clone(), Arc::new(bytes)))).collect::<HostResult<_>>()?;
            let mut nvs = self.2.lock().unwrap();
            let active = nvs.active_bank();
            codec::commit_snapshot_transaction(&packed, active, &mut *nvs)?;
            assert_eq!(codec::decode_packed_files(&nvs.selected()).unwrap(), packed);
        }
        *files = next; Ok(())
    }
    fn write(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.update(|files| { files.insert(path.into(),bytes.into()); })
    }
}
impl RuntimeStorageHost for Storage {
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> { None }
    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> { None }
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> { self.0.lock().unwrap().get(path).cloned().ok_or_else(|| HostError::new("missing")) }
    fn writeBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> { self.write(path, bytes) }
    fn appendBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> { self.update(|files| { files.entry(path.into()).or_default().extend_from_slice(bytes); }) }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> { self.update(|files| { let prefix = format!("{path}/"); files.retain(|key,_| key != path && (!recursive || !key.starts_with(&prefix))); }) }
    fn exists(&self, path: &str) -> HostResult<bool> {
        let files = self.0.lock().unwrap();
        // Full Core workspace scanning needs native-style directory existence.
        // The bounded ESP32 fixture retains its exact virtual-entry semantics.
        Ok(files.contains_key(path) || (!self.1 && files.keys().any(|key| key.starts_with(&format!("{path}/")))))
    }
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> { Ok(self.0.lock().unwrap().iter().filter(|(path, _)| path.starts_with(prefix)).map(|(path, bytes)| RuntimeStorageEntry { path: path.clone(), isDirectory: false, size: bytes.len() as i64 }).collect()) }
}
fn node(bounded: bool) -> (Arc<Storage>, Arc<EdgePeerRouter>, Arc<HostRuntimePeerService>, Arc<NodeSpaceService>) {
    node_from_storage(Arc::new(Storage(Mutex::default(), bounded, Mutex::default(), Mutex::default())))
}
fn node_from_storage(storage: Arc<Storage>) -> (Arc<Storage>, Arc<EdgePeerRouter>, Arc<HostRuntimePeerService>, Arc<NodeSpaceService>) {
    let bounded = storage.1;
    // Latest main migrates an empty host config eagerly; configure fresh test
    // storage before that migration can hide the absence of saved settings.
    let configureListener = !storage.exists("runtime/link_access/host_config.preferences.json").unwrap();
    let id = operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage.clone()).initialize().unwrap().nodeId;
    let router = EdgePeerRouter::new(id.clone());
    let host = Arc::new(HostManager {
        runtimeStorageHost: Some(storage.clone()), tcpHost: Some(Arc::new(NativeTcpHost)),
        hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost)), ..HostManager::default()
    });
    let peer = HostRuntimePeerService::new(host.clone(), &router, operit_link::protocol::LinkDeviceInfo { platform: "test".into(), model: "test".into() }).unwrap();
    let space = NodeSpaceService::new(storage.clone(), peer.clone()).unwrap();
    space.initialize(CoreSpaceDeviceProfile { nodeId: id, displayName: if bounded { "ESP32-2432S028" } else { "HAN-DESKTOP (Windows)" }.into(), userName: String::new(), platform: if bounded { "esp32" } else { "windows" }.into(), model: if bounded { "ESP32-2432S028" } else { "Windows desktop" }.into(), coreVersion: None, updatedAt: 1 }).unwrap();
    router.installSpace(space.clone()).unwrap();
    router.install(Arc::new(EdgeNode::fromHostManager((*host).clone()).withNodeServices(NodeServices::new(peer.clone())))).unwrap();
    if configureListener {
        PeerStateStore::new(storage.clone()).saveHostConfig(&PeerHostConfig { bindAddress: "127.0.0.1:0".into(), token: "test-token".into(), transports: vec![PeerTransport::Tcp], discoveryEnabled: false, portMode: PeerHostPortMode::Automatic, updatedAt: 1 }).unwrap();
    }
    (storage, router, peer, space)
}
// A listener restart retires the previous pooled TCP generation asynchronously.
// Wait for discovery to reconnect, rather than assuming the old lease can survive.
async fn wait_for_peer_availability(peers: &HostRuntimePeerService, node: &str, online: bool) {
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        while peers.activePeerNodeIds().unwrap().iter().any(|peer| peer == node) != online {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }).await.expect("peer availability did not converge after listener restart");
}

fn print_size(stage: &str, storage: &Storage) {
    let files = storage.0.lock().unwrap();
    let packed: codec::Files = files.iter().map(|(path, bytes)| (path.clone(), Arc::new(pack::pack(bytes).unwrap()))).collect();
    let mut snapshot = Vec::new();
    codec::commit_packed_snapshot(&packed, 0, |_, chunk| { snapshot.extend_from_slice(&pack::unpack(chunk).unwrap()); Ok(()) }, |_| Ok(())).unwrap();
    assert_eq!(codec::decode_packed_files(&snapshot).unwrap(), packed);
    println!("NVS_ENVELOPE {stage}: {} bytes; compressed transaction: {} entries", snapshot.len(), codec::snapshot_required_entries(&packed).unwrap());

    for (path, bytes) in &packed { assert_eq!(&pack::unpack(bytes).unwrap(), files.get(path).unwrap()); }
    for (path, bytes) in files.iter() { println!("  {path}: {} / {} packed", bytes.len(), packed[path].len()); }
}
#[tokio::test]
async fn paired_edge_requires_explicit_approval_and_restores_space() {
    let (a_store, _a_router, a_peer, a_space) = node(false);
    let (b_store, b_router, b_peer, b_space) = node(true);
    print_size("bootstrap", &b_store);
    let result = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        a_peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
        b_peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
        let address = PeerStateStore::new(b_store.clone()).hostConfig().unwrap().unwrap().bindAddress;
        let b_id = b_space.localNodeId(); let a_id = a_space.localNodeId();
        let pairing = a_peer.startPairing(PeerEndpoint { nodeId: b_id.clone(), address }, PeerTransport::Tcp, Some("test-token")).await.unwrap();
        let code = b_peer.pairingPrompts().unwrap()[0].confirmationCode.clone();
        a_peer.finishPairing(&pairing.pairingId, &code).await.unwrap();
        print_size("paired", &b_store);
        assert!(b_store.list("runtime/sync").unwrap().is_empty(), "endpoint pairing must not create a replica journal");
        assert_eq!(b_space.spaceChannelScope(&a_id).unwrap(), None);
        let request = a_space.requestDeviceSpaceJoin(b_id.clone()).await.unwrap();
        assert_eq!(request.status, SpaceJoinStatus::Pending);
        assert!(!b_space.spaceStore().initialize().unwrap().members.contains(&a_id));
        let incoming = b_space.incomingDeviceSpaceJoins().await.unwrap();
        assert_eq!(incoming.len(), 1); assert!(incoming[0].canApprove);
        print_size("pending", &b_store);
        assert!(b_space.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion+1, true).await.is_err());
        assert_eq!(b_space.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await.unwrap().status, SpaceJoinStatus::Approved);
        let revision = b_space.spaceStore().space().unwrap().spaceRevision;
        assert_eq!(b_space.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await.unwrap().status, SpaceJoinStatus::Approved);
        assert_eq!(b_space.spaceStore().space().unwrap().spaceRevision, revision);
        assert_eq!(a_space.refreshDeviceSpaceJoin(request.requestId).await.unwrap().status, SpaceJoinStatus::Joined);
        assert_eq!(a_space.spaceStore().space().unwrap(), b_space.spaceStore().space().unwrap());
        assert!(!a_space.networkControlStore().nodeHasCapability(&a_id, "network.approval", None).unwrap());
        print_size("approved", &b_store);
        assert!(b_store.list("runtime/sync").unwrap().is_empty(), "endpoint approval must not create replica clocks or logs");
        let receipt: serde_json::Value = serde_json::from_slice(&b_store.readBytes("runtime/link_access/space_merge_inbound.preferences.json").unwrap()).unwrap();
        for value in receipt.as_object().unwrap().values() {
            let record: serde_json::Value = serde_json::from_str(value.as_str().unwrap()).unwrap();
            if !record["accepted"].is_null() { assert_eq!(record["accepted"]["controlOperations"].as_array().unwrap().len(), 0); }
        }
        let restored = NodeSpaceService::new(b_store.clone(), b_peer.clone()).unwrap();
        assert_eq!(restored.spaceChannelScope(&a_id).unwrap(), b_space.spaceChannelScope(&a_id).unwrap());
        // Spoofed/relayed admission cannot use the paired peer as an origin.
        use operit_node_runtime::PeerRouter::PeerRouter;
        let wrong_space = operit_link::RoutedCoreRequest { spaceId: "other-space".into(), originNodeId: a_id.clone(), targetNodeId: b_id.clone(), ttl: 0, routeKind: operit_link::RoutedCoreRequestKind::Target, payload: CoreCallRequest::new("wrong-space", NODE_SPACE_APPROVAL_TARGET, "assigned", CoreValue::Null) };
        assert_eq!(b_router.routedCall(a_id.clone(), wrong_space).await.result.unwrap_err().code, "SPACE_ID_MISMATCH");
        let forged = operit_link::RoutedCoreRequest { spaceId: String::new(), originNodeId: "forged".into(), targetNodeId: b_id, ttl: 0, routeKind: operit_link::RoutedCoreRequestKind::Target, payload: CoreCallRequest::new("forged", NODE_SPACE_TARGET, "snapshot", CoreValue::Null) };
        assert_eq!(b_router.routedCall(a_id, forged).await.result.unwrap_err().code, "PEER_DIRECT_CALL_REQUIRED");
    }).await;
    a_peer.stop().await.unwrap(); b_peer.stop().await.unwrap();
    result.unwrap();
    drop(a_store);
}

#[tokio::test]
async fn failed_local_cancellation_preserves_pending_request_and_can_retry() {
    let (storage, _applicantRouter, applicantPeer, applicant) = node(false);
    let (gatewayStorage, _gatewayRouter, gatewayPeer, gateway) = node(true);
    applicantPeer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    gatewayPeer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    let address = PeerStateStore::new(gatewayStorage).hostConfig().unwrap().unwrap().bindAddress;
    let pairing = applicantPeer.startPairing(PeerEndpoint {
        nodeId: gateway.localNodeId(), address,
    }, PeerTransport::Tcp, Some("test-token")).await.unwrap();
    let code = gatewayPeer.pairingPrompts().unwrap()[0].confirmationCode.clone();
    applicantPeer.finishPairing(&pairing.pairingId, &code).await.unwrap();
    let request = applicant.requestDeviceSpaceJoin(gateway.localNodeId()).await.unwrap();
    let before = applicant.spaceStore().space().unwrap();

    *storage.3.lock().unwrap() = Some(0);
    let error = applicant.cancelDeviceSpaceJoin(request.requestId.clone()).await.unwrap_err();
    assert!(error.contains("injected approval persistence failure"), "{error}");
    let saved: BTreeMap<String, String> = serde_json::from_slice(&storage.readBytes(
        "runtime/link_access/space_merge_outbound.preferences.json").unwrap()).unwrap();
    let record: serde_json::Value = serde_json::from_str(&saved[&request.requestId]).unwrap();
    assert_eq!(record["request"]["status"], "pending");
    assert_eq!(gateway.incomingDeviceSpaceJoins().await.unwrap()[0].status, SpaceJoinStatus::Pending);
    assert_eq!(applicant.spaceStore().space().unwrap(), before);

    let cancelled = applicant.cancelDeviceSpaceJoin(request.requestId.clone()).await.unwrap();
    assert_eq!(cancelled.status, SpaceJoinStatus::Cancelled);
    let saved: BTreeMap<String, String> = serde_json::from_slice(&storage.readBytes(
        "runtime/link_access/space_merge_outbound.preferences.json").unwrap()).unwrap();
    let record: serde_json::Value = serde_json::from_str(&saved[&request.requestId]).unwrap();
    assert_eq!(record["request"]["status"], "cancelled");
    assert!(gateway.incomingDeviceSpaceJoins().await.unwrap().is_empty());
    assert_eq!(applicant.spaceStore().space().unwrap(), before);
    applicantPeer.stop().await.unwrap();
    gatewayPeer.stop().await.unwrap();
}

/// Cleanup must not evict the cache when the host rejects the durable delete.
#[test]
fn node_local_delete_failure_preserves_shared_snapshot_and_durable_file() {
    use operit_store::PreferencesDataStore::{CoreNodeStateStore, emptyPreferences, stringPreferencesKey};
    let storage = Arc::new(Storage::default());
    let path = "runtime/space/device_profiles/delete-failure.preferences.json";
    let first = CoreNodeStateStore::newWithStorage(storage.clone(), path);
    let second = CoreNodeStateStore::newWithStorage(storage.clone(), path);
    let mut snapshot = emptyPreferences();
    snapshot.set(&stringPreferencesKey("record"), "unchanged-profile".to_string());
    first.replaceRecoverably(snapshot.clone()).unwrap();
    *storage.3.lock().unwrap() = Some(0);
    assert!(first.delete().is_err());
    assert!(storage.exists(path).unwrap());
    assert_eq!(second.data().unwrap(), snapshot);
    first.delete().unwrap();
    assert!(!storage.exists(path).unwrap());
    assert_eq!(second.data().unwrap(), emptyPreferences());
    second.replaceRecoverably(snapshot.clone()).unwrap();
    assert!(storage.exists(path).unwrap());
    assert_eq!(first.data().unwrap(), snapshot);
}

#[tokio::test]
async fn approval_retry_recovers_each_partial_write_without_duplicate_admission() {
    for failure in 0..32 {
        let (_, _applicant_router, applicant_peer, applicant) = node(false);
        let (mut storage, mut gateway_router, mut gateway_peer, mut gateway) = node(true);
        let result = tokio::time::timeout(std::time::Duration::from_secs(20), async {
            applicant_peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
            gateway_peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
            let gateway_id = gateway.localNodeId();
            let applicant_id = applicant.localNodeId();
            let address = PeerStateStore::new(storage.clone()).hostConfig().unwrap().unwrap().bindAddress;
            let pairing = applicant_peer.startPairing(PeerEndpoint { nodeId: gateway_id.clone(), address }, PeerTransport::Tcp, Some("test-token")).await.unwrap();
            let code = gateway_peer.pairingPrompts().unwrap()[0].confirmationCode.clone();
            applicant_peer.finishPairing(&pairing.pairingId, &code).await.unwrap();
            // Real hardware contains earlier declined/expired applications.
            // Keep that history instead of testing only an empty database.
            for _ in 0..3 {
                let request = applicant.requestDeviceSpaceJoin(gateway_id.clone()).await.unwrap();
                gateway.incomingDeviceSpaceJoins().await.unwrap();
                assert_eq!(gateway.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, false).await.unwrap().status, SpaceJoinStatus::Rejected);
                assert_eq!(applicant.refreshDeviceSpaceJoin(request.requestId).await.unwrap().status, SpaceJoinStatus::Rejected);
            }
            let request = applicant.requestDeviceSpaceJoin(gateway_id).await.unwrap();
            gateway.incomingDeviceSpaceJoins().await.unwrap();
            *storage.3.lock().unwrap() = Some(failure);
            let first = gateway.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await;
            *storage.3.lock().unwrap() = None;
            if first.is_err() {
                eprintln!("Recovering failure point {failure}: {}", first.unwrap_err());
                if failure % 2 == 1 {
                    // A real restart: new storage identity, policy/preferences
                    // caches, router, peer engine and TCP listener. No stale
                    // in-memory decision or operation can rescue this retry.
                    gateway_peer.stop().await.unwrap();
                    wait_for_peer_availability(&applicant_peer, &gateway.localNodeId(), false).await;
                    let recovered = Arc::new(Storage(Mutex::new(storage.0.lock().unwrap().clone()), true, Mutex::new(storage.2.lock().unwrap().clone()), Mutex::default()));
                    (storage, gateway_router, gateway_peer, gateway) = node_from_storage(recovered);
                    gateway_peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
                    wait_for_peer_availability(&applicant_peer, &gateway.localNodeId(), true).await;
                }
                assert_eq!(gateway.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await.unwrap().status, SpaceJoinStatus::Approved, "failure point {failure}");
            }
            assert_eq!(applicant.refreshDeviceSpaceJoin(request.requestId).await.unwrap().status, SpaceJoinStatus::Joined, "failure point {failure}");
            let admissions = gateway.networkControlStore().currentSpaceOperations().unwrap().into_iter().filter(|operation| {
                let record: operit_store::NetworkControlStore::NetworkControlCommandRecord = serde_json::from_value(operation.payload.clone()).unwrap();
                matches!(record.command, operit_store::NetworkControlStore::NetworkControlCommand::AdmitSpace { nodeIds, .. } if nodeIds.contains(&applicant_id))
            }).count();
            assert_eq!(admissions, 1, "failure point {failure} appended duplicate admissions");
            // Reopen fresh storage identity: no process-shared store caches.
            let reopened = Arc::new(Storage(Mutex::new(storage.0.lock().unwrap().clone()), true, Mutex::new(storage.2.lock().unwrap().clone()), Mutex::default()));
            let restored = NodeSpaceService::new(reopened, gateway_peer.clone()).unwrap();
            assert_eq!(restored.spaceChannelScope(&applicant_id).unwrap(), gateway.spaceChannelScope(&applicant_id).unwrap());
        }).await;
        applicant_peer.stop().await.unwrap(); gateway_peer.stop().await.unwrap();
        result.unwrap();
        drop(gateway_router);
    }
}


// Cross-kind coverage: neither endpoint is a pretend desktop implemented by Edge.
mod core_edge {
    use super::*;
    use operit_link::{
        CoreEventStream, CoreLinkPushSession, CoreLinkSharedClient, CorePushRequest,
        CoreWatchRequest, RoutedCoreRequest, RoutedCoreRequestKind,
    };
    use operit_node_runtime::CoreNodeRouter::{CoreNodeLocalRuntime, CoreNodeRouter};
    use operit_node_runtime::NodeSpaceService::NODE_SPACE_CONTROL_TARGET;
    use operit_node_runtime::PeerRouter::PeerRouter;
    use operit_node_runtime::RuntimeRemoteLinkService::RuntimeRemoteLinkService;
    use operit_node_runtime::SpacePersistenceSyncService::SpacePersistenceSyncService;
    use operit_store::CoreSpaceStore::CoreSpaceStore;
    use operit_store::NetworkControlStore::NetworkControlIdentityAssignment;
    use std::time::Duration;

    static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// Only the application backend is a spy. Pairing, routing, policy, admission,
    /// persistence synchronization and the TCP transport are production code.
    #[derive(Default)]
    struct ApplicationSpy(Mutex<Vec<String>>);
    #[async_trait(?Send)]
    impl CoreLinkSharedClient for ApplicationSpy {
        async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
            self.0.lock().unwrap().push(request.methodName.clone());
            CoreCallResponse::err(
                request.requestId,
                CoreLinkError::new("UNEXPECTED_APPLICATION_CALL", request.methodName),
            )
        }
        async fn watchSnapshot(&self, _: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
            Err(CoreLinkError::methodNotFound("test.watchSnapshot"))
        }
        async fn watch(&self, _: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
            Err(CoreLinkError::methodNotFound("test.watch"))
        }
    }

    struct RecordingEdgeRouter {
        inner: Arc<EdgePeerRouter>,
        traffic: Mutex<Vec<(String, String, u32)>>,
    }
    #[async_trait(?Send)]
    impl PeerRouter for RecordingEdgeRouter {
        fn localNodeId(&self) -> String {
            self.inner.localNodeId()
        }
        async fn routedCall(
            &self,
            previous: String,
            request: RoutedCoreRequest<CoreCallRequest>,
        ) -> CoreCallResponse {
            self.traffic.lock().unwrap().push((
                request.payload.target.clone(),
                request.payload.methodName.clone(),
                request.ttl,
            ));
            self.inner.routedCall(previous, request).await
        }
        async fn routedWatchSnapshot(
            &self,
            previous: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEvent, CoreLinkError> {
            self.traffic.lock().unwrap().push((
                request.payload.target.clone(),
                request.payload.propertyName.clone(),
                request.ttl,
            ));
            self.inner.routedWatchSnapshot(previous, request).await
        }
        async fn routedWatch(
            &self,
            previous: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEventStream, CoreLinkError> {
            self.traffic.lock().unwrap().push((
                request.payload.target.clone(),
                request.payload.propertyName.clone(),
                request.ttl,
            ));
            self.inner.routedWatch(previous, request).await
        }
        async fn routedOpenPush(
            &self,
            previous: String,
            request: RoutedCoreRequest<CorePushRequest>,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            self.traffic.lock().unwrap().push((
                request.payload.target.clone(),
                request.payload.methodName.clone(),
                request.ttl,
            ));
            self.inner.routedOpenPush(previous, request).await
        }
        fn spaceChannelScope(&self, peer: &str) -> Result<Option<String>, CoreLinkError> {
            self.inner.spaceChannelScope(peer)
        }
    }

    struct CoreFixture {
        storage: Arc<Storage>,
        router: Arc<CoreNodeRouter>,
        peers: Arc<HostRuntimePeerService>,
        service: RuntimeRemoteLinkService,
        sync: SpacePersistenceSyncService,
        application: Arc<ApplicationSpy>,
    }
    struct EdgeFixture {
        storage: Arc<Storage>,
        router: Arc<RecordingEdgeRouter>,
        peers: Arc<HostRuntimePeerService>,
        service: Arc<NodeSpaceService>,
    }

    fn host(storage: Arc<Storage>) -> Arc<HostManager> {
        Arc::new(HostManager {
            runtimeStorageHost: Some(storage),
            tcpHost: Some(Arc::new(NativeTcpHost)),
            hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost)),
            ..HostManager::default()
        })
    }
    fn configure_listener(storage: &Arc<Storage>) {
        // hostConfig() eagerly migrates an empty store to the default fixed
        // listener. Check the raw entry first so fresh tests use isolated ports.
        if !storage
            .exists("runtime/link_access/host_config.preferences.json")
            .unwrap()
        {
            PeerStateStore::new(storage.clone())
                .saveHostConfig(&PeerHostConfig {
                    bindAddress: "127.0.0.1:0".into(),
                    token: "test-token".into(),
                    transports: vec![PeerTransport::Tcp],
                    discoveryEnabled: false,
                    portMode: PeerHostPortMode::Automatic,
                    updatedAt: 1,
                })
                .unwrap();
        }
    }
    fn core_node() -> CoreFixture {
        core_node_with_storage(Arc::new(Storage::default()))
    }
    fn core_node_with_storage(storage: Arc<Storage>) -> CoreFixture {
        operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
            NativeHostRuntimeTaskSchedulerHost,
        ));
        configure_listener(&storage);
        let application = Arc::new(ApplicationSpy::default());
        let runtime = CoreNodeLocalRuntime::new(
            application.clone(),
            application.clone(),
            storage.clone(),
            Arc::new(|schema| match schema {
                "application" => Some("test.application"),
                "services.syncBlobTransferManager" => Some("test.blob"),
                _ => None,
            }),
            Arc::new(|_| Ok(())),
            Arc::new(|_| {
                Err(CoreLinkError::new(
                    "UNEXPECTED_BLOB_RECEIVER",
                    "No storage grant",
                ))
            }),
            application.clone(),
        );
        let router = Arc::new(CoreNodeRouter::new(runtime.clone()));
        let info = operit_link::protocol::LinkDeviceInfo {
            platform: "windows".into(),
            model: "Windows desktop".into(),
        };
        let peers =
            HostRuntimePeerService::new(host(storage.clone()), &router, info.clone()).unwrap();
        router
            .installNodeServices(NodeServices::new(peers.clone()))
            .unwrap();
        let service = RuntimeRemoteLinkService::newWithRouter(runtime.clone(), (*router).clone());
        service.initializeDeviceInfo(info).unwrap();
        let sync = SpacePersistenceSyncService::new(
            Arc::new(runtime),
            (*router).clone(),
            CoreSpaceStore::new(storage.clone()),
        );
        CoreFixture {
            storage,
            router,
            peers,
            service,
            sync,
            application,
        }
    }
    fn edge_node(storage: Arc<Storage>) -> EdgeFixture {
        configure_listener(&storage);
        let id = operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage.clone())
            .initialize()
            .unwrap()
            .nodeId;
        let inner = EdgePeerRouter::new(id.clone());
        let router = Arc::new(RecordingEdgeRouter {
            inner: inner.clone(),
            traffic: Mutex::default(),
        });
        let host = host(storage.clone());
        let peers = HostRuntimePeerService::new(
            host.clone(),
            &router,
            operit_link::protocol::LinkDeviceInfo {
                platform: "esp32".into(),
                model: "ESP32-2432S028".into(),
            },
        )
        .unwrap();
        let service = NodeSpaceService::new(storage.clone(), peers.clone()).unwrap();
        service
            .initialize(CoreSpaceDeviceProfile {
                nodeId: id,
                displayName: "ESP32-2432S028".into(),
                userName: String::new(),
                platform: "esp32".into(),
                model: "ESP32-2432S028".into(),
                coreVersion: None,
                updatedAt: 1,
            })
            .unwrap();
        inner.installSpace(service.clone()).unwrap();
        inner
            .install(Arc::new(
                EdgeNode::fromHostManager((*host).clone())
                    .withNodeServices(NodeServices::new(peers.clone())),
            ))
            .unwrap();
        EdgeFixture {
            storage,
            router,
            peers,
            service,
        }
    }
    fn fresh_edge() -> EdgeFixture {
        edge_node(Arc::new(Storage(
            Mutex::default(),
            true,
            Mutex::default(),
            Mutex::default(),
        )))
    }
    fn reopened(storage: &Arc<Storage>) -> Arc<Storage> {
        Arc::new(Storage(
            Mutex::new(storage.0.lock().unwrap().clone()),
            storage.1,
            Mutex::new(storage.2.lock().unwrap().clone()),
            Mutex::default(),
        ))
    }
    async fn start(core: &CoreFixture, edge: &EdgeFixture) {
        core.peers
            .startListening(&[PeerTransport::Tcp])
            .await
            .unwrap();
        edge.peers
            .startListening(&[PeerTransport::Tcp])
            .await
            .unwrap();
    }
    async fn pair(
        source: &Arc<HostRuntimePeerService>,
        target: &Arc<HostRuntimePeerService>,
        target_storage: &Arc<Storage>,
        target_id: String,
    ) {
        let address = PeerStateStore::new(target_storage.clone())
            .hostConfig()
            .unwrap()
            .unwrap()
            .bindAddress;
        let pending = source
            .startPairing(
                PeerEndpoint {
                    nodeId: target_id,
                    address,
                },
                PeerTransport::Tcp,
                Some("test-token"),
            )
            .await
            .unwrap();
        let prompt = target
            .pairingPrompts()
            .unwrap()
            .into_iter()
            .find(|p| p.pairingId == pending.pairingId)
            .unwrap();
        assert_eq!(prompt.confirmationCode.len(), 6);
        let wrong = if prompt.confirmationCode == "000000" {
            "111111"
        } else {
            "000000"
        };
        assert!(source
            .finishPairing(&pending.pairingId, wrong)
            .await
            .is_err());
        assert!(source.pairedPeers().unwrap().is_empty());
        assert!(target.pairedPeers().unwrap().is_empty());
        source
            .finishPairing(&pending.pairingId, &prompt.confirmationCode)
            .await
            .unwrap();
        assert!(target.pairingPrompts().unwrap().is_empty());
    }
    fn assert_no_replica(core: &CoreFixture, edge: &EdgeFixture) {
        assert!(
            core.application.0.lock().unwrap().is_empty(),
            "non-storage synchronization entered the application backend"
        );
        for (target, method, _) in edge.router.traffic.lock().unwrap().iter() {
            assert!(
                (target == NODE_SPACE_TARGET
                    || target == NODE_SPACE_APPROVAL_TARGET
                    || target == NODE_SPACE_CONTROL_TARGET)
                    && !method.starts_with("sync"),
                "business replication reached Edge: {target}.{method}"
            );
        }
        assert!(edge.storage.list("runtime/sync").unwrap().is_empty());
        assert!(edge.storage.list("workspaces").unwrap().is_empty());
        assert!(!edge
            .storage
            .exists("runtime/link_access/storage_replica.preferences.json")
            .unwrap());
    }

    #[tokio::test]
    async fn full_core_pairs_with_edge_and_only_explicit_approval_joins() {
        let _guard = TEST_LOCK.lock().await;
        let core = core_node();
        let edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            let core_id = core.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            let original = core.service.deviceSpace().unwrap();
            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            assert_eq!(
                core.service.deviceSpace().unwrap(),
                original,
                "pairing is not admission"
            );
            let request = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            assert_eq!(request.status, SpaceJoinStatus::Pending);
            assert_eq!(edge.service.spaceChannelScope(&core_id).unwrap(), None);
            assert_eq!(
                edge.service.incomingDeviceSpaceJoins().await.unwrap().len(),
                1
            );
            assert!(edge
                .service
                .decideDeviceSpaceJoin(
                    request.requestId.clone(),
                    request.assignmentVersion + 1,
                    true
                )
                .await
                .is_err());
            let approved = edge
                .service
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await
                .unwrap();
            assert_eq!(approved.status, SpaceJoinStatus::Approved);
            let revision = edge.service.spaceStore().space().unwrap().spaceRevision;
            edge.service
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await
                .unwrap();
            assert_eq!(
                edge.service.spaceStore().space().unwrap().spaceRevision,
                revision
            );
            assert_eq!(
                core.service
                    .refreshDeviceSpaceJoin(request.requestId)
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Joined
            );
            assert_eq!(
                core.service.deviceSpace().unwrap(),
                edge.service.spaceStore().space().unwrap()
            );
            assert!(
                PeerRouter::spaceChannelScope(core.router.as_ref(), &edge_id)
                    .unwrap()
                    .is_some()
            );
            assert!(!core
                .service
                .networkControlStore()
                .nodeHasCapability(&core_id, "storage.provide", None)
                .unwrap());
            core.sync.synchronizeOnce().await.unwrap();
            assert!(edge
                .router
                .traffic
                .lock()
                .unwrap()
                .iter()
                .any(|(target, _, ttl)| target == NODE_SPACE_CONTROL_TARGET && *ttl > 0));
            // A remaining hop budget is legal on a direct connection. A forged
            // origin or wrong Space is not; accepting Core TTLs must not loosen this.
            for (origin, space, expected) in [
                (
                    "forged-origin".to_string(),
                    core.service.deviceSpace().unwrap().spaceId,
                    "PEER_DIRECT_CALL_REQUIRED",
                ),
                (
                    core_id.clone(),
                    "wrong-space".to_string(),
                    "SPACE_ID_MISMATCH",
                ),
            ] {
                let response = core
                    .peers
                    .call(
                        &edge_id,
                        RoutedCoreRequest {
                            spaceId: space,
                            originNodeId: origin,
                            targetNodeId: edge_id.clone(),
                            ttl: 2,
                            routeKind: RoutedCoreRequestKind::Target,
                            payload: CoreCallRequest::new(
                                "forged-control",
                                NODE_SPACE_CONTROL_TARGET,
                                "exchange",
                                operit_link::toCoreValue(Vec::<
                                    operit_store::SyncOperationStore::SyncOperation,
                                >::new())
                                .unwrap(),
                            ),
                        },
                    )
                    .await;
                assert_eq!(response.result.unwrap_err().code, expected);
            }
            assert_no_replica(&core, &edge);
        })
        .await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }

    /// Reproduce the physical device scenario: the SAME Core keeps the former
    /// group (including Edge) while Edge leaves and creates a new singleton.
    #[tokio::test]
    async fn same_core_rejoins_edge_after_local_exit_without_losing_member_profiles() {
        let _guard = TEST_LOCK.lock().await;
        let core = core_node();
        let edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let core_id = core.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            let first = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            edge.service.decideDeviceSpaceJoin(first.requestId.clone(), first.assignmentVersion, true).await.unwrap();
            assert_eq!(core.service.refreshDeviceSpaceJoin(first.requestId).await.unwrap().status, SpaceJoinStatus::Joined);
            for _cycle in 0..3 {
            let former = core.service.deviceSpace().unwrap();
            assert!(former.members.contains(&edge_id) && former.members.contains(&core_id));
            let credentials = edge.peers.pairedPeers().unwrap();
            let singleton = edge.service.leaveDeviceSpace().unwrap();
            assert_eq!(singleton.members, vec![edge_id.clone()]);
            assert_eq!(core.service.deviceSpace().unwrap(), former, "the remote Core retains its original group");
            assert!(!edge.service.spaceStore().deviceProfiles().unwrap().contains_key(&core_id));
            let withdrawn = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            assert_eq!(core.service.cancelDeviceSpaceJoin(withdrawn.requestId.clone()).await.unwrap().status, SpaceJoinStatus::Cancelled);
            assert!(edge.service.decideDeviceSpaceJoin(withdrawn.requestId, withdrawn.assignmentVersion, true).await.is_err());
            assert_eq!(core.service.deviceSpace().unwrap(), former, "withdrawal changed the applicant group");
            assert_eq!(edge.service.spaceStore().space().unwrap(), singleton, "withdrawal admitted the applicant");
            let request = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            let before = edge.service.spaceStore().space().unwrap();
            assert_eq!(before, singleton);
            let decision = edge.service.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await;
            assert!(decision.is_ok(), "same-Core rejoin approval failed: {decision:?}");
            assert_eq!(decision.unwrap().status, SpaceJoinStatus::Approved);
            assert_eq!(core.service.refreshDeviceSpaceJoin(request.requestId).await.unwrap().status, SpaceJoinStatus::Joined);
            assert_eq!(core.service.deviceSpace().unwrap(), edge.service.spaceStore().space().unwrap());
            let joined = edge.service.spaceStore().space().unwrap();
            let profiles = edge.service.spaceStore().deviceProfiles().unwrap();
            assert!(joined.members.iter().all(|node| profiles.contains_key(node)));
            assert_eq!(edge.peers.pairedPeers().unwrap(), credentials);
            assert_no_replica(&core, &edge);
            }
        }).await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }

    /// Model the durable state left by the old deleted-profile cache bug:
    /// admission/member writes succeeded but the receipt could not be built.
    #[tokio::test]
    async fn committed_rejoin_with_missing_profile_recovers_after_cancel_and_restart() {
        let _guard = TEST_LOCK.lock().await;
        let mut core = core_node();
        let mut edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(40), async {
            let core_id = core.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            let first = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            edge.service.decideDeviceSpaceJoin(first.requestId.clone(), first.assignmentVersion, true).await.unwrap();
            core.service.refreshDeviceSpaceJoin(first.requestId).await.unwrap();
            edge.service.leaveDeviceSpace().unwrap();
            let request = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            let credentials = edge.peers.pairedPeers().unwrap();
            let core_credentials = core.peers.pairedPeers().unwrap();
            // Claim through the production dispatcher, then persist the exact
            // admission and membership prefix completed before the old failure.
            edge.service.acceptSpaceApprovalCall(&edge_id, CoreCallRequest::new(
                "partial-rejoin-claim", NODE_SPACE_APPROVAL_TARGET, "claim",
                operit_link::toCoreValue(serde_json::json!({"requestId": request.requestId,
                    "assignmentVersion": request.assignmentVersion, "approve": true})).unwrap(),
            )).await.unwrap();
            let records: BTreeMap<String, String> = serde_json::from_slice(&edge.storage.readBytes(
                "runtime/link_access/space_merge_inbound.preferences.json").unwrap()).unwrap();
            let record: serde_json::Value = serde_json::from_str(&records[&request.requestId]).unwrap();
            let source_space: operit_store::CoreSpaceStore::CoreSpace = serde_json::from_value(record["source"]["space"].clone()).unwrap();
            let profiles: Vec<CoreSpaceDeviceProfile> = serde_json::from_value(record["source"]["deviceProfiles"].clone()).unwrap();
            edge.service.spaceStore().importDeviceProfiles(profiles).unwrap();
            edge.service.networkControlStore().admitSpaceForReview(source_space.spaceId.clone(),
                source_space.members.iter().cloned().collect(), &format!("{}-{}", request.requestId, request.assignmentVersion)).unwrap();
            let mut joined = edge.service.spaceStore().space().unwrap();
            joined.members.extend(source_space.members); joined.members.sort(); joined.members.dedup();
            joined.spaceRevision = record["decisionRevision"].as_i64().unwrap();
            edge.service.spaceStore().adoptAt(joined, request.createdAt).unwrap();
            operit_store::PreferencesDataStore::CoreNodeStateStore::newWithStorage(edge.storage.clone(),
                format!("runtime/space/device_profiles/{core_id}.preferences.json")).delete().unwrap();
            assert!(edge.service.spaceStore().deviceProfilesForCurrentSpace().is_err());
            // An admission already exists: cancellation must NOT revoke it or
            // manufacture success. It returns the real in-progress state.
            assert_eq!(core.service.cancelDeviceSpaceJoin(request.requestId.clone()).await.unwrap().status, SpaceJoinStatus::Approving);
            // Restart BOTH processes against fresh storage-host handles. The
            // applicant's persisted in-progress request must survive too.
            core.peers.stop().await.unwrap();
            edge.peers.stop().await.unwrap();
            core = core_node_with_storage(reopened(&core.storage));
            edge = edge_node(reopened(&edge.storage));
            assert_eq!(core.router.localNodeId(), core_id);
            assert_eq!(edge.service.localNodeId(), edge_id);
            assert_eq!(core.service.outgoingDeviceSpaceJoins().unwrap().into_iter()
                .find(|saved| saved.requestId == request.requestId).unwrap().status, SpaceJoinStatus::Approving);
            start(&core, &edge).await;
            wait_for_peer_availability(&core.peers, &edge_id, true).await;
            wait_for_peer_availability(&edge.peers, &core_id, true).await;
            let recovered = edge.service.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await;
            assert!(recovered.is_ok(), "durable rejoin did not recover after restart: {recovered:?}");
            // The same Core cancel action must reconcile the now-completed
            // approval as Joined, never falsely report Cancelled or stay stuck.
            assert_eq!(core.service.cancelDeviceSpaceJoin(request.requestId.clone()).await.unwrap().status, SpaceJoinStatus::Joined);
            assert_eq!(core.service.deviceSpace().unwrap(), edge.service.spaceStore().space().unwrap());
            assert!(edge.service.spaceStore().deviceProfilesForCurrentSpace().is_ok());
            let review_id = format!("control-review-{}-{}", request.requestId, request.assignmentVersion);
            assert_eq!(edge.service.networkControlStore().currentSpaceOperations().unwrap().iter()
                .filter(|operation| operation.entityId == review_id).count(), 1, "retry duplicated a committed admission");
            assert_eq!(edge.peers.pairedPeers().unwrap(), credentials);
            assert_eq!(core.peers.pairedPeers().unwrap(), core_credentials);
            assert_no_replica(&core, &edge);
        }).await;
        core.peers.stop().await.unwrap(); edge.peers.stop().await.unwrap(); result.unwrap();
    }

    /// A failed Pending cancellation is durable intent, not a fake terminal
    /// result. Reopening BOTH peers must retry it before any admission can win.
    #[tokio::test]
    async fn same_core_pending_rejoin_cancel_intent_survives_both_restarts() {
        let _guard = TEST_LOCK.lock().await;
        let mut core = core_node();
        let mut edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let core_id = core.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            let first = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            edge.service.decideDeviceSpaceJoin(first.requestId.clone(), first.assignmentVersion, true).await.unwrap();
            core.service.refreshDeviceSpaceJoin(first.requestId).await.unwrap();
            let former = core.service.deviceSpace().unwrap();
            let singleton = edge.service.leaveDeviceSpace().unwrap();
            let request = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            let core_credentials = core.peers.pairedPeers().unwrap();
            let edge_credentials = edge.peers.pairedPeers().unwrap();
            edge.peers.stop().await.unwrap();
            wait_for_peer_availability(&core.peers, &edge_id, false).await;
            assert!(core.service.cancelDeviceSpaceJoin(request.requestId.clone()).await.is_err());
            let records: BTreeMap<String, String> = serde_json::from_slice(&core.storage.readBytes(
                "runtime/link_access/space_merge_outbound.preferences.json").unwrap()).unwrap();
            let saved: serde_json::Value = serde_json::from_str(&records[&request.requestId]).unwrap();
            assert_eq!(saved["cancelRequested"], true, "lost cancellation must retain durable intent");
            assert_eq!(core.service.outgoingDeviceSpaceJoins().unwrap().into_iter()
                .find(|saved| saved.requestId == request.requestId).unwrap().status, SpaceJoinStatus::Pending);
            core.peers.stop().await.unwrap();
            core = core_node_with_storage(reopened(&core.storage));
            edge = edge_node(reopened(&edge.storage));
            start(&core, &edge).await;
            wait_for_peer_availability(&core.peers, &edge_id, true).await;
            wait_for_peer_availability(&edge.peers, &core_id, true).await;
            // The regular dialog/background refresh path retries saved intent;
            // the user does not have to submit another cancellation request.
            assert_eq!(core.service.refreshDeviceSpaceJoin(request.requestId.clone()).await.unwrap().status, SpaceJoinStatus::Cancelled);
            assert!(!edge.service.incomingDeviceSpaceJoins().await.unwrap().iter()
                .any(|saved| saved.requestId == request.requestId));
            assert!(edge.service.decideDeviceSpaceJoin(request.requestId, request.assignmentVersion, true).await.is_err());
            assert_eq!(core.service.deviceSpace().unwrap(), former);
            assert_eq!(edge.service.spaceStore().space().unwrap(), singleton);
            assert!(!edge.service.spaceStore().deviceProfiles().unwrap().contains_key(&core_id));
            assert_eq!(core.peers.pairedPeers().unwrap(), core_credentials);
            assert_eq!(edge.peers.pairedPeers().unwrap(), edge_credentials);
            assert_no_replica(&core, &edge);
            // Cancellation must also release the original request slot, so the
            // SAME applicant can apply again and complete a fresh admission.
            let next = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            edge.service.decideDeviceSpaceJoin(next.requestId.clone(), next.assignmentVersion, true).await.unwrap();
            assert_eq!(core.service.refreshDeviceSpaceJoin(next.requestId).await.unwrap().status, SpaceJoinStatus::Joined);
            assert_eq!(core.service.deviceSpace().unwrap(), edge.service.spaceStore().space().unwrap());
            assert_no_replica(&core, &edge);
        }).await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }

    /// A pending/claimed source is not authority to repair arbitrary member
    /// references: recovery requires the exact durable admission operation.
    #[tokio::test]
    async fn uncommitted_claim_cannot_restore_profiles_from_unapproved_source() {
        let _guard = TEST_LOCK.lock().await;
        let core = core_node();
        let edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            let core_id = core.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            let request = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            edge.service.acceptSpaceApprovalCall(&edge_id, CoreCallRequest::new(
                "uncommitted-claim", NODE_SPACE_APPROVAL_TARGET, "claim",
                operit_link::toCoreValue(serde_json::json!({"requestId": request.requestId,
                    "assignmentVersion": request.assignmentVersion, "approve": true})).unwrap(),
            )).await.unwrap();
            // Emulate an inconsistent member reference, without an admission.
            let mut inconsistent = edge.service.spaceStore().space().unwrap();
            inconsistent.members.push(core_id.clone()); inconsistent.spaceRevision += 1;
            edge.service.spaceStore().adopt(inconsistent).unwrap();
            assert!(!edge.service.spaceStore().deviceProfiles().unwrap().contains_key(&core_id));
            let before = edge.storage.0.lock().unwrap().clone();
            let decision = edge.service.decideDeviceSpaceJoin(request.requestId, request.assignmentVersion, true).await;
            assert!(decision.unwrap_err().contains("Device profile is missing"));
            assert!(!edge.service.spaceStore().deviceProfiles().unwrap().contains_key(&core_id));
            assert_eq!(*edge.storage.0.lock().unwrap(), before, "recovery fabricated a record without admission authority");
            assert_no_replica(&core, &edge);
        }).await;
        core.peers.stop().await.unwrap(); edge.peers.stop().await.unwrap(); result.unwrap();
    }

    #[tokio::test]
    async fn edge_can_leave_an_offline_admin_space_and_approve_a_new_core() {
        let _guard = TEST_LOCK.lock().await;
        let old = core_node();
        let new = core_node();
        let edge = fresh_edge();
        start(&old, &edge).await;
        new.peers.startListening(&[PeerTransport::Tcp]).await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let old_id = old.router.localNodeId();
            let new_id = new.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            // A normal desktop has presentation records from earlier Spaces.
            // They must remain on Core, not inflate this admission snapshot.
            let new_store = CoreSpaceStore::new(new.storage.clone());
            for index in 0..4 {
                new_store.importDeviceProfiles(vec![CoreSpaceDeviceProfile {
                    nodeId: format!("previous-space-peer-{index}"), displayName: format!("Unrelated desktop {index}"),
                    userName: String::new(), platform: "windows".into(), model: "Previous desktop".into(), coreVersion: None, updatedAt: 1,
                }]).unwrap();
            }
            pair(&edge.peers, &old.peers, &old.storage, old_id.clone()).await;
            let join = edge.service.requestDeviceSpaceJoin(old_id.clone()).await.unwrap();
            old.service.decideDeviceSpaceJoin(join.requestId.clone(), join.assignmentVersion, true).await.unwrap();
            assert_eq!(edge.service.refreshDeviceSpaceJoin(join.requestId).await.unwrap().status, SpaceJoinStatus::Joined);
            let previous = edge.service.spaceStore().space().unwrap();
            assert!(!edge.service.networkControlStore().nodeHasCapability(&edge_id, "network.approval", None).unwrap());
            assert!(!edge.service.networkControlStore().nodeHasCapability(&edge_id, "storage.provide", None).unwrap());
            edge.peers.removePairedPeer(&old_id).await.unwrap();
            old.peers.stop().await.unwrap();
            assert_eq!(edge.service.spaceStore().space().unwrap(), previous, "unpairing must not silently leave a Space");

            pair(&new.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            let stuck = new.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            assert_eq!(stuck.status, SpaceJoinStatus::Pending);
            assert_eq!(stuck.reviewerDeviceId, None, "the offline old administrator cannot review");
            assert!(edge.service.incomingDeviceSpaceJoins().await.unwrap().is_empty());
            assert_eq!(new.service.cancelDeviceSpaceJoin(stuck.requestId).await.unwrap().status, SpaceJoinStatus::Cancelled);
            let saved: BTreeMap<String, String> = serde_json::from_slice(&edge.storage.readBytes(
                "runtime/link_access/space_merge_inbound.preferences.json").unwrap()).unwrap();
            let receipt: serde_json::Value = serde_json::from_str(saved.values().next().unwrap()).unwrap();
            for field in ["deviceProfiles", "topology", "controlOperations"] {
                assert!(receipt["source"][field].as_array().unwrap().is_empty(), "cancelled request retained a complete source snapshot: {field}");
            }
            let credentials = edge.peers.pairedPeers().unwrap();
            let identity = operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(edge.storage.clone()).initialize().unwrap();

            // Firmware updates preserve NVS, including files from the retired
            // single-device schema. Exit must delete them, not decode them.
            let retired = [
                "runtime/link_access/space_join_inbound.preferences.json",
                "runtime/link_access/space_join_outbound.preferences.json",
                "runtime/link_access/space_join_review_inbox.preferences.json",
                "runtime/link_access/space_join_review_results.preferences.json",
            ];
            for path in retired {
                edge.storage.writeBytes(path, b"retired schema; deliberately not valid JSON").unwrap();
            }
            *edge.storage.3.lock().unwrap() = Some(0);
            assert!(edge.service.leaveDeviceSpace().is_err(), "failed cleanup must not report a successful exit");
            assert_eq!(edge.service.spaceStore().space().unwrap(), previous);
            assert!(edge.storage.exists(retired[0]).unwrap());
            print_size("before-leave", &edge.storage);
            let left = edge.service.leaveDeviceSpace().unwrap();
            for path in retired { assert!(!edge.storage.exists(path).unwrap(), "retired approval file survived exit: {path}"); }
            print_size("after-leave", &edge.storage);
            assert_ne!(left.spaceId, previous.spaceId);
            assert_eq!(left.members, vec![edge_id.clone()]);
            assert_eq!(edge.service.spaceStore().deviceProfiles().unwrap().len(), 1, "endpoint retained retired profile history");
            assert_eq!(edge.storage.list("runtime/space/members").unwrap().len(), 1, "endpoint retained retired membership history");
            assert_eq!(edge.peers.pairedPeers().unwrap(), credentials, "leave must preserve direct pairing");
            assert_eq!(operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(edge.storage.clone()).initialize().unwrap(), identity);
            assert_eq!(edge.service.spaceChannelScope(&old_id).unwrap(), None);
            assert_eq!(edge.service.spaceChannelScope(&new_id).unwrap(), None);
            assert!(edge.service.networkControlStore().nodeHasCapability(&edge_id, "network.approval", None).unwrap());
            let current_policy = edge.service.networkControlStore().currentSpaceOperations().unwrap();
            let current_audit = edge.service.networkControlStore().audit().unwrap();
            edge.service.spaceStore().pruneNodeLocalProjection().unwrap();
            edge.service.networkControlStore().pruneNodeLocalProjection().unwrap();
            assert_eq!(edge.service.networkControlStore().currentSpaceOperations().unwrap(), current_policy);
            assert_eq!(edge.service.networkControlStore().audit().unwrap(), current_audit, "cache eviction changed current authority");
            assert_eq!(old.service.deviceSpace().unwrap(), previous, "a local exit must not rewrite the old Core's Space");
            let restored = NodeSpaceService::new(reopened(&edge.storage), edge.peers.clone()).unwrap();
            assert_eq!(restored.spaceStore().space().unwrap(), left);
            assert!(restored.networkControlStore().nodeHasCapability(&edge_id, "network.approval", None).unwrap());

            let request = new.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            assert_eq!(request.status, SpaceJoinStatus::Pending);
            assert_eq!(request.reviewerDeviceId, Some(edge_id.clone()));
            let saved: BTreeMap<String, String> = serde_json::from_slice(&new.storage.readBytes(
                "runtime/link_access/space_merge_outbound.preferences.json").unwrap()).unwrap();
            let record: serde_json::Value = serde_json::from_str(&saved[&request.requestId]).unwrap();
            assert_eq!(record["source"]["deviceProfiles"].as_array().unwrap().len(), 1, "unrelated historical profiles entered the admission wire snapshot");
            let core_policy = new.service.networkControlStore().currentSpaceOperations().unwrap();
            let core_files = new.storage.0.lock().unwrap().clone();
            new_store.pruneNodeLocalProjection().unwrap();
            new.service.networkControlStore().pruneNodeLocalProjection().unwrap();
            assert_eq!(*new.storage.0.lock().unwrap(), core_files, "endpoint pruning changed full Core history or audit");
            assert_eq!(new.service.networkControlStore().currentSpaceOperations().unwrap(), core_policy);
            assert_eq!(new_store.deviceProfiles().unwrap().len(), 5, "history remains on its owning Core");
            let incoming = edge.service.incomingDeviceSpaceJoins().await.unwrap();
            assert_eq!(incoming.len(), 1);assert!(incoming[0].canApprove);
            edge.service.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await.unwrap();
            assert_eq!(new.service.refreshDeviceSpaceJoin(request.requestId).await.unwrap().status, SpaceJoinStatus::Joined);
            assert_eq!(edge.service.spaceChannelScope(&new_id).unwrap(), Some(left.spaceId.clone()));
            assert_eq!(new.service.deviceSpace().unwrap(), edge.service.spaceStore().space().unwrap());
            new.storage.writeBytes("workspaces/large/fixture.bin", &vec![0x5a; 1024 * 1024]).unwrap();
            assert!(!edge.storage.exists("runtime/link_access/space_merge_review_results.preferences.json").unwrap(),
                "local approval duplicated its admission operation instead of reusing the policy log");
            print_size("new-core-joined", &edge.storage);
            new.sync.synchronizeOnce().await.unwrap();
            assert_no_replica(&new, &edge);
            assert_no_replica(&old, &edge);
        }).await;
        old.peers.stop().await.unwrap();
        new.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }

    #[tokio::test]
    async fn full_core_never_replicates_business_data_to_a_non_storage_edge() {
        let _guard = TEST_LOCK.lock().await;
        let core = core_node();
        let edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(25), async {
            let core_id = core.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            pair(&edge.peers, &core.peers, &core.storage, core_id.clone()).await;
            let request = edge
                .service
                .requestDeviceSpaceJoin(core_id.clone())
                .await
                .unwrap();
            assert_eq!(request.status, SpaceJoinStatus::Pending);
            core.service.incomingDeviceSpaceJoins().await.unwrap();
            core.service
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await
                .unwrap();
            assert_eq!(
                edge.service
                    .refreshDeviceSpaceJoin(request.requestId)
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Joined
            );
            assert!(!core
                .service
                .networkControlStore()
                .nodeHasCapability(&edge_id, "storage.provide", None)
                .unwrap());
            assert!(!edge
                .service
                .networkControlStore()
                .nodeHasCapability(&edge_id, "storage.provide", None)
                .unwrap());
            // The normal node really has a file much larger than the board's budget.
            // Its production scanner creates a blob/log, but none may reach Edge.
            core.storage
                .writeBytes("workspaces/large/fixture.bin", &vec![0x5a; 1024 * 1024])
                .unwrap();
            core.sync.synchronizeOnce().await.unwrap();
            assert!(!core.storage.list("runtime/sync").unwrap().is_empty());
            assert_no_replica(&core, &edge);
            let space_id = core.service.deviceSpace().unwrap().spaceId;
            for method in [
                "syncClock",
                "syncOperationsSince",
                "syncApplyOperations",
                "syncBlobExists",
                "syncReadBlobChunk",
            ] {
                let response = edge
                    .peers
                    .call(
                        &core_id,
                        RoutedCoreRequest {
                            spaceId: space_id.clone(),
                            originNodeId: edge_id.clone(),
                            targetNodeId: core_id.clone(),
                            ttl: 0,
                            routeKind: RoutedCoreRequestKind::Target,
                            payload: CoreCallRequest::new(
                                "denied",
                                "$node.sync",
                                method,
                                operit_link::toCoreValue(serde_json::json!({"operations":[]}))
                                    .unwrap(),
                            ),
                        },
                    )
                    .await;
                assert_eq!(
                    response.result.unwrap_err().code,
                    "SPACE_STORAGE_PERMISSION_DENIED",
                    "{method}"
                );
            }
            let push = edge
                .peers
                .openPush(
                    &core_id,
                    RoutedCoreRequest {
                        spaceId: space_id,
                        originNodeId: edge_id.clone(),
                        targetNodeId: core_id.clone(),
                        ttl: 0,
                        routeKind: RoutedCoreRequestKind::Target,
                        payload: CorePushRequest::new(
                            "denied-blob",
                            "$node.sync",
                            "syncReceiveBlob",
                        ),
                    },
                )
                .await;
            match push {
                Ok(_) => panic!("non-storage Edge opened a blob receiver"),
                Err(e) => assert_eq!(e.code, "SPACE_STORAGE_PERMISSION_DENIED"),
            }
            assert!(core.service.pairedDeviceOnline(edge_id.clone()).unwrap());
            assert!(
                !core.peers.pairedPeers().unwrap()[0].outbound,
                "Space return credentials must not create a reverse pairing"
            );
            assert!(edge.peers.pairedPeers().unwrap()[0].outbound);
            // Permission independently blocks replication even when stale
            // metadata incorrectly advertises a full Core engine on the endpoint.
            let mut profile = core
                .service
                .spaceStore()
                .deviceProfiles()
                .unwrap()
                .remove(&edge_id)
                .unwrap();
            profile.coreVersion = Some("stale-core-version".into());
            profile.updatedAt += 1;
            core.service
                .spaceStore()
                .importDeviceProfiles(vec![profile.clone()])
                .unwrap();
            core.sync.synchronizeOnce().await.unwrap();
            assert_no_replica(&core, &edge);
            profile.coreVersion = None;
            profile.updatedAt += 1;
            core.service
                .spaceStore()
                .importDeviceProfiles(vec![profile])
                .unwrap();
            // Even an accidental role grant cannot install the absent Core store.
            core.service
                .setDeviceSpaceIdentity(NetworkControlIdentityAssignment {
                    nodeId: edge_id.clone(),
                    roleId: "storage".into(),
                })
                .unwrap();
            core.sync.synchronizeOnce().await.unwrap();
            assert!(edge
                .service
                .networkControlStore()
                .nodeHasCapability(&edge_id, "storage.provide", None)
                .unwrap(), "Policy grants are separate from persistence service availability");
            assert_eq!(
                core.service.spaceStore().deviceProfiles().unwrap()[&edge_id].coreVersion,
                None
            );
            assert_no_replica(&core, &edge);
            core.service
                .clearDeviceSpaceIdentity(edge_id.clone())
                .unwrap();
            core.sync.synchronizeOnce().await.unwrap();
            assert!(!edge
                .service
                .networkControlStore()
                .nodeHasCapability(&edge_id, "storage.provide", None)
                .unwrap());
            assert_no_replica(&core, &edge);
        })
        .await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }

    #[tokio::test]
    async fn full_core_edge_rejection_and_failed_cancellation_never_admit() {
        let _guard = TEST_LOCK.lock().await;
        let core = core_node();
        let edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            let edge_id = edge.service.localNodeId();
            let core_id = core.router.localNodeId();
            let original = core.service.deviceSpace().unwrap();
            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            let request = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            edge.service.incomingDeviceSpaceJoins().await.unwrap();
            assert_eq!(
                edge.service
                    .decideDeviceSpaceJoin(
                        request.requestId.clone(),
                        request.assignmentVersion,
                        false
                    )
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Rejected
            );
            assert_eq!(
                core.service
                    .refreshDeviceSpaceJoin(request.requestId)
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Rejected
            );
            let request = core.service.requestDeviceSpaceJoin(edge_id).await.unwrap();
            *core.storage.3.lock().unwrap() = Some(0);
            assert!(core
                .service
                .cancelDeviceSpaceJoin(request.requestId.clone())
                .await
                .is_err());
            assert_eq!(
                edge.service.incomingDeviceSpaceJoins().await.unwrap()[0].status,
                SpaceJoinStatus::Pending
            );
            assert_eq!(
                core.service
                    .cancelDeviceSpaceJoin(request.requestId.clone())
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Cancelled
            );
            assert_eq!(
                core.service
                    .refreshDeviceSpaceJoin(request.requestId)
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Cancelled
            );
            assert!(edge
                .service
                .incomingDeviceSpaceJoins()
                .await
                .unwrap()
                .is_empty());
            assert_eq!(core.service.deviceSpace().unwrap(), original);
            assert!(!edge
                .service
                .spaceStore()
                .space()
                .unwrap()
                .members
                .contains(&core_id));
            assert_eq!(edge.service.spaceChannelScope(&core_id).unwrap(), None);
            assert_no_replica(&core, &edge);
        })
        .await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }

    #[tokio::test]
    async fn full_core_edge_approval_retry_and_restart_keep_the_original_pairing() {
        let _guard = TEST_LOCK.lock().await;
        let core = core_node();
        let mut edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(25), async {
            let edge_id = edge.service.localNodeId();
            let core_id = core.router.localNodeId();
            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            let request = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            edge.service.incomingDeviceSpaceJoins().await.unwrap();
            *edge.storage.3.lock().unwrap() = Some(0);
            assert!(edge
                .service
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await
                .is_err());
            assert!(!edge
                .service
                .spaceStore()
                .space()
                .unwrap()
                .members
                .contains(&core_id));
            edge.peers.stop().await.unwrap();
            wait_for_peer_availability(&core.peers, &edge_id, false).await;
            edge = edge_node(reopened(&edge.storage));
            edge.peers
                .startListening(&[PeerTransport::Tcp])
                .await
                .unwrap();
            assert_eq!(edge.service.localNodeId(), edge_id);
            wait_for_peer_availability(&core.peers, &edge_id, true).await;
            assert_eq!(
                edge.service.incomingDeviceSpaceJoins().await.unwrap()[0].requestId,
                request.requestId
            );
            edge.service
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await
                .unwrap();
            assert_eq!(
                core.service
                    .refreshDeviceSpaceJoin(request.requestId)
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Joined
            );
            let expected_space = edge.service.spaceStore().space().unwrap();
            // Reconstruct identity, store caches, router, peer engine and listener.
            edge.peers.stop().await.unwrap();
            wait_for_peer_availability(&core.peers, &edge_id, false).await;
            edge = edge_node(reopened(&edge.storage));
            edge.peers
                .startListening(&[PeerTransport::Tcp])
                .await
                .unwrap();
            assert_eq!(edge.service.spaceStore().space().unwrap(), expected_space);
            tokio::time::timeout(Duration::from_secs(8), async {
                while !core.service.pairedDeviceOnline(edge_id.clone()).unwrap()
                    || !edge.peers.activePeerNodeIds().unwrap().contains(&core_id)
                {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap();
            assert!(core.peers.pairingPrompts().unwrap().is_empty());
            assert!(edge.peers.pairingPrompts().unwrap().is_empty());
            assert_eq!(core.peers.pairedPeers().unwrap().len(), 1);
            assert_eq!(edge.peers.pairedPeers().unwrap().len(), 1);
            assert!(edge.service.spaceChannelScope(&core_id).unwrap().is_some());
            core.sync.synchronizeOnce().await.unwrap();
            assert_no_replica(&core, &edge);
        })
        .await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }
}
