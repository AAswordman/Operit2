//! 已鉴权 Link 会话：路由元数据留在标准 args 中，执行仍使用既有 CoreLinkSession。
use super::*;
use super::crypto::EncodedMessage;
use futures_util::{stream::{self, FuturesUnordered}, FutureExt, Stream, StreamExt};

/// Both native maps and MessagePack struct sequences carry payload args in this slot.
/// Move the owned subtree aside while Serde validates the remaining route metadata.
fn routedArgs(value: &mut CoreValue) -> Option<&mut CoreValue> {
    let payload = match value {
        CoreValue::Map(fields) => fields.get_mut("payload"),
        CoreValue::List(fields) if fields.len() == 6 => fields.get_mut(5),
        _ => None,
    }?;
    match payload {
        CoreValue::Map(fields) => fields.get_mut("args"),
        CoreValue::List(fields) if fields.len() == 4 => fields.get_mut(3),
        _ => None,
    }
}
fn encodeRouted<T: Clone + serde::Serialize>(
    mut request: RoutedCoreRequest<T>, args: fn(&mut T) -> &mut CoreValue,
) -> Result<T, CoreLinkError> {
    let payload = std::mem::replace(args(&mut request.payload), CoreValue::Null);
    let mut wire = request.payload.clone();
    let mut envelope = toCoreValue(request).map_err(|failure| error(failure.to_string()))?;
    *routedArgs(&mut envelope).ok_or_else(|| error("Missing routed args"))? = payload;
    *args(&mut wire) = envelope;
    Ok(wire)
}
fn decodeRouted<T: serde::de::DeserializeOwned>(
    mut envelope: CoreValue, args: fn(&mut T) -> &mut CoreValue,
) -> Result<RoutedCoreRequest<T>, CoreLinkError> {
    let payload = routedArgs(&mut envelope).map(|value| std::mem::replace(value, CoreValue::Null));
    let mut request: RoutedCoreRequest<T> = fromCoreValue(envelope).map_err(|failure| error(failure.to_string()))?;
    if let Some(payload) = payload { *args(&mut request.payload) = payload; }
    Ok(request)
}
pub(super) fn routedCall(request: RoutedCoreRequest<CoreCallRequest>) -> Result<CoreCallRequest, CoreLinkError> {
    encodeRouted(request, |request| &mut request.args)
}
pub(super) fn routedWatch(request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreWatchRequest, CoreLinkError> {
    encodeRouted(request, |request| &mut request.args)
}
fn routedPush(request: RoutedCoreRequest<CorePushRequest>) -> Result<CorePushRequest, CoreLinkError> {
    encodeRouted(request, |request| &mut request.args)
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
        let routed = decodeRouted::<CoreCallRequest>(request.args, |request| &mut request.args);
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
        let r: RoutedCoreRequest<CoreWatchRequest> = decodeRouted(request.args, |request: &mut CoreWatchRequest| &mut request.args)?;
        if r.payload.requestId != request.requestId { return Err(error("Routed correlation mismatch")); }
        self.router()?.routedWatchSnapshot(self.peer.clone(), r).await
    }
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let r: RoutedCoreRequest<CoreWatchRequest> = decodeRouted(request.args, |request: &mut CoreWatchRequest| &mut request.args)?;
        if r.payload.requestId != request.requestId { return Err(error("Routed correlation mismatch")); }
        self.router()?.routedWatch(self.peer.clone(), r).await
    }
    async fn openPush(&mut self, request: CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let r: RoutedCoreRequest<CorePushRequest> = decodeRouted(request.args, |request: &mut CorePushRequest| &mut request.args)?;
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
            Authorization::Inbound => self.service.inboundCredential(&self.sessionId)?
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
        channel.receive().await.transpose().map(|message| (message.and_then(EncodedMessage::new), channel))
    });
    let side = InboundSide::new(service, peer, sessionId, Authorization::Inbound);
    serveIncoming(side, incoming, |messages| channel.sendEncodedBatch(messages)).await
}

