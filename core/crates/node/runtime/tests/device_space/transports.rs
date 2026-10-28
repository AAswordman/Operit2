use super::*;

/// One TCP pairing carries both directions before and after admission, with no second carrier.
#[tokio::test]
async fn real_tcp_single_pairing_uses_one_bidirectional_channel() {
    tcpAdmissionSingleChannel("127.0.0.1:0").await;
}

/// An IPv6 listener does not require a separate reverse connection.
#[tokio::test]
async fn real_tcp_single_channel_with_ipv6_listener() {
    tcpAdmissionSingleChannel("[::1]:0").await;
}

/// A wildcard listener uses the same authenticated carrier in both directions.
#[tokio::test]
async fn real_tcp_single_channel_with_wildcard_listener() {
    tcpAdmissionSingleChannel("0.0.0.0:0").await;
}

/// Exercises admission, bidirectional calls and reconnection on a single authenticated carrier.
async fn tcpAdmissionSingleChannel(macBind: &str) {
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
        pairingIncarnation: None,
    },
    )
    .unwrap();
    let iosPeer = HostRuntimePeerService::new(
        makeHost(&iosRouter),
        &iosRouter,
        LinkDeviceInfo {
            platform: "ios".into(),
            model: "test".into(),
        pairingIncarnation: None,
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
    for (router, peer, bind) in [(&macRouter, &macPeer, macBind), (&iosRouter, &iosPeer, "127.0.0.1:0")] {
        let reservation = std::net::TcpListener::bind(bind).unwrap();
        let bind = reservation.local_addr().unwrap().to_string();
        drop(reservation);
        PeerStateStore::new(router.localCore.runtimeStorageHost())
            .saveHostConfig(&PeerHostConfig {
                bindAddress: bind.into(),

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
                Some(PeerTransport::Tcp),
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
        // Pre-admission applicant traffic rides the gateway's own connection:
        // the join snapshot uses the same carrier before Space admission.
        // Admission grants business authority without creating another connection.
        let request = mac.requestDeviceSpaceJoin("tcp-ios".into()).await.unwrap();
        assert!(iosRouter.spaceChannelScope("tcp-mac").unwrap().is_none());
        assert!(macRouter.spaceChannelScope("tcp-ios").unwrap().is_none());
        let pendingReturn = iosPeer
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
            .await;
        assert!(
            pendingReturn.result.is_ok(),
            "pre-admission join traffic rides the paired connection: {:?}",
            pendingReturn.result
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
        assert!(iosPeer.spaceClient().is_some(), "The admitted carrier exposes the shared Space client");
        // The receiver reuses the caller's connection without opening a reverse socket.
        assert_eq!(iosPeer.channelCount("tcp-mac").await, 1);
        assert_eq!(macPeer.channelCount("tcp-ios").await, 1);
        // Different origins may reuse the same request ID at a shared relay hop.
        let (first, second) = tokio::join!(
            iosPeer.call("tcp-mac", returnRequest()),
            iosPeer.call("tcp-mac", returnRequest()),
        );
        for response in [first, second] {
            assert_eq!(response.requestId.0, "return-space");
            assert!(response.result.is_ok(), "Concurrent correlation: {:?}", response.result);
        }
        // Only the pairing initiator reconnects; the receiver never opens a reverse socket.
        iosPeer.disconnectPeer("tcp-mac").await.unwrap();
        macPeer.disconnectPeer("tcp-ios").await.unwrap();
        let forward = macPeer.call("tcp-ios", RoutedCoreRequest {
            spaceId: mac.deviceSpace().unwrap().spaceId,
            originNodeId: "tcp-mac".into(),
            targetNodeId: "tcp-ios".into(),
            ttl: 0,
            routeKind: RoutedCoreRequestKind::Target,
            payload: PeerSyncMethod::DeviceSpace.request("reconnect-space".into(), CoreValue::Null),
        }).await;
        assert!(forward.result.is_ok(), "{:?}", forward.result);
        assert!(iosPeer.call("tcp-mac", returnRequest()).await.result.is_ok());
        assert_eq!(iosPeer.channelCount("tcp-mac").await, 1);
        assert_eq!(macPeer.channelCount("tcp-ios").await, 1);
        // Admission does not expose local-only management surfaces.
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
        pairingIncarnation: None,
    },
    )
    .unwrap();
    let iosPeer = HostRuntimePeerService::new(
        makeHost(&iosRouter),
        &iosRouter,
        LinkDeviceInfo {
            platform: "ios".into(),
            model: "test".into(),
        pairingIncarnation: None,
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
                Some(PeerTransport::Tcp),
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
                Some(PeerTransport::Tcp),
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
        pairingIncarnation: None,
    },
    )
    .unwrap();
    let iosPeer = HostRuntimePeerService::new(
        makeHost(&iosRouter),
        &iosRouter,
        LinkDeviceInfo {
            platform: "ios".into(),
            model: "test".into(),
        pairingIncarnation: None,
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
                Some(PeerTransport::Tcp),
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
        pairingIncarnation: None,
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
            assert!(server.pairingPrompts().unwrap().is_empty());
            let pairing = client.startPairing(serverRouter.localNodeId(), endpoint.clone(), Some(transport)).await.unwrap();
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
                    Some(transport)).await.unwrap_err();
                assert_eq!(duplicate.code, "PEER_ALREADY_PAIRED");
            }
            let reverse = serverPeer.startPairing(PeerEndpoint { nodeId: clientRouter.localNodeId(), address: endpoint.clone() },
                Some(transport)).await.unwrap_err();
            assert_eq!(reverse.code, "PEER_ALREADY_PAIRED");
            // Address-only discovery learned the identity after starting a new
            // challenge. Receiver now allows challenges for retained pairings;
            // without a fresh code this must not alter the stored credential.
            assert_eq!(server.pairingPrompts().unwrap().len(), 1);
            let clientStore = PeerStateStore::new(clientRouter.localCore.runtimeStorageHost());
            let serverStore = PeerStateStore::new(serverRouter.localCore.runtimeStorageHost());
            assert_eq!(clientStore.records::<crate::PeerStateStore::StoredOutbound>(operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap().len(), 1);
            assert_eq!(serverStore.records::<crate::PeerStateStore::StoredInbound>(operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap().len(), 1);
            // One-sided unpair and a new code replace the receiver's credential
            // over both HTTP and WebSocket, without deleting the remote record
            // until confirmation succeeds.
            let old = serverStore.records::<crate::PeerStateStore::StoredInbound>(operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap();
            clientPeer.removePairedPeer(&serverRouter.localNodeId()).await.unwrap();
            assert_eq!(serverPeer.pairedPeers().unwrap().len(), 1);
            let fresh = client.startPairing(serverRouter.localNodeId(), endpoint.clone(), Some(transport)).await.unwrap();
            let code = server.pairingPrompts().unwrap().into_iter()
                .find(|prompt| prompt.pairingId == fresh.pairingId).unwrap().confirmationCode;
            assert!(old.keys().all(|id| serverStore.records::<crate::PeerStateStore::StoredInbound>(operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap().contains_key(id)));
            client.finishPairing(fresh.pairingId.clone(), code).await.unwrap();
            let updated = serverStore.records::<crate::PeerStateStore::StoredInbound>(operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap();
            assert_eq!(updated.len(), 1);
            assert!(updated.contains_key(&fresh.pairingId));
            assert!(old.keys().all(|id| !updated.contains_key(id)));
            assert!(server.pairingPrompts().unwrap().is_empty());
            drop((promptFlow, clientPromptFlow, overview));
            clientPeer.stop().await.unwrap();
            let restored = HostRuntimePeerService::new(clientHost.clone(), &clientRouter,
                LinkDeviceInfo { platform: "test".into(), model: "restored".into(), pairingIncarnation: None }).unwrap();
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
        pairingIncarnation: None,
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
    assert_eq!(binds.load(Ordering::SeqCst), 1);
    assert_ne!(store.hostConfig().unwrap().unwrap().bindAddress, "127.0.0.1:0");
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
    pairingIncarnation: None,
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

/// Automatic ports recover from collisions and reuse their allocation on restart.
#[tokio::test]
async fn listener_port_policy_uses_host_allocation_and_preserves_explicit_fixed_ports() {
    use crate::HostRuntimePeerService::HostRuntimePeerService;
    use crate::PeerStateStore::{PeerHostPortMode, PeerStateStore};
    use operit_host_api::HostManager::HostManager;
    use operit_host_native_common::NativeTcpHost;
    use operit_link::protocol::LinkDeviceInfo;
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, _) = approvalService("port-policy");
    let router = Arc::new(router);
    let host = Arc::new(HostManager {
        runtimeStorageHost: Some(router.localCore.runtimeStorageHost()),
        tcpHost: Some(Arc::new(NativeTcpHost)),
        hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
        ..HostManager::default()
    });
    let peer = HostRuntimePeerService::new(
        host,
        &router,
        LinkDeviceInfo {
            platform: "test".into(),
            model: "test".into(),
            pairingIncarnation: None,
        },
    )
    .unwrap();
    let store = PeerStateStore::new(router.localCore.runtimeStorageHost());
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let occupiedAddress = occupied.local_addr().unwrap().to_string();
    let mut config = store.hostConfig().unwrap().unwrap();
    config.bindAddress = occupiedAddress.clone();
    config.transports = vec![PeerTransport::Tcp];
    config.discoveryEnabled = false;
    store.saveHostConfig(&config).unwrap();
    peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    let selected = store.hostConfig().unwrap().unwrap();
    assert_eq!(selected.portMode, PeerHostPortMode::Automatic);
    assert_ne!(selected.bindAddress, occupiedAddress);
    let socket: std::net::SocketAddr = selected.bindAddress.parse().unwrap();
    assert!(socket.ip().is_loopback());
    assert_ne!(socket.port(), 0);
    assert!(std::net::TcpListener::bind(socket).is_err());
    peer.stop().await.unwrap();
    peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    assert_eq!(
        store.hostConfig().unwrap().unwrap().bindAddress,
        selected.bindAddress
    );
    peer.stop().await.unwrap();

    config.portMode = PeerHostPortMode::Fixed;
    store.saveHostConfig(&config).unwrap();
    assert!(peer.startListening(&[PeerTransport::Tcp]).await.is_err());
    assert_eq!(
        store.hostConfig().unwrap().unwrap().bindAddress,
        occupiedAddress
    );
    drop(occupied);
    peer.startListening(&[PeerTransport::Tcp]).await.unwrap();
    assert_eq!(
        store.hostConfig().unwrap().unwrap().bindAddress,
        occupiedAddress
    );
    peer.stop().await.unwrap();
    config.bindAddress = "127.0.0.1:0".into();
    store.saveHostConfig(&config).unwrap();
    assert!(peer
        .startListening(&[PeerTransport::Tcp])
        .await
        .unwrap_err()
        .message
        .contains("Fixed listener port must be nonzero"));
}

/// Default HTTP/WS listeners advertise the same nonzero socket selected by the Host.
#[tokio::test]
async fn automatic_default_listener_advertises_its_actual_shared_port() {
    use crate::HostRuntimePeerService::HostRuntimePeerService;
    use crate::PeerStateStore::{PeerHostPortMode, PeerStateStore};
    use operit_host_api::HostManager::HostManager;
    use operit_host_api::ServiceDiscovery::{
        DiscoveredService, DiscoveryAdvertisement, DiscoveryCallback, DiscoverySubscription,
        ServiceAdvertisement, ServiceDiscoveryHost,
    };
    use operit_host_api::{HostError, HostResult};
    use operit_host_native_common::NativeHttpServerHost;
    use operit_link::protocol::LinkDeviceInfo;

    #[derive(Default)]
    struct Discovery(std::sync::Mutex<Vec<ServiceAdvertisement>>);
    struct Advertisement;
    impl DiscoveryAdvertisement for Advertisement {}
    impl ServiceDiscoveryHost for Discovery {
        fn localAddresses(&self) -> HostResult<Vec<std::net::IpAddr>> {
            Ok(vec!["192.168.1.44".parse().unwrap(), "127.0.0.1".parse().unwrap()])
        }
        fn supportsAdvertisement(&self) -> bool {
            true
        }
        fn advertise(
            &self,
            record: ServiceAdvertisement,
        ) -> HostResult<Box<dyn DiscoveryAdvertisement>> {
            self.0.lock().unwrap().push(record);
            Ok(Box::new(Advertisement))
        }
        fn discover(&self, _: &str, _: u64) -> HostResult<Vec<DiscoveredService>> {
            Ok(vec![])
        }
        fn subscribe(
            &self,
            _: &str,
            _: DiscoveryCallback,
        ) -> HostResult<Box<dyn DiscoverySubscription>> {
            Err(HostError::new("Snapshot-only test discovery"))
        }
    }
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (router, _) = approvalService("automatic-default");
    let router = Arc::new(router);
    let discovery = Arc::new(Discovery::default());
    let host = Arc::new(HostManager {
        runtimeStorageHost: Some(router.localCore.runtimeStorageHost()),
        httpServerHost: Some(Arc::new(NativeHttpServerHost)),
        serviceDiscoveryHost: Some(discovery.clone()),
        hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
        ..HostManager::default()
    });
    let peer = HostRuntimePeerService::new(
        host,
        &router,
        LinkDeviceInfo {
            platform: "test".into(),
            model: "test".into(),
            pairingIncarnation: None,
        },
    )
    .unwrap();
    let store = PeerStateStore::new(router.localCore.runtimeStorageHost());
    let initial = store.hostConfig().unwrap().unwrap();
    assert_eq!(initial.bindAddress, "0.0.0.0:0");
    assert_eq!(initial.portMode, PeerHostPortMode::Automatic);
    peer.startListening(&initial.transports).await.unwrap();
    let bound = store.hostConfig().unwrap().unwrap();
    let address: std::net::SocketAddr = bound.bindAddress.parse().unwrap();
    assert_ne!(address.port(), 0);
    assert!(address.ip().is_unspecified());
    let records = discovery.0.lock().unwrap().clone();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].port, address.port());
    assert_eq!(records[0].properties["transports"], "http,ws");
    assert!(std::net::TcpListener::bind(address).is_err());
    let expected = format!("http://192.168.1.44:{}", address.port());
    assert!(peer.pairingAddresses().await.unwrap().contains(&expected));
    peer.stop().await.unwrap();
    assert!(peer.pairingAddresses().await.is_err(), "stopped listeners cannot produce a code");
    let mut config = store.hostConfig().unwrap().unwrap();
    config.discoveryEnabled = false;
    store.saveHostConfig(&config).unwrap();
    peer.startListening(&config.transports).await.unwrap();
    let current: std::net::SocketAddr = store.hostConfig().unwrap().unwrap().bindAddress.parse().unwrap();
    let info = crate::NodeServices::PeerConnectionInfo {
        version: crate::PeerConnectionCode::VERSION, nodeId: router.localNodeId(),
        displayName: "CLI".into(), addresses: peer.pairingAddresses().await.unwrap(),
    };
    let code = crate::PeerConnectionCode::encode(&info).unwrap();
    let decoded = crate::PeerConnectionCode::decode(&code, "scanner").unwrap();
    assert!(decoded.addresses.contains(&format!("http://192.168.1.44:{}", current.port())));
    assert_eq!(discovery.0.lock().unwrap().len(), 1, "QR generation does not advertise or browse");
    peer.stop().await.unwrap();
}

/// A broken address stays local to that attempt; the successful handshake still binds
/// the advertised identity and requires the receiver's ordinary confirmation code.
#[tokio::test]
async fn qr_pairing_isolates_addresses_and_checks_identity_before_confirmation() {
    use crate::HostRuntimePeerService::HostRuntimePeerService;
    use crate::PeerConnectionCode;
    use crate::PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore};
    use operit_host_api::{HostManager::HostManager, HostResult, TcpConnection, TcpHost, TcpListener};
    use operit_host_native_common::NativeTcpHost;
    use operit_link::protocol::LinkDeviceInfo;

    struct QrTcpHost {
        destination: String,
        attempts: StdMutex<Vec<String>>,
    }
    #[async_trait]
    impl TcpHost for QrTcpHost {
        async fn connect(&self, address: &str) -> HostResult<Arc<dyn TcpConnection>> {
            self.attempts.lock().unwrap().push(address.into());
            if address.starts_with("192.0.2.1:") { panic!("one QR endpoint panicked"); }
            if address.starts_with("192.0.2.2:") { return std::future::pending().await; }
            if address.starts_with("192.0.2.3:") { return Err(operit_host_api::HostError::new("unreachable endpoint")); }
            NativeTcpHost.connect(&self.destination).await
        }
        async fn bind(&self, address: &str) -> HostResult<Arc<dyn TcpListener>> {
            NativeTcpHost.bind(address).await
        }
    }

    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (serverRouter, server) = approvalService("qr-server");
    let (clientRouter, client) = approvalService("qr-client");
    let serverHost = Arc::new(HostManager {
        runtimeStorageHost: Some(serverRouter.localCore.runtimeStorageHost()),
        tcpHost: Some(Arc::new(NativeTcpHost)),
        hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
        ..HostManager::default()
    });
    let info = LinkDeviceInfo { platform: "test".into(), model: "qr".into(), pairingIncarnation: None };
    let serverPeer = HostRuntimePeerService::new(serverHost, &Arc::new(serverRouter.clone()), info.clone()).unwrap();
    serverRouter.installNodeServices(NodeServices::new(serverPeer.clone())).unwrap();
    PeerStateStore::new(serverRouter.localCore.runtimeStorageHost()).saveHostConfig(&PeerHostConfig {
        bindAddress: "127.0.0.1:0".into(), transports: vec![PeerTransport::Tcp],
        discoveryEnabled: false, portMode: PeerHostPortMode::Automatic, updatedAt: 1,
    }).unwrap();
    server.startListening(vec![PeerTransport::Tcp]).await.unwrap();
    let destination = PeerStateStore::new(serverRouter.localCore.runtimeStorageHost())
        .hostConfig().unwrap().unwrap().bindAddress;
    let mappedHost = Arc::new(QrTcpHost { destination, attempts: StdMutex::new(Vec::new()) });
    let clientHost = Arc::new(HostManager {
        runtimeStorageHost: Some(clientRouter.localCore.runtimeStorageHost()),
        tcpHost: Some(mappedHost.clone()),
        hostRuntimeTaskSchedulerHost: Some(defaultHostRuntimeTaskSchedulerHost()),
        ..HostManager::default()
    });
    let clientPeer = HostRuntimePeerService::new(clientHost, &Arc::new(clientRouter.clone()), info).unwrap();
    clientRouter.installNodeServices(NodeServices::new(clientPeer.clone())).unwrap();
    let connectionInfo = crate::NodeServices::PeerConnectionInfo {
        version: PeerConnectionCode::VERSION, nodeId: serverRouter.localNodeId(), displayName: "QR server".into(),
        addresses: (1..=4).map(|i| format!("tcp://192.0.2.{i}:7777")).collect(),
    };
    let result = tokio::time::timeout(Duration::from_secs(12), async {
        let mut wrongIdentity = connectionInfo.clone();
        wrongIdentity.nodeId = "wrong-device".into();
        wrongIdentity.addresses = vec![connectionInfo.addresses[3].clone()];
        assert!(client.startPairingFromCode(PeerConnectionCode::encode(&wrongIdentity).unwrap()).await.is_err());
        assert!(server.pairingPrompts().unwrap().is_empty());
        mappedHost.attempts.lock().unwrap().clear();

        let pending = client.startPairingFromCode(PeerConnectionCode::encode(&connectionInfo).unwrap()).await.unwrap();
        assert_eq!(pending.peerNodeId, serverRouter.localNodeId());
        assert_eq!(mappedHost.attempts.lock().unwrap().len(), 4);
        assert!(clientPeer.pairedPeers().unwrap().is_empty());
        let prompts = server.pairingPrompts().unwrap();
        assert_eq!(prompts.len(), 1);
        let paired = client.finishPairing(pending.pairingId, prompts[0].confirmationCode.clone()).await.unwrap();
        assert_eq!(paired.nodeId, serverRouter.localNodeId());
        assert!(paired.outbound);
        assert!(serverPeer.pairedPeers().unwrap()[0].inbound);
    }).await;
    clientPeer.stop().await.unwrap();
    serverPeer.stop().await.unwrap();
    result.unwrap();
}
