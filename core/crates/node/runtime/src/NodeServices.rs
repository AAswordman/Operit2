//! 节点服务的共享注入容器；不重复声明 RuntimePeerService 的业务接口。
use super::RuntimePeerService::RuntimePeerService;
pub use operit_peer_link::{PeerEndpoint, PeerTransport};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 可交给 UI 的待确认事务，不包含密钥。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPairing {
    pub pairingId: String,
    pub peerNodeId: String,
    pub displayName: String,
}

/// 局域网发现结果只包含寻址和展示信息；token 不进入发现结果或外围 UI。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    pub nodeId: String,
    pub address: String,
    pub displayName: String,
}

/// 已配对身份，不携带底层连接或凭证；传输渠道不改变身份。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedPeer {
    pub nodeId: String,
    pub displayName: String,
    pub inbound: bool,
    pub outbound: bool,
}

/// 只供本地操作界面展示，不通过匿名 Link 请求返回。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingPrompt {
    pub pairingId: String,
    pub peerNodeId: String,
    pub displayName: String,
    pub confirmationCode: String,
}

/// Volatile transport identity, not a Binding or a business-state replica.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceClientIdentity {
    pub spaceId: String,
    pub peerNodeId: String,
    pub generation: String,
}

pub struct SpaceClientConnection {
    pub identity: SpaceClientIdentity,
    pub client: Arc<dyn operit_link::CoreLinkSharedClient + Send + Sync>,
}

/// 一个节点共享一个实例；业务操作由注入的 RuntimePeerService 负责。
/// 克隆只共享服务，不创建第二套配对记录、监听器或连接。
#[derive(Clone)]
pub struct NodeServices {
    peers: Arc<dyn RuntimePeerService>,
}

impl NodeServices {
    /// 由启动装配注入真正的服务；没有默认成功实现或全局实例。
    pub fn new(peers: Arc<dyn RuntimePeerService>) -> Self {
        Self { peers }
    }

