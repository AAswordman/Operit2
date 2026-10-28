//! 已鉴权 Link 会话：路由元数据留在标准 args 中，执行仍使用既有 CoreLinkSession。
use super::*;
use futures_util::{stream::{self, FuturesUnordered}, FutureExt, Stream, StreamExt};

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
pub(super) enum Authorization { Inbound, Outbound }

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
    /// Creates one dispatcher bound to the authenticated pairing direction.
    pub(super) fn new(service: HostRuntimePeerService, peer: String, sessionId: String, authorization: Authorization) -> Self {
        let session = CoreLinkSession::new(RoutedClient { router: service.state.router.clone(), peer: peer.clone() }, 32);
        Self { service, peer, sessionId, authorization, session }
    }
    /// Re-checks the channel's authorization before one business entry. Every
    /// entry re-confirms it: a revocation must not be bypassed by a connection
    /// that is already open.
    pub(super) fn validate(&self) -> Result<(), CoreLinkError> {
        self.service.requirePeerConnectionAllowed(&self.peer)?;
        let valid = match self.authorization {
            Authorization::Inbound => self.service.inboundCredentials()?.get(&self.sessionId)
                .is_some_and(|credential| credential.deviceId == self.peer && credential.pairingServiceVersion == PAIRING_SERVICE_VERSION),
            Authorization::Outbound => self.service.outbound(&self.peer).is_ok_and(|credential| credential.sessionId == self.sessionId),
        };
        if !valid { return Err(error("Inbound authorization revoked")); }
        Ok(())
    }


}

/// Drives the authenticated dispatcher directly in transport-independent tests.
#[cfg(test)]
pub(super) async fn serve(service: HostRuntimePeerService, channel: Arc<Channel>, peer: String, sessionId: String) -> Result<(), CoreLinkError> {
    let incoming = stream::unfold(channel.clone(), |channel| async move {
        channel.receive().await.transpose().map(|message| (message, channel))
    });
    let side = InboundSide::new(service, peer, sessionId, Authorization::Inbound);
    serveIncoming(side, incoming, |messages| channel.sendBatch(messages)).await
}

/// Calls have independent correlations and may forward through this same carrier.
/// Serializing them creates wait cycles while routes change. Stream state remains
/// owned by one session so Watch/Push operations retain their connection order.
pub(super) async fn serveIncoming<I, S, F>(
    side: InboundSide, incoming: I, send: S,
) -> Result<(), CoreLinkError>
where
    I: Stream<Item = Result<PeerMessage, CoreLinkError>>,
    S: Fn(Vec<PeerMessage>) -> F,
    F: std::future::Future<Output = Result<(), CoreLinkError>>,
{
    let scheduler = side.service.state.host.hostRuntimeTaskSchedulerHost.clone()
        .ok_or_else(|| error("Host scheduler is not installed"))?;
    // Poll the ordered session independently: opening a Watch or a backpressured
    // Push may itself await a Call carried by this same connection.
    let session = InboundSide::new(side.service.clone(), side.peer.clone(), side.sessionId.clone(), side.authorization);
    let (requests, queue) = tokio::sync::mpsc::unbounded_channel();
    let sessions = stream::unfold((session, queue), |(mut session, mut queue)| async move {
        enum Item { Request(Option<CoreLinkRequest>), Event(Option<Vec<CoreLinkResponse>>) }
        let item = if session.session.hasWatches() {
            tokio::select! {
                request = queue.recv() => Item::Request(request),
                event = session.session.nextWatchBatch() => Item::Event(event),
            }
        } else { Item::Request(queue.recv().await) };
        let response = match session.validate() {
            Err(error) => Err(error),
            Ok(()) => match item {
                Item::Request(Some(request)) => {
                    let mut responses = vec![session.session.dispatch(request).await];
                    if let Some(Some(events)) = session.session.nextWatchBatch().now_or_never() {
                        responses.extend(events);
                    }
                    Ok(responses)
                },
                Item::Event(Some(event)) => Ok(event),
                _ => return None,
            },
        };
        Some((response, (session, queue)))
    });
    futures_util::pin_mut!(incoming, sessions);
    let mut calls = FuturesUnordered::<tokio::sync::oneshot::Receiver<CoreLinkResponse>>::new();
    loop {
        enum Incoming {
            Message(Option<Result<PeerMessage, CoreLinkError>>),
            Response(Result<CoreLinkResponse, tokio::sync::oneshot::error::RecvError>),
            Session(Option<Result<Vec<CoreLinkResponse>, CoreLinkError>>),
        }
        let item = tokio::select! {
            message = incoming.next() => Incoming::Message(message),
            Some(response) = calls.next(), if !calls.is_empty() => Incoming::Response(response),
            response = sessions.next() => Incoming::Session(response),
        };
        side.validate()?;
        let responses = match item {
            Incoming::Message(Some(Ok(PeerMessage::Request(CoreLinkRequest::Call(request))))) => {
                let service = side.service.clone();
                let peer = side.peer.clone();
                let sessionId = side.sessionId.clone();
                let authorization = side.authorization;
                let (mut completed, result) = tokio::sync::oneshot::channel();
                // Synchronous business I/O must not occupy the task that drains Watch batches.
                scheduler.scheduleHostRuntimeBlockingAsyncTask("peer-inbound-call", Box::new(move || Box::pin(async move {
                    let mut call = InboundSide::new(service, peer, sessionId, authorization);
                    tokio::select! {
                        biased;
                        _ = completed.closed() => {},
                        response = async {
                            if let Err(error) = call.validate() { return CoreLinkResponse::Call(CoreCallResponse::err(request.requestId, error)); }
                            call.session.dispatch(CoreLinkRequest::Call(request)).await
                        } => { let _ = completed.send(response); },
                    }
                }))).map_err(|failure| error(failure.to_string()))?;
                calls.push(result);
                continue;
            }
            Incoming::Message(Some(Ok(PeerMessage::Request(request)))) => {
                requests.send(request).map_err(|_| error("Inbound stream dispatcher stopped"))?;
                continue;
            }
            Incoming::Message(Some(Err(error))) | Incoming::Session(Some(Err(error))) => return Err(error),
            Incoming::Message(None) | Incoming::Session(None) => break,
            Incoming::Response(response) => vec![response.map_err(|_| error("Inbound Call task stopped"))?],
            Incoming::Session(Some(Ok(responses))) => responses,
            _ => return Err(error("Unexpected inbound Link response")),
        };
        send(responses.into_iter().map(PeerMessage::Response).collect()).await?;
    }
    Ok(())
}

