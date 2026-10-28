//! Real TCP, shared pairing and shared admission, with no application runtime.
#[path = "../../../src/runtime_storage_pack.rs"]
mod runtime_storage_pack;
use runtime_storage_pack as pack;
#[path = "../../../src/runtime_storage_codec.rs"]
mod codec;
use operit_edge_sdk::device::*;
use std::sync::Arc;
use operit_link::*;
use operit_host_api::HostManager::HostManager;
use operit_edge_sdk::device::PeerRouter::EdgePeerRouter;
use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
use operit_host_native_common::{NativeHostRuntimeTaskSchedulerHost, NativeTcpHost};
use operit_edge_sdk::runtime::HostRuntimePeerService::HostRuntimePeerService;
use operit_edge_sdk::runtime::NodeServices::{NodeServices, PeerEndpoint, PeerTransport};
use operit_edge_sdk::runtime::NodeSpaceService::{
    NodeSpaceContext, NodeSpaceService, SpaceJoinStatus, NODE_SPACE_APPROVAL_TARGET,
    NODE_SPACE_TARGET,
};
use operit_edge_sdk::runtime::PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore};
use operit_edge_sdk::runtime::RuntimePeerService::RuntimePeerService;
use operit_edge_sdk::storage::CoreSpaceStore::CoreSpaceDeviceProfile;
use std::collections::BTreeMap;
use std::sync::Mutex;
use async_trait::async_trait;

#[derive(Default)]
struct Storage(
    Mutex<BTreeMap<String, Vec<u8>>>,
    bool,
    Mutex<codec::tests::PhysicalNvs>,
    Mutex<Option<usize>>,
);
impl Storage {
    fn update(&self, change: impl FnOnce(&mut BTreeMap<String, Vec<u8>>)) -> HostResult<()> {
        let mut files = self.0.lock().unwrap();
        let mut next = files.clone();
        change(&mut next);
        if next == *files {
            return Ok(());
        }
        let mut failure = self.3.lock().unwrap();
        if let Some(remaining) = failure.as_mut() {
            if *remaining == 0 {
                *failure = None;
                return Err(HostError::new("injected approval persistence failure"));
            }
            *remaining -= 1;
        }
        drop(failure);
        if self.1 {
            let packed: codec::Files = next
                .iter()
                .map(|(path, bytes)| pack::pack(bytes).map(|bytes| (path.clone(), bytes)))
                .collect::<HostResult<_>>()?;
            let mut nvs = self.2.lock().unwrap();
            let active = nvs.active_bank();
            codec::commit_snapshot_transaction(&packed, active, &mut *nvs)?;
            assert_eq!(codec::decode_packed_files(&nvs.selected()).unwrap(), packed);
        }
        *files = next;
        Ok(())
    }
    fn write(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.update(|files| {
            files.insert(path.into(), bytes.into());
        })
    }
}
impl RuntimeStorageHost for Storage {
    fn retainPreferencesCache(&self) -> bool { !self.1 }
    fn nodeLocalApplicationHistoryLimit(&self) -> Option<usize> { self.1.then_some(2) }

    /// Confirms synchronous in-memory publication without copying or rewriting bytes.
    fn flushFile(&self, path: &str) -> operit_host_api::HostResult<()> {
        if !self.exists(path)? {
            return Err(operit_host_api::HostError::new(
                "runtime storage entry not found",
            ));
        }
        Ok(())
    }

    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }

    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.0
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| HostError::new("missing"))
    }
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let files = self.0.lock().unwrap();
        let bytes = files.get(path).ok_or_else(|| HostError::new("missing"))?;
        let start = usize::try_from(offset).unwrap_or(bytes.len()).min(bytes.len());
        let end = start.saturating_add(length).min(bytes.len());
        Ok(bytes[start..end].to_vec())
    }
    /// Atomically publishes one complete in-memory test entry.
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        self.writeBytes(path, content)
    }

    fn writeBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.write(path, bytes)
    }
    fn appendBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.update(|files| {
            files
                .entry(path.into())
                .or_default()
                .extend_from_slice(bytes);
        })
    }
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.update(|files| {
            let prefix = format!("{path}/");
            files.retain(|key, _| key != path && (!recursive || !key.starts_with(&prefix)));
        })
    }
    fn exists(&self, path: &str) -> HostResult<bool> {
        let files = self.0.lock().unwrap();
        // Full Core workspace scanning needs native-style directory existence.
        // The bounded ESP32 fixture retains its exact virtual-entry semantics.
        Ok(files.contains_key(path)
            || (!self.1 && files.keys().any(|key| key.starts_with(&format!("{path}/")))))
    }
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path.starts_with(prefix))
            .map(|(path, bytes)| RuntimeStorageEntry {
                path: path.clone(),
                isDirectory: false,
                size: bytes.len() as i64,
            })
            .collect())
    }
}

/// Reads one peer state preferences file the way the product store does after
/// the native JSON migration: structured JSON entries keyed by record id, not
/// the pre-migration `{id: "<record json string>"}` blobs.
fn storedRecords(storage: &Arc<Storage>, path: &str) -> BTreeMap<String, serde_json::Value> {
    let preferences = operit_edge_sdk::storage::PreferencesDataStore::CoreNodeStateStore::newWithStorage(
        storage.clone(),
        path,
    )
    .data()
    .expect("peer state preferences must parse");
    preferences
        .iterEntries()
        .filter(|(name, _)| {
            *name != operit_edge_sdk::storage::PreferencesDataStore::PREFERENCES_SCHEMA_VERSION_KEY_NAME
        })
        .map(|(name, value)| (name.to_owned(), value.clone()))
        .collect()
}

fn node(
    bounded: bool,
) -> (
    Arc<Storage>,
    Arc<EdgePeerRouter>,
    Arc<HostRuntimePeerService>,
    Arc<NodeSpaceService>,
) {
    node_from_storage(Arc::new(Storage(
        Mutex::default(),
        bounded,
        Mutex::default(),
        Mutex::default(),
    )))
}
fn node_from_storage(
    storage: Arc<Storage>,
) -> (
    Arc<Storage>,
    Arc<EdgePeerRouter>,
    Arc<HostRuntimePeerService>,
    Arc<NodeSpaceService>,
) {
    let bounded = storage.1;
    // Latest main migrates an empty host config eagerly; configure fresh test
    // storage before that migration can hide the absence of saved settings.
    let configureListener = !storage
        .exists("runtime/link_access/host_config.preferences.json")
        .unwrap();
    let id = operit_edge_sdk::storage::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage.clone())
        .initialize()
        .unwrap()
        .nodeId;
    let router = EdgePeerRouter::new(id.clone());
    let host = Arc::new(HostManager {
        runtimeStorageHost: Some(storage.clone()),
        tcpHost: Some(Arc::new(NativeTcpHost)),
        hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost)),
        ..HostManager::default()
    });
    let peer = operit_edge_sdk::device::peer::createPeerService(
        host.clone(),
        &router,
        operit_link::protocol::LinkDeviceInfo {
            platform: "test".into(),
            model: "test".into(),
            pairingIncarnation: None,
        },
        operit_edge_sdk::runtime::HostRuntimePeerService::PeerRuntimeLimits::default(),
    )
    .unwrap();
    let space = NodeSpaceService::new(storage.clone(), peer.clone()).unwrap();
    space
        .initialize(CoreSpaceDeviceProfile {
            nodeId: id,
            displayName: if bounded {
                "ESP32-2432S028"
            } else {
                "HAN-DESKTOP (Windows)"
            }
            .into(),
            userName: String::new(),
            platform: if bounded { "esp32" } else { "windows" }.into(),
            model: if bounded {
                "ESP32-2432S028"
            } else {
                "Windows desktop"
            }
            .into(),
            coreVersion: None,
            pairingIncarnation: None,
            updatedAt: 1,
        })
        .unwrap();
    router.installSpace(space.clone()).unwrap();
    router
        .install(Arc::new(
            EdgeNode::fromHostManager((*host).clone())
                .withNodeServices(NodeServices::new(peer.clone())),
        ))
        .unwrap();
    if configureListener {
        PeerStateStore::new(storage.clone())
            .saveHostConfig(&PeerHostConfig {
                bindAddress: "127.0.0.1:0".into(),
                transports: vec![PeerTransport::Tcp],
                discoveryEnabled: false,
                portMode: PeerHostPortMode::Automatic,
                updatedAt: 1,
            })
            .unwrap();
    }
    (storage, router, peer, space)
}
// A listener restart retires the previous pooled TCP generation asynchronously.
// Wait for discovery to reconnect, rather than assuming the old lease can survive.
async fn wait_for_peer_availability(peers: &HostRuntimePeerService, node: &str, online: bool) {
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        while peers
            .activePeerNodeIds()
            .unwrap()
            .iter()
            .any(|peer| peer == node)
            != online
        {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("peer availability did not converge after listener restart");
}

#[tokio::test]
async fn endpoint_restart_completes_durable_member_removal_without_peer_redelivery() {
    let (storage, _, peers, service) = node(true);
    let peer = "retired-offline-peer";
    service.spaceStore().admitRemoteMember(peer.into(), "Peer".into(), "test".into(), "test".into(), "test".into()).unwrap();
    service.networkControlStore().admitMember(peer.into()).unwrap();
    let original = service.spaceStore().space().unwrap();
    let profile = service.spaceStore().deviceProfiles().unwrap().remove(&service.localNodeId()).unwrap();
    service.networkControlStore().removeMember(peer.into()).unwrap();
    assert!(service.spaceStore().space().unwrap().members.contains(&peer.to_string()),
        "fixture must retain the interrupted membership projection");
    let identity = operit_edge_sdk::storage::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage.clone()).initialize().unwrap();
    let authority = service.networkControlStore().currentSpaceOperations().unwrap();
    let restored = NodeSpaceService::new(storage.clone(), peers).unwrap();
    restored.initialize(profile.clone()).unwrap();
    let current = restored.spaceStore().space().unwrap();
    assert_eq!(current.spaceId, original.spaceId);
    assert_eq!(current.members, vec![identity.nodeId.clone()]);
    assert!(!restored.spaceStore().deviceProfiles().unwrap().contains_key(peer));
    assert_eq!(restored.networkControlStore().currentSpaceOperations().unwrap(), authority);
    let before = storage.0.lock().unwrap().clone();
    restored.initialize(profile).unwrap();
    assert_eq!(*storage.0.lock().unwrap(), before, "completed startup recovery must be idempotent");
    assert_eq!(operit_edge_sdk::storage::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage).initialize().unwrap(), identity);
}

