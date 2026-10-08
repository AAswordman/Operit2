//! Peer synchronization adapter to the existing generated application/service dispatch.
use super::*;

impl CoreNodeRouter {
    /// Check the replication endpoints, not a relay hop. Authorization comes
    /// from the accepted Space policy, never platform names or remote claims.
    pub(crate) fn requirePeerStorageProviders(
        &self, originNodeId: &str, targetNodeId: &str,
    ) -> Result<(), CoreLinkError> {
        requireStorageProviders(&self.spaceStore, &self.networkControlStore, originNodeId, targetNodeId)
    }

    pub(super) fn guardPeerSyncPush(&self, origin: String, target: String, spaceId: String,
        inner: Box<dyn CoreLinkPushSession>) -> Box<dyn CoreLinkPushSession> {
        Box::new(AuthorizedSyncPush { control: self.networkControlStore.clone(), space: self.spaceStore.clone(),
            origin, target, spaceId, inner })
    }

    pub(super) fn validatePeerSyncRoute<T>(
        &self,
        previousNodeId: &str,
        request: &RoutedCoreRequest<T>,
    ) -> Result<bool, CoreLinkError> {
        // Synchronization is a same-Space operation, unlike pre-admission
        // join calls. A member may arrive over its admitted return channel.
        let peers = self.nodeServices().map_err(CoreLinkError::internal)?.peers().pairedPeers()?;
        if !peers.iter().any(|p| p.nodeId == previousNodeId && (p.inbound || p.outbound))
            || self.spaceChannelScope(previousNodeId)?.is_none() {
            return Err(CoreLinkError::new("PEER_NOT_AUTHORIZED", "Synchronization requires an authenticated admitted Space member"));
        }
        if request.routeKind != RoutedCoreRequestKind::Target {
            return Err(CoreLinkError::new(
                "PEER_SYNC_INVALID_ROUTE",
                "Synchronization requires a target route",
            ));
        }
        // Includes Space identity, membership and revocation checks for every hop.
        self.validateIncomingRoute(previousNodeId, request)
    }

    pub(super) async fn dispatchPeerSyncCall(
        &self,
        method: PeerSyncMethod,
        mut request: CoreCallRequest,
    ) -> CoreCallResponse {
        if method == PeerSyncMethod::DeviceSpace {
            let service =
                RuntimeRemoteLinkService::newWithRouter((*self.localCore).clone(), self.clone());
            let result = service
                .deviceSpace()
                .map_err(CoreLinkError::internal)
                .and_then(|space| {
                    operit_link::toCoreValue(space)
                        .map_err(|error| CoreLinkError::internal(error.to_string()))
                });
            return CoreCallResponse {
                requestId: request.requestId,
                result,
            };
        }
        let Some(target) = self.targetForSchema("application") else {
            return CoreCallResponse::err(
                request.requestId,
                CoreLinkError::internal("Application schema missing"),
            );
        };
        request.target = target.into();
        self.executeLocalCall(request).await
    }

    pub(super) fn dispatchPeerSyncPush(
        &self,
        mut request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let target = self
            .targetForSchema("services.syncBlobTransferManager")
            .ok_or_else(|| CoreLinkError::internal("Sync blob schema missing"))?;
        request.target = target.into();
        self.localCore.openPush(request)
    }
}


/// Opening a stream is not a permanent storage grant. Recheck every chunk and
/// the final commit so an already-open transfer cannot outlive revocation.
struct AuthorizedSyncPush {
    control: NetworkControlStore,
    space: CoreSpaceStore,
    origin: String,
    target: String,
    spaceId: String,
    inner: Box<dyn CoreLinkPushSession>,
}
impl AuthorizedSyncPush {
    fn check(&self) -> Result<(), CoreLinkError> {
        if self.space.space().map_err(CoreLinkError::internal)?.spaceId != self.spaceId {
            return Err(CoreLinkError::new("SPACE_STORAGE_PERMISSION_DENIED", "Storage stream belongs to a previous Space"));
        }
        requireStorageProviders(&self.space, &self.control, &self.origin, &self.target)
    }
}
#[async_trait]
impl CoreLinkPushSession for AuthorizedSyncPush {
    async fn send(&mut self, value: CoreValue) -> Result<(), CoreLinkError> {
        self.check()?;
        self.inner.send(value).await
    }
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
        self.check()?;
        self.inner.close().await
    }
}

/// The same service-availability and policy check guards admission, each chunk
/// and final commit. Unknown profiles do not advertise a storage engine.
fn requireStorageProviders(space: &CoreSpaceStore, control: &NetworkControlStore,
    origin: &str, target: &str) -> Result<(), CoreLinkError> {
    let profiles = space.deviceProfiles().map_err(CoreLinkError::internal)?;
    for node in [origin, target] {
        if profiles.get(node).and_then(|profile| profile.coreVersion.as_ref()).is_none()
            || control.nodeIsDisconnected(node).map_err(CoreLinkError::internal)?
            || !control.nodeHasCapability(node, "storage.provide", None).map_err(CoreLinkError::internal)? {
            return Err(CoreLinkError::new("SPACE_STORAGE_PERMISSION_DENIED",
                format!("Node {node} cannot provide authorized Space storage")));
        }
    }
    Ok(())
}