    /// Router 使用同一服务转发 call/watch/push，不另建通信实例。
    pub fn peers(&self) -> Arc<dyn RuntimePeerService> {
        self.peers.clone()
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use operit_link::*;
    use std::sync::Mutex;

    struct RecordingPeerService {
        calls: Mutex<Vec<String>>,
        changes: tokio::sync::broadcast::Sender<()>,
    }

    impl Default for RecordingPeerService {
        fn default() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                changes: tokio::sync::broadcast::channel(1).0,
            }
        }
    }

    #[async_trait(?Send)]
    impl RuntimePeerService for RecordingPeerService {
        async fn discoverPeers(
            &self,
            _timeoutMs: u64,
        ) -> Result<Vec<DiscoveredPeer>, CoreLinkError> {
            Ok(vec![DiscoveredPeer {
                nodeId: "board".into(),
                address: "lan".into(),
                displayName: "board".into(),
            }])
        }
        async fn startPairing(
            &self,
            target: PeerEndpoint,
            transport: PeerTransport,
            token: Option<&str>,
        ) -> Result<PendingPairing, CoreLinkError> {
            // LAN discovery may intentionally start without a user-supplied token.
            let _token = token;
            self.calls.lock().unwrap().push(format!(
                "start:{}:{}:{transport:?}",
                target.nodeId, target.address
            ));
            Ok(PendingPairing {
                pairingId: "transaction".into(),
                peerNodeId: target.nodeId,
                displayName: "board".into(),
            })
        }
        async fn finishPairing(&self, id: &str, code: &str) -> Result<PairedPeer, CoreLinkError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("finish:{id}:{code}"));
            Err(CoreLinkError::new("REJECTED", "test rejection"))
        }
        async fn cancelPairing(&self, id: &str) -> Result<(), CoreLinkError> {
            self.calls.lock().unwrap().push(format!("cancel:{id}"));
            Ok(())
        }
        fn pairedPeers(&self) -> Result<Vec<PairedPeer>, CoreLinkError> {
            Ok(vec![PairedPeer {
                nodeId: "board".into(),
                displayName: "board".into(),
                inbound: false,
                outbound: true,
            }])
        }
        fn outboundPeerNodeIds(&self) -> Result<std::collections::BTreeSet<String>, CoreLinkError> {
            Ok(["board".into()].into())
        }
        fn activePeerNodeIds(&self) -> Result<std::collections::BTreeSet<String>, CoreLinkError> {
            Ok(std::collections::BTreeSet::new())
        }
        fn subscribePeerChanges(&self) -> tokio::sync::broadcast::Receiver<()> {
            self.changes.subscribe()
        }
        fn pairingPrompts(&self) -> Result<Vec<PairingPrompt>, CoreLinkError> {
            Ok(vec![PairingPrompt {
                pairingId: "incoming".into(),
                peerNodeId: "other".into(),
                displayName: "desktop".into(),
                confirmationCode: "123456".into(),
            }])
        }
        async fn disconnectPeer(&self, nodeId: &str) -> Result<(), CoreLinkError> {
            self.calls.lock().unwrap().push(format!("disconnect:{nodeId}"));
            Ok(())
        }
        async fn removePairedPeer(&self, id: &str) -> Result<(), CoreLinkError> {
            self.calls.lock().unwrap().push(format!("remove:{id}"));
            Ok(())
        }
        /// Declares the listener operations recorded by this injected test service.
        fn listenerCapabilities(&self) -> operit_peer_link::PeerListenerCapabilities {
            operit_peer_link::PeerListenerCapabilities { transports: vec![PeerTransport::Tcp, PeerTransport::Http, PeerTransport::WebSocket, PeerTransport::Serial, PeerTransport::Bluetooth], discoveryAdvertisement: false }
        }
        async fn startListening(&self, transports: &[PeerTransport]) -> Result<(), CoreLinkError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("listen:{transports:?}"));
            Ok(())
        }
        async fn stop(&self) -> Result<(), CoreLinkError> {
            self.calls.lock().unwrap().push("stop".into());
            Ok(())
        }
        async fn call(
            &self,
            next: &str,
            request: RoutedCoreRequest<CoreCallRequest>,
        ) -> CoreCallResponse {
            self.calls.lock().unwrap().push(format!(
                "call:{next}:{}:{}:{}",
                request.originNodeId, request.targetNodeId, request.ttl
            ));
            CoreCallResponse::ok(request.payload.requestId, CoreValue::Null)
        }
        async fn watchSnapshot(
            &self,
            _: &str,
            _: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEvent, CoreLinkError> {
            panic!("unexpected watchSnapshot")
        }
        async fn watch(
            &self,
            _: &str,
            _: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEventStream, CoreLinkError> {
            panic!("unexpected watch")
        }
        async fn openPush(
            &self,
            _: &str,
            _: RoutedCoreRequest<CorePushRequest>,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            panic!("unexpected push")
        }
    }

    #[tokio::test]
    async fn disconnect_preserves_pairing_and_uses_the_shared_service() {
        let backend = Arc::new(RecordingPeerService::default());
        let services = NodeServices::new(backend.clone());
        services.peers().disconnectPeer("board").await.unwrap();
        assert_eq!(*backend.calls.lock().unwrap(), ["disconnect:board"]);
        assert_eq!(services.peers().pairedPeers().unwrap()[0].nodeId, "board");
    }

    #[tokio::test]
    async fn lan_discovery_does_not_require_or_expose_a_token() {
        let services = NodeServices::new(Arc::new(RecordingPeerService::default()));
        let peers = services.peers().discoverPeers(2000).await.unwrap();
        assert_eq!(peers[0].address, "lan");
        assert_eq!(peers[0].displayName, "board");
    }

    #[tokio::test]
    async fn pairing_and_listening_use_the_injected_service_for_every_transport() {
        let backend = Arc::new(RecordingPeerService::default());
        let services = NodeServices::new(backend.clone());
        for transport in [
            PeerTransport::Http,
            PeerTransport::WebSocket,
            PeerTransport::Tcp,
            PeerTransport::Serial,
            PeerTransport::Bluetooth,
        ] {
            let pending = services.peers()
                .startPairing(
                    PeerEndpoint {
                        nodeId: "board".into(),
                        address: "endpoint".into(),
                    },
                    transport,
                    None,
                )
                .await
                .unwrap();
            assert_eq!(pending.pairingId, "transaction");
            services.peers().startListening(&[transport]).await.unwrap();
        }
        assert_eq!(backend.calls.lock().unwrap().len(), 10);
        assert_eq!(services.peers().pairedPeers().unwrap()[0].nodeId, "board");
        assert_eq!(
            services.peers().pairingPrompts().unwrap()[0].confirmationCode,
            "123456"
        );
    }

    #[tokio::test]
    async fn lan_pairing_can_start_without_user_token_and_revoke_is_device_wide() {
        let backend = Arc::new(RecordingPeerService::default());
        let services = NodeServices::new(backend.clone());
        let endpoint = PeerEndpoint {
            nodeId: "board".into(),
            address: "endpoint".into(),
        };
        services.peers()
            .startPairing(endpoint.clone(), PeerTransport::Tcp, None)
            .await
            .unwrap();
        services.peers()
            .startPairing(endpoint, PeerTransport::Tcp, Some("secret-token"))
            .await
            .unwrap();
        services.peers().removePairedPeer("board").await.unwrap();
        let calls = backend.calls.lock().unwrap();
        assert_eq!(&calls[2..], &["remove:board"]);
        assert!(!calls.iter().any(|call| call.contains("secret-token")));
    }

    #[tokio::test]
    async fn connectivity_uses_the_same_service_without_treating_pairing_as_online() {
        let backend = Arc::new(RecordingPeerService::default());
        let services = NodeServices::new(backend.clone());
        let mut changed = services.clone().peers().subscribePeerChanges();
        assert_eq!(
            services.peers().outboundPeerNodeIds().unwrap(),
            ["board".into()].into()
        );
        assert!(services.peers().activePeerNodeIds().unwrap().is_empty());
        backend.changes.send(()).unwrap();
        changed.recv().await.unwrap();
    }

    #[tokio::test]
    async fn clones_share_state_and_propagate_rejections_without_claiming_success() {
        let backend = Arc::new(RecordingPeerService::default());
        let services = NodeServices::new(backend.clone());
        let cloned = services.clone();
        let error = cloned.peers()
            .finishPairing("transaction", "bad-code")
            .await
            .unwrap_err();
        assert_eq!(error.code, "REJECTED");
        services.peers().cancelPairing("transaction").await.unwrap();
        cloned.peers().removePairedPeer("board").await.unwrap();
        services.peers().stop().await.unwrap();
        assert_eq!(
            *backend.calls.lock().unwrap(),
            [
                "finish:transaction:bad-code",
                "cancel:transaction",
                "remove:board",
                "stop"
            ]
        );
        assert!(Arc::ptr_eq(&services.peers(), &cloned.peers()));
    }

    #[tokio::test]
    async fn routing_uses_same_service_and_preserves_next_hop_and_final_target() {
        let backend = Arc::new(RecordingPeerService::default());
        let services = NodeServices::new(backend.clone());
        services
            .peers()
            .call(
                "relay",
                RoutedCoreRequest {
                    spaceId: "space".into(),
                    originNodeId: "origin".into(),
                    targetNodeId: "destination".into(),
                    ttl: 3,
                    routeKind: RoutedCoreRequestKind::Target,
                    payload: CoreCallRequest::new("request", "object", "method", CoreValue::Null),
                },
            )
            .await;
        assert_eq!(
            *backend.calls.lock().unwrap(),
            ["call:relay:origin:destination:3"]
        );
    }
}