/// Calls have no session stream state. Preserve routing/correlation checks while
/// allowing a constrained local handler to encode its typed result directly.
async fn executeCall(side: &InboundSide, request: CoreCallRequest, compact: bool) -> Result<EncodedMessage<CoreLinkResponse>, CoreLinkError> {
    let id = request.requestId.clone();
    let response = match decodeRouted::<CoreCallRequest>(request.args, |request| &mut request.args) {
        Ok(routed) if routed.payload.requestId == id => match side.service.state.router.upgrade() {
            Some(router) => {
                if compact {
                    if let Some(encoded) = router.prepareRoutedCall(&side.peer, &routed) { return encoded.map(EncodedMessage::fromBytes); }
                }
                router.routedCall(side.peer.clone(), routed).await
            },
            None => CoreCallResponse::err(id, CoreLinkError::new("PEER_ROUTER_CLOSED", "Peer router was released")),
        },
        Ok(_) => CoreCallResponse::err(id, error("Routed correlation mismatch")),
        Err(error) => CoreCallResponse::err(id, error),
    };
    EncodedMessage::new(CoreLinkResponse::Call(response))
}

/// Calls have independent correlations and may forward through this same carrier.
/// Serializing them creates wait cycles while routes change. Stream state remains
/// owned by one session so Watch/Push operations retain their connection order.
fn encodeResponses(responses: Vec<CoreLinkResponse>) -> Result<Vec<Vec<u8>>, CoreLinkError> {
    responses.into_iter().map(|response| encodeLink(response).map_err(|failure| error(failure.to_string()))).collect()
}

