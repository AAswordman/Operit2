use crate::runtime::{
    NodeServices::NodeServices,
    NodeSpaceService::NodeSpaceService,
    PeerStateStore::{PeerHostConfig, PeerStateStore},
};
use crate::space::{createPeerService, EdgePeerRouter, PeerRuntimeLimits};
use crate::storage::{
    CoreNodeIdentityStore::CoreNodeIdentityStore, CoreSpaceStore::CoreSpaceDeviceProfile,
};
use crate::{device::EdgeNode, host::HostManager::HostManager, link::protocol::LinkDeviceInfo};
use std::sync::Arc;

/// Owns one device, one pairing engine and one node-local Space projection.
/// Keep this value alive for as long as the product accepts remote calls.
pub struct SpaceDevice {
    pub node: Arc<EdgeNode>,
    pub services: NodeServices,
    pub space: Arc<NodeSpaceService>,
    pub node_id: String,
    _router: Arc<EdgePeerRouter>,
}
impl SpaceDevice {
    /// Builds the shared protocol around a manufacturer's custom service surface.
    /// The Host supplies persistent storage, sockets, entropy and affine tasks.
    pub fn assemble(
        host: HostManager,
        info: LinkDeviceInfo,
        node: EdgeNode,
        limits: PeerRuntimeLimits,
    ) -> Result<Self, String> {
        let storage = host
            .runtimeStorageHost
            .clone()
            .ok_or("Install a RuntimeStorageHost")?;
        let node_id = CoreNodeIdentityStore::new(storage.clone())
            .initialize()?
            .nodeId;
        let router = EdgePeerRouter::new(node_id.clone());
        let peers = createPeerService(Arc::new(host), &router, info.clone(), limits)?;
        let services = NodeServices::new(peers);
        let space = NodeSpaceService::new(storage, services.peers())?;
        space.initialize(CoreSpaceDeviceProfile {
            nodeId: node_id.clone(),
            displayName: info.model.clone(),
            userName: String::new(),
            platform: info.platform,
            model: info.model,
            coreVersion: None,
            updatedAt: crate::host::TimeUtils::currentTimeMillis(),
            pairingIncarnation: None,
        })?;
        router.installSpace(space.clone())?;
        let node = Arc::new(node.withNodeServices(services.clone()));
        router.install(node.clone())?;
        Ok(Self {
            node,
            services,
            space,
            node_id,
            _router: router,
        })
    }

    /// Saves the actual product listener configuration before opening resources.
    pub async fn listen(&self, config: PeerHostConfig) -> Result<(), String> {
        use crate::space::NodeSpaceContext;
        PeerStateStore::new(self.space.storage()).saveHostConfig(&config)?;
        self.services
            .peers()
            .startListening(&config.transports)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn stop(&self) -> Result<(), String> {
        self.services
            .peers()
            .stop()
            .await
            .map_err(|e| e.to_string())
    }
}
