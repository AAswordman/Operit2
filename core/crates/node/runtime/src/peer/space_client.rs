//! Space clients bound to the peer's only authenticated channel and current admission.
use super::*;

/// Binds a Space entry to the current carrier generation and replayed membership.
fn entryIdentity(service: &HostRuntimePeerService, peer: &str) -> Option<crate::NodeServices::SpaceClientIdentity> {
    service.requirePeerConnectionAllowed(peer).ok()?;
    let channel = service.peerChannel(peer)?;
    let spaceId = service.router().ok()?.spaceChannelScope(peer).ok()??;
    let generation = channel.generation().to_string();
    Some(crate::NodeServices::SpaceClientIdentity { spaceId, peerNodeId: peer.into(), generation })
}
#[derive(Clone)]
struct SessionSpaceClient {
    service: HostRuntimePeerService,
    identity: crate::NodeServices::SpaceClientIdentity,
}
impl SessionSpaceClient {
    /// Validates the original Space entry before routing an operation.
    fn route<T>(&self, payload: T, routeKind: RoutedCoreRequestKind) -> Result<RoutedCoreRequest<T>, CoreLinkError> {
        if entryIdentity(&self.service, &self.identity.peerNodeId).as_ref() != Some(&self.identity) {
            return Err(CoreLinkError::new("SPACE_ENTRY_CHANGED", "Space entry was replaced or revoked"));
        }
        Ok(RoutedCoreRequest { spaceId: self.identity.spaceId.clone(), originNodeId: self.service.state.nodeId.clone(),
            targetNodeId: self.identity.peerNodeId.clone(), ttl: 4, routeKind, payload })
    }
}
/// Classifies annotated calls by their Binding contract.
fn callRouteKind(request: &CoreCallRequest) -> RoutedCoreRequestKind {
    // Includes explicit create_binding commands; only Core allocates the Binding.
    if crate::generated_space_call_route(request).is_some() { RoutedCoreRequestKind::SpaceBinding }
    else { RoutedCoreRequestKind::SpaceRoute }
}
/// Classifies watch snapshots and live embedded streams.
fn watchRouteKind(request: &CoreWatchRequest, snapshot: bool) -> RoutedCoreRequestKind {
    // Embedded streams are reopened via their source, not treated as ordinary
    // annotated properties. Snapshot has no source-reopen semantics in Core.
    if crate::generated_space_watch_route(request).is_some()
        || (!snapshot && request.target == operit_link::CORE_STREAM_TARGET) {
        RoutedCoreRequestKind::SpaceBinding
    } else { RoutedCoreRequestKind::SpaceRoute }
}
/// Selects Binding routing for annotated inputs and the target namespace for node services.
fn pushRouteKind(request: &CorePushRequest) -> RoutedCoreRequestKind {
    if crate::generated_space_push_route(request).is_some() {
        RoutedCoreRequestKind::SpaceBinding
    } else {
        RoutedCoreRequestKind::Target
    }
}
#[async_trait(?Send)]
impl CoreLinkSharedClient for SessionSpaceClient {
    /// Reports the capacity of the actual shared carrier.
    fn nominalBytesPerSecond(&self) -> Option<u32> {
        let transport = self.service.peerChannel(&self.identity.peerNodeId)?.transport();
        if transport == PeerTransport::Serial { Some(115200 / 10) }
        else { None }
    }

    /// Routes one call through the bound authenticated channel.
    async fn call(&self, mut r: CoreCallRequest) -> CoreCallResponse {
        r.requestId = CoreRequestId::new(nextCoreRouteRequestId("edge-ui"));
        let id = r.requestId.clone();
        let kind = callRouteKind(&r);
        match self.route(r, kind) { Ok(route) => self.service.call(&self.identity.peerNodeId, route).await, Err(e) => CoreCallResponse::err(id, e) }
    }
    /// Reads one snapshot through the bound authenticated channel.
    async fn watchSnapshot(&self, mut r: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        r.requestId = CoreRequestId::new(nextCoreRouteRequestId("edge-snapshot"));
        let kind = watchRouteKind(&r, true);
        self.service.watchSnapshot(&self.identity.peerNodeId, self.route(r, kind)?).await
    }
    /// Opens one watch through the bound authenticated channel.
    async fn watch(&self, mut r: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        r.requestId = CoreRequestId::new(nextCoreRouteRequestId("edge-watch"));
        let kind = watchRouteKind(&r, false);
        self.service.watch(&self.identity.peerNodeId, self.route(r, kind)?).await
    }

    /// Opens an authenticated input stream bound to the current Space admission and session.
    async fn openPush(
        &self,
        mut request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn operit_link::CoreLinkPushSession>, operit_link::CoreLinkError> {
        request.requestId = CoreRequestId::new(nextCoreRouteRequestId("edge-push"));
        let kind = pushRouteKind(&request);
        let route = self.route(request, kind)?;
        let session = self.service.openPush(&self.identity.peerNodeId, route).await?;
        Ok(Box::new(AdmittedPush { client: self.clone(), session }))
    }
}
struct AdmittedPush { client: SessionSpaceClient, session: Box<dyn CoreLinkPushSession> }
#[async_trait]
impl CoreLinkPushSession for AdmittedPush {
    /// Checks admission before sending the next ordered input item.
    async fn send(&mut self, value: CoreValue) -> Result<(), CoreLinkError> {
        self.client.route((), RoutedCoreRequestKind::SpaceRoute)?;
        self.session.send(value).await
    }
    /// Closes the admitted input stream.
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> { self.session.close().await }
}
/// Selects one currently admitted peer entry.
pub(super) fn spaceConnection(service: &HostRuntimePeerService) -> Option<crate::NodeServices::SpaceClientConnection> {
    let candidates = service.activePeerNodeIds().ok()?;
    let identity = candidates.iter().find_map(|peer| entryIdentity(service, peer))?;
    let client = Arc::new(SessionSpaceClient { service: service.clone(), identity: identity.clone() });
    Some(crate::NodeServices::SpaceClientConnection { identity, client })
}
/// Exposes the selected Space entry as a shared client.
pub(super) fn spaceClient(service: &HostRuntimePeerService) -> Option<Arc<dyn CoreLinkSharedClient + Send + Sync>> {
    spaceConnection(service).map(|connection| connection.client)
}
