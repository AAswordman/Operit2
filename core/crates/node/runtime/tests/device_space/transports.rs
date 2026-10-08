use super::*;

/// Single real TCP pairing: independent peers have no return authority, while
/// admitted members exchange scoped return credentials without a second pairing.
/// Reconnect works; current revocation and local management boundaries still hold.
#[tokio::test]
async fn real_tcp_single_pairing_admission_enables_scoped_return_channel() {
    use crate::HostRuntimePeerService::HostRuntimePeerService;
    use crate::PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore};
    use operit_host_api::HostManager::HostManager;
    use operit_host_native_common::NativeTcpHost;
    use operit_link::protocol::LinkDeviceInfo;
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (macRouter, _) = approvalService("tcp-mac");
    let (iosRouter, _) = approvalService("tcp-ios");
    let macRouter = Arc::new(macRouter);
    let iosRouter = Arc::new(iosRouter);
    let makeHost = |router: &CoreNodeRouter| {
        Arc::new(HostManager {
            runtimeStorageHost: Some(router.localCore.runtimeStorageHost()),
            tcpHost: Some(Arc::new(NativeTcpHost)),
            hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
            ..HostManager::default()
        })
    };
    let macPeer = HostRuntimePeerService::new(
        makeHost(&macRouter),
        &macRouter,
        LinkDeviceInfo {
            platform: "mac".into(),
            model: "test".into(),
        },
    )
    .unwrap();
    let iosPeer = HostRuntimePeerService::new(
        makeHost(&iosRouter),
        &iosRouter,
        LinkDeviceInfo {
            platform: "ios".into(),
            model: "test".into(),
        },
    )
    .unwrap();
    macRouter
        .installNodeServices(NodeServices::new(macPeer.clone()))
        .unwrap();
    iosRouter
        .installNodeServices(NodeServices::new(iosPeer.clone()))
        .unwrap();
    let mac = RuntimeRemoteLinkService::newWithRouter(
        (*macRouter.localCore).clone(),
        (*macRouter).clone(),
    );
    let ios = RuntimeRemoteLinkService::newWithRouter(
        (*iosRouter.localCore).clone(),
        (*iosRouter).clone(),
    );
    // Both devices expose a listener; only Mac performs the six-digit pairing.
    for (router, peer) in [(&macRouter, &macPeer), (&iosRouter, &iosPeer)] {
        PeerStateStore::new(router.localCore.runtimeStorageHost())
            .saveHostConfig(&PeerHostConfig {
                bindAddress: "127.0.0.1:0".into(),
                token: "test-token".into(),
                transports: vec![PeerTransport::Tcp],
                discoveryEnabled: false,
                portMode: PeerHostPortMode::Automatic,
                updatedAt: 1,
            })
            .unwrap();
        peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    }
    let address = PeerStateStore::new(iosRouter.localCore.runtimeStorageHost())
        .hostConfig()
        .unwrap()
        .unwrap()
        .bindAddress;
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let pairing = macPeer
            .startPairing(
                PeerEndpoint {
                    nodeId: "tcp-ios".into(),
                    address,
                },
                PeerTransport::Tcp,
                Some("test-token"),
            )
            .await
            .unwrap();
        let prompts = iosPeer.pairingPrompts().unwrap();
        let code = &prompts
            .iter()
            .find(|p| p.pairingId == pairing.pairingId)
            .unwrap()
            .confirmationCode;
        assert_eq!(code.len(), 6);
        macPeer
            .finishPairing(&pairing.pairingId, code)
            .await
            .unwrap();
        assert!(mac.pairedDeviceOnline("tcp-ios".into()).unwrap());
        assert!(ios.pairedDeviceOnline("tcp-mac".into()).unwrap());
        assert!(iosPeer.pairingPrompts().unwrap().is_empty());
        let makeRequest = |origin: &str, target: &str, payload| RoutedCoreRequest {
            spaceId: "independent-space".into(),
            originNodeId: origin.into(),
            targetNodeId: target.into(),
            ttl: 0,
            routeKind: RoutedCoreRequestKind::Target,
            payload,
        };
        // iOS cannot actively dial Mac without an outbound grant. Still online.
        let response = iosPeer
            .call(
                "tcp-mac",
                makeRequest(
                    "tcp-ios",
                    "tcp-mac",
                    CoreCallRequest::new("reverse", NODE_SPACE_TARGET, "snapshot", CoreValue::Null),
                ),
            )
            .await;
        assert_eq!(
            response.result.unwrap_err().code,
            "PEER_OUTBOUND_NOT_AUTHORIZED"
        );
        assert!(ios.pairedDeviceOnline("tcp-mac".into()).unwrap());
        // A real encrypted business rejection is not a disconnect on Mac either.
        let response = macPeer
            .call(
                "tcp-ios",
                makeRequest(
                    "tcp-mac",
                    "tcp-ios",
                    CoreCallRequest::new(
                        "rejected",
                        "core/server.runtimeRemoteLinkService",
                        "deviceSpaceControlAudit",
                        CoreValue::Null,
                    ),
                ),
            )
            .await;
        assert_eq!(response.result.unwrap_err().code, "LOCAL_MANAGEMENT_ONLY");
        assert!(mac.pairedDeviceOnline("tcp-ios".into()).unwrap());
        assert!(ios.pairedDeviceOnline("tcp-mac".into()).unwrap());
        // Admission establishes a scoped return channel, never a reverse pairing.
        let request = mac.requestDeviceSpaceJoin("tcp-ios".into()).await.unwrap();
        assert!(iosRouter.spaceChannelScope("tcp-mac").unwrap().is_none());
        assert!(macRouter.spaceChannelScope("tcp-ios").unwrap().is_none());
        assert_eq!(
            iosPeer
                .call(
                    "tcp-mac",
                    makeRequest(
                        "tcp-ios",
                        "tcp-mac",
                        CoreCallRequest::new(
                            "pending-return",
                            NODE_SPACE_TARGET,
                            "snapshot",
                            CoreValue::Null
                        )
                    )
                )
                .await
                .result
                .unwrap_err()
                .code,
            "PEER_OUTBOUND_NOT_AUTHORIZED"
        );
        ios.incomingDeviceSpaceJoins().await.unwrap();
        ios.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
            .await
            .unwrap();
        assert_eq!(
            mac.refreshDeviceSpaceJoin(request.requestId)
                .await
                .unwrap()
                .status,
            SpaceJoinStatus::Joined
        );
        assert!(
            !iosPeer
                .pairedPeers()
                .unwrap()
                .iter()
                .find(|p| p.nodeId == "tcp-mac")
                .unwrap()
                .outbound
        );
        assert!(iosRouter.spaceChannelScope("tcp-mac").unwrap().is_some());
        let returnRequest = || RoutedCoreRequest {
            spaceId: ios.deviceSpace().unwrap().spaceId,
            originNodeId: "tcp-ios".into(),
            targetNodeId: "tcp-mac".into(),
            ttl: 0,
            routeKind: RoutedCoreRequestKind::Target,
            payload: PeerSyncMethod::DeviceSpace.request("return-space".into(), CoreValue::Null),
        };
        let reverse = iosPeer.call("tcp-mac", returnRequest()).await;
        assert!(reverse.result.is_ok(), "{:?}", reverse.result);
        assert!(iosPeer.spaceClient().is_some(), "TCP grants expose the shared Space client");
        assert_eq!(iosPeer.pooledChannelCount("tcp-mac").await, 1);
        // Return credentials survive reconnect without any second confirmation.
        iosPeer.disconnectPeer("tcp-mac").await.unwrap();
        assert_eq!(iosPeer.pooledChannelCount("tcp-mac").await, 0);
        assert!(iosPeer
            .call("tcp-mac", returnRequest())
            .await
            .result
            .is_ok());
        assert_eq!(iosPeer.pooledChannelCount("tcp-mac").await, 1);
        // A Space grant does not expose local-only management surfaces.
        let mut denied = returnRequest();
        denied.payload = CoreCallRequest::new(
            "return-management",
            "core/server.runtimeRemoteLinkService",
            "deviceSpaceControlAudit",
            CoreValue::Null,
        );
        assert_eq!(
            iosPeer
                .call("tcp-mac", denied)
                .await
                .result
                .unwrap_err()
                .code,
            "LOCAL_MANAGEMENT_ONLY"
        );
        // Listener restart clears online evidence. Restored credentials must
        // reconnect without a business call, manual sync, or reverse pairing.
        macPeer.stop().await.unwrap();
        iosPeer.stop().await.unwrap();
        assert!(!mac.pairedDeviceOnline("tcp-ios".into()).unwrap());
        assert!(!ios.pairedDeviceOnline("tcp-mac".into()).unwrap());
        macPeer.startListening(&[PeerTransport::Tcp]).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!mac.pairedDeviceOnline("tcp-ios".into()).unwrap());
        iosPeer.startListening(&[PeerTransport::Tcp]).await.unwrap();
        tokio::time::timeout(Duration::from_secs(7), async {
            while !mac.pairedDeviceOnline("tcp-ios".into()).unwrap()
                || !ios.pairedDeviceOnline("tcp-mac".into()).unwrap()
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert!(
            !iosPeer
                .pairedPeers()
                .unwrap()
                .iter()
                .find(|p| p.nodeId == "tcp-mac")
                .unwrap()
                .outbound
        );
        // Current policy, not the persisted grant, owns revocation.
        ios.disconnectDeviceSpaceNode("tcp-mac".into())
            .await
            .unwrap();
        for operation in iosRouter
            .networkControlStore
            .currentSpaceOperations()
            .unwrap()
        {
            macRouter
                .networkControlStore
                .applyBootstrapOperation(&operation)
                .unwrap();
        }
        assert!(iosPeer
            .call("tcp-mac", returnRequest())
            .await
            .result
            .is_err());
        let revoked = macPeer
            .call(
                "tcp-ios",
                makeRequest(
                    "tcp-mac",
                    "tcp-ios",
                    PeerSyncMethod::DeviceSpace.request("revoked-ordinary".into(), CoreValue::Null),
                ),
            )
            .await;
        assert!(
            revoked.result.is_err(),
            "Revoked ordinary pairing must not reconnect"
        );
        assert!(!iosPeer.activePeerNodeIds().unwrap().contains("tcp-mac"));
        mac.leaveDeviceSpace().unwrap();
        assert!(macRouter.spaceChannelScope("tcp-ios").unwrap().is_none());
        assert!(iosPeer
            .call("tcp-mac", returnRequest())
            .await
            .result
            .is_err());
        // This fix must not mask a real network failure.
        iosPeer.stop().await.unwrap();
        let failed = macPeer
            .call(
                "tcp-ios",
                makeRequest(
                    "tcp-mac",
                    "tcp-ios",
                    CoreCallRequest::new("offline", NODE_SPACE_TARGET, "snapshot", CoreValue::Null),
                ),
            )
            .await;
        assert!(failed.result.is_err());
        assert!(!mac.pairedDeviceOnline("tcp-ios".into()).unwrap());
    })
    .await;
    macPeer.stop().await.unwrap();
    iosPeer.stop().await.unwrap();
    result.unwrap();
}

