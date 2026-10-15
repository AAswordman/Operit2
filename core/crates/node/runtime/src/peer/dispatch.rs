//! 已鉴权 Link 会话：路由元数据留在标准 args 中，执行仍使用既有 CoreLinkSession。
use super::*;
use futures_util::{future::LocalBoxFuture, stream::{self, FuturesUnordered}, Stream, StreamExt};

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
#[derive(Clone, Copy)]
pub(super) enum Authorization { Pairing, SpaceChannel, SessionReturn, PairingDuplex, SpaceDuplex }

/// Answers the inbound direction of one authenticated channel: requests are
/// executed here and watch events are flushed by whoever drives the frames.
/// Vendored to both roles so the dialing side can be called back over the
/// connection it opened, instead of dialing an address that may not be
/// routable from here.
pub(super) struct InboundSide {
    service: HostRuntimePeerService, peer: String, sessionId: String, authorization: Authorization,
    session: CoreLinkSession<RoutedClient>,
}
impl InboundSide {
    pub(super) fn new(service: HostRuntimePeerService, peer: String, sessionId: String, authorization: Authorization) -> Self {
        let session = CoreLinkSession::new(RoutedClient { router: service.state.router.clone(), peer: peer.clone() }, 32);
        Self { service, peer, sessionId, authorization, session }
    }
    pub(super) fn hasWatches(&self) -> bool { self.session.hasWatches() }
    pub(super) async fn nextWatchEvent(&mut self) -> Option<CoreLinkResponse> { self.session.nextWatchEvent().await }

    /// Re-checks the channel's authorization before one business entry. Every
    /// entry re-confirms it: a revocation must not be bypassed by a connection
    /// that is already open.
    pub(super) fn validate(&self, channel: &Arc<Channel>) -> Result<(), CoreLinkError> {
        let valid = match self.authorization {
            Authorization::SpaceChannel => self.service.spaceInbound(&self.sessionId, &self.peer).is_ok(),
            Authorization::Pairing => self.service.inboundCredentials()?.get(&self.sessionId)
                .is_some_and(|c| c.deviceId == self.peer && c.pairingServiceVersion == PAIRING_SERVICE_VERSION),
            Authorization::SessionReturn => self.service.outbound(&self.peer).is_ok_and(|c| c.sessionId == self.sessionId)
                && self.service.sessionScopeAllowed(&self.peer, channel)?,
            // Admission may reach the gateway before the applicant's refresh finishes.
            // Keep the authenticated pairing alive; the router rejects premature
            // same-Space calls without tearing down the in-flight refresh.
            Authorization::PairingDuplex => self.service.outbound(&self.peer).is_ok_and(|c| c.sessionId == self.sessionId),
            // The dialing side answers a peer that granted it this channel: the
            // grant that let this device dial is what lets the peer call back,
            // so both directions end when that grant is revoked.
            Authorization::SpaceDuplex => self.service.spaceOutbound(&self.peer).is_ok(),
        };
        if !valid { return Err(error("Inbound authorization revoked")); }
        Ok(())
    }

    /// Answers one inbound request. The caller validated this channel first.
    pub(super) async fn answer(&mut self, channel: &Arc<Channel>, request: CoreLinkRequest) -> CoreLinkResponse {
        match request {
            CoreLinkRequest::Call(r) if r.target == "$peer.keepalive" && r.methodName == "ping" && r.args == CoreValue::Null => {
                CoreLinkResponse::Call(CoreCallResponse { requestId: r.requestId, result: Ok(CoreValue::Null) })
            },
            CoreLinkRequest::Call(request) if request.target == space_channel::TARGET => {
                let result = if !matches!(self.authorization, Authorization::Pairing) { Err(error("Return channels cannot issue pairing-scoped offers")) }
                else { self.service.acceptSessionManagement(&self.peer, &self.sessionId, channel, &request) };
                CoreLinkResponse::Call(CoreCallResponse { requestId: request.requestId, result })
            }
            request => self.session.dispatch(request).await,
        }
    }
}

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
    let incoming = stream::unfold((channel.clone(), live.clone()), |(channel, live)| async move {
        let message = match &live { Some(live) => live.receive().await, None => channel.receive().await };
        message.transpose().map(|message| (message, (channel, live)))
    });
    let side = InboundSide::new(service, peer, sessionId, authorization);
    serveIncoming(side, channel.clone(), incoming, |message| async {
        match &live { Some(live) => live.send(message).await, None => channel.send(message).await }
    }).await
}