fn print_size(stage: &str, storage: &Storage) {
    let files = storage.0.lock().unwrap();
    let packed: codec::Files = files
        .iter()
        .map(|(path, bytes)| (path.clone(), pack::pack(bytes).unwrap()))
        .collect();
    let mut snapshot = Vec::new();
    codec::commit_packed_snapshot(
        &packed,
        0,
        |_, chunk| {
            snapshot.extend_from_slice(&pack::unpack(chunk).unwrap());
            Ok(())
        },
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(codec::decode_packed_files(&snapshot).unwrap(), packed);
    println!(
        "NVS_ENVELOPE {stage}: {} bytes; compressed transaction: {} entries",
        snapshot.len(),
        codec::snapshot_required_entries(&packed).unwrap()
    );

    for (path, bytes) in packed.iter() {
        assert_eq!(&pack::unpack(bytes).unwrap(), files.get(path).unwrap());
    }
    for (path, bytes) in files.iter() {
        println!("  {path}: {} / {} packed", bytes.len(), packed[path].len());
    }
}
/// Prunes consent receipts through projections without reloading their command evidence.
#[tokio::test]
async fn constrained_gateway_prunes_receipts_without_decoding_or_rewriting_evidence() {
    use operit_edge_sdk::storage::PreferencesDataStore::{CoreNodeStateStore, stringPreferencesKey};
    let (a_store, _a_router, a_peer, a_space) = node(false);
    let (b_store, _b_router, b_peer, b_space) = node(true);
    let result = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        a_peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
        b_peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
        let b_id = b_space.localNodeId();
        let address = PeerStateStore::new(b_store.clone()).hostConfig().unwrap().unwrap().bindAddress;
        let pairing = a_peer.startPairing(PeerEndpoint { nodeId: b_id.clone(), address }, Some(PeerTransport::Tcp)).await.unwrap();
        let code = b_peer.pairingPrompts().unwrap()[0].confirmationCode.clone();
        a_peer.finishPairing(&pairing.pairingId, &code).await.unwrap();
        let request = a_space.requestDeviceSpaceJoin(b_id).await.unwrap();
        let path = "runtime/link_access/space_merge_inbound.preferences.json";
        let mut pending = storedRecords(&b_store, path)[&request.requestId].clone();
        for field in ["accepted", "unavailableSince", "approvedDecision", "decisionRevision", "cancelRequested"] {
            assert!(pending.get(field).is_none(), "default field {field} needlessly occupies endpoint storage");
        }
        assert!(pending["source"]["controlOperations"].as_array().unwrap().is_empty());
        let outbound = storedRecords(&a_store, "runtime/link_access/space_merge_outbound.preferences.json");
        let sourceOperations = outbound[&request.requestId]["source"]["controlOperations"].clone();
        assert!(!sourceOperations.as_array().unwrap().is_empty(), "outgoing retry lost its source authority");
        // Lists need consent facts only. Evidence remains opaque until a command validates it.
        pending["source"]["controlOperations"] = serde_json::json!({"unreadableEvidence": true});
        CoreNodeStateStore::newWithStorage(b_store.clone(), path).edit(|prefs| {
            prefs.set(&stringPreferencesKey(&request.requestId), pending.clone());
            for index in 0..6 {
                let mut record = pending.clone();
                let id = format!("retired-{index}");
                record["request"]["requestId"] = serde_json::json!(id);
                record["request"]["status"] = serde_json::json!("expired");
                record["request"]["createdAt"] = serde_json::json!(index);
                for field in ["controlOperations", "deviceProfiles", "topology"] {
                    record["source"][field] = serde_json::json!([]);
                }
                record["accepted"] = record["source"].clone();
                prefs.set(&stringPreferencesKey(&id), record);
            }
        }).unwrap();
        let before = storedRecords(&b_store, path);
        *b_store.3.lock().unwrap() = Some(0);
        assert!(b_space.incomingDeviceSpaceJoins().await.unwrap().is_empty());
        assert_eq!(storedRecords(&b_store, path), before, "failed publication retired an application");
        let incoming = b_space.incomingDeviceSpaceJoins().await.unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].requestId, request.requestId);
        let retained = storedRecords(&b_store, path);
        assert_eq!(retained.len(), 3);
        assert!(retained.contains_key("retired-4") && retained.contains_key("retired-5"));
        assert_eq!(retained[&request.requestId], pending, "a list query rewrote application evidence");
        assert!(b_space.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await.is_err(),
            "a read projection must never authorize undecodable application evidence");
        for id in ["retired-4", "retired-5"] {
            assert_eq!(retained[id]["accepted"], before[id]["accepted"], "terminal queries rewrote obsolete evidence");
            for field in ["controlOperations", "deviceProfiles", "topology"] {
                assert!(retained[id]["source"][field].as_array().unwrap().is_empty());
            }
        }
        assert_eq!(b_space.spaceStore().space().unwrap().members, vec![b_space.localNodeId()]);
        assert_eq!(a_space.spaceChannelScope(&b_space.localNodeId()).unwrap(), None);
        a_space.refreshDeviceSpaceJoin(request.requestId.clone()).await.unwrap();
        let outbound = storedRecords(&a_store, "runtime/link_access/space_merge_outbound.preferences.json");
        assert!(!outbound[&request.requestId]["source"]["controlOperations"].as_array().unwrap().is_empty(),
            "compacted gateway response changed the original outgoing submission");
    }).await;
    a_peer.stop().await.unwrap();
    b_peer.stop().await.unwrap();
    result.unwrap();
}

#[tokio::test]
async fn paired_edge_requires_explicit_approval_and_restores_space() {
    let (a_store, _a_router, a_peer, a_space) = node(false);
    let (b_store, b_router, b_peer, b_space) = node(true);
    print_size("bootstrap", &b_store);
    let result = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        a_peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
        b_peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
        let address = PeerStateStore::new(b_store.clone())
            .hostConfig()
            .unwrap()
            .unwrap()
            .bindAddress;
        let b_id = b_space.localNodeId();
        let a_id = a_space.localNodeId();
        let pairing = a_peer
            .startPairing(
                PeerEndpoint {
                    nodeId: b_id.clone(),
                    address,
                },
                Some(PeerTransport::Tcp),
            )
            .await
            .unwrap();
        let code = b_peer.pairingPrompts().unwrap()[0].confirmationCode.clone();
        a_peer
            .finishPairing(&pairing.pairingId, &code)
            .await
            .unwrap();
        // Typed encoded replies preserve the ordinary snapshot result and all
        // local route checks, including direct-origin and current pairing.
        {
            use operit_edge_sdk::runtime::PeerRouter::PeerRouter as _;
            use operit_link::{CoreCallRequest, CoreLinkResponse, CoreValue, RoutedCoreRequest, RoutedCoreRequestKind};
            let call = CoreCallRequest::new("snapshot-parity", NODE_SPACE_TARGET, "snapshot", CoreValue::Null);
            let routed = RoutedCoreRequest {
                spaceId: a_space.spaceStore().initialize().unwrap().spaceId,
                originNodeId: a_id.clone(), targetNodeId: b_id.clone(), ttl: 0,
                routeKind: RoutedCoreRequestKind::Target, payload: call.clone(),
            };
            let encoded = b_router.prepareRoutedCall(&a_id, &routed).unwrap().unwrap();
            let CoreLinkResponse::Call(decoded) = operit_link::decodeLink(&encoded).unwrap() else { panic!("Call reply expected") };
            assert_eq!(decoded.requestId, call.requestId);
            assert_eq!(decoded.result.unwrap(), b_space.acceptPeerSpaceCall(&a_id, call).unwrap());
            for invalid in 0..4 {
                let mut altered = routed.clone();
                match invalid {
                    0 => altered.targetNodeId = "other".into(),
                    1 => altered.originNodeId = "other".into(),
                    2 => altered.ttl = 1,
                    _ => altered.routeKind = RoutedCoreRequestKind::SpaceRoute,
                }
                let encoded = b_router.prepareRoutedCall(&a_id, &altered).unwrap().unwrap();
                let CoreLinkResponse::Call(decoded) = operit_link::decodeLink(&encoded).unwrap() else { panic!("Call reply expected") };
                assert!(decoded.result.is_err());
            }
            let encoded = b_router.prepareRoutedCall("unpaired", &RoutedCoreRequest { originNodeId: "unpaired".into(), ..routed }).unwrap().unwrap();
            let CoreLinkResponse::Call(decoded) = operit_link::decodeLink(&encoded).unwrap() else { panic!("Call reply expected") };
            assert!(decoded.result.is_err());
        }
        print_size("paired", &b_store);
        assert!(
            b_store.list("runtime/sync").unwrap().is_empty(),
            "endpoint pairing must not create a replica journal"
        );
        assert_eq!(b_space.spaceChannelScope(&a_id).unwrap(), None);
        let request = a_space.requestDeviceSpaceJoin(b_id.clone()).await.unwrap();
        assert_eq!(request.status, SpaceJoinStatus::Pending);
        assert!(!b_space
            .spaceStore()
            .initialize()
            .unwrap()
            .members
            .contains(&a_id));
        let incoming = b_space.incomingDeviceSpaceJoins().await.unwrap();
        assert_eq!(incoming.len(), 1);
        assert!(incoming[0].canApprove);
        print_size("pending", &b_store);
        assert!(b_space
            .decideDeviceSpaceJoin(
                request.requestId.clone(),
                request.assignmentVersion + 1,
                true
            )
            .await
            .is_err());
        assert_eq!(
            b_space
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await
                .unwrap()
                .status,
            SpaceJoinStatus::Approved
        );
        let revision = b_space.spaceStore().space().unwrap().spaceRevision;
        assert_eq!(
            b_space
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await
                .unwrap()
                .status,
            SpaceJoinStatus::Approved
        );
        assert_eq!(
            b_space.spaceStore().space().unwrap().spaceRevision,
            revision
        );
        assert_eq!(
            a_space
                .refreshDeviceSpaceJoin(request.requestId)
                .await
                .unwrap()
                .status,
            SpaceJoinStatus::Joined
        );
        assert_eq!(
            a_space.spaceStore().space().unwrap(),
            b_space.spaceStore().space().unwrap()
        );
        assert!(!a_space
            .networkControlStore()
            .nodeHasCapability(&a_id, "network.approval", None)
            .unwrap());
        {
            use operit_edge_sdk::runtime::PeerRouter::PeerRouter as _;
            let snapshot = b_space.acceptPeerSpaceCall(&a_id, CoreCallRequest::new("snapshot", NODE_SPACE_TARGET, "snapshot", CoreValue::Null)).unwrap();
            let call = CoreCallRequest::new("typed-observe", NODE_SPACE_TARGET, "observeSpaceSnapshot", snapshot);
            let routed = operit_link::RoutedCoreRequest {
                spaceId: b_space.spaceStore().space().unwrap().spaceId,
                originNodeId: a_id.clone(), targetNodeId: b_id.clone(), ttl: 0,
                routeKind: operit_link::RoutedCoreRequestKind::Target, payload: call.clone(),
            };
            let encode = |route: operit_link::RoutedCoreRequest<CoreCallRequest>, mismatch: bool| {
                let mut wire = route.payload.clone();
                wire.args = operit_link::toCoreValue(route).unwrap();
                if mismatch { wire.requestId = operit_link::CoreRequestId::new("mismatch"); }
                operit_link::encodeLink(operit_link::CoreLinkRequest::Call(wire)).unwrap()
            };
            let mut ingress = encode(routed.clone(), false);
            let encoded = b_router.prepareEncodedCall(&a_id, &mut ingress).unwrap().unwrap();
            assert!(ingress.is_empty(), "handled ingress retained its consumed frame");
            let mut unsupported = routed.clone();
            unsupported.payload.methodName = "snapshot".into();
            let mut untouched = encode(unsupported, false);
            let original = untouched.clone();
            assert!(b_router.prepareEncodedCall(&a_id, &mut untouched).is_none());
            assert_eq!(untouched, original, "fallback ingress was consumed");
            let operit_link::CoreLinkResponse::Call(decoded) = operit_link::decodeLink(&encoded).unwrap() else { panic!("Call reply expected") };
            assert_eq!(decoded.result.unwrap(), b_space.acceptPeerSpaceCall(&a_id, call).unwrap());
            let mut control = routed.clone();
            control.ttl = 4; // Same-Space control routes retain their ordinary positive budget.
            control.payload.target = operit_edge_sdk::runtime::NodeSpaceService::NODE_SPACE_CONTROL_TARGET.into();
            control.payload.methodName = "exchange".into();
            control.payload.args = operit_link::toCoreValue(b_space.networkControlStore().currentSpaceOperations().unwrap()).unwrap();
            let expected = operit_edge_sdk::runtime::NodeSpaceService::acceptSpaceControlCall(b_space.as_ref(), &a_id, control.payload.clone()).unwrap();
            let mut ingress = encode(control.clone(), false);
            let encoded = b_router.prepareEncodedCall(&a_id, &mut ingress).unwrap().unwrap();
            assert!(ingress.is_empty());
            let operit_link::CoreLinkResponse::Call(decoded) = operit_link::decodeLink(&encoded).unwrap() else { panic!("Call reply expected") };
            assert_eq!(decoded.result.unwrap(), expected);
            let mut bounded = control.clone();
            bounded.payload.methodName = "exchangeBounded".into();
            bounded.payload.args = operit_link::toCoreValue(serde_json::json!({
                "version": 1, "maxMessageBytes": b_peer.maxMessageBytes(),
                "operations": b_space.networkControlStore().currentSpaceOperations().unwrap(),
            })).unwrap();
            let expected = operit_edge_sdk::runtime::NodeSpaceService::acceptSpaceControlCall(b_space.as_ref(), &a_id, bounded.payload.clone()).unwrap();
            let mut ingress = encode(bounded, false);
            let encoded = b_router.prepareEncodedCall(&a_id, &mut ingress).unwrap().unwrap();
            assert!(ingress.is_empty());
            let operit_link::CoreLinkResponse::Call(decoded) = operit_link::decodeLink(&encoded).unwrap() else { panic!("Call reply expected") };
            assert_eq!(decoded.result.unwrap(), expected);
            for invalid in 0..3 {
                let mut altered = control.clone();
                match invalid {
                    0 => altered.spaceId = "other-space".into(),
                    1 => altered.targetNodeId = "other-node".into(),
                    _ => altered.originNodeId = "other-peer".into(),
                }
                let encoded = b_router.prepareEncodedCall(&a_id, &mut encode(altered, false)).unwrap().unwrap();
                let operit_link::CoreLinkResponse::Call(decoded) = operit_link::decodeLink(&encoded).unwrap() else { panic!("Call reply expected") };
                assert!(decoded.result.is_err());
            }
            for invalid in 0..5 {
                let mut altered = routed.clone();
                match invalid {
                    0 => altered.targetNodeId = "other".into(),
                    1 => altered.originNodeId = "other".into(),
                    2 => altered.ttl = 1,
                    3 => altered.routeKind = operit_link::RoutedCoreRequestKind::SpaceRoute,
                    _ => {},
                }
                let encoded = b_router.prepareEncodedCall(&a_id, &mut encode(altered, invalid == 4)).unwrap().unwrap();
                let operit_link::CoreLinkResponse::Call(decoded) = operit_link::decodeLink(&encoded).unwrap() else { panic!("Call reply expected") };
                assert!(decoded.result.is_err());
            }
        }
        print_size("approved", &b_store);
        assert!(
            b_store.list("runtime/sync").unwrap().is_empty(),
            "endpoint approval must not create replica clocks or logs"
        );
        for record in storedRecords(
            &b_store,
            "runtime/link_access/space_merge_inbound.preferences.json",
        )
        .into_values()
        {
            if !record["accepted"].is_null() {
                assert_eq!(
                    record["accepted"]["controlOperations"]
                        .as_array()
                        .unwrap()
                        .len(),
                    0
                );
            }
        }
        let restored = NodeSpaceService::new(b_store.clone(), b_peer.clone()).unwrap();
        assert_eq!(
            restored.spaceChannelScope(&a_id).unwrap(),
            b_space.spaceChannelScope(&a_id).unwrap()
        );
        // Spoofed/relayed admission cannot use the paired peer as an origin.
        use operit_edge_sdk::runtime::PeerRouter::PeerRouter;
        let wrong_space = operit_link::RoutedCoreRequest {
            spaceId: "other-space".into(),
            originNodeId: a_id.clone(),
            targetNodeId: b_id.clone(),
            ttl: 0,
            routeKind: operit_link::RoutedCoreRequestKind::Target,
            payload: CoreCallRequest::new(
                "wrong-space",
                NODE_SPACE_APPROVAL_TARGET,
                "assigned",
                CoreValue::Null,
            ),
        };
        assert_eq!(
            b_router
                .routedCall(a_id.clone(), wrong_space)
                .await
                .result
                .unwrap_err()
                .code,
            "SPACE_ID_MISMATCH"
        );
        let forged = operit_link::RoutedCoreRequest {
            spaceId: String::new(),
            originNodeId: "forged".into(),
            targetNodeId: b_id,
            ttl: 0,
            routeKind: operit_link::RoutedCoreRequestKind::Target,
            payload: CoreCallRequest::new("forged", NODE_SPACE_TARGET, "snapshot", CoreValue::Null),
        };
        assert_eq!(
            b_router
                .routedCall(a_id, forged)
                .await
                .result
                .unwrap_err()
                .code,
            "PEER_DIRECT_CALL_REQUIRED"
        );
    })
    .await;
    a_peer.stop().await.unwrap();
    b_peer.stop().await.unwrap();
    result.unwrap();
    drop(a_store);
}