/// Space removal deletes the local pairing credentials together with the
/// membership grant, so the same device can pair again instead of being
/// refused with PEER_ALREADY_PAIRED. The removed side must still forget its
/// own stale outbound record first: pairing credentials are node-local, so
/// an offline removal cannot wipe the remote half.
#[tokio::test]
async fn real_tcp_space_remove_deletes_pairing_and_allows_repairing() {
    use crate::HostRuntimePeerService::HostRuntimePeerService;
    use crate::PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore};
    use operit_host_api::HostManager::HostManager;
    use operit_host_native_common::NativeTcpHost;
    use operit_link::protocol::LinkDeviceInfo;
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (macRouter, mac) = approvalService("remove-mac");
    let (iosRouter, ios) = approvalService("remove-ios");
    let macRouter = Arc::new(macRouter);
    let iosRouter = Arc::new(iosRouter);
    let makeHost = |router: &CoreNodeRouter| {
        Arc::new(HostManager {
            runtimeStorageHost: Some(router.localCore.runtimeStorageHost()),
            tcpHost: Some(Arc::new(NativeTcpHost)),
            hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
            ..HostManager::default()
        })
    };
    let macPeer = HostRuntimePeerService::new(
        makeHost(&macRouter),
        &macRouter,
        LinkDeviceInfo {
            platform: "mac".into(),
            model: "test".into(),
        },
    )
    .unwrap();
    let iosPeer = HostRuntimePeerService::new(
        makeHost(&iosRouter),
        &iosRouter,
        LinkDeviceInfo {
            platform: "ios".into(),
            model: "test".into(),
        },
    )
    .unwrap();
    macRouter
        .installNodeServices(NodeServices::new(macPeer.clone()))
        .unwrap();
    iosRouter
        .installNodeServices(NodeServices::new(iosPeer.clone()))
        .unwrap();
    for router in [&macRouter, &iosRouter] {
        PeerStateStore::new(router.localCore.runtimeStorageHost())
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
    iosPeer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    let address = PeerStateStore::new(iosRouter.localCore.runtimeStorageHost())
        .hostConfig()
        .unwrap()
        .unwrap()
        .bindAddress;
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let pairing = macPeer
            .startPairing(
                PeerEndpoint {
                    nodeId: "remove-ios".into(),
                    address: address.clone(),
                },
                PeerTransport::Tcp,
                Some("test-token"),
            )
            .await
            .unwrap();
        let code = iosPeer
            .pairingPrompts()
            .unwrap()
            .into_iter()
            .find(|prompt| prompt.pairingId == pairing.pairingId)
            .unwrap()
            .confirmationCode;
        macPeer.finishPairing(&pairing.pairingId, &code).await.unwrap();
        let request = mac.requestDeviceSpaceJoin("remove-ios".into()).await.unwrap();
        ios.incomingDeviceSpaceJoins().await.unwrap();
        ios.decideDeviceSpaceJoin(request.requestId.clone(), request.assignmentVersion, true)
            .await
            .unwrap();
        assert!(iosPeer.pairedPeers().unwrap().iter().any(|p| p.nodeId == "remove-mac"));

        // Removal ejects the device from the Space and deletes the inbound
        // session, pending records and Space channel.
        ios.removeDeviceSpaceMember("remove-mac".into()).await.unwrap();
        assert!(!iosPeer.pairedPeers().unwrap().iter().any(|p| p.nodeId == "remove-mac"));
        assert!(iosRouter.spaceChannelScope("remove-mac").unwrap().is_none());
        let iosStore = PeerStateStore::new(iosRouter.localCore.runtimeStorageHost());
        assert!(iosStore
            .records::<crate::PeerStateStore::StoredInbound>(
                operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH
            )
            .unwrap()
            .is_empty());
        // The policy forgets the device completely and the Space ejects it:
        // a fresh pairing plus a new join approval is the only way back.
        let state = iosRouter.networkControlStore.currentState().unwrap();
        assert!(!state.memberNodeIds.contains("remove-mac"));
        assert!(!state.disconnectedNodeIds.contains("remove-mac"));
        assert!(!iosRouter.spaceStore.contains("remove-mac".into()).unwrap());

        // The removed device keeps its stale outbound record until it forgets it.
        assert!(macPeer.pairedPeers().unwrap().iter().any(|p| p.nodeId == "remove-ios"));
        macPeer.removePairedPeer("remove-ios").await.unwrap();
        assert!(macPeer.pairedPeers().unwrap().is_empty());
        assert!(mac.outgoingDeviceSpaceJoins().unwrap().is_empty(),
            "forgetting credentials must also retire their old join request");

        // Re-pairing the same node succeeds instead of PEER_ALREADY_PAIRED.
        let repaired = macPeer
            .startPairing(
                PeerEndpoint {
                    nodeId: "remove-ios".into(),
                    address,
                },
                PeerTransport::Tcp,
                Some("test-token"),
            )
            .await
            .unwrap();
        assert_ne!(repaired.pairingId, pairing.pairingId);
        let code = iosPeer
            .pairingPrompts()
            .unwrap()
            .into_iter()
            .find(|prompt| prompt.pairingId == repaired.pairingId)
            .unwrap()
            .confirmationCode;
        macPeer.finishPairing(&repaired.pairingId, &code).await.unwrap();
        assert!(macPeer.pairedPeers().unwrap().iter().any(|p| p.nodeId == "remove-ios"));
        assert!(iosPeer.pairedPeers().unwrap().iter().any(|p| p.nodeId == "remove-mac"));

        // The forgotten join history must not block a fresh application, and
        // approval readmits the device through the ordinary join lifecycle.
        let rejoined = mac.requestDeviceSpaceJoin("remove-ios".into()).await.unwrap();
        assert_ne!(rejoined.requestId, request.requestId,
            "a revoked admission must never be reused by a new pairing");
        ios.incomingDeviceSpaceJoins().await.unwrap();
        ios.decideDeviceSpaceJoin(rejoined.requestId.clone(), rejoined.assignmentVersion, true)
            .await
            .unwrap();
        assert_eq!(
            mac.refreshDeviceSpaceJoin(rejoined.requestId)
                .await
                .unwrap()
                .status,
            SpaceJoinStatus::Joined
        );
    })
    .await;
    macPeer.stop().await.unwrap();
    iosPeer.stop().await.unwrap();
    result.unwrap();
}