/// Calls have independent correlations and may forward through this same carrier.
/// Serializing them creates wait cycles while routes change. Stream state remains
/// owned by one session so Watch/Push operations retain their connection order.
pub(super) async fn serveIncoming<I, S, F>(
    side: InboundSide, channel: Arc<Channel>, incoming: I, send: S,
) -> Result<(), CoreLinkError>
where
    I: Stream<Item = Result<PeerMessage, CoreLinkError>>,
    S: Fn(PeerMessage) -> F,
    F: std::future::Future<Output = Result<(), CoreLinkError>>,
{
    // Poll the ordered session independently: opening a Watch or a backpressured
    // Push may itself await a Call carried by this same connection.
    let session = InboundSide::new(side.service.clone(), side.peer.clone(), side.sessionId.clone(), side.authorization);
    let (requests, queue) = tokio::sync::mpsc::unbounded_channel();
    let sessions = stream::unfold((session, queue, channel.clone()), |(mut session, mut queue, channel)| async move {
        enum Item { Request(Option<CoreLinkRequest>), Event(Option<CoreLinkResponse>) }
        let item = if session.hasWatches() {
            tokio::select! {
                request = queue.recv() => Item::Request(request),
                event = session.nextWatchEvent() => Item::Event(event),
            }
        } else { Item::Request(queue.recv().await) };
        let response = match session.validate(&channel) {
            Err(error) => Err(error),
            Ok(()) => match item {
                Item::Request(Some(request)) => Ok(session.answer(&channel, request).await),
                Item::Event(Some(event)) => Ok(event),
                _ => return None,
            },
        };
        Some((response, (session, queue, channel)))
    });
    futures_util::pin_mut!(incoming, sessions);
    let mut calls = FuturesUnordered::<LocalBoxFuture<'static, CoreLinkResponse>>::new();
    loop {
        enum Incoming {
            Message(Option<Result<PeerMessage, CoreLinkError>>),
            Response(CoreLinkResponse),
            Session(Option<Result<CoreLinkResponse, CoreLinkError>>),
        }
        let item = tokio::select! {
            message = incoming.next() => Incoming::Message(message),
            Some(response) = calls.next(), if !calls.is_empty() => Incoming::Response(response),
            response = sessions.next() => Incoming::Session(response),
        };
        side.validate(&channel)?;
        let response = match item {
            Incoming::Message(Some(Ok(PeerMessage::Request(CoreLinkRequest::Call(request))))) => {
                let mut call = InboundSide::new(side.service.clone(), side.peer.clone(), side.sessionId.clone(), side.authorization);
                let channel = channel.clone();
                calls.push(Box::pin(async move {
                    if let Err(error) = call.validate(&channel) { return CoreLinkResponse::Call(CoreCallResponse::err(request.requestId, error)); }
                    call.answer(&channel, CoreLinkRequest::Call(request)).await
                }));
                continue;
            }
            Incoming::Message(Some(Ok(PeerMessage::Request(request)))) => {
                requests.send(request).map_err(|_| error("Inbound stream dispatcher stopped"))?;
                continue;
            }
            Incoming::Message(Some(Err(error))) | Incoming::Session(Some(Err(error))) => return Err(error),
            Incoming::Message(None) | Incoming::Session(None) => break,
            Incoming::Response(response) | Incoming::Session(Some(Ok(response))) => response,
            _ => return Err(error("Unexpected inbound Link response")),
        };
        send(PeerMessage::Response(response)).await?;
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
    lease: Option<ChannelLease>,
    scheduler: Arc<dyn operit_host_api::HostRuntimeTaskSchedulerHost>,
    id: String,
    next: u64,
}

#[async_trait]
impl CoreLinkPushSession for Push {
    /// Sends one ordered Push item through the shared multiplexed channel.
    async fn send(&mut self, args: CoreValue) -> Result<(), CoreLinkError> {
        let sequence = self.next;
        self.next = sequence.checked_add(1).ok_or_else(|| error("Push sequence exhausted"))?;
        match self.lease.as_ref().unwrap().exchange(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(CorePushItem {
            pushId: self.id.clone(), sequence, args,
        }))).await? {
            CoreLinkResponse::Push { pushId, result: Ok(CoreLinkPushResponse::ItemAccepted { sequence: accepted }) }
                if pushId == self.id && accepted == sequence => Ok(()),
            CoreLinkResponse::Push { result: Err(error), .. } => Err(error),
            _ => Err(error("Push sequence response mismatch")),
        }
    }

    /// Closes one logical Push session while retaining the shared channel.
    async fn close(mut self: Box<Self>) -> Result<(), CoreLinkError> {
        let lease = self.lease.take().unwrap();
        let result = lease.exchange(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close {
            pushId: self.id.clone(),
        })).await;
        match result? {
            CoreLinkResponse::Push { pushId, result: Ok(CoreLinkPushResponse::Closed) } if pushId == self.id => Ok(()),
            CoreLinkResponse::Push { result: Err(error), .. } => Err(error),
            _ => Err(error("Push close response mismatch")),
        }
    }
}
impl Drop for Push {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            let pushId = self.id.clone();
            let _ = self.scheduler.scheduleHostRuntimeAsyncTask("peer-push-close", Box::new(move || Box::pin(async move {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(10), lease.exchange(
                    CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close { pushId }))).await;
            })));
        }
    }
}
pub(super) async fn openPush(service: &HostRuntimePeerService, node: &str, request: RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
    let scheduler = service.state.host.hostRuntimeTaskSchedulerHost.clone().ok_or_else(|| error("Host scheduler missing"))?;
    let lease = service.acquirePooledChannel(node).await?;
    let id = request.payload.requestId.0.clone();
    match lease.exchange(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Open(routedPush(request)?))).await? {
        CoreLinkResponse::Push { pushId, result: Ok(CoreLinkPushResponse::Opened) } if pushId == id =>
            Ok(Box::new(Push { lease: Some(lease), scheduler, id, next: 0 })),
        CoreLinkResponse::Push { result: Err(error), .. } => Err(error),
        _ => Err(error("Push open response mismatch")),
    }
}
