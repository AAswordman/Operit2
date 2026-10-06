//! 已鉴权 Link 会话：路由元数据留在标准 args 中，执行仍使用既有 CoreLinkSession。
use super::*;

pub(super) fn routedCall(request: RoutedCoreRequest<CoreCallRequest>) -> Result<CoreCallRequest, CoreLinkError> {
    let mut wire = request.payload.clone();
    wire.args = toCoreValue(request).map_err(|e| error(e.to_string()))?; Ok(wire)
}
pub(super) fn routedWatch(request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreWatchRequest, CoreLinkError> {
    let mut wire = request.payload.clone(); wire.args = toCoreValue(request).map_err(|e| error(e.to_string()))?; Ok(wire)
}
fn routedPush(request: RoutedCoreRequest<CorePushRequest>) -> Result<CorePushRequest, CoreLinkError> {
    let mut wire = request.payload.clone(); wire.args = toCoreValue(request).map_err(|e| error(e.to_string()))?; Ok(wire)
}
struct RoutedClient { router: Weak<dyn PeerRouter>, peer: String }
impl RoutedClient {
    fn router(&self) -> Result<Arc<dyn PeerRouter>, CoreLinkError> {
        self.router.upgrade().ok_or_else(|| CoreLinkError::new("PEER_ROUTER_CLOSED", "Peer router was released"))
    }
}
#[async_trait(?Send)]
impl CoreLinkClient for RoutedClient {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        let id = request.requestId.clone();
        let routed = fromCoreValue::<RoutedCoreRequest<CoreCallRequest>>(request.args).map_err(|e| error(e.to_string()));
        match routed {
            Ok(r) if r.payload.requestId == id => match self.router() {
                Ok(router) => router.routedCall(self.peer.clone(), r).await,
                Err(e) => CoreCallResponse::err(id, e),
            },
            Ok(_) => CoreCallResponse::err(id, error("Routed correlation mismatch")),
            Err(e) => CoreCallResponse::err(id, e),
        }
    }
    async fn watchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        let r: RoutedCoreRequest<CoreWatchRequest> = fromCoreValue(request.args).map_err(|e| error(e.to_string()))?;
        if r.payload.requestId != request.requestId { return Err(error("Routed correlation mismatch")); }
        self.router()?.routedWatchSnapshot(self.peer.clone(), r).await
    }
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let r: RoutedCoreRequest<CoreWatchRequest> = fromCoreValue(request.args).map_err(|e| error(e.to_string()))?;
        if r.payload.requestId != request.requestId { return Err(error("Routed correlation mismatch")); }
        self.router()?.routedWatch(self.peer.clone(), r).await
    }
    async fn openPush(&mut self, request: CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let r: RoutedCoreRequest<CorePushRequest> = fromCoreValue(request.args).map_err(|e| error(e.to_string()))?;
        if r.payload.requestId != request.requestId { return Err(error("Routed correlation mismatch")); }
        self.router()?.routedOpenPush(self.peer.clone(), r).await
    }
}
pub(super) enum Authorization { Pairing, SpaceChannel, SessionReturn }
pub(super) async fn serve(service: HostRuntimePeerService, channel: Arc<Channel>, peer: String, sessionId: String, spaceChannel: bool) -> Result<(), CoreLinkError> {
    serveAuthorized(service, channel, peer, sessionId, if spaceChannel { Authorization::SpaceChannel } else { Authorization::Pairing }).await
}
pub(super) async fn serveAuthorized(service: HostRuntimePeerService, channel: Arc<Channel>, peer: String, sessionId: String, authorization: Authorization) -> Result<(), CoreLinkError> {
    serveChannel(service, channel, None, peer, sessionId, authorization).await
}
pub(super) async fn serveLive(service: HostRuntimePeerService, live: Arc<LiveChannel>, channel: Arc<Channel>, peer: String, sessionId: String, spaceChannel: bool) -> Result<(), CoreLinkError> {
    serveChannel(service, channel, Some(live), peer, sessionId, if spaceChannel { Authorization::SpaceChannel } else { Authorization::Pairing }).await
}
async fn serveChannel(service: HostRuntimePeerService, channel: Arc<Channel>, live: Option<Arc<LiveChannel>>, peer: String, sessionId: String, authorization: Authorization) -> Result<(), CoreLinkError> {
    let receive = || async {
        match &live { Some(live) => live.receive().await, None => channel.receive().await }
    };
    let send = |message| async {
        match &live { Some(live) => live.send(message).await, None => channel.send(message).await }
    };
    let mut session = CoreLinkSession::new(RoutedClient { router: service.state.router.clone(), peer: peer.clone() }, 32);
    loop {
        enum Incoming { Message(Option<PeerMessage>), Event(Option<CoreLinkResponse>) }
        let incoming = if session.hasWatches() {
            tokio::select! {
                message = receive() => Incoming::Message(message?),
                event = session.nextWatchEvent() => Incoming::Event(event),
            }
        } else { Incoming::Message(receive().await?) };
        // 每次业务入口重新确认入站授权。撤销不能被存活中的旧连接绕过。
        let valid = match authorization {
            Authorization::SpaceChannel => service.spaceInbound(&sessionId, &peer).is_ok(),
            Authorization::Pairing => service.inboundCredentials()?.get(&sessionId)
                .is_some_and(|c| c.deviceId == peer && c.pairingServiceVersion == PAIRING_SERVICE_VERSION),
            Authorization::SessionReturn => service.outbound(&peer).is_ok_and(|c| c.sessionId == sessionId)
                && service.sessionScopeAllowed(&peer, &channel)?,
        };
        if !valid { return Err(error("Inbound authorization revoked")); }
        match incoming {
            Incoming::Message(Some(PeerMessage::Request(request))) => {
                let response = match request {
                    CoreLinkRequest::Call(r) if r.target == "$peer.keepalive" && r.methodName == "ping" && r.args == CoreValue::Null => {
                        CoreLinkResponse::Call(CoreCallResponse { requestId: r.requestId, result: Ok(CoreValue::Null) })
                    },
                    CoreLinkRequest::Call(request) if request.target == space_channel::TARGET => {
                        let result = if !matches!(authorization, Authorization::Pairing) { Err(error("Return channels cannot issue pairing-scoped offers")) }
                        else { service.acceptSessionManagement(&peer, &sessionId, &channel, &request) };
                        CoreLinkResponse::Call(CoreCallResponse { requestId: request.requestId, result })
                    }
                    request => session.dispatch(request).await,
                };
                send(PeerMessage::Response(response)).await?;
            },
            Incoming::Event(Some(event)) => send(PeerMessage::Response(event)).await?,
            Incoming::Message(None) => break,
            _ => return Err(error("Unexpected inbound Link response")),
        }
    }
    Ok(())
}
pub(super) async fn watchSnapshot(service: &HostRuntimePeerService, node: &str, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEvent, CoreLinkError> {
    let channel = service.acquirePooledChannel(node).await?;
    let id = request.payload.requestId.clone();
    let result = channel.exchange(CoreLinkRequest::Watch(CoreLinkWatchRequest::Snapshot(routedWatch(request)?))).await?;
    match result {
        CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Snapshot(event)) } if requestId == id => Ok(event),
        CoreLinkResponse::Watch { result: Err(error), .. } => Err(error),
        _ => Err(error("Watch snapshot response mismatch")),
    }
}
pub(super) async fn watch(service: &HostRuntimePeerService, node: &str, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream, CoreLinkError> {
    let channel = service.acquirePooledChannel(node).await?;
    if let Some(shared) = channel.shared() {
        let duplex = shared.duplex().ok_or_else(|| error("Shared session has no dispatcher"))?;
        return duplex.watch(service, shared.clone(), request).await;
    }
    let pooled = channel.pooled().ok_or_else(|| error("Pooled session is missing"))?;
    let (watchLease, stream) = pooled.openWatch(routedWatch(request)?).await?;
    drop(channel);
    Ok(stream.withOnClose(move || drop(watchLease)))
}
struct Push {
    lease: ChannelLease,
    id: String,
    next: u64,
}

#[async_trait]
impl CoreLinkPushSession for Push {
    /// Sends one ordered Push item through the shared multiplexed channel.
    async fn send(&mut self, args: CoreValue) -> Result<(), CoreLinkError> {
        let sequence = self.next;
        self.next = sequence.checked_add(1).ok_or_else(|| error("Push sequence exhausted"))?;
        match self.lease.exchange(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(CorePushItem {
            pushId: self.id.clone(), sequence, args,
        }))).await? {
            CoreLinkResponse::Push { pushId, result: Ok(CoreLinkPushResponse::ItemAccepted { sequence: accepted }) }
                if pushId == self.id && accepted == sequence => Ok(()),
            CoreLinkResponse::Push { result: Err(error), .. } => Err(error),
            _ => Err(error("Push sequence response mismatch")),
        }
    }

    /// Closes one logical Push session while retaining the shared channel.
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
        let result = self.lease.exchange(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close {
            pushId: self.id.clone(),
        })).await;
        match result? {
            CoreLinkResponse::Push { pushId, result: Ok(CoreLinkPushResponse::Closed) } if pushId == self.id => Ok(()),
            CoreLinkResponse::Push { result: Err(error), .. } => Err(error),
            _ => Err(error("Push close response mismatch")),
        }
    }
}
pub(super) async fn openPush(service: &HostRuntimePeerService, node: &str, request: RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
    let lease = service.acquirePooledChannel(node).await?;
    let id = request.payload.requestId.0.clone();
    match lease.exchange(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Open(routedPush(request)?))).await? {
        CoreLinkResponse::Push { pushId, result: Ok(CoreLinkPushResponse::Opened) } if pushId == id =>
            Ok(Box::new(Push { lease, id, next: 0 })),
        CoreLinkResponse::Push { result: Err(error), .. } => Err(error),
        _ => Err(error("Push open response mismatch")),
    }
}