pub(super) async fn watchSnapshot(service: &HostRuntimePeerService, node: &str, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEvent, CoreLinkError> {
    let channel = service.acquireChannel(node).await?;
    let id = request.payload.requestId.clone();
    let result = channel.exchange(CoreLinkRequest::Watch(CoreLinkWatchRequest::Snapshot(routedWatch(request)?))).await?;
    match result {
        CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Snapshot(event)) } if requestId == id => Ok(event),
        CoreLinkResponse::Watch { result: Err(error), .. } => Err(error),
        _ => Err(error("Watch snapshot response mismatch")),
    }
}
pub(super) async fn watch(service: &HostRuntimePeerService, node: &str, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream, CoreLinkError> {
    let channel = service.acquireChannel(node).await?;
    channel.openWatch(routedWatch(request)?).await
}
struct Push {
    channel: Option<Arc<MultiplexedChannel>>,
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
        match self.channel.as_ref().ok_or_else(|| error("Push session is closed"))?.exchange(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(CorePushItem {
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
        let channel = self.channel.take().ok_or_else(|| error("Push session is closed"))?;
        let result = channel.exchange(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close {
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
    /// Releases only this logical Push while retaining the peer carrier.
    fn drop(&mut self) {
        if let Some(channel) = self.channel.take() {
            let pushId = self.id.clone();
            let _ = self.scheduler.scheduleHostRuntimeAsyncTask("peer-push-close", Box::new(move || Box::pin(async move {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(10), channel.exchange(
                    CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close { pushId }))).await;
            })));
        }
    }
}
pub(super) async fn openPush(service: &HostRuntimePeerService, node: &str, request: RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
    let scheduler = service.state.host.hostRuntimeTaskSchedulerHost.clone().ok_or_else(|| error("Host scheduler missing"))?;
    let channel = service.acquireChannel(node).await?;
    let id = request.payload.requestId.0.clone();
    match channel.exchange(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Open(routedPush(request)?))).await? {
        CoreLinkResponse::Push { pushId, result: Ok(CoreLinkPushResponse::Opened) } if pushId == id =>
            Ok(Box::new(Push { channel: Some(channel), scheduler, id, next: 0 })),
        CoreLinkResponse::Push { result: Err(error), .. } => Err(error),
        _ => Err(error("Push open response mismatch")),
    }
}