/// A dialer refused by the listener's Space gate receives the revocation
/// reason instead of a silent close, which clients cannot tell apart from a
/// transport or correlation failure.
#[tokio::test]
async fn real_tcp_listener_gate_reports_revocation_to_dialer() {
    use crate::HostRuntimePeerService::HostRuntimePeerService;
    use crate::PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore};
    use operit_host_api::HostManager::HostManager;
    use operit_host_native_common::NativeTcpHost;
    use operit_link::protocol::LinkDeviceInfo;
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (macRouter, mac) = approvalService("gate-mac");
    let (iosRouter, ios) = approvalService("gate-ios");
    let macRouter = Arc::new(macRouter);
    let iosRouter = Arc::new(iosRouter);
    let makeHost = |router: &CoreNodeRouter| {
        Arc::new(HostManager {
            runtimeStorageHost: Some(router.localCore.runtimeStorageHost()),
            tcpHost: Some(Arc::new(NativeTcpHost)),
            hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
            ..HostManager::default()
        })
    };
    let macPeer = HostRuntimePeerService::new(
        makeHost(&macRouter),
        &macRouter,
        LinkDeviceInfo {
            platform: "mac".into(),
            model: "test".into(),
        },
    )
    .unwrap();
    let iosPeer = HostRuntimePeerService::new(
        makeHost(&iosRouter),
        &iosRouter,
        LinkDeviceInfo {
            platform: "ios".into(),
            model: "test".into(),
        },
    )
    .unwrap();
    macRouter
        .installNodeServices(NodeServices::new(macPeer.clone()))
        .unwrap();
    iosRouter
        .installNodeServices(NodeServices::new(iosPeer.clone()))
        .unwrap();
    for router in [&macRouter, &iosRouter] {
        PeerStateStore::new(router.localCore.runtimeStorageHost())
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
    iosPeer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    let address = PeerStateStore::new(iosRouter.localCore.runtimeStorageHost())
        .hostConfig()
        .unwrap()
        .unwrap()
        .bindAddress;
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let pairing = macPeer
            .startPairing(
                PeerEndpoint {
                    nodeId: "gate-ios".into(),
                    address,
                },
                PeerTransport::Tcp,
                Some("test-token"),
            )
            .await
            .unwrap();
        let code = iosPeer
            .pairingPrompts()
            .unwrap()
            .into_iter()
            .find(|prompt| prompt.pairingId == pairing.pairingId)
            .unwrap()
            .confirmationCode;
        macPeer.finishPairing(&pairing.pairingId, &code).await.unwrap();
        let request = mac.requestDeviceSpaceJoin("gate-ios".into()).await.unwrap();
        ios.incomingDeviceSpaceJoins().await.unwrap();
        ios.decideDeviceSpaceJoin(request.requestId, request.assignmentVersion, true)
            .await
            .unwrap();

        // The Space owner revokes the member without syncing the policy to it,
        // so the dialer's own gate still lets the connection attempt through.
        ios.disconnectDeviceSpaceNode("gate-mac".into()).await.unwrap();
        iosPeer.disconnectPeer("gate-mac").await.unwrap();
        macPeer.disconnectPeer("gate-ios").await.unwrap();
        let refused = macPeer
            .call(
                "gate-ios",
                RoutedCoreRequest {
                    spaceId: "gate-space".into(),
                    originNodeId: "gate-mac".into(),
                    targetNodeId: "gate-ios".into(),
                    ttl: 0,
                    routeKind: RoutedCoreRequestKind::Target,
                    payload: CoreCallRequest::new(
                        "revoked",
                        NODE_SPACE_TARGET,
                        "snapshot",
                        CoreValue::Null,
                    ),
                },
            )
            .await;
        assert_eq!(
            refused.result.unwrap_err().code,
            "PEER_CONNECTION_REVOKED",
            "the listener must report the policy gate instead of closing silently"
        );
    })
    .await;
    macPeer.stop().await.unwrap();
    iosPeer.stop().await.unwrap();
    result.unwrap();
}