#[tokio::test]
async fn failed_local_cancellation_preserves_pending_request_and_can_retry() {
    let (storage, _applicantRouter, applicantPeer, applicant) = node(false);
    let (gatewayStorage, _gatewayRouter, gatewayPeer, gateway) = node(true);
    applicantPeer
        .startListening(&[PeerTransport::Tcp])
        .await
        .unwrap();
    gatewayPeer
        .startListening(&[PeerTransport::Tcp])
        .await
        .unwrap();
    let address = PeerStateStore::new(gatewayStorage)
        .hostConfig()
        .unwrap()
        .unwrap()
        .bindAddress;
    let pairing = applicantPeer
        .startPairing(
            PeerEndpoint {
                nodeId: gateway.localNodeId(),
                address,
            },
            Some(PeerTransport::Tcp),
        )
        .await
        .unwrap();
    let code = gatewayPeer.pairingPrompts().unwrap()[0]
        .confirmationCode
        .clone();
    applicantPeer
        .finishPairing(&pairing.pairingId, &code)
        .await
        .unwrap();
    let request = applicant
        .requestDeviceSpaceJoin(gateway.localNodeId())
        .await
        .unwrap();
    let before = applicant.spaceStore().space().unwrap();

    *storage.3.lock().unwrap() = Some(0);
    let error = applicant
        .cancelDeviceSpaceJoin(request.requestId.clone())
        .await
        .unwrap_err();
    assert!(
        error.contains("injected approval persistence failure"),
        "{error}"
    );
    let saved = storedRecords(
        &storage,
        "runtime/link_access/space_merge_outbound.preferences.json",
    );
    let record = saved[&request.requestId].clone();
    assert_eq!(record["request"]["status"], "pending");
    assert_eq!(
        gateway.incomingDeviceSpaceJoins().await.unwrap()[0].status,
        SpaceJoinStatus::Pending
    );
    assert_eq!(applicant.spaceStore().space().unwrap(), before);

    let cancelled = applicant
        .cancelDeviceSpaceJoin(request.requestId.clone())
        .await
        .unwrap();
    assert_eq!(cancelled.status, SpaceJoinStatus::Cancelled);
    let saved = storedRecords(
        &storage,
        "runtime/link_access/space_merge_outbound.preferences.json",
    );
    let record = saved[&request.requestId].clone();
    assert_eq!(record["request"]["status"], "cancelled");
    assert!(gateway.incomingDeviceSpaceJoins().await.unwrap().is_empty());
    assert_eq!(applicant.spaceStore().space().unwrap(), before);
    applicantPeer.stop().await.unwrap();
    gatewayPeer.stop().await.unwrap();
}