pub(super) async fn serveIncoming<I, S, F>(
    side: InboundSide, incoming: I, send: S,
) -> Result<(), CoreLinkError>
where
    I: Stream<Item = Result<EncodedMessage<PeerMessage>, CoreLinkError>>,
    S: Fn(Vec<Vec<u8>>) -> F,
    F: std::future::Future<Output = Result<(), CoreLinkError>>,
{
    let scheduler = side.service.state.host.hostRuntimeTaskSchedulerHost.clone()
        .ok_or_else(|| error("Host scheduler is not installed"))?;
    let compact = side.service.state.host.runtimeStorageHost.as_ref().is_some_and(|storage| !storage.retainPreferencesCache());
    // Stream ordering stays independent of Calls, including forwarded Calls that
    // release an opening Watch or a backpressured Push on this same carrier.
    let session = InboundSide::new(side.service.clone(), side.peer.clone(), side.sessionId.clone(), side.authorization);
    let (requests, queue) = tokio::sync::mpsc::unbounded_channel::<EncodedMessage<CoreLinkRequest>>();
    let sessions = stream::unfold((session, queue), |(mut session, mut queue)| async move {
        enum Item { Request(Option<EncodedMessage<CoreLinkRequest>>), Event(Option<Vec<CoreLinkResponse>>) }
        let item = if session.session.hasWatches() {
            tokio::select! {
                request = queue.recv() => Item::Request(request),
                event = session.session.nextWatchBatch() => Item::Event(event),
            }
        } else { Item::Request(queue.recv().await) };
        let response = match item {
            Item::Request(Some(request)) => match session.validate().and_then(|_| request.decodeWith(decodeLinkRequest)) {
                Err(error) => Err(error),
                Ok(request) => {
                    let mut responses = vec![session.session.dispatch(request).await];
                    if let Some(Some(events)) = session.session.nextWatchBatch().now_or_never() { responses.extend(events); }
                    encodeResponses(responses)
                }
            },
            Item::Event(Some(events)) => encodeResponses(events)
                .and_then(|encoded| session.validate().map(|_| encoded)),
            _ => return None,
        };
        Some((response, (session, queue)))
    });
    futures_util::pin_mut!(incoming, sessions);
    type Reply = Result<EncodedMessage<CoreLinkResponse>, CoreLinkError>;
    let mut calls = FuturesUnordered::<tokio::sync::oneshot::Receiver<Reply>>::new();
    loop {
        enum Incoming {
            Message(Option<Result<EncodedMessage<PeerMessage>, CoreLinkError>>),
            Response(Result<Reply, tokio::sync::oneshot::error::RecvError>),
            Session(Option<Result<Vec<Vec<u8>>, CoreLinkError>>),
        }
        let item = tokio::select! {
            message = incoming.next() => Incoming::Message(message),
            Some(response) = calls.next(), if !calls.is_empty() => Incoming::Response(response),
            response = sessions.next() => Incoming::Session(response),
        };
        // Queued messages retain only their encoded frame on every Host.
        // Recheck revocation at the original boundary, before any request entry
        // or response transmission, without another full tree in memory.
        side.validate()?;
        let responses = match item {
            Incoming::Message(Some(Ok(mut message))) => {
                if let Some(header) = message.callHeader()?.filter(|header| header.request) {
                    let service = side.service.clone();
                    let peer = side.peer.clone();
                    let sessionId = side.sessionId.clone();
                    let authorization = side.authorization;
                    let id = header.requestId;
                    let (mut completed, result) = tokio::sync::oneshot::channel();
                    scheduler.scheduleHostRuntimeBlockingAsyncTask("peer-inbound-call", Box::new(move || Box::pin(async move {
                        let call = InboundSide::new(service, peer, sessionId, authorization);
                        tokio::select! {
                            biased;
                            _ = completed.closed() => {},
                            response = async {
                                if let Err(error) = call.validate() {
                                    return EncodedMessage::new(CoreLinkResponse::Call(CoreCallResponse::err(id, error)));
                                }
                                if compact {
                                    if let Some(router) = call.service.state.router.upgrade() {
                                        if let Some(encoded) = router.prepareEncodedCall(&call.peer, message.bytesMut()) { return encoded.map(EncodedMessage::fromBytes); }
                                    }
                                }
                                match message.decodeWith(PeerMessage::decode) {
                                    Ok(PeerMessage::Request(CoreLinkRequest::Call(request))) => executeCall(&call, request, compact).await,
                                    Ok(_) => EncodedMessage::new(CoreLinkResponse::Call(CoreCallResponse::err(id, error("Expected inbound Call")))),
                                    Err(error) => EncodedMessage::new(CoreLinkResponse::Call(CoreCallResponse::err(id, error))),
                                }
                            } => { let _ = completed.send(response); },
                        }
                    }))).map_err(|failure| error(failure.to_string()))?;
                    calls.push(result);
                    continue;
                }
                if message.isResponse()? { return Err(error("Unexpected inbound Link response")); }
                requests.send(EncodedMessage::fromBytes(message.intoBytes()))
                    .map_err(|_| error("Inbound stream dispatcher stopped"))?;
                continue;
            },
            Incoming::Message(Some(Err(error))) | Incoming::Session(Some(Err(error))) => return Err(error),
            Incoming::Message(None) | Incoming::Session(None) => break,
            Incoming::Response(response) => vec![response.map_err(|_| error("Inbound Call task stopped"))??.intoBytes()],
            Incoming::Session(Some(Ok(responses))) => responses,
        };
        send(responses).await?;
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

#[cfg(test)]
mod ownership_tests {
    use super::*;
    fn request() -> RoutedCoreRequest<CoreCallRequest> {
        RoutedCoreRequest {
            spaceId: "space".into(), originNodeId: "origin".into(), targetNodeId: "target".into(),
            ttl: 8, routeKind: RoutedCoreRequestKind::Target,
            payload: CoreCallRequest::new("id", "test", "call", CoreValue::List(vec![CoreValue::String("owned payload".repeat(100))])),
        }
    }
    fn pointer(value: &CoreValue) -> *const u8 {
        match value { CoreValue::List(items) => match &items[0] {
            CoreValue::String(value) => value.as_ptr(), _ => panic!("string expected"),
        }, _ => panic!("list expected") }
    }
    #[test]
    fn routing_moves_args_without_changing_wire_or_metadata_validation() {
        let original = request();
        let expected = toCoreValue(original.clone()).unwrap();
        let before = pointer(&original.payload.args);
        let mut encoded = routedCall(original.clone()).unwrap();
        assert_eq!(encoded.args, expected);
        let moved = routedCall(original).unwrap();
        let decoded = decodeRouted::<CoreCallRequest>(moved.args, |r| &mut r.args).unwrap();
        assert_eq!(pointer(&decoded.payload.args), before);
        assert_eq!(decoded, request());
        // Native maps and wire struct sequences must both preserve the subtree.
        let original = request();
        let mut sequence = CoreValue::List(vec![
            CoreValue::String(original.spaceId), CoreValue::String(original.originNodeId), CoreValue::String(original.targetNodeId),
            CoreValue::Unsigned(original.ttl.into()), CoreValue::String("Target".into()),
            CoreValue::List(vec![CoreValue::String(original.payload.requestId.0), CoreValue::String(original.payload.target),
                CoreValue::String(original.payload.methodName), original.payload.args]),
        ]);
        let before = pointer(routedArgs(&mut sequence).unwrap());
        let decoded = decodeRouted::<CoreCallRequest>(sequence, |r| &mut r.args).unwrap();
        assert_eq!(pointer(&decoded.payload.args), before);
        assert_eq!(decoded, request());
        if let CoreValue::Map(fields) = &mut encoded.args { fields.insert("ttl".into(), CoreValue::String("bad ttl".into())); }
        assert!(decodeRouted::<CoreCallRequest>(encoded.args, |r| &mut r.args).is_err());
    }
}