/// Exercises shared pairing with client-only Host capabilities and no TCP or listener on the initiator.
#[tokio::test]
async fn outbound_only_hosts_pair_over_http_and_websocket() {
    use crate::HostRuntimePeerService::HostRuntimePeerService;
    use crate::PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore};
    use operit_host_api::HostManager::HostManager;
    use operit_host_native_common::{NativeHttpHost, NativeHttpServerHost};
    use operit_link::protocol::LinkDeviceInfo;

    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    for (transport, scheme) in [
        (PeerTransport::Http, "http"),
        (PeerTransport::WebSocket, "ws"),
    ] {
        let (clientRouter, _) = approvalService(&format!("client-only-{scheme}"));
        let (serverRouter, _) = approvalService(&format!("server-{scheme}"));
        let clientRouter = Arc::new(clientRouter);
        let serverRouter = Arc::new(serverRouter);
        let network = Arc::new(NativeHttpHost::new());
        let clientHost = Arc::new(HostManager {
            runtimeStorageHost: Some(clientRouter.localCore.runtimeStorageHost()),
            httpHost: Some(network.clone()),
            webSocketHost: Some(network),
            hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
            ..HostManager::default()
        });
        assert!(clientHost.tcpHost.is_none());
        assert!(clientHost.httpServerHost.is_none());
        assert!(clientHost.serviceDiscoveryHost.is_none());
        let serverHost = Arc::new(HostManager {
            runtimeStorageHost: Some(serverRouter.localCore.runtimeStorageHost()),
            httpServerHost: Some(Arc::new(NativeHttpServerHost)),
            hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
            ..HostManager::default()
        });
        let info = LinkDeviceInfo {
            platform: "test".into(),
            model: scheme.into(),
        };
        let clientPeer =
            HostRuntimePeerService::new(clientHost.clone(), &clientRouter, info.clone()).unwrap();
        let serverPeer = HostRuntimePeerService::new(serverHost, &serverRouter, info).unwrap();
        clientRouter
            .installNodeServices(NodeServices::new(clientPeer.clone()))
            .unwrap();
        serverRouter
            .installNodeServices(NodeServices::new(serverPeer.clone()))
            .unwrap();
        let client = RuntimeRemoteLinkService::newWithRouter(
            (*clientRouter.localCore).clone(),
            (*clientRouter).clone(),
        );
        let server = RuntimeRemoteLinkService::newWithRouter(
            (*serverRouter.localCore).clone(),
            (*serverRouter).clone(),
        );
        PeerStateStore::new(serverRouter.localCore.runtimeStorageHost())
            .saveHostConfig(&PeerHostConfig {
                bindAddress: "127.0.0.1:0".into(),
                token: "required-test-token".into(),
                transports: vec![transport],
                discoveryEnabled: false,
                portMode: PeerHostPortMode::Automatic,
                updatedAt: 1,
            })
            .unwrap();
        server.startListening(vec![transport]).await.unwrap();
        let address = PeerStateStore::new(serverRouter.localCore.runtimeStorageHost())
            .hostConfig()
            .unwrap()
            .unwrap()
            .bindAddress;
        let endpoint = format!("{scheme}://{address}/link");
        let result = tokio::time::timeout(Duration::from_secs(20), async {
            let promptFlow = server.pairingPromptsFlow().unwrap();
            let clientPromptFlow = client.pairingPromptsFlow().unwrap();
            let overview = client.deviceSpaceSnapshotFlow().unwrap();
            assert!(promptFlow.value().is_empty());
            assert!(clientPromptFlow.value().is_empty());
            assert!(client.discoverPeers(1).await.is_err());
            assert!(client.startPairing(serverRouter.localNodeId(), endpoint.clone(), transport,
                Some("wrong-token".into())).await.is_err());
            assert!(server.pairingPrompts().unwrap().is_empty());
            let pairing = client.startPairing(serverRouter.localNodeId(), endpoint.clone(), transport,
                Some("required-test-token".into())).await.unwrap();
            let prompts = server.pairingPrompts().unwrap();
            assert_eq!(prompts.len(), 1);
            let code = prompts.iter().find(|prompt| prompt.pairingId == pairing.pairingId)
                .unwrap().confirmationCode.clone();
            assert_eq!(code.len(), 6);
            let wrongCode = if code == "000000" { "111111" } else { "000000" };
            assert!(client.finishPairing(pairing.pairingId.clone(), wrongCode.into()).await.is_err());
            assert!(clientPeer.pairedPeers().unwrap().is_empty());
            let paired = client.finishPairing(pairing.pairingId, code).await.unwrap();
            assert!(paired.outbound && !paired.inbound);
            assert_eq!(paired.nodeId, serverRouter.localNodeId());
            assert!(client.pairedDeviceOnline(serverRouter.localNodeId()).unwrap());
            assert!(server.pairedDeviceOnline(clientRouter.localNodeId()).unwrap());
            assert!(server.pairingPrompts().unwrap().is_empty());
            let stored = clientPeer.pairedPeers().unwrap();
            assert_eq!(stored.len(), 1);
            assert!(stored[0].outbound && !stored[0].inbound);
            assert!(serverPeer.pairedPeers().unwrap()[0].inbound);
            for nodeId in [serverRouter.localNodeId(), String::new()] {
                let duplicate = clientPeer.startPairing(PeerEndpoint { nodeId, address: endpoint.clone() },
                    transport, Some("required-test-token")).await.unwrap_err();
                assert_eq!(duplicate.code, "PEER_ALREADY_PAIRED");
            }
            let reverse = serverPeer.startPairing(PeerEndpoint { nodeId: clientRouter.localNodeId(), address: endpoint.clone() },
                transport, Some("required-test-token")).await.unwrap_err();
            assert_eq!(reverse.code, "PEER_ALREADY_PAIRED");
            assert!(server.pairingPrompts().unwrap().is_empty());
            let clientStore = PeerStateStore::new(clientRouter.localCore.runtimeStorageHost());
            let serverStore = PeerStateStore::new(serverRouter.localCore.runtimeStorageHost());
            assert_eq!(clientStore.records::<crate::PeerStateStore::StoredOutbound>(operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap().len(), 1);
            assert_eq!(serverStore.records::<crate::PeerStateStore::StoredInbound>(operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap().len(), 1);
            drop((promptFlow, clientPromptFlow, overview));
            clientPeer.stop().await.unwrap();
            let restored = HostRuntimePeerService::new(clientHost.clone(), &clientRouter,
                LinkDeviceInfo { platform: "test".into(), model: "restored".into() }).unwrap();
            let mut changed = restored.subscribePeerChanges();
            while !restored.activePeerNodeIds().unwrap().contains(&serverRouter.localNodeId()) {
                changed.recv().await.unwrap();
            }
            assert!(restored.pairedPeers().unwrap()[0].outbound);
            restored.stop().await.unwrap();
        }).await;
        clientPeer.stop().await.unwrap();
        serverPeer.stop().await.unwrap();
        result.unwrap();
    }
}

/// Preflights mixed requests atomically and keeps unsupported advertisements independent of TCP.
#[tokio::test]
async fn listener_capabilities_validate_all_transports_before_binding() {
    use crate::HostRuntimePeerService::HostRuntimePeerService;
    use crate::PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore};
    use operit_host_api::{
        HostManager::{defaultHostRuntimeTaskSchedulerHost, HostManager},
        HostResult, TcpConnection, TcpHost, TcpListener,
    };
    use operit_host_native_common::Tcp::NativeTcpHost;
    use operit_link::protocol::LinkDeviceInfo;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingTcpHost(Arc<AtomicUsize>);
    #[async_trait::async_trait]
    impl TcpHost for CountingTcpHost {
        /// Delegates outgoing connections to the real Host socket provider.
        async fn connect(&self, address: &str) -> HostResult<Arc<dyn TcpConnection>> {
            NativeTcpHost.connect(address).await
        }
        /// Records every resource acquisition before delegating to real TCP sockets.
        async fn bind(&self, address: &str) -> HostResult<Arc<dyn TcpListener>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            NativeTcpHost.bind(address).await
        }
    }

    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, _) = approvalService("listener-capabilities");
    let router = Arc::new(router);
    let binds = Arc::new(AtomicUsize::new(0));
    let host = Arc::new(HostManager {
        runtimeStorageHost: Some(router.localCore.runtimeStorageHost()),
        tcpHost: Some(Arc::new(CountingTcpHost(binds.clone()))),
        hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
        ..HostManager::default()
    });
    let peer = HostRuntimePeerService::new(
        host,
        &router,
        LinkDeviceInfo {
            platform: "test".into(),
            model: "test".into(),
        },
    )
    .unwrap();
    router
        .installNodeServices(NodeServices::new(peer.clone()))
        .unwrap();
    let facade =
        RuntimeRemoteLinkService::newWithRouter((*router.localCore).clone(), (*router).clone());
    let capabilities = facade.listenerCapabilities().unwrap();
    assert_eq!(capabilities.transports, vec![PeerTransport::Tcp]);
    assert!(!capabilities.discoveryAdvertisement);
    let store = PeerStateStore::new(router.localCore.runtimeStorageHost());
    store
        .saveHostConfig(&PeerHostConfig {
            bindAddress: "127.0.0.1:0".into(),
            token: "capability-test-token".into(),
            transports: vec![PeerTransport::Tcp, PeerTransport::Http],
            discoveryEnabled: true,
            portMode: PeerHostPortMode::Automatic,
            updatedAt: 1,
        })
        .unwrap();
    for unsupported in [
        PeerTransport::Http,
        PeerTransport::WebSocket,
        PeerTransport::Serial,
        PeerTransport::Bluetooth,
    ] {
        let error = peer
            .startListening(&[PeerTransport::Tcp, unsupported])
            .await
            .unwrap_err();
        assert_eq!(
            error.message,
            format!("Host does not support {unsupported:?} peer listeners")
        );
        assert_eq!(binds.load(Ordering::SeqCst), 0);
        assert_eq!(
            store.hostConfig().unwrap().unwrap().bindAddress,
            "127.0.0.1:0"
        );
    }
    peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    // Another process may already hold the automatic port; the fallback retry
    // then binds twice while still binding only the requested transport.
    assert!(binds.load(Ordering::SeqCst) >= 1);
    let config = store.hostConfig().unwrap().unwrap();
    assert!(config.discoveryEnabled);
    assert_eq!(
        config.transports,
        vec![PeerTransport::Tcp, PeerTransport::Http]
    );
    peer.stop().await.unwrap();
}