/// Cleanup must not evict the cache when the host rejects the durable delete.
#[test]
fn node_local_delete_failure_preserves_shared_snapshot_and_durable_file() {
    use operit_edge_sdk::storage::PreferencesDataStore::{
        emptyPreferences, stringPreferencesKey, CoreNodeStateStore,
    };
    let storage = Arc::new(Storage::default());
    let path = "runtime/space/device_profiles/delete-failure.preferences.json";
    let first = CoreNodeStateStore::newWithStorage(storage.clone(), path);
    let second = CoreNodeStateStore::newWithStorage(storage.clone(), path);
    let mut snapshot = emptyPreferences();
    snapshot.set(
        &stringPreferencesKey("record"),
        "unchanged-profile".to_string(),
    );
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
            let pairing = applicant_peer.startPairing(PeerEndpoint { nodeId: gateway_id.clone(), address }, Some(PeerTransport::Tcp)).await.unwrap();
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
                let record: operit_edge_sdk::storage::NetworkControlStore::NetworkControlCommandRecord = serde_json::from_value(operation.payload.clone()).unwrap();
                matches!(record.command, operit_edge_sdk::storage::NetworkControlStore::NetworkControlCommand::AdmitSpace { identities, .. } if identities.contains_key(&applicant_id))
            }).count();
            assert_eq!(admissions, 1, "failure point {failure} appended duplicate admissions");
            // Reopen fresh storage identity: no process-shared store caches.
            let reopened = Arc::new(Storage(Mutex::new(storage.0.lock().unwrap().clone()), true, Mutex::new(storage.2.lock().unwrap().clone()), Mutex::default()));
            let restored = NodeSpaceService::new(reopened, gateway_peer.clone()).unwrap();
            assert_eq!(restored.spaceChannelScope(&applicant_id).unwrap(), gateway.spaceChannelScope(&applicant_id).unwrap());
        }).await;
        applicant_peer.stop().await.unwrap();
        gateway_peer.stop().await.unwrap();
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
    use operit_edge_sdk::runtime::CoreNodeRouter::{CoreNodeLocalRuntime, CoreNodeRouter};
    use operit_edge_sdk::runtime::NodeSpaceService::NODE_SPACE_CONTROL_TARGET;
    use operit_edge_sdk::runtime::PeerRouter::PeerRouter;
    use operit_edge_sdk::runtime::RuntimeRemoteLinkService::RuntimeRemoteLinkService;
    use operit_edge_sdk::runtime::SpacePersistenceSyncService::SpacePersistenceSyncService;
    use operit_edge_sdk::storage::CoreSpaceStore::CoreSpaceStore;
    use operit_edge_sdk::storage::NetworkControlStore::NetworkControlIdentityAssignment;
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

        /// Rejects input methods outside the application spy contract.
        async fn openPush(
            &self,
            request: operit_link::CorePushRequest,
        ) -> Result<Box<dyn operit_link::CoreLinkPushSession>, operit_link::CoreLinkError> {
            Err(CoreLinkError::methodNotFound(&request.methodName))
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
        let router = Arc::new(CoreNodeRouter::new(runtime.clone()).unwrap());
        let info = operit_link::protocol::LinkDeviceInfo {
            platform: "windows".into(),
            model: "Windows desktop".into(),
            pairingIncarnation: None,
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
        edge_node_with_limits(storage, operit_edge_sdk::runtime::HostRuntimePeerService::PeerRuntimeLimits::default())
    }
    fn edge_node_with_limits(storage: Arc<Storage>, limits: operit_edge_sdk::runtime::HostRuntimePeerService::PeerRuntimeLimits) -> EdgeFixture {
        configure_listener(&storage);
        let id = operit_edge_sdk::storage::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage.clone())
            .initialize()
            .unwrap()
            .nodeId;
        let inner = EdgePeerRouter::new(id.clone());
        let router = Arc::new(RecordingEdgeRouter {
            inner: inner.clone(),
            traffic: Mutex::default(),
        });
        let host = host(storage.clone());
        let peers = operit_edge_sdk::device::peer::createPeerService(
            host.clone(),
            &router,
            operit_link::protocol::LinkDeviceInfo {
                platform: "esp32".into(),
                model: "ESP32-2432S028".into(),
                pairingIncarnation: None,
            },
            limits,
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
                pairingIncarnation: None,
            })
            .unwrap();
        inner.installSpace(service.clone()).unwrap();
        let edge = EdgeNode::fromHostManager((*host).clone())
            .withNodeServices(NodeServices::new(peers.clone()));
        inner.install(Arc::new(edge)).unwrap();
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
        let config = PeerStateStore::new(target_storage.clone())
            .hostConfig()
            .unwrap()
            .unwrap();
        let address = config.bindAddress.clone();
        let pending = source
            .startPairing(
                PeerEndpoint {
                    nodeId: target_id,
                    address,
                },
                Some(PeerTransport::Tcp),
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
    #[tokio::test]
    async fn code_only_pairing_in_both_directions_keeps_space_approval() {
        let core = core_node();
        let edge = edge_node(Arc::new(Storage::default()));
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(20), async {
            start(&core, &edge).await;
            let core_config = PeerStateStore::new(core.storage.clone())
                .hostConfig()
                .unwrap()
                .unwrap();
            let edge_config = PeerStateStore::new(edge.storage.clone())
                .hostConfig()
                .unwrap()
                .unwrap();
            // Code confirmation applies in both directions, independently of
            // discovery. Pairing does not grant Space admission.
            assert!(!core_config.discoveryEnabled && !edge_config.discoveryEnabled);
            pair(
                &edge.peers,
                &core.peers,
                &core.storage,
                core.router.localNodeId(),
            )
            .await;
            assert_eq!(core.peers.pairedPeers().unwrap().len(), 1);
            assert_eq!(edge.peers.pairedPeers().unwrap().len(), 1);

            edge.peers
                .removePairedPeer(&core.router.localNodeId())
                .await
                .unwrap();
            core.peers
                .removePairedPeer(&edge.service.localNodeId())
                .await
                .unwrap();

            // The helper checks wrong-code rejection before confirming with
            // the Edge's code; both directions use the same shared engine.
            pair(
                &core.peers,
                &edge.peers,
                &edge.storage,
                edge.service.localNodeId(),
            )
            .await;
            assert_eq!(core.peers.pairedPeers().unwrap().len(), 1);
            assert_eq!(edge.peers.pairedPeers().unwrap().len(), 1);
            assert_ne!(
                operit_edge_sdk::storage::CoreSpaceStore::CoreSpaceStore::new(core.storage.clone())
                    .space()
                    .unwrap()
                    .spaceId,
                edge.service.spaceStore().space().unwrap().spaceId
            );
            assert_eq!(
                edge.service
                    .spaceChannelScope(&core.router.localNodeId())
                    .unwrap(),
                None
            );
            let request = core
                .service
                .requestDeviceSpaceJoin(edge.service.localNodeId())
                .await
                .unwrap();
            assert_eq!(request.status, SpaceJoinStatus::Pending);
            assert!(!edge
                .service
                .spaceStore()
                .space()
                .unwrap()
                .members
                .contains(&core.router.localNodeId()));
        })
        .await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        outcome.unwrap();
    }

    /// Fresh devices use upstream confirmation, without revoking other attempts.
    #[tokio::test]
    async fn first_pairing_preserves_other_pending_confirmation_attempts() {
        let _guard = TEST_LOCK.lock().await;
        for core_initiates in [true, false] {
            let core = core_node();
            let edge = fresh_edge();
            let result = tokio::time::timeout(Duration::from_secs(20), async {
                start(&core, &edge).await;
                let (source, target, target_id, storage) = if core_initiates {
                    (
                        &core.peers,
                        &edge.peers,
                        edge.service.localNodeId(),
                        &edge.storage,
                    )
                } else {
                    (
                        &edge.peers,
                        &core.peers,
                        core.router.localNodeId(),
                        &core.storage,
                    )
                };
                let config = PeerStateStore::new(storage.clone())
                    .hostConfig()
                    .unwrap()
                    .unwrap();
                let endpoint = PeerEndpoint {
                    nodeId: target_id,
                    address: config.bindAddress,
                };
                let first = source
                    .startPairing(endpoint.clone(), Some(PeerTransport::Tcp))
                    .await
                    .unwrap();
                let second = source
                    .startPairing(endpoint, Some(PeerTransport::Tcp))
                    .await
                    .unwrap();
                let prompts = target.pairingPrompts().unwrap();
                assert_eq!(prompts.len(), 2);
                let code = &prompts
                    .iter()
                    .find(|p| p.pairingId == first.pairingId)
                    .unwrap()
                    .confirmationCode;
                source.finishPairing(&first.pairingId, code).await.unwrap();
                let retained = target.pairingPrompts().unwrap();
                assert_eq!(
                    retained.len(),
                    1,
                    "a first pairing must not run credential revocation"
                );
                assert_eq!(retained[0].pairingId, second.pairingId);
                source.cancelPairing(&second.pairingId).await.unwrap();
                assert_eq!(source.pairedPeers().unwrap().len(), 1);
                assert_eq!(target.pairedPeers().unwrap().len(), 1);
            })
            .await;
            core.peers.stop().await.unwrap();
            edge.peers.stop().await.unwrap();
            result.unwrap();
        }
    }

    /// Edge removal follows the same upstream control ledger and pairing generation.
    #[tokio::test]
    async fn upstream_control_removal_retires_edge_pairing_before_fresh_pairing() {
        let _guard = TEST_LOCK.lock().await;
        let core = core_node();
        let edge = fresh_edge();
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            start(&core, &edge).await;
            let core_id = core.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            let identity = PeerStateStore::new(edge.storage.clone());
            let previous_generation = identity.pairingIncarnation().unwrap();
            pair(&edge.peers, &core.peers, &core.storage, core_id.clone()).await;
            let request = edge
                .service
                .requestDeviceSpaceJoin(core_id.clone())
                .await
                .unwrap();
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
            let former_space = core.service.deviceSpace().unwrap().spaceId;
            assert_eq!(
                edge.service.spaceStore().space().unwrap().spaceId,
                former_space
            );

            // Deliver the upstream authority log at its authenticated receiver
            // boundary. Notification transport is best effort; receipt, replay,
            // pairing cleanup and generation renewal share the upstream handler.
            let control =
                operit_edge_sdk::storage::NetworkControlStore::NetworkControlStore::new(core.storage.clone())
                    .unwrap();
            control.removeMember(edge_id.clone()).unwrap();
            let operations = control.currentSpaceOperations().unwrap();
            operit_edge_sdk::runtime::NodeSpaceService::acceptSpaceControlCall(
                edge.service.as_ref(),
                &core_id,
                CoreCallRequest::new(
                    "remove-edge",
                    NODE_SPACE_CONTROL_TARGET,
                    "exchange",
                    toCoreValue(operations).unwrap(),
                ),
            )
            .unwrap();
            core.peers.removePairedPeer(&edge_id).await.unwrap();
            assert!(core.peers.pairedPeers().unwrap().is_empty());
            assert!(edge.peers.pairedPeers().unwrap().is_empty());
            assert_ne!(
                edge.service.spaceStore().space().unwrap().spaceId,
                former_space
            );
            let next_generation = identity.pairingIncarnation().unwrap();
            assert!(next_generation.is_some());
            assert_ne!(next_generation, previous_generation);

            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            assert_eq!(
                core.peers.pairedPeers().unwrap()[0].pairingIncarnation,
                next_generation
            );
            assert_eq!(edge.service.spaceChannelScope(&core_id).unwrap(), None);
            assert_eq!(
                core.service
                    .requestDeviceSpaceJoin(edge_id)
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Pending
            );
        })
        .await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }

    #[tokio::test]
    async fn one_sided_unpair_allows_code_verified_replacement_without_space_admission() {
        const RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH: &str =
            "runtime/link_access/inbound_sessions.preferences.json";
        const RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH: &str =
            "runtime/link_access/outbound_sessions.preferences.json";
        let _guard = TEST_LOCK.lock().await;
        for core_initiates in [true, false] {
            for reverse_original in [false, true] {
                let core = core_node();
                let edge = fresh_edge();
                start(&core, &edge).await;
                let outcome = tokio::time::timeout(std::time::Duration::from_secs(30), async {
                    let core_id = core.router.localNodeId();
                    let edge_id = edge.service.localNodeId();
                    let (source, target, source_id, target_id, target_storage) = if core_initiates {
                        (
                            &core.peers,
                            &edge.peers,
                            core_id.clone(),
                            edge_id.clone(),
                            &edge.storage,
                        )
                    } else {
                        (
                            &edge.peers,
                            &core.peers,
                            edge_id.clone(),
                            core_id.clone(),
                            &core.storage,
                        )
                    };
                    if reverse_original {
                        let source_storage = if core_initiates {
                            &core.storage
                        } else {
                            &edge.storage
                        };
                        pair(target, source, source_storage, source_id.clone()).await;
                    } else {
                        pair(source, target, target_storage, target_id.clone()).await;
                    }
                    let stale_source_storage = reopened(if core_initiates {
                        &core.storage
                    } else {
                        &edge.storage
                    });
                    let core_space = core.service.deviceSpace().unwrap();
                    let edge_space = edge.service.spaceStore().space().unwrap();
                    let credentials = |path: &str| storedRecords(&target_storage, path);
                    let old_inbound = credentials(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH);
                    let old_outbound = credentials(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH);
                    source.removePairedPeer(&target_id).await.unwrap();
                    assert!(source.pairedPeers().unwrap().is_empty());
                    assert_eq!(
                        target.pairedPeers().unwrap().len(),
                        1,
                        "unpair remains local-only"
                    );
                    let config = PeerStateStore::new(target_storage.clone())
                        .hostConfig()
                        .unwrap()
                        .unwrap();
                    let begin = || {
                        source.startPairing(
                            PeerEndpoint {
                                nodeId: target_id.clone(),
                                address: config.bindAddress.clone(),
                            },
                            Some(PeerTransport::Tcp),
                        )
                    };
                    let cancelled = begin().await.unwrap();
                    source.cancelPairing(&cancelled.pairingId).await.unwrap();
                    assert!(source
                        .finishPairing(&cancelled.pairingId, "123456")
                        .await
                        .is_err());
                    assert_eq!(
                        credentials(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH),
                        old_inbound
                    );
                    assert_eq!(
                        credentials(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH),
                        old_outbound
                    );
                    let pending = begin().await.unwrap();
                    let code = target
                        .pairingPrompts()
                        .unwrap()
                        .into_iter()
                        .find(|p| p.pairingId == pending.pairingId)
                        .unwrap()
                        .confirmationCode;
                    let wrong = if code == "000000" { "111111" } else { "000000" };
                    assert!(source
                        .finishPairing(&pending.pairingId, wrong)
                        .await
                        .is_err());
                    assert_eq!(
                        credentials(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH),
                        old_inbound
                    );
                    assert_eq!(
                        credentials(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH),
                        old_outbound
                    );
                    source
                        .finishPairing(&pending.pairingId, &code)
                        .await
                        .unwrap();
                    let new_inbound = credentials(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH);
                    assert!(new_inbound.contains_key(&pending.pairingId));
                    assert!(old_inbound
                        .keys()
                        .filter(|k| k.as_str() != "__operit_preferences_schema_version")
                        .all(|key| !new_inbound.contains_key(key)));
                    assert!(credentials(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
                        .values()
                        .all(|v| !v.to_string().contains(&source_id)));
                    let peers = target.pairedPeers().unwrap();
                    assert_eq!(peers.len(), 1);
                    assert!(peers[0].inbound && !peers[0].outbound);
                    assert!(target.pairingPrompts().unwrap().is_empty());
                    assert!(
                        source
                            .finishPairing(&pending.pairingId, &code)
                            .await
                            .is_err(),
                        "no replay"
                    );
                    assert_eq!(core.service.deviceSpace().unwrap(), core_space);
                    assert_eq!(edge.service.spaceStore().space().unwrap(), edge_space);
                    // A restarted copy of the previous initiator still holding
                    // the old secret cannot authenticate after replacement.
                    if !reverse_original {
                        if core_initiates {
                            let stale = core_node_with_storage(stale_source_storage);
                            assert!(stale
                                .service
                                .requestDeviceSpaceJoin(edge_id.clone())
                                .await
                                .is_err());
                            stale.peers.stop().await.unwrap();
                        } else {
                            let stale = edge_node(stale_source_storage);
                            assert!(stale
                                .service
                                .requestDeviceSpaceJoin(core_id.clone())
                                .await
                                .is_err());
                            stale.peers.stop().await.unwrap();
                        }
                    }
                    let join = if core_initiates {
                        core.service.requestDeviceSpaceJoin(edge_id).await.unwrap()
                    } else {
                        edge.service.requestDeviceSpaceJoin(core_id).await.unwrap()
                    };
                    assert_eq!(
                        join.status,
                        SpaceJoinStatus::Pending,
                        "replacement is not Space approval"
                    );
                })
                .await;
                core.peers.stop().await.unwrap();
                edge.peers.stop().await.unwrap();
                outcome.unwrap();
            }
        }
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

    /// The real Core pulls a constrained endpoint one authority record at a
    /// time, and refuses oversized reverse traffic before it reaches TCP.
    #[tokio::test]
    async fn core_uses_endpoint_control_pages_and_stops_oversized_reverse_projection() {
        let _guard = TEST_LOCK.lock().await;
        let core = core_node();
        let edge = edge_node_with_limits(Arc::new(Storage(Mutex::default(), true,
            Mutex::default(), Mutex::default())),
            operit_edge_sdk::runtime::HostRuntimePeerService::PeerRuntimeLimits::constrained());
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let core_id = core.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            let request = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            edge.service.incomingDeviceSpaceJoins().await.unwrap();
            edge.service.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await.unwrap();
            assert_eq!(core.service.refreshDeviceSpaceJoin(request.requestId).await.unwrap().status, SpaceJoinStatus::Joined);
            edge.router.traffic.lock().unwrap().clear();
            core.sync.synchronizeOnce().await.unwrap();
            let traffic = edge.router.traffic.lock().unwrap().clone();
            assert!(traffic.iter().any(|(_, method, _)| method == "projectionProtocol"));
            assert!(traffic.iter().any(|(_, method, _)| method == "snapshotHeader"));
            assert!(traffic.iter().filter(|(_, method, _)| method == "snapshotControlPage").count() >= 2);
            assert!(!traffic.iter().any(|(_, method, _)| matches!(method.as_str(), "snapshot" | "exchange")), "{traffic:?}");
            assert!(traffic.iter().any(|(_, method, _)| method == "exchangePreferences"), "{traffic:?}");
            assert!(traffic.iter().any(|(_, method, _)| method == "observeSpaceProjection"), "{traffic:?}");
            assert!(!traffic.iter().any(|(_, method, _)| method == "observeSpaceSnapshot"), "{traffic:?}");
            // Metadata-only observations must bind to the exact durable export;
            // they cannot import control, cross Spaces or invent membership.
            let inspect = |method: &str| edge.service.acceptPeerSpaceCall(&core_id,
                CoreCallRequest::new("reference-inspect", NODE_SPACE_TARGET, method, operit_link::CoreValue::Null)).unwrap();
            let header: serde_json::Value = operit_link::fromCoreValue(inspect("snapshotHeader")).unwrap();
            let valid = serde_json::json!({"controlRevision":header["controlRevision"], "space":header["space"],
                "deviceProfiles":header["deviceProfiles"], "topology":header["topology"]});
            let apply = |args: serde_json::Value| edge.service.acceptPeerSpaceCall(&core_id,
                CoreCallRequest::new("reference-test", NODE_SPACE_TARGET, "observeSpaceProjection",
                    operit_link::toCoreValue(args).unwrap()));
            let before = edge.storage.0.lock().unwrap().clone();
            for field in ["controlRevision", "spaceId", "unauthorizedMember"] {
                let mut invalid = valid.clone();
                match field {
                    "controlRevision" => invalid["controlRevision"] = serde_json::json!("0".repeat(64)),
                    "spaceId" => invalid["space"]["spaceId"] = serde_json::json!("unapproved-Space"),
                    _ => {
                        invalid["space"]["members"].as_array_mut().unwrap().push(serde_json::json!("unapproved-member"));
                        let mut profile = invalid["deviceProfiles"][0].clone();
                        profile["nodeId"] = serde_json::json!("unapproved-member");
                        invalid["deviceProfiles"].as_array_mut().unwrap().push(profile);
                    }
                }
                let error = apply(invalid).unwrap_err();
                assert!(error.contains("SPACE_EXPORT_CHANGED") || error.contains("unauthorized member"), "{field}: {error}");
                assert_eq!(*edge.storage.0.lock().unwrap(), before);
            }
            apply(valid.clone()).unwrap();
            assert_eq!(*edge.storage.0.lock().unwrap(), before, "identical metadata refresh should be idempotent");
            let mut forbidden = RoutedCoreRequest { spaceId: header["space"]["spaceId"].as_str().unwrap().into(),
                originNodeId: core_id.clone(), targetNodeId: edge_id.clone(), ttl: 0,
                routeKind: RoutedCoreRequestKind::Target, payload: CoreCallRequest::new("reference-origin",
                    NODE_SPACE_TARGET, "observeSpaceProjection", operit_link::toCoreValue(valid).unwrap()) };
            forbidden.originNodeId = "unpaired-origin".into();
            let denied = edge.router.inner.routedCall(core_id.clone(), forbidden).await;
            assert_eq!(denied.result.unwrap_err().code, "PEER_DIRECT_CALL_REQUIRED");
            assert_eq!(*edge.storage.0.lock().unwrap(), before);

            assert_eq!(core.service.deviceSpaceControl().unwrap().deviceIdentityIds[&core_id], "user");
            assert_no_replica(&core, &edge);
            // A deliberately small response budget must produce a short
            // protocol error, rather than a host backtrace that breaks TCP.
            let bounded = CoreCallRequest::new("bounded-error-budget", NODE_SPACE_CONTROL_TARGET,
                "exchangeBounded", operit_link::toCoreValue(serde_json::json!({
                    "version": 1, "maxMessageBytes": 1100, "operations": [],
                })).unwrap());
            let response = edge.service.callNode(core_id.clone(), bounded).await;
            let error = response.result.as_ref().unwrap_err();
            assert_eq!(error.code, "SPACE_MESSAGE_LIMIT");
            assert!(error.backtrace.is_none());
            assert!(operit_link::encodeLink(operit_link::CoreLinkResponse::Call(response)).unwrap().len() < 1100);
            assert!(edge.peers.spaceConnection().is_some());
            // Add durable unknown fields only on Core. They are part of the
            // immutable envelope and must not be dropped to make the batch fit.
            let store = core.service.networkControlStore();
            let mut large = store.currentSpaceOperations().unwrap().remove(0);
            large.payload["futureAuthorityExtension"] = serde_json::json!("x".repeat(9000));
            let projection = operit_edge_sdk::storage::NetworkControlStore::NetworkControlStore::projectionStore(core.storage.clone());
            large.opId = "large-archived-operation".into();
            large.entityId = large.opId.clone();
            large.payload["commandId"] = serde_json::json!(large.entityId);
            // It remains in the current Space export, but a second bootstrap is
            // ignored by policy replay rather than granting additional rights.
            projection.setTypedEntry(&large.opId, &large).unwrap();
            edge.router.traffic.lock().unwrap().clear();
            let before = edge.storage.0.lock().unwrap().clone();
            let error = core.sync.synchronizeOnce().await.unwrap_err();
            assert!(error.contains("SPACE_MESSAGE_LIMIT"), "{error}");
            let traffic = edge.router.traffic.lock().unwrap().clone();
            assert!(!traffic.iter().any(|(_, method, _)| method == "observeSpaceSnapshot" || method == "exchange"), "{traffic:?}");
            assert_eq!(*edge.storage.0.lock().unwrap(), before);
            assert!(core.peers.spaceConnection().is_some());
            assert!(edge.peers.spaceConnection().is_some());
            // Both ordinary and prepared ingress reject the oversized bytes
            // before attempting to decode the deliberately malformed policy.
            for (target, method) in [(NODE_SPACE_TARGET, "observeSpaceSnapshot"),
                (NODE_SPACE_CONTROL_TARGET, "exchange"), (NODE_SPACE_CONTROL_TARGET, "exchangeBounded")] {
                let request = CoreCallRequest::new("oversized-ingress", target, method,
                    operit_link::CoreValue::String("x".repeat(9000)));
                let ordinary = if target == NODE_SPACE_TARGET {
                    edge.service.acceptPeerSpaceCall(&core_id, request.clone())
                } else {
                    operit_edge_sdk::runtime::NodeSpaceService::acceptSpaceControlCall(edge.service.as_ref(), &core_id, request.clone())
                };
                assert!(ordinary.unwrap_err().contains("SPACE_MESSAGE_LIMIT"));
                let routed = RoutedCoreRequest { spaceId: edge.service.spaceStore().space().unwrap().spaceId,
                    originNodeId: core_id.clone(), targetNodeId: edge_id.clone(), ttl: 0,
                    routeKind: RoutedCoreRequestKind::Target, payload: request.clone() };
                let wire = CoreCallRequest::new(request.requestId.0.clone(), target, method, operit_link::toCoreValue(routed).unwrap());
                let mut ingress = operit_link::encodeLink(operit_link::CoreLinkRequest::Call(wire)).unwrap();
                let encoded = edge.router.inner.prepareEncodedCall(&core_id, &mut ingress).unwrap().unwrap();
                let operit_link::CoreLinkResponse::Call(response) = operit_link::decodeLink(&encoded).unwrap() else { panic!("Call expected") };
                assert!(response.result.unwrap_err().message.contains("SPACE_MESSAGE_LIMIT"));
                assert_eq!(*edge.storage.0.lock().unwrap(), before);
            }

        }).await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
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
            // Source members enter as users. The current built-in user role
            // may provide storage; the endpoint Edge still cannot replicate.
            assert_eq!(core.service.deviceSpaceControl().unwrap().deviceIdentityIds[&core_id], "user");
            assert!(core
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
                    operit_edge_sdk::storage::CoreSpaceStore::CoreSpaceStore::new(core.storage.clone())
                        .space()
                        .unwrap()
                        .spaceId,
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
                                    operit_edge_sdk::storage::SyncOperationStore::SyncOperation,
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

    /// The motivating reverse-direction case: the Edge initiated the pairing,
    /// so the Core holds only the inbound credential. Joining the Edge's Space
    /// must not depend on that direction — the applicant's join request rides
    /// the connection the Edge opened, and the Edge still grants membership
    /// only by explicit approval on its own Space.
    #[tokio::test]
    async fn edge_initiated_pairing_lets_the_dialed_core_join_the_edge_space() {
        let _guard = TEST_LOCK.lock().await;
        let core = core_node();
        let edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let core_id = core.router.localNodeId();
            let edge_id = edge.service.localNodeId();
            pair(&edge.peers, &core.peers, &core.storage, core_id.clone()).await;
            let core_projection = &core.peers.pairedPeers().unwrap()[0];
            assert!(
                core_projection.inbound && !core_projection.outbound,
                "the Core answered the pairing, so it holds no outbound credential"
            );
            let edge_projection = &edge.peers.pairedPeers().unwrap()[0];
            assert!(edge_projection.outbound && !edge_projection.inbound);
            assert_eq!(edge.service.spaceChannelScope(&core_id).unwrap(), None);
            // The dialer side may always call the peer it dialed; this
            // pre-admission snapshot probe also opens the return route that
            // the Core's own join request then rides.
            let snapshot = edge
                .peers
                .call(
                    &core_id,
                    RoutedCoreRequest {
                        spaceId: "pre-join-probe".into(),
                        originNodeId: edge_id.clone(),
                        targetNodeId: core_id.clone(),
                        ttl: 0,
                        routeKind: RoutedCoreRequestKind::Target,
                        payload: CoreCallRequest::new(
                            "pre-join-snapshot",
                            NODE_SPACE_TARGET,
                            "snapshot",
                            CoreValue::Null,
                        ),
                    },
                )
                .await;
            assert!(
                snapshot.result.is_ok(),
                "paired dialer may probe the peer's space: {:?}",
                snapshot.result
            );
            // The applicant is the dialed side: no outbound credential exists,
            // so the request must ride the Edge's own connection.
            let request = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            assert_eq!(request.status, SpaceJoinStatus::Pending);
            assert!(!edge
                .service
                .spaceStore()
                .space()
                .unwrap()
                .members
                .contains(&core_id));
            let incoming = edge.service.incomingDeviceSpaceJoins().await.unwrap();
            assert_eq!(incoming.len(), 1);
            assert!(incoming[0].canApprove);
            let approved = edge
                .service
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await
                .unwrap();
            assert_eq!(approved.status, SpaceJoinStatus::Approved);
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
        })
        .await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }

    /// A compact receipt survives restart, but cannot stand in for its exact
    /// admission or an explicit approval. The applicant still verifies policy.
    #[tokio::test]
    async fn membership_projection_survives_restart_and_rejects_mismatched_admission() {
        use operit_edge_sdk::storage::PreferencesDataStore::CoreNodeStateStore;
        let _guard = TEST_LOCK.lock().await;
        let mut core = core_node();
        let mut edge = fresh_edge();
        start(&core, &edge).await;
        let result = tokio::time::timeout(Duration::from_secs(40), async {
            let edge_id = edge.service.localNodeId();
            pair(&core.peers, &edge.peers, &edge.storage, edge_id.clone()).await;
            let request = core.service.requestDeviceSpaceJoin(edge_id.clone()).await.unwrap();
            let path = "runtime/link_access/space_merge_inbound.preferences.json";
            let pending = storedRecords(&edge.storage, path)[&request.requestId].clone();
            edge.service.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true).await.unwrap();
            let receipt = storedRecords(&edge.storage, path)[&request.requestId].clone();
            assert!(receipt.get("acceptedReference").is_none());
            assert!(receipt.get("accepted").is_none(), "membership must be projected from current authority");
            assert_eq!(receipt["source"], pending["source"], "receipt dropped original applicant facts");
            assert_eq!(receipt["profile"], pending["profile"]);
            let now = operit_host_api::TimeUtils::currentTimeMillis();
            let topology: operit_edge_sdk::storage::CoreSpaceStore::CoreSpaceTopologyRecord = serde_json::from_value(serde_json::json!({
                "nodeId": core.router.localNodeId(), "peers": [edge_id.clone()], "updatedAt": now + 1000,
                "links": [{"targetNodeId":edge_id.clone(), "channelEpoch":"topology-roundtrip", "sequence":7,
                    "measuredAt":now, "expiresAt":now + 60_000, "smoothedRttMs":123, "lossPermille":17, "congestionPermille":29}]
            })).unwrap();
            edge.service.spaceStore().importTopologyRecords(vec![topology.clone()]).unwrap();
            let topology_path = format!("runtime/space/topology/{}.preferences.json", core.router.localNodeId());
            let compact: serde_json::Value = serde_json::from_slice(&edge.storage.readBytes(&topology_path).unwrap()).unwrap();
            assert!(compact["topologyV1"].is_array());
            assert!(compact.get("record").is_none());
            // Compact corruption and conflicting legacy data cannot silently
            // fall back to a different adjacency or link-quality projection.
            for invalid_tuple in [serde_json::json!([]), serde_json::json!([[], [], "bad-timestamp"])] {
                let mut invalid = compact.clone();
                invalid["topologyV1"] = invalid_tuple;
                edge.storage.writeBytes(&topology_path, &serde_json::to_vec(&invalid).unwrap()).unwrap();
                assert!(!edge.service.spaceStore().topologyRecords().unwrap().contains_key(&core.router.localNodeId()),
                    "a damaged topology row must be isolated instead of falling back to legacy adjacency");
            }
            let mut ambiguous = compact.clone();
            ambiguous["record"] = serde_json::to_value(&topology).unwrap();
            edge.storage.writeBytes(&topology_path, &serde_json::to_vec(&ambiguous).unwrap()).unwrap();
            assert!(!edge.service.spaceStore().topologyRecords().unwrap().contains_key(&core.router.localNodeId()),
                    "a damaged topology row must be isolated instead of falling back to legacy adjacency");
            edge.storage.writeBytes(&topology_path, &serde_json::to_vec(&compact).unwrap()).unwrap();
            let history = edge.service.networkControlStore().currentSpaceOperations().unwrap();
            core.peers.stop().await.unwrap();
            edge.peers.stop().await.unwrap();
            core = core_node_with_storage(reopened(&core.storage));
            edge = edge_node(reopened(&edge.storage));
            start(&core, &edge).await;
            wait_for_peer_availability(&core.peers, &edge_id, true).await;
            assert_eq!(edge.service.spaceStore().topologyRecords().unwrap()[&core.router.localNodeId()], topology,
                "compact topology changed link quality or adjacency on restart");
            assert!(serde_json::to_value(&topology).unwrap().is_object(), "local encoding escaped onto the wire");
            let wire = edge.service.acceptPeerSpaceCall(&core.router.localNodeId(), CoreCallRequest::new(
                "bounded-receipt", NODE_SPACE_TARGET, "joinStatus",
                operit_link::toCoreValue(request.requestId.clone()).unwrap())).unwrap();
            let wire: serde_json::Value = operit_link::fromCoreValue(wire).unwrap();
            assert!(wire["accepted"].get("controlOperations").is_none(),
                "approval response duplicated authority instead of using the paged target export");
            assert!(wire.get("source").is_none(), "response echoed the applicant's evidence");
            assert!(wire.get("profile").is_none(), "response echoed the applicant's profile");
            assert!(wire.get("acceptedReference").is_none(), "local reference escaped onto the wire");
            // Corrupt correlation facts while keeping projected members and Approved status.
            // Current admission authority must still reject the response.
            let prefs = CoreNodeStateStore::newWithStorage(edge.storage.clone(), path);
            for field in ["sourceSpaceId", "approvedDecision", "decisionRevision"] {
                let mut invalid = receipt.clone();
                invalid[field] = match field {
                    "sourceSpaceId" => serde_json::json!("space-wrong-admission"),
                    "approvedDecision" => serde_json::json!(false),
                    _ => serde_json::Value::Null,
                };
                prefs.setTypedEntry(&request.requestId, &invalid).unwrap();
                assert!(core.service.refreshDeviceSpaceJoin(request.requestId.clone()).await.is_err(),
                    "invalid {field} manufactured membership");
                assert_ne!(core.service.deviceSpace().unwrap().spaceId, edge.service.spaceStore().space().unwrap().spaceId);
            }
            prefs.setTypedEntry(&request.requestId, &receipt).unwrap();
            assert_eq!(core.service.refreshDeviceSpaceJoin(request.requestId.clone()).await.unwrap().status, SpaceJoinStatus::Joined);
            assert_eq!(core.service.deviceSpace().unwrap(), edge.service.spaceStore().space().unwrap());
            assert_eq!(edge.service.networkControlStore().currentSpaceOperations().unwrap(), history,
                "receipt recovery changed authority history");
            // Even an exact committed admission cannot resurrect a member
            // removed by later policy. Exercise the gateway response directly.
            let core_id = core.router.localNodeId();
            edge.service.networkControlStore().removeMember(core_id.clone()).unwrap();
            let revoked = edge.service.acceptPeerSpaceCall(&core_id, CoreCallRequest::new(
                "revoked-receipt", NODE_SPACE_TARGET, "joinStatus",
                operit_link::toCoreValue(request.requestId.clone()).unwrap()));
            assert!(revoked.is_err(), "obsolete approval resurrected a removed member");
            assert_no_replica(&core, &edge);
        }).await;
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
            let first = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            edge.service
                .decideDeviceSpaceJoin(first.requestId.clone(), first.assignmentVersion, true)
                .await
                .unwrap();
            assert_eq!(
                core.service
                    .refreshDeviceSpaceJoin(first.requestId)
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Joined
            );
            for _cycle in 0..3 {
                let former = core.service.deviceSpace().unwrap();
                assert!(former.members.contains(&edge_id) && former.members.contains(&core_id));
                let credentials = edge.peers.pairedPeers().unwrap();
                let singleton = edge.service.leaveDeviceSpace().unwrap();
                assert_eq!(singleton.members, vec![edge_id.clone()]);
                assert_eq!(
                    core.service.deviceSpace().unwrap(),
                    former,
                    "the remote Core retains its original group"
                );
                assert!(!edge
                    .service
                    .spaceStore()
                    .deviceProfiles()
                    .unwrap()
                    .contains_key(&core_id));
                let withdrawn = core
                    .service
                    .requestDeviceSpaceJoin(edge_id.clone())
                    .await
                    .unwrap();
                // Requesting the new Space fetches the peer's durable exit receipt.
                // The old Space loses only Edge before this new admission is approved.
                let retained = core.service.deviceSpace().unwrap();
                assert_eq!(retained.spaceId, former.spaceId);
                assert_eq!(retained.members, vec![core_id.clone()]);
                assert_eq!(
                    core.service
                        .cancelDeviceSpaceJoin(withdrawn.requestId.clone())
                        .await
                        .unwrap()
                        .status,
                    SpaceJoinStatus::Cancelled
                );
                assert!(edge
                    .service
                    .decideDeviceSpaceJoin(withdrawn.requestId, withdrawn.assignmentVersion, true)
                    .await
                    .is_err());
                assert_eq!(
                    core.service.deviceSpace().unwrap(),
                    retained,
                    "withdrawal changed the applicant group"
                );
                assert_eq!(
                    edge.service.spaceStore().space().unwrap(),
                    singleton,
                    "withdrawal admitted the applicant"
                );
                let request = core
                    .service
                    .requestDeviceSpaceJoin(edge_id.clone())
                    .await
                    .unwrap();
                let before = edge.service.spaceStore().space().unwrap();
                assert_eq!(before, singleton);
                let decision = edge
                    .service
                    .decideDeviceSpaceJoin(
                        request.requestId.clone(),
                        request.assignmentVersion,
                        true,
                    )
                    .await;
                assert!(
                    decision.is_ok(),
                    "same-Core rejoin approval failed: {decision:?}"
                );
                assert_eq!(decision.unwrap().status, SpaceJoinStatus::Approved);
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
                let joined = edge.service.spaceStore().space().unwrap();
                let profiles = edge.service.spaceStore().deviceProfiles().unwrap();
                assert!(joined
                    .members
                    .iter()
                    .all(|node| profiles.contains_key(node)));
                assert_eq!(edge.peers.pairedPeers().unwrap(), credentials);
                assert_no_replica(&core, &edge);
            }
        })
        .await;
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
            let first = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            edge.service
                .decideDeviceSpaceJoin(first.requestId.clone(), first.assignmentVersion, true)
                .await
                .unwrap();
            core.service
                .refreshDeviceSpaceJoin(first.requestId)
                .await
                .unwrap();
            edge.service.leaveDeviceSpace().unwrap();
            let request = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            let credentials = edge.peers.pairedPeers().unwrap();
            let core_credentials = core.peers.pairedPeers().unwrap();
            // Claim through the production dispatcher, then persist the exact
            // admission and membership prefix completed before the old failure.
            edge.service
                .acceptSpaceApprovalCall(
                    &edge_id,
                    CoreCallRequest::new(
                        "partial-rejoin-claim",
                        NODE_SPACE_APPROVAL_TARGET,
                        "claim",
                        operit_link::toCoreValue(
                            serde_json::json!({"requestId": request.requestId,
                    "assignmentVersion": request.assignmentVersion, "approve": true}),
                        )
                        .unwrap(),
                    ),
                )
                .await
                .unwrap();
            let records = storedRecords(
                &edge.storage,
                "runtime/link_access/space_merge_inbound.preferences.json",
            );
            let record = records[&request.requestId].clone();
            let source_space: operit_edge_sdk::storage::CoreSpaceStore::CoreSpace =
                serde_json::from_value(record["source"]["space"].clone()).unwrap();
            let profiles: Vec<CoreSpaceDeviceProfile> =
                serde_json::from_value(record["source"]["deviceProfiles"].clone()).unwrap();
            edge.service
                .spaceStore()
                .importDeviceProfiles(profiles)
                .unwrap();
            edge.service
                .networkControlStore()
                .admitSpaceForReview(
                    source_space.spaceId.clone(),
                    source_space
                        .members
                        .iter()
                        .map(|node| (node.clone(), "user".into()))
                        .collect(),
                    &format!("{}-{}", request.requestId, request.assignmentVersion),
                )
                .unwrap();
            let mut joined = edge.service.spaceStore().space().unwrap();
            joined.members.extend(source_space.members);
            joined.members.sort();
            joined.members.dedup();
            joined.spaceRevision = record["decisionRevision"].as_i64().unwrap();
            edge.service
                .spaceStore()
                .adoptAt(joined, request.createdAt)
                .unwrap();
            operit_edge_sdk::storage::PreferencesDataStore::CoreNodeStateStore::newWithStorage(
                edge.storage.clone(),
                format!("runtime/space/device_profiles/{core_id}.preferences.json"),
            )
            .delete()
            .unwrap();
            assert!(edge
                .service
                .spaceStore()
                .deviceProfilesForCurrentSpace()
                .is_err());
            // The admission reached the control log, so the applicant derives
            // completion from durable state instead of waiting for the vanished
            // reviewer: cancellation must NOT revoke it or manufacture success,
            // and must never leave the request stuck in Approving.
            assert_eq!(
                core.service
                    .cancelDeviceSpaceJoin(request.requestId.clone())
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Joined
            );
            assert_eq!(
                core.service.deviceSpace().unwrap(),
                edge.service.spaceStore().space().unwrap()
            );
            // Restart BOTH processes against fresh storage-host handles. The
            // applicant's persisted completed receipt must survive too.
            core.peers.stop().await.unwrap();
            edge.peers.stop().await.unwrap();
            core = core_node_with_storage(reopened(&core.storage));
            edge = edge_node(reopened(&edge.storage));
            assert_eq!(core.router.localNodeId(), core_id);
            assert_eq!(edge.service.localNodeId(), edge_id);
            assert_eq!(
                core.service
                    .outgoingDeviceSpaceJoins()
                    .unwrap()
                    .into_iter()
                    .find(|saved| saved.requestId == request.requestId)
                    .unwrap()
                    .status,
                SpaceJoinStatus::Joined
            );
            start(&core, &edge).await;
            wait_for_peer_availability(&core.peers, &edge_id, true).await;
            wait_for_peer_availability(&edge.peers, &core_id, true).await;
            let recovered = edge
                .service
                .decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
                .await;
            assert!(
                recovered.is_ok(),
                "durable rejoin did not recover after restart: {recovered:?}"
            );
            // The same Core cancel action must reconcile the now-completed
            // approval as Joined, never falsely report Cancelled or stay stuck.
            assert_eq!(
                core.service
                    .cancelDeviceSpaceJoin(request.requestId.clone())
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Joined
            );
            assert_eq!(
                core.service.deviceSpace().unwrap(),
                edge.service.spaceStore().space().unwrap()
            );
            assert!(edge
                .service
                .spaceStore()
                .deviceProfilesForCurrentSpace()
                .is_ok());
            let review_id = format!(
                "control-review-{}-{}",
                request.requestId, request.assignmentVersion
            );
            assert_eq!(
                edge.service
                    .networkControlStore()
                    .currentSpaceOperations()
                    .unwrap()
                    .iter()
                    .filter(|operation| operation.entityId == review_id)
                    .count(),
                1,
                "retry duplicated a committed admission"
            );
            assert_eq!(edge.peers.pairedPeers().unwrap(), credentials);
            assert_eq!(core.peers.pairedPeers().unwrap(), core_credentials);
            assert_no_replica(&core, &edge);
        })
        .await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
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
            let first = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            edge.service
                .decideDeviceSpaceJoin(first.requestId.clone(), first.assignmentVersion, true)
                .await
                .unwrap();
            core.service
                .refreshDeviceSpaceJoin(first.requestId)
                .await
                .unwrap();
            let former = core.service.deviceSpace().unwrap();
            let singleton = edge.service.leaveDeviceSpace().unwrap();
            let request = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            // Fetching the new Space also delivers the Edge's departure from
            // the former Space. Cancelling must preserve that received fact.
            let retained = core.service.deviceSpace().unwrap();
            assert_eq!(retained.spaceId, former.spaceId);
            assert_eq!(retained.members, vec![core_id.clone()]);
            let core_credentials = core.peers.pairedPeers().unwrap();
            let edge_credentials = edge.peers.pairedPeers().unwrap();
            edge.peers.stop().await.unwrap();
            wait_for_peer_availability(&core.peers, &edge_id, false).await;
            assert!(core
                .service
                .cancelDeviceSpaceJoin(request.requestId.clone())
                .await
                .is_err());
            let records = storedRecords(
                &core.storage,
                "runtime/link_access/space_merge_outbound.preferences.json",
            );
            let saved = records[&request.requestId].clone();
            assert_eq!(
                saved["cancelRequested"], true,
                "lost cancellation must retain durable intent"
            );
            assert_eq!(
                core.service
                    .outgoingDeviceSpaceJoins()
                    .unwrap()
                    .into_iter()
                    .find(|saved| saved.requestId == request.requestId)
                    .unwrap()
                    .status,
                SpaceJoinStatus::Pending
            );
            core.peers.stop().await.unwrap();
            core = core_node_with_storage(reopened(&core.storage));
            edge = edge_node(reopened(&edge.storage));
            start(&core, &edge).await;
            wait_for_peer_availability(&core.peers, &edge_id, true).await;
            wait_for_peer_availability(&edge.peers, &core_id, true).await;
            // The regular dialog/background refresh path retries saved intent;
            // the user does not have to submit another cancellation request.
            assert_eq!(
                core.service
                    .refreshDeviceSpaceJoin(request.requestId.clone())
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Cancelled
            );
            assert!(!edge
                .service
                .incomingDeviceSpaceJoins()
                .await
                .unwrap()
                .iter()
                .any(|saved| saved.requestId == request.requestId));
            assert!(edge
                .service
                .decideDeviceSpaceJoin(request.requestId, request.assignmentVersion, true)
                .await
                .is_err());
            assert_eq!(core.service.deviceSpace().unwrap(), retained);
            assert_eq!(edge.service.spaceStore().space().unwrap(), singleton);
            assert!(!edge
                .service
                .spaceStore()
                .deviceProfiles()
                .unwrap()
                .contains_key(&core_id));
            assert_eq!(core.peers.pairedPeers().unwrap(), core_credentials);
            assert_eq!(edge.peers.pairedPeers().unwrap(), edge_credentials);
            assert_no_replica(&core, &edge);
            // Cancellation must also release the original request slot, so the
            // SAME applicant can apply again and complete a fresh admission.
            let next = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            edge.service
                .decideDeviceSpaceJoin(next.requestId.clone(), next.assignmentVersion, true)
                .await
                .unwrap();
            assert_eq!(
                core.service
                    .refreshDeviceSpaceJoin(next.requestId)
                    .await
                    .unwrap()
                    .status,
                SpaceJoinStatus::Joined
            );
            assert_eq!(
                core.service.deviceSpace().unwrap(),
                edge.service.spaceStore().space().unwrap()
            );
            assert_no_replica(&core, &edge);
        })
        .await;
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
            let request = core
                .service
                .requestDeviceSpaceJoin(edge_id.clone())
                .await
                .unwrap();
            edge.service
                .acceptSpaceApprovalCall(
                    &edge_id,
                    CoreCallRequest::new(
                        "uncommitted-claim",
                        NODE_SPACE_APPROVAL_TARGET,
                        "claim",
                        operit_link::toCoreValue(
                            serde_json::json!({"requestId": request.requestId,
                    "assignmentVersion": request.assignmentVersion, "approve": true}),
                        )
                        .unwrap(),
                    ),
                )
                .await
                .unwrap();
            // Emulate an inconsistent member reference, without an admission.
            let mut inconsistent = edge.service.spaceStore().space().unwrap();
            inconsistent.members.push(core_id.clone());
            inconsistent.spaceRevision += 1;
            edge.service.spaceStore().adopt(inconsistent).unwrap();
            assert!(!edge
                .service
                .spaceStore()
                .deviceProfiles()
                .unwrap()
                .contains_key(&core_id));
            let before = edge.storage.0.lock().unwrap().clone();
            let decision = edge
                .service
                .decideDeviceSpaceJoin(request.requestId, request.assignmentVersion, true)
                .await;
            assert!(decision.unwrap_err().contains("Device profile is missing"));
            assert!(!edge
                .service
                .spaceStore()
                .deviceProfiles()
                .unwrap()
                .contains_key(&core_id));
            let after = edge.storage.0.lock().unwrap().clone();
            let changed = before.keys().chain(after.keys()).filter(|key| before.get(*key) != after.get(*key))
                .collect::<std::collections::BTreeSet<_>>();
            for key in &changed {
                eprintln!("changed {key}: before={:?}, after={:?}",
                    before.get(*key).map(|bytes| String::from_utf8_lossy(bytes)),
                    after.get(*key).map(|bytes| String::from_utf8_lossy(bytes)));
            }
            assert!(changed.is_empty(), "recovery mutated storage without admission authority");
            assert_no_replica(&core, &edge);
        })
        .await;
        core.peers.stop().await.unwrap();
        edge.peers.stop().await.unwrap();
        result.unwrap();
    }

    #[tokio::test]
    async fn edge_can_leave_an_offline_admin_space_and_approve_a_new_core() {
        let _guard = TEST_LOCK.lock().await;
        let old = core_node();
        let new = core_node();
        let edge = fresh_edge();
        start(&old, &edge).await;
        new.peers
            .startListening(&[PeerTransport::Tcp])
            .await
            .unwrap();
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
                    pairingIncarnation: None,
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
            let receipt = storedRecords(&edge.storage,
                "runtime/link_access/space_merge_inbound.preferences.json").into_values().next().unwrap();
            for field in ["deviceProfiles", "topology", "controlOperations"] {
                assert!(receipt["source"][field].as_array().unwrap().is_empty(), "cancelled request retained a complete source snapshot: {field}");
            }
            let credentials = edge.peers.pairedPeers().unwrap();
            let identity = operit_edge_sdk::storage::CoreNodeIdentityStore::CoreNodeIdentityStore::new(edge.storage.clone()).initialize().unwrap();

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
            // main makes departure notification and retired-file cleanup
            // independent: a transient ancillary failure cannot trap a device
            // in an offline administrator's Space.
            print_size("before-leave", &edge.storage);
            let left = edge.service.leaveDeviceSpace().unwrap();
            for path in retired { assert!(!edge.storage.exists(path).unwrap(), "retired approval file survived exit: {path}"); }
            print_size("after-leave", &edge.storage);
            assert_ne!(left.spaceId, previous.spaceId);
            assert_eq!(left.members, vec![edge_id.clone()]);
            assert_eq!(edge.service.spaceStore().deviceProfiles().unwrap().len(), 1, "endpoint retained retired profile history");
            assert_eq!(edge.storage.list("runtime/space/members").unwrap().len(), 1, "endpoint retained retired membership history");
            assert_eq!(edge.peers.pairedPeers().unwrap(), credentials, "leave must preserve direct pairing");
            assert_eq!(operit_edge_sdk::storage::CoreNodeIdentityStore::CoreNodeIdentityStore::new(edge.storage.clone()).initialize().unwrap(), identity);
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
            let saved = storedRecords(&new.storage,
                "runtime/link_access/space_merge_outbound.preferences.json");
            let record = saved[&request.requestId].clone();
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
            assert!(storedRecords(&edge.storage, "runtime/link_access/space_merge_review_results.preferences.json").is_empty(),
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
            assert_eq!(
                core.service.deviceSpaceControl().unwrap().deviceIdentityIds[&edge_id],
                "edge"
            );
            let space_id = core.service.deviceSpace().unwrap().spaceId;
            let core_preferences =
                operit_edge_sdk::storage::DevicePreferencesStore::DevicePreferencesStore::new(
                    core.storage.clone(),
                );
            let edge_preferences =
                operit_edge_sdk::storage::DevicePreferencesStore::DevicePreferencesStore::new(
                    edge.storage.clone(),
                );
            core.service
                .setDevicePreference(
                    edge_id.clone(),
                    "volume".into(),
                    Some(serde_json::json!(25)),
                )
                .unwrap();
            core.sync.synchronizeOnce().await.unwrap();
            assert_eq!(
                edge_preferences.read(&space_id, &edge_id).unwrap().values()["volume"],
                serde_json::json!(25)
            );
            edge_preferences
                .set(
                    &space_id,
                    &edge_id,
                    &edge_id,
                    "brightness",
                    Some(serde_json::json!(75)),
                )
                .unwrap();
            edge.service.synchronizePreferences().await.unwrap();
            assert_eq!(
                core_preferences.read(&space_id, &edge_id).unwrap().values()["brightness"],
                serde_json::json!(75)
            );
            core.service
                .setDevicePreference(edge_id.clone(), "volume".into(), None)
                .unwrap();
            edge.service.synchronizePreferences().await.unwrap();
            assert!(!edge_preferences
                .read(&space_id, &edge_id)
                .unwrap()
                .values()
                .contains_key("volume"));
            // A member cannot fetch the configuration of a third device.
            let unrelated = core_preferences
                .read(&space_id, "unrelated-device")
                .unwrap();
            let denied = edge
                .service
                .callNode(
                    core_id.clone(),
                    CoreCallRequest::new(
                        "foreign-preferences",
                        operit_edge_sdk::runtime::NodeSpaceService::NODE_SPACE_CONTROL_TARGET,
                        "exchangePreferences",
                        toCoreValue(unrelated).unwrap(),
                    ),
                )
                .await;
            assert!(denied.result.is_err());
            // The normal node really has a file much larger than the board's budget.
            // Its production scanner creates a blob/log, but none may reach Edge.
            core.storage
                .writeBytes("workspaces/large/fixture.bin", &vec![0x5a; 1024 * 1024])
                .unwrap();
            core.sync.synchronizeOnce().await.unwrap();
            assert!(!core.storage.list("runtime/sync").unwrap().is_empty());
            assert_no_replica(&core, &edge);
            let space_id = operit_edge_sdk::storage::CoreSpaceStore::CoreSpaceStore::new(core.storage.clone())
                .space()
                .unwrap()
                .spaceId;
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
                                operit_link::toCoreValue(
                                    serde_json::json!({"operations":[]}),
                                )
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
                .defineDeviceSpaceRole(operit_edge_sdk::storage::NetworkControlStore::NetworkControlRole {
                    roleId: "replica".into(),
                    displayName: "Test replica".into(),
                    capabilities: std::collections::BTreeSet::from(["storage.provide".into()]),
                })
                .unwrap();
            core.service
                .setDeviceSpaceIdentity(NetworkControlIdentityAssignment {
                    nodeId: edge_id.clone(),
                    roleId: "replica".into(),
                })
                .unwrap();
            core.sync.synchronizeOnce().await.unwrap();
            assert!(
                edge.service
                    .networkControlStore()
                    .nodeHasCapability(&edge_id, "storage.provide", None)
                    .unwrap(),
                "Policy grants are separate from persistence service availability"
            );
            assert_eq!(
                core.service.spaceStore().deviceProfiles().unwrap()[&edge_id].coreVersion,
                None
            );
            assert_no_replica(&core, &edge);
            // Clearing a role restores the default user, which may provide
            // storage. Explicitly restore the Edge identity to revoke it.
            core.service
                .setDeviceSpaceIdentity(NetworkControlIdentityAssignment {
                    nodeId: edge_id.clone(), roleId: "edge".into(),
                })
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
    /// Two bounded endpoints have no Core replication worker. The remaining
    /// endpoint must still discover an exit over its existing one-way pairing.
    #[tokio::test]
    async fn bounded_endpoint_learns_its_paired_neighbors_exit_without_a_core() {
        let _guard = TEST_LOCK.lock().await;
        operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
            NativeHostRuntimeTaskSchedulerHost,
        ));
        let a = fresh_edge();
        let c = fresh_edge();
        a.peers.startListening(&[PeerTransport::Tcp]).await.unwrap();
        c.peers.startListening(&[PeerTransport::Tcp]).await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            let a_id = a.service.localNodeId();
            let c_id = c.service.localNodeId();
            pair(&a.peers, &c.peers, &c.storage, c_id.clone()).await;
            let join = a
                .service
                .requestDeviceSpaceJoin(c_id.clone())
                .await
                .unwrap();
            c.service
                .decideDeviceSpaceJoin(join.requestId.clone(), join.assignmentVersion, true)
                .await
                .unwrap();
            a.service
                .refreshDeviceSpaceJoin(join.requestId)
                .await
                .unwrap();
            let original = a.service.spaceStore().space().unwrap().spaceId;
            let own = c.service.leaveDeviceSpace().unwrap();
            assert_ne!(own.spaceId, original);
            let reopened = NodeSpaceService::new(c.storage.clone(), c.peers.clone()).unwrap();
            // C was only the pairing receiver; its caller A discovers and acks
            // the receipt without C gaining a reverse pairing or storage grant.
            reopened.synchronizePreferences().await.unwrap();
            a.service.synchronizePreferences().await.unwrap();
            assert_eq!(a.service.spaceStore().space().unwrap().spaceId, original);
            assert_eq!(
                a.service.spaceStore().space().unwrap().members,
                vec![a_id.clone()]
            );
            let state = a.service.networkControlStore().currentState().unwrap();
            assert!(!state.memberNodeIds.contains(&c_id));
            assert_ne!(
                state.deviceIdentityIds.get(&a_id).map(String::as_str),
                Some("admin")
            );
            assert_eq!(reopened.spaceStore().space().unwrap(), own);
            assert_eq!(
                storedRecords(
                    &c.storage,
                    "runtime/link_access/space_departures.preferences.json"
                )
                .len(),
                0
            );
            assert!(a.storage.list("runtime/sync").unwrap().is_empty());
            assert!(c.storage.list("runtime/sync").unwrap().is_empty());
        })
        .await;
        a.peers.stop().await.unwrap();
        c.peers.stop().await.unwrap();
        result.unwrap();
    }
}