/// Filters inbound and outbound pairings in Core discovery and exposes removed devices again.
#[tokio::test]
async fn core_discovery_returns_only_unpaired_devices_for_all_callers() {
    use crate::HostRuntimePeerService::HostRuntimePeerService;
    use crate::PeerStateStore::{
        PeerStateStore, StoredInbound, StoredOutbound, PAIRING_SERVICE_VERSION,
    };
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use operit_host_api::HostManager::HostManager;
    use operit_host_api::ServiceDiscovery::{
        DiscoveredService, DiscoveryCallback, DiscoverySubscription, ServiceDiscoveryHost,
    };
    use operit_host_api::{HostError, HostResult};
    use operit_link::protocol::LinkDeviceInfo;
    use operit_util::RuntimeStorageLayout::{
        RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
    };

    struct SnapshotDiscovery(Vec<DiscoveredService>);
    impl ServiceDiscoveryHost for SnapshotDiscovery {
        /// Declares this test Host's snapshot-only discovery capability.
        fn supportsAdvertisement(&self) -> bool {
            false
        }
        /// Returns advertisements without granting authentication to any discovered device.
        fn discover(&self, serviceType: &str, _: u64) -> HostResult<Vec<DiscoveredService>> {
            assert_eq!(serviceType, "_operit-link._tcp.local.");
            Ok(self.0.clone())
        }
        /// Rejects subscriptions outside this test's snapshot discovery contract.
        fn subscribe(
            &self,
            _: &str,
            _: DiscoveryCallback,
        ) -> HostResult<Box<dyn DiscoverySubscription>> {
            Err(HostError::new(
                "Snapshot discovery does not provide subscriptions",
            ))
        }
    }

    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, _) = approvalService("discovery-local");
    let router = Arc::new(router);
    let record = |node: &str, ip: &str| DiscoveredService {
        fullName: "test".into(),
        hostname: "test.local.".into(),
        port: 37195,
        addresses: vec![ip.parse().unwrap()],
        properties: [
            ("nodeId".into(), node.into()),
            ("displayName".into(), "Same name".into()),
            ("transports".into(), "http,ws,tcp".into()),
        ]
        .into_iter()
        .collect(),
    };
    let host = Arc::new(HostManager {
        runtimeStorageHost: Some(router.localCore.runtimeStorageHost()),
        serviceDiscoveryHost: Some(Arc::new(SnapshotDiscovery(vec![
            record("discovery-local", "192.168.1.1"),
            record("incoming", "192.168.1.2"),
            record("incoming", "fd00::2"),
            record("outgoing", "192.168.1.3"),
            record("new", "192.168.1.4"),
        ]))),
        hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
        ..HostManager::default()
    });
    let info = LinkDeviceInfo {
        platform: "test".into(),
        model: "test".into(),
    };
    let peer = HostRuntimePeerService::new(host, &router, info.clone()).unwrap();
    router
        .installNodeServices(NodeServices::new(peer.clone()))
        .unwrap();
    let service =
        RuntimeRemoteLinkService::newWithRouter((*router.localCore).clone(), (*router).clone());
    assert_eq!(peer.discoverPeers(1).await.unwrap().len(), 3);
    let store = PeerStateStore::new(router.localCore.runtimeStorageHost());
    store
        .putRecord(
            RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
            "incoming-session",
            &StoredInbound {
                deviceId: "incoming".into(),
                deviceInfo: info.clone(),
                pairingServiceVersion: PAIRING_SERVICE_VERSION,
                sessionSecret: BASE64.encode([1; 32]),
            },
        )
        .unwrap();
    store
        .putRecord(
            RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
            "outgoing-session",
            &StoredOutbound {
                endpoint: "192.168.1.3:37195".into(),
                sessionId: "outgoing-session".into(),
                deviceId: router.localNodeId(),
                peerNodeId: "outgoing".into(),
                peerDeviceInfo: info,
                pairingServiceVersion: PAIRING_SERVICE_VERSION,
                sessionSecret: BASE64.encode([2; 32]),
                transport: "tcp".into(),
            },
        )
        .unwrap();
    assert!(peer.activePeerNodeIds().unwrap().is_empty());
    let direct = peer.discoverPeers(1).await.unwrap();
    let app = service.discoverPeers(1).await.unwrap();
    assert_eq!(direct, app);
    assert_eq!(
        app.iter()
            .map(|candidate| candidate.nodeId.as_str())
            .collect::<Vec<_>>(),
        vec!["new"]
    );
    peer.removePairedPeer("incoming").await.unwrap();
    let app = service.discoverPeers(1).await.unwrap();
    assert_eq!(
        app.iter()
            .map(|candidate| candidate.nodeId.as_str())
            .collect::<Vec<_>>(),
        vec!["incoming", "new"]
    );
    peer.removePairedPeer("outgoing").await.unwrap();
    assert_eq!(service.discoverPeers(1).await.unwrap().len(), 3);
    peer.stop().await.unwrap();
}
