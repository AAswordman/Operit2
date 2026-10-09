//! Reusable session multiplexing: one decrypting reader, bounded queues and maps.
//! All requests remain ordinary encrypted Call/Watch/Push messages.
use super::*;
use tokio::sync::{mpsc, oneshot};
use std::sync::atomic::{AtomicBool, Ordering};
const CAPACITY: usize = 8;
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
pub(super) fn diagnostic() -> Option<String> { LAST_ERROR.lock().unwrap().clone() }
pub(super) struct Duplex {
    incoming: AsyncMutex<mpsc::Receiver<PeerMessage>>,
    pending: Mutex<BTreeMap<String, oneshot::Sender<CoreLinkResponse>>>,
    watches: Mutex<BTreeMap<String, mpsc::Sender<CoreEvent>>>,
    pub closed: AtomicBool,
    shutdown: tokio::sync::Notify,
    failure: Mutex<Option<CoreLinkError>>,
}
fn requestKey(r: &CoreLinkRequest) -> String {
    match r {
        CoreLinkRequest::Call(r) => format!("c:{}", r.requestId.0),
        CoreLinkRequest::Watch(r) => format!("w:{}", match r {
            CoreLinkWatchRequest::Open(r) | CoreLinkWatchRequest::Snapshot(r) => &r.requestId.0,
            CoreLinkWatchRequest::Close { requestId } => &requestId.0 }),
        CoreLinkRequest::Push(r) => format!("p:{}", match r {
            CoreLinkPushRequestMessage::Open(r) => &r.requestId.0,
            CoreLinkPushRequestMessage::Item(r) => &r.pushId,
            CoreLinkPushRequestMessage::Close { pushId } => pushId }),
    }
}
fn responseKey(r: &CoreLinkResponse) -> String {
    match r { CoreLinkResponse::Call(r) => format!("c:{}", r.requestId.0),
        CoreLinkResponse::Watch { requestId, .. } => format!("w:{}", requestId.0),
        CoreLinkResponse::Push { pushId, .. } => format!("p:{pushId}") }
}
struct PendingGuard<'a>(&'a Duplex, String, bool);
impl Drop for PendingGuard<'_> {
    fn drop(&mut self) {
        self.0.pending.lock().unwrap().remove(&self.1);
        if !self.2 { self.0.fail(error("In-flight Link request cancelled before acknowledgement")); } // Cannot reuse an ambiguous transaction.
    }
}
impl Duplex {
    pub fn new() -> (Arc<Self>, mpsc::Sender<PeerMessage>) {
        let (sender, incoming) = mpsc::channel(CAPACITY);
        (Arc::new(Self { incoming: AsyncMutex::new(incoming), pending: Mutex::default(), watches: Mutex::default(), closed: AtomicBool::new(false), shutdown: tokio::sync::Notify::new(), failure: Mutex::new(None) }), sender)
    }
    pub fn fail(&self, e: CoreLinkError) {
        // Preserve the cause, not a later "connection closed" from dispatch.
        let first = {
            let mut failure = self.failure.lock().unwrap();
            if failure.is_some() { false } else { *failure = Some(e.clone()); true }
        };
        if first {
            *LAST_ERROR.lock().unwrap() = Some(e.message.clone());
            #[cfg(not(target_os = "espidf"))]
            operit_util::AppLogger::AppLogger::trace("RuntimePeerService", &format!("Reusable carrier failed: {e}"));
        }
        self.finish();
    }
    pub fn finish(&self) {
        self.closed.store(true, Ordering::Release);
        self.shutdown.notify_one();
        self.pending.lock().unwrap().clear();
        self.watches.lock().unwrap().clear();
    }
    pub async fn receive(&self) -> Option<PeerMessage> { self.incoming.lock().await.recv().await }
    pub async fn exchange(&self, channel: &Channel, request: CoreLinkRequest) -> Result<CoreLinkResponse, CoreLinkError> {
        if self.closed.load(Ordering::Acquire) { return Err(error("Serial duplex session closed")); }
        let key = requestKey(&request);
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap();
            if pending.len() >= CAPACITY || pending.contains_key(&key) { return Err(error("Serial request capacity/correlation conflict")); }
            pending.insert(key.clone(), sender);
        }
        let mut guard = PendingGuard(self, key, false);
        let result = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            channel.send(PeerMessage::Request(request)).await?;
            receiver.await.map_err(|_| self.failure.lock().unwrap().clone().unwrap_or_else(|| error("Serial carrier ended before response")))
        }).await.unwrap_or_else(|_| Err(error("Serial Link call response timed out")));
        // Never replay a mutation after a missed reply. Close ambiguous carrier.
        if let Err(e) = &result { self.fail(e.clone()); channel.close().await; }
        guard.2 = true;
        result
    }
    fn response(&self, r: CoreLinkResponse) -> Result<(), CoreLinkError> {
        if let CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Event(event)) } = r {
            if let Some(sender) = self.watches.lock().unwrap().get(&requestId.0) {
                match sender.try_send(event) {
                    Ok(()) | Err(mpsc::error::TrySendError::Closed(_)) => {},
                    Err(mpsc::error::TrySendError::Full(_)) => return Err(error("Serial watch overflow; consumer did not keep up")),
                }
            }
            return Ok(()); // A cancelled watch can have an event already in flight.
        }
        if let Some(sender) = self.pending.lock().unwrap().remove(&responseKey(&r)) { let _ = sender.send(r); }
        else if let CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Closed) } = r { self.watches.lock().unwrap().remove(&requestId.0); }
        else { return Err(error("Unexpected serial response correlation")); }
        Ok(())
    }
    pub async fn pump(&self, channel: Arc<Channel>, incoming: mpsc::Sender<PeerMessage>) {
        loop {
            if self.closed.load(Ordering::Acquire) { break; }
            let received = tokio::select! {
                _ = self.shutdown.notified() => break,
                received = channel.receiveEncrypted() => received,
            };
            let result = match received {
                Ok(Some(PeerMessage::Response(r))) => self.response(r),
                Ok(Some(PeerMessage::Request(r))) => incoming.try_send(PeerMessage::Request(r)).map_err(|_| error("Serial inbound queue overflow")),
                Ok(None) => Err(error("Serial peer ended the session")),
                Err(e) => Err(e),
            };
            if let Err(e) = result {
                self.fail(e);
                break;
            }
            // A coalesced UART read can contain many complete events. Neither
            // receive nor try_send has to suspend, so give bounded consumers
            // and inbound dispatch a turn before draining the next frame.
            // Keep overflow an error; never drop deltas or grow the queues.
            tokio::task::yield_now().await;
        }
        self.finish();
        channel.close().await;
    }
    pub async fn watch(&self, service: &HostRuntimePeerService, channel: Arc<Channel>, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream, CoreLinkError> {
        let id = request.payload.requestId.clone();
        let (sender, stream) = CoreEventStream::boundedChannel(CAPACITY);
        {
            let mut watches = self.watches.lock().unwrap();
            if watches.len() >= CAPACITY || watches.contains_key(&id.0) { return Err(error("Serial watch capacity/correlation conflict")); }
            watches.insert(id.0.clone(), sender);
        }
        match channel.exchange(CoreLinkRequest::Watch(CoreLinkWatchRequest::Open(dispatch::routedWatch(request)?))).await {
            Ok(CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Opened) }) if requestId == id => {},
            result => { self.watches.lock().unwrap().remove(&id.0); return Err(match result {
                Err(e) | Ok(CoreLinkResponse::Watch { result: Err(e), .. }) => e, _ => error("Serial watch open response mismatch") }); }
        }
        let scheduler = service.state.host.hostRuntimeTaskSchedulerHost.clone().ok_or_else(|| error("Host scheduler missing"))?;
        Ok(stream.withOnClose(move || {
            let Some(mux) = channel.duplex() else { return; };
            mux.watches.lock().unwrap().remove(&id.0);
            if !mux.closed.load(Ordering::Acquire) {
                let _ = scheduler.scheduleHostRuntimeAsyncTask("serial-watch-close", Box::new(move || Box::pin(async move {
                    let _ = channel.exchange(CoreLinkRequest::Watch(CoreLinkWatchRequest::Close { requestId: id })).await;
                })));
            }
        }))
    }
}
pub(super) const RETURN_METHOD: &str = "session-return";
const CAPABILITIES_METHOD: &str = "capabilities";
fn scopeMatches(granted: Option<&str>, current: Option<&str>) -> bool {
    granted.is_some() && granted == current
}
impl HostRuntimePeerService {
    pub(super) async fn supportsSessionReturn(&self, channel: &Arc<Channel>) -> Result<bool, CoreLinkError> {
        let id = nextCoreRouteRequestId("peer-capabilities");
        let response = tokio::time::timeout(std::time::Duration::from_secs(15),
            channel.exchange(CoreLinkRequest::Call(CoreCallRequest::new(
                id.clone(), space_channel::TARGET, CAPABILITIES_METHOD, CoreValue::Null))))
            .await.map_err(|_| error("Session capability response timed out"))??;
        match response {
            CoreLinkResponse::Call(CoreCallResponse { requestId, result }) if requestId.0 == id => match result {
                Ok(value) => {
                    let capabilities: Vec<String> = fromCoreValue(value).map_err(|e| error(e.to_string()))?;
                    Ok(capabilities.iter().any(|c| c == RETURN_METHOD))
                }
                Err(e) => Err(e),
            },
            _ => Err(error("Session capability response mismatch")),
        }
    }
    pub(super) fn acceptSessionManagement(&self, peer: &str, pairing: &str, channel: &Arc<Channel>, request: &CoreCallRequest) -> Result<CoreValue, CoreLinkError> {
        match request.methodName.as_str() {
            CAPABILITIES_METHOD if request.args == CoreValue::Null => {
                let capabilities = if channel.raw.requiresSessionReuse() { vec![RETURN_METHOD] } else { vec![] };
                toCoreValue(capabilities).map_err(|e| error(e.to_string()))
            }
            RETURN_METHOD => {
                if channel.duplex().is_none() { return Err(CoreLinkError::methodNotFound(RETURN_METHOD)); }
                let scope: String = fromCoreValue(request.args.clone()).map_err(|e| error(e.to_string()))?;
                if self.router()?.spaceChannelScope(peer)?.as_deref() != Some(&scope) {
                    return Err(CoreLinkError::new("SPACE_CHANNEL_NOT_ADMITTED", "Both devices must be admitted to this Space"));
                }
                *channel.returnScope.lock().unwrap() = Some(scope);
                self.changed();
                Ok(CoreValue::Null)
            }
            "offer" => self.acceptSpaceChannel(peer, pairing, &channel.raw, request),
            _ => Err(CoreLinkError::methodNotFound(&request.methodName)),
        }
    }

    pub(super) fn startDuplex(&self, channel: &Arc<Channel>, peer: &str) -> Result<(), CoreLinkError> {
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.as_ref().ok_or_else(|| error("Host scheduler missing"))?;
        let (duplex, incoming) = Duplex::new();
        channel.setDuplex(duplex.clone());
        let reader = channel.clone(); let finished = reader.clone();
        let state = Arc::downgrade(&self.state); let peer = peer.to_owned();
        scheduler.scheduleHostRuntimeAsyncTask("serial-reader", Box::new(move || Box::pin(async move {
            duplex.pump(reader, incoming).await;
            if let Some(state) = state.upgrade() {
                let removed = {
                    let mut sessions = state.sharedSessions.lock().unwrap();
                    if sessions.get(&peer).is_some_and(|old| Arc::ptr_eq(old, &finished)) { sessions.remove(&peer); true } else { false }
                };
                if removed { state.active.lock().unwrap().remove(&peer); let _ = state.changes.send(()); }
            }
        })))
            .map_err(|e| error(e.to_string()))
    }
    pub(super) async fn offerSessionReturn(&self, peer: &str, channel: &Arc<Channel>) -> Result<(), CoreLinkError> {
        let scope = self.router()?.spaceChannelScope(peer)?;
        let old = channel.returnScope.lock().unwrap().clone();
        if old.is_some() && old != scope { channel.close().await; return Err(error("Serial return Space changed; reconnect required")); }
        let Some(scope) = scope else { return Ok(()); };
        if old.as_ref() == Some(&scope) { return Ok(()); }
        *channel.returnScope.lock().unwrap() = Some(scope.clone());
        let id = nextCoreRouteRequestId("serial-return");
        let response = channel.exchange(CoreLinkRequest::Call(CoreCallRequest::new(id.clone(), space_channel::TARGET,
            RETURN_METHOD, CoreValue::String(scope)))).await;
        match response {
            Ok(CoreLinkResponse::Call(CoreCallResponse { requestId, result: Ok(CoreValue::Null) })) if requestId.0 == id => Ok(()),
            Ok(CoreLinkResponse::Call(CoreCallResponse { requestId, result: Err(e) })) if requestId.0 == id && e.code == "SPACE_CHANNEL_NOT_ADMITTED" => {
                *channel.returnScope.lock().unwrap() = None; Ok(())
            }
            result => { channel.close().await; Err(match result {
                Err(e) | Ok(CoreLinkResponse::Call(CoreCallResponse { result: Err(e), .. })) => e,
                _ => error("Serial return grant response mismatch") }) }
        }
    }
    pub(super) fn sessionScopeAllowed(&self, peer: &str, channel: &Channel) -> Result<bool, CoreLinkError> {
        self.requirePeerConnectionAllowed(peer)?;
        let scope = channel.returnScope.lock().unwrap().clone();
        Ok(scopeMatches(scope.as_deref(), self.router()?.spaceChannelScope(peer)?.as_deref()))
    }
    pub(super) fn cachedSession(&self, peer: &str) -> Option<Arc<Channel>> {
        self.state.sharedSessions.lock().unwrap().get(peer).filter(|c| c.isOpen()).cloned()
    }
}
// Select once and bind each client to that admission/session generation. An old
// client must not silently acquire authority in a different Space on reconnect.
fn entryIdentity(service: &HostRuntimePeerService, peer: &str) -> Option<crate::NodeServices::SpaceClientIdentity> {
    service.requirePeerConnectionAllowed(peer).ok()?;
    let (spaceId, generation) = if let Some(channel) = service.cachedSession(peer) {
        if !service.sessionScopeAllowed(peer, &channel).ok()? { return None; }
        (channel.returnScope.lock().unwrap().clone()?, format!("session:{}", channel.generation))
    } else {
        if !service.activePeerNodeIds().ok()?.contains(peer) { return None; }
        let grant = service.spaceOutbound(peer).ok()?;
        (grant.spaceId, format!("grant:{}:{:?}:{}", grant.id, grant.transport, grant.endpoint))
    };
    Some(crate::NodeServices::SpaceClientIdentity { spaceId, peerNodeId: peer.into(), generation })
}
struct SessionSpaceClient { service: HostRuntimePeerService, identity: crate::NodeServices::SpaceClientIdentity }
impl SessionSpaceClient {
    fn route<T>(&self, payload: T, routeKind: RoutedCoreRequestKind) -> Result<RoutedCoreRequest<T>, CoreLinkError> {
        if entryIdentity(&self.service, &self.identity.peerNodeId).as_ref() != Some(&self.identity) {
            return Err(CoreLinkError::new("SPACE_ENTRY_CHANGED", "Space entry was replaced or revoked"));
        }
        Ok(RoutedCoreRequest { spaceId: self.identity.spaceId.clone(), originNodeId: self.service.state.nodeId.clone(),
            targetNodeId: self.identity.peerNodeId.clone(), ttl: 4, routeKind, payload })
    }
}
fn callRouteKind(request: &CoreCallRequest) -> RoutedCoreRequestKind {
    // Includes explicit create_binding commands; only Core allocates the Binding.
    if crate::generated_space_call_route(request).is_some() { RoutedCoreRequestKind::SpaceBinding }
    else { RoutedCoreRequestKind::SpaceRoute }
}
fn watchRouteKind(request: &CoreWatchRequest, snapshot: bool) -> RoutedCoreRequestKind {
    // Embedded streams are reopened via their source, not treated as ordinary
    // annotated properties. Snapshot has no source-reopen semantics in Core.
    if crate::generated_space_watch_route(request).is_some()
        || (!snapshot && request.target == operit_link::CORE_STREAM_TARGET) {
        RoutedCoreRequestKind::SpaceBinding
    } else { RoutedCoreRequestKind::SpaceRoute }
}
#[async_trait(?Send)]
impl CoreLinkSharedClient for SessionSpaceClient {
    async fn call(&self, mut r: CoreCallRequest) -> CoreCallResponse {
        r.requestId = CoreRequestId::new(nextCoreRouteRequestId("edge-ui"));
        let id = r.requestId.clone();
        let kind = callRouteKind(&r);
        match self.route(r, kind) { Ok(route) => self.service.call(&self.identity.peerNodeId, route).await, Err(e) => CoreCallResponse::err(id, e) }
    }
    async fn watchSnapshot(&self, mut r: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        r.requestId = CoreRequestId::new(nextCoreRouteRequestId("edge-snapshot"));
        let kind = watchRouteKind(&r, true);
        self.service.watchSnapshot(&self.identity.peerNodeId, self.route(r, kind)?).await
    }
    async fn watch(&self, mut r: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        r.requestId = CoreRequestId::new(nextCoreRouteRequestId("edge-watch"));
        let kind = watchRouteKind(&r, false);
        self.service.watch(&self.identity.peerNodeId, self.route(r, kind)?).await
    }
}
pub(super) fn spaceConnection(service: &HostRuntimePeerService) -> Option<crate::NodeServices::SpaceClientConnection> {
    let candidates: BTreeSet<String> = service.state.sharedSessions.lock().unwrap().keys().cloned()
        .chain(service.activePeerNodeIds().ok()?.into_iter()).collect();
    let identity = candidates.iter().find_map(|peer| entryIdentity(service, peer))?;
    let client = Arc::new(SessionSpaceClient { service: service.clone(), identity: identity.clone() });
    Some(crate::NodeServices::SpaceClientConnection { identity, client })
}
pub(super) fn spaceClient(service: &HostRuntimePeerService) -> Option<Arc<dyn CoreLinkSharedClient + Send + Sync>> {
    spaceConnection(service).map(|connection| connection.client)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    struct MemoryPeer {
        endpoint: PeerEndpoint, tx: Mutex<Option<mpsc::Sender<PeerMessage>>>, rx: AsyncMutex<mpsc::Receiver<PeerMessage>>,
        closed: AtomicBool, stopped: tokio::sync::Notify, sends: AtomicUsize,
    }
    #[async_trait]
    impl PeerConnection for MemoryPeer {
        fn source(&self) -> &PeerEndpoint { &self.endpoint }
        fn target(&self) -> &PeerEndpoint { &self.endpoint }
        fn transport(&self) -> PeerTransport { PeerTransport::Serial }
        fn requiresSessionReuse(&self) -> bool { true }
        async fn send(&self, r: PeerMessage) -> Result<(), String> {
            self.sends.fetch_add(1, Ordering::SeqCst);
            let tx = self.tx.lock().unwrap().clone().ok_or_else(|| "closed".to_owned())?;
            tx.send(r).await.map_err(|_| "closed".into())
        }
        async fn receive(&self) -> Result<Option<PeerMessage>, String> {
            if self.closed.load(Ordering::Acquire) { return Ok(None); }
            let mut rx = self.rx.lock().await;
            Ok(tokio::select! { _ = self.stopped.notified() => None, r = rx.recv() => r })
        }
        async fn close(&self) { self.closed.store(true, Ordering::Release); self.tx.lock().unwrap().take(); self.stopped.notify_one(); }
    }
    fn pair() -> (Arc<Channel>, Arc<Channel>) {
        let (atx, arx) = mpsc::channel(32); let (btx, brx) = mpsc::channel(32);
        let make = |tx, rx, client| Channel::new(Arc::new(MemoryPeer {
            endpoint: PeerEndpoint { nodeId: "test".into(), address: "memory".into() }, tx: Mutex::new(Some(tx)), rx: AsyncMutex::new(rx),
            closed: AtomicBool::new(false), stopped: tokio::sync::Notify::new(), sends: AtomicUsize::new(0),
        }), &[7;32], b"test-scope", client).unwrap();
        (make(atx, brx, true), make(btx, arx, false))
    }
    fn start(channel: &Arc<Channel>) -> tokio::task::JoinHandle<()> {
        let (d, incoming) = Duplex::new(); channel.setDuplex(d.clone()); let c = channel.clone();
        tokio::spawn(async move { d.pump(c, incoming).await; })
    }
    fn echo(channel: Arc<Channel>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Ok(Some(PeerMessage::Request(CoreLinkRequest::Call(r)))) = channel.receive().await {
                if channel.send(PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse { requestId: r.requestId, result: Ok(r.args) }))).await.is_err() { break; }
            }
        })
    }
    #[tokio::test]
    async fn one_reader_multiplexes_one_hundred_simultaneous_reverse_calls() {
        let (a,b) = pair(); let ap = start(&a); let bp = start(&b);
        let ae = echo(a.clone()); let be = echo(b.clone());
        for round in 0..100 {
            let request = |id: String| CoreLinkRequest::Call(CoreCallRequest::new(id, "echo", "read", CoreValue::Unsigned(round)));
            let (ar,br) = tokio::join!(a.exchange(request(format!("a-{round}"))), b.exchange(request(format!("b-{round}"))));
            for r in [ar.unwrap(),br.unwrap()] { assert!(matches!(r, CoreLinkResponse::Call(CoreCallResponse { result: Ok(CoreValue::Unsigned(n)), .. }) if n == round)); }
        }
        assert!(a.duplex().unwrap().pending.lock().unwrap().is_empty());
        a.close().await; b.close().await; ap.await.unwrap(); bp.await.unwrap(); ae.await.unwrap(); be.await.unwrap();
    }
    #[tokio::test]
    async fn slow_keepalive_uses_exchange_deadline_without_abandoning_other_calls() {
        let (a,b) = pair(); let ap = start(&a); let bp = start(&b);
        let server = tokio::spawn({ let b = b.clone(); async move {
            let Some(PeerMessage::Request(CoreLinkRequest::Call(ping))) = b.receive().await.unwrap() else { panic!("ping"); };
            assert_eq!(ping.target, "$peer.keepalive");
            // A busy shared UART can acknowledge after the connect deadline.
            tokio::time::sleep(std::time::Duration::from_millis(5_200)).await;
            b.send(PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse { requestId: ping.requestId, result: Ok(CoreValue::Null) }))).await.unwrap();
            let Some(PeerMessage::Request(CoreLinkRequest::Call(call))) = b.receive().await.unwrap() else { panic!("call"); };
            b.send(PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse { requestId: call.requestId, result: Ok(call.args) }))).await.unwrap();
        }});
        let (probe, call) = tokio::join!(
            super::super::availability::probeConnectedSession(&a),
            async {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                a.exchange(CoreLinkRequest::Call(CoreCallRequest::new("business", "echo", "read", CoreValue::Unsigned(42)))).await
            }
        );
        probe.unwrap();
        assert!(matches!(call.unwrap(), CoreLinkResponse::Call(CoreCallResponse { result: Ok(CoreValue::Unsigned(42)), .. })));
        assert!(a.isOpen(), "slow keepalive must not poison a healthy carrier");
        assert!(a.duplex().unwrap().pending.lock().unwrap().is_empty());
        server.await.unwrap(); a.close().await; b.close().await; ap.await.unwrap(); bp.await.unwrap();
    }

    #[tokio::test]
    async fn watch_events_cannot_steal_a_call_reply() {
        let (a,b) = pair(); let ap = start(&a); let bp = start(&b);
        let (tx,mut stream) = CoreEventStream::boundedChannel(8);
        a.duplex().unwrap().watches.lock().unwrap().insert("watch".into(),tx);
        let server = tokio::spawn(async move {
            let Some(PeerMessage::Request(CoreLinkRequest::Call(r))) = b.receive().await.unwrap() else { panic!("call"); };
            b.send(PeerMessage::Response(CoreLinkResponse::Watch { requestId: CoreRequestId::new("watch"),
                result: Ok(CoreLinkWatchResponse::Event(CoreEvent { requestId: None, target: "echo".into(), propertyName: "watch".into(), kind: CoreEventKind::Snapshot, value: CoreValue::Unsigned(1) })) })).await.unwrap();
            b.send(PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse { requestId:r.requestId,result:Ok(CoreValue::Unsigned(2)) }))).await.unwrap();
            b
        });
        let r = a.exchange(CoreLinkRequest::Call(CoreCallRequest::new("call", "echo", "read", CoreValue::Null))).await.unwrap();
        assert!(matches!(r, CoreLinkResponse::Call(CoreCallResponse { result: Ok(CoreValue::Unsigned(2)), .. })));
        assert_eq!(stream.recv().await.unwrap().value, CoreValue::Unsigned(1));
        let b = server.await.unwrap(); a.close().await; b.close().await; ap.await.unwrap(); bp.await.unwrap();
    }
    #[tokio::test]
    async fn watch_dispatch_transfers_payload_without_cloning_it() {
        let (d, _) = Duplex::new();
        let (sender, mut stream) = CoreEventStream::boundedChannel(CAPACITY);
        d.watches.lock().unwrap().insert("owned".into(), sender);
        let text = "unchanged payload".repeat(1024);
        let allocation = text.as_ptr();
        d.response(CoreLinkResponse::Watch {
            requestId: CoreRequestId::new("owned"),
            result: Ok(CoreLinkWatchResponse::Event(CoreEvent {
                requestId: None, target: "echo".into(), propertyName: "owned".into(),
                kind: CoreEventKind::Snapshot, value: CoreValue::String(text),
            })),
        }).unwrap();
        let event = stream.recv().await.unwrap();
        let CoreValue::String(text) = event.value else { panic!("string") };
        assert_eq!(text.as_ptr(), allocation, "dispatch must move, not clone, the event tree");
    }

    #[tokio::test]
    async fn coalesced_watch_burst_gives_bounded_consumer_a_turn() {
        let (a,b) = pair(); let ap = start(&a); let bp = start(&b);
        let (tx,mut stream) = CoreEventStream::boundedChannel(CAPACITY);
        a.duplex().unwrap().watches.lock().unwrap().insert("burst".into(),tx);
        let consumer = tokio::spawn(async move {
            for index in 0..200 {
                let event = stream.recv().await.expect("healthy burst must not close watch");
                assert_eq!(event.value, CoreValue::Unsigned(index));
            }
        });
        for index in 0..200 {
            b.send(PeerMessage::Response(CoreLinkResponse::Watch {
                requestId: CoreRequestId::new("burst"), result: Ok(CoreLinkWatchResponse::Event(
                    CoreEvent { requestId: None, target: "echo".into(), propertyName: "burst".into(), kind: CoreEventKind::Snapshot, value: CoreValue::Unsigned(index) },
                )),
            })).await.unwrap();
        }
        tokio::time::timeout(std::time::Duration::from_secs(2),consumer).await.unwrap().unwrap();
        assert!(a.isOpen(), "burst is not a stalled consumer");
        a.close().await; b.close().await; ap.await.unwrap(); bp.await.unwrap();
    }

    #[test]
    fn truly_stalled_watch_consumer_still_fails_at_the_original_bound() {
        let (d, _) = Duplex::new();
        let (tx,_stream) = CoreEventStream::boundedChannel(CAPACITY);
        d.watches.lock().unwrap().insert("stalled".into(),tx);
        for index in 0..=CAPACITY {
            let response = d.response(CoreLinkResponse::Watch {
                requestId: CoreRequestId::new("stalled"), result: Ok(CoreLinkWatchResponse::Event(
                    CoreEvent { requestId: None, target: "echo".into(), propertyName: "stalled".into(), kind: CoreEventKind::Snapshot, value: CoreValue::Unsigned(index as u64) },
                )),
            });
            if index < CAPACITY { response.unwrap(); }
            else { assert_eq!(response.unwrap_err().message, "Serial watch overflow; consumer did not keep up"); }
        }
    }

    #[test]
    fn carrier_diagnostic_preserves_first_failure() {
        let (d, _) = Duplex::new();
        d.fail(error("original transport error"));
        d.fail(error("Peer connection is closed"));
        assert_eq!(d.failure.lock().unwrap().as_ref().unwrap().message, "original transport error");
    }

    #[tokio::test]
    async fn cancelled_mutation_poisoned_session_without_resending() {
        let (a,b) = pair(); let ap = start(&a); let bp = start(&b);
        let result = tokio::time::timeout(std::time::Duration::from_millis(10),
            a.exchange(CoreLinkRequest::Call(CoreCallRequest::new("mutate", "echo", "write", CoreValue::Null)))).await;
        assert!(result.is_err());
        assert!(!a.isOpen()); assert!(a.duplex().unwrap().pending.lock().unwrap().is_empty());
        assert!(matches!(b.receive().await.unwrap(), Some(PeerMessage::Request(_))));
        a.close().await; b.close().await; ap.await.unwrap(); bp.await.unwrap();
    }
    #[test]
    fn reverse_permission_requires_explicit_grant_and_current_membership() {
        assert!(!scopeMatches(None, Some("space"))); // Pairing alone grants nothing.
        assert!(!scopeMatches(Some("space"), None)); // Admission revoked.
        assert!(!scopeMatches(Some("old-space"), Some("new-space")));
        assert!(scopeMatches(Some("space"), Some("space")));
        let (a,b) = pair();
        *a.returnScope.lock().unwrap() = Some("space".into());
        assert!(b.returnScope.lock().unwrap().is_none(), "Grant is session-local, not inferred/reverse paired");
        let (new,_) = pair();
        assert!(new.returnScope.lock().unwrap().is_none(), "Reconnection must negotiate a new grant");
    }
    #[test]
    fn bounded_watch_overflow_and_wrong_correlation_fail_closed() {
        let (d,_) = Duplex::new();
        let (tx,_stream) = CoreEventStream::boundedChannel(1);
        d.watches.lock().unwrap().insert("watch".into(),tx);
        let event = || CoreLinkResponse::Watch { requestId:CoreRequestId::new("watch"),result:Ok(CoreLinkWatchResponse::Event(CoreEvent { requestId: None, target: "echo".into(), propertyName: "watch".into(), kind:CoreEventKind::Snapshot,value:CoreValue::Null })) };
        assert!(d.response(event()).is_ok()); assert!(d.response(event()).is_err());
        assert!(d.response(CoreLinkResponse::Call(CoreCallResponse { requestId:CoreRequestId::new("unknown"),result:Ok(CoreValue::Null) })).is_err());
    }
    #[derive(Default)]
    struct Storage(Mutex<BTreeMap<String,Vec<u8>>>);
    impl operit_host_api::RuntimeStorageHost for Storage {
        fn runtimeRootDir(&self) -> Option<std::path::PathBuf> { None }
        fn workspaceRootDir(&self) -> Option<std::path::PathBuf> { None }
        fn readBytes(&self,p:&str) -> operit_host_api::HostResult<Vec<u8>> { self.0.lock().unwrap().get(p).cloned().ok_or_else(||operit_host_api::HostError::new("missing")) }
        fn writeBytes(&self,p:&str,b:&[u8]) -> operit_host_api::HostResult<()> { self.0.lock().unwrap().insert(p.into(),b.into());Ok(()) }
        fn appendBytes(&self,p:&str,b:&[u8]) -> operit_host_api::HostResult<()> { self.0.lock().unwrap().entry(p.into()).or_default().extend_from_slice(b);Ok(()) }
        fn delete(&self,p:&str,_:bool) -> operit_host_api::HostResult<()> { self.0.lock().unwrap().remove(p);Ok(()) }
        fn exists(&self,p:&str) -> operit_host_api::HostResult<bool> { Ok(self.0.lock().unwrap().contains_key(p)) }
        fn list(&self,p:&str) -> operit_host_api::HostResult<Vec<operit_host_api::RuntimeStorageEntry>> { Ok(self.0.lock().unwrap().iter().filter(|(k,_)| k.starts_with(p)).map(|(k,v)|operit_host_api::RuntimeStorageEntry {path:k.clone(),isDirectory:false,size:v.len() as i64}).collect()) }
    }
    struct Router { node:String,scope:Mutex<Option<String>>,calls:AtomicUsize }
    #[async_trait(?Send)]
    impl PeerRouter for Router {
        fn localNodeId(&self) -> String { self.node.clone() }
        fn spaceChannelScope(&self,_:&str) -> Result<Option<String>,CoreLinkError> { Ok(self.scope.lock().unwrap().clone()) }
        async fn routedCall(&self,_:String,r:RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse {
            self.calls.fetch_add(1,Ordering::SeqCst);CoreCallResponse { requestId:r.payload.requestId,result:Ok(r.payload.args) }
        }
        async fn routedWatchSnapshot(&self,_:String,_:RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEvent,CoreLinkError> { Err(error("unused")) }
        async fn routedWatch(&self,_:String,_:RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream,CoreLinkError> { Err(error("unused")) }
        async fn routedOpenPush(&self,_:String,_:RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>,CoreLinkError> { Err(error("unused")) }
    }
    fn service(node:&str) -> (HostRuntimePeerService,Arc<Router>) {
        let storage=Arc::new(Storage::default());
        operit_store::CoreSpaceStore::CoreSpaceStore::newNodeLocal(storage.clone()).initialize().unwrap();
        let router=Arc::new(Router {node:node.into(),scope:Mutex::new(Some("space".into())),calls:AtomicUsize::new(0)});
        let service=HostRuntimePeerService { state:Arc::new(State { limits:PeerRuntimeLimits::default(),
            host:Arc::new(HostManager {runtimeStorageHost:Some(storage.clone()),..HostManager::default()}),link:HostPeerLink::default(),
            store:PeerStateStore::new(storage),nodeId:node.into(),info:LinkDeviceInfo {platform:"test".into(),model:"test".into()},router:Arc::downgrade(&(router.clone() as Arc<dyn PeerRouter>)),
            listeners:AsyncMutex::default(),connections:Mutex::default(),pooledChannels:Mutex::default(),liveChannels:Mutex::default(),advertisements:Mutex::default(),slots:Arc::new(tokio::sync::Semaphore::new(2)),
            active:Mutex::default(),changes:broadcast::channel(32).0,availability:Mutex::new(None),lifecycle:AsyncMutex::new(()),mutation:Mutex::new(()),
            connectLocks:Mutex::default(),sharedSessions:Mutex::default(),
        })};(service,router)
    }
    #[test]
    fn space_client_uses_network_return_grant_and_rechecks_scope() {
        let (service, router) = service("edge");
        service.state.store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "pairing", &StoredInbound {
            deviceId: "core".into(), deviceInfo: LinkDeviceInfo {platform:"test".into(),model:"Core".into()}, sessionSecret: BASE64.encode([1;32]),
            pairingServiceVersion: PAIRING_SERVICE_VERSION,
        }).unwrap();
        let grant = space_channel::Grant { id: "return".into(), spaceId: "space".into(), peer: "core".into(),
            pairingId: "pairing".into(), secret: BASE64.encode([2;32]), port: 1234,
            transport: PeerTransport::Tcp, endpoint: "127.0.0.1:1234".into() };
        service.state.store.putRecord("runtime/link_access/space_channel_outbound.preferences.json", "return", &grant).unwrap();
        service.state.active.lock().unwrap().insert("core".into());
        assert!(spaceClient(&service).is_some());
        let client = SessionSpaceClient { service: service.clone(), identity: entryIdentity(&service, "core").unwrap() };
        assert_eq!(client.route((), RoutedCoreRequestKind::SpaceRoute).unwrap().spaceId, "space");
        *router.scope.lock().unwrap() = None;
        assert!(client.route((), RoutedCoreRequestKind::SpaceRoute).is_err());
        assert!(spaceClient(&service).is_none());
    }
    #[test]
    fn edge_route_classes_do_not_turn_every_request_into_a_binding() {
        let args = CoreValue::Map(BTreeMap::from([("chatId".into(), CoreValue::String("chat".into()))]));
        let call = |method| CoreCallRequest::new("test", CORE_INTERNAL_TARGET, method, args.clone());
        assert_eq!(callRouteKind(&call("ensureRoutedChat")), RoutedCoreRequestKind::SpaceBinding);
        assert!(!crate::generated_space_call_route(&call("ensureRoutedChat")).unwrap().createBindingCapability.is_empty());
        assert_eq!(callRouteKind(&call("createRoutedChat")), RoutedCoreRequestKind::SpaceBinding);
        for method in ["chatAvailablePlugins", "chatPluginStatus"] {
            let request = call(method);
            assert_eq!(callRouteKind(&request), RoutedCoreRequestKind::SpaceBinding);
            let route = crate::generated_space_call_route(&request).unwrap();
            assert_eq!(route.permissionCapability, "chat.read");
            assert!(route.createBindingCapability.is_empty());
        }
        assert!(crate::generated_space_call_route(&call("createRoutedChat")).unwrap().createBindingCapability.is_empty());
        assert_eq!(callRouteKind(&CoreCallRequest::new("test", "device.status", "read", CoreValue::Null)), RoutedCoreRequestKind::SpaceRoute);
        let watch = CoreWatchRequest::new("test", CORE_INTERNAL_TARGET, "chatStateFlow", args);
        assert_eq!(watchRouteKind(&watch, false), RoutedCoreRequestKind::SpaceBinding);
        assert_eq!(watchRouteKind(&watch, true), RoutedCoreRequestKind::SpaceBinding);
        let embedded = CoreWatchRequest::new("test", CORE_STREAM_TARGET, "events", CoreValue::Null);
        assert_eq!(watchRouteKind(&embedded, false), RoutedCoreRequestKind::SpaceBinding);
        assert_eq!(watchRouteKind(&embedded, true), RoutedCoreRequestKind::SpaceRoute);
        assert_eq!(watchRouteKind(&CoreWatchRequest::new("test", "unknown", "events", CoreValue::Null), false), RoutedCoreRequestKind::SpaceRoute);
    }
    #[tokio::test]
    async fn replaced_entry_invalidates_old_clients_without_an_offline_poll() {
        let (service, router) = service("edge");
        // Emulate an authenticated inbound pairing with an admitted return grant.
        service.state.store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "pairing", &StoredInbound {
            deviceId: "core".into(), deviceInfo: service.state.info.clone(), pairingServiceVersion: PAIRING_SERVICE_VERSION,
            sessionSecret: BASE64.encode([1;32]),
        }).unwrap();
        let (first, remote) = pair();
        let firstPump = start(&first); let remotePump = start(&remote);
        *first.returnScope.lock().unwrap() = Some("space".into());
        service.state.sharedSessions.lock().unwrap().insert("core".into(), first.clone());
        let old = spaceConnection(&service).unwrap();
        let unchanged = spaceConnection(&service).unwrap();
        assert_eq!(old.identity, unchanged.identity);
        let (replacement, remote2) = pair();
        let replacementPump = start(&replacement); let remote2Pump = start(&remote2);
        *replacement.returnScope.lock().unwrap() = Some("space".into());
        service.state.sharedSessions.lock().unwrap().insert("core".into(), replacement.clone());
        let new = spaceConnection(&service).unwrap();
        assert_ne!(old.identity, new.identity);
        assert_eq!(old.identity.spaceId, new.identity.spaceId);
        let result = old.client.call(CoreCallRequest::new("stale", CORE_INTERNAL_TARGET, "ensureRoutedChat", CoreValue::Null)).await;
        assert_eq!(result.result.unwrap_err().code, "SPACE_ENTRY_CHANGED");
        *router.scope.lock().unwrap() = Some("other-space".into());
        *replacement.returnScope.lock().unwrap() = Some("other-space".into());
        assert_eq!(spaceConnection(&service).unwrap().identity.spaceId, "other-space");
        assert_eq!(new.client.call(CoreCallRequest::new("stale-space", "unknown", "send", CoreValue::Null)).await.result.unwrap_err().code, "SPACE_ENTRY_CHANGED");
        first.close().await; remote.close().await; replacement.close().await; remote2.close().await;
        firstPump.await.unwrap(); remotePump.await.unwrap(); replacementPump.await.unwrap(); remote2Pump.await.unwrap();
    }
    #[test]
    fn peer_service_does_not_keep_router_alive() {
        let (service, router) = service("test");
        assert_eq!(Arc::strong_count(&router), 1);
        drop(router);
        assert!(matches!(service.router(), Err(e) if e.code == "PEER_ROUTER_CLOSED"));
    }
    #[tokio::test]
    async fn connect_locks_are_per_endpoint_and_weakly_retained() {
        let (service, _router) = service("test");
        let first = service.connectionLock("endpoint-a");
        let same = service.connectionLock("endpoint-a");
        let other = service.connectionLock("endpoint-b");
        assert!(Arc::ptr_eq(&first, &same));
        let held = first.lock().await;
        assert!(same.try_lock().is_err());
        assert!(other.try_lock().is_ok());
        drop(held); drop(first); drop(same); drop(other);
        let _new = service.connectionLock("endpoint-c");
        assert_eq!(service.state.connectLocks.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn retired_session_method_is_not_a_wire_alias() {
        let (service, _router) = service("test");
        let (channel, remote) = pair();
        let request = CoreCallRequest::new("old-method", space_channel::TARGET,
            "serial-duplex", CoreValue::String("space".into()));
        let result = service.acceptSessionManagement("remote", "pairing", &channel, &request);
        assert_eq!(result.unwrap_err().code, "METHOD_NOT_FOUND");
        channel.close().await;
        remote.close().await;
    }
    #[tokio::test]
    async fn capability_negotiation_rejects_security_malformed_and_mismatched_responses() {
        let (service, _router) = service("test");
        for (result, mismatched, expected) in [
            (Ok(toCoreValue(vec![RETURN_METHOD]).unwrap()), false, Some(true)),
            (Ok(toCoreValue(Vec::<String>::new()).unwrap()), false, Some(false)),
            (Err(CoreLinkError::methodNotFound("capabilities")), false, None),
            (Err(error("Unknown Space channel operation")), false, None),
            (Err(error("Bad authorization")), false, None),
            (Ok(CoreValue::Null), false, None),
            (Err(error("Unknown Space channel operation")), true, None),
        ] {
            let (a, b) = pair();
            let server = async {
                let Some(PeerMessage::Request(CoreLinkRequest::Call(r))) = b.receive().await.unwrap() else { panic!() };
                b.send(PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse {
                    requestId: if mismatched { CoreRequestId::new("wrong") } else { r.requestId }, result,
                }))).await.unwrap();
            };
            let caller = async { assert_eq!(service.supportsSessionReturn(&a).await.ok(), expected); };
            tokio::time::timeout(std::time::Duration::from_secs(2), async { tokio::join!(server, caller); }).await.unwrap();
            a.close().await; b.close().await;
        }
    }
    #[tokio::test]
    async fn serial_return_uses_original_directional_pairing_and_rejects_revocation() {
        let (a,b)=pair();let ap=start(&a);let bp=start(&b);
        let (core,coreRouter)=service("core");let (edge,edgeRouter)=service("edge");
        core.state.store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,"pairing",&StoredOutbound {
            endpoint:"memory".into(),sessionId:"pairing".into(),deviceId:"core".into(),peerNodeId:"edge".into(),
            peerDeviceInfo:core.state.info.clone(),pairingServiceVersion:PAIRING_SERVICE_VERSION,sessionSecret:BASE64.encode([1;32]),transport:"serial".into()
        }).unwrap();
        edge.state.store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,"pairing",&StoredInbound {
            deviceId:"core".into(),deviceInfo:edge.state.info.clone(),pairingServiceVersion:PAIRING_SERVICE_VERSION,sessionSecret:BASE64.encode([1;32])
        }).unwrap();
        core.state.sharedSessions.lock().unwrap().insert("edge".into(),a.clone());
        edge.state.sharedSessions.lock().unwrap().insert("core".into(),b.clone());
        assert!(spaceClient(&edge).is_none());
        let scenario=async {
            core.offerSessionReturn("edge",&a).await.unwrap();
            assert!(spaceClient(&edge).is_some(), "open={} scope={:?} allowed={:?} diagnostics={:?}",b.isOpen(), b.returnScope.lock().unwrap().clone(), edge.sessionScopeAllowed("core", &b), diagnostic());
            assert!(edge.outbound("core").is_err(),"Ephemeral grant must not manufacture reverse pairing");
            let request=CoreCallRequest::new("chat","chat","send",CoreValue::String("physical-transport-contract".into()));
            let response=spaceClient(&edge).unwrap().call(request).await;
            assert_eq!(response.result.unwrap(),CoreValue::String("physical-transport-contract".into()));
            assert_eq!(coreRouter.calls.load(Ordering::SeqCst),1);
            *coreRouter.scope.lock().unwrap()=None;
            let response=spaceClient(&edge).unwrap().call(CoreCallRequest::new("revoked","chat","send",CoreValue::Null)).await;
            assert!(response.result.is_err());
            assert_eq!(coreRouter.calls.load(Ordering::SeqCst),1,"Revoked scope cannot execute even on an existing encrypted carrier");
            a.close().await;b.close().await;
        };
        let incoming=async {
            let result=dispatch::serveAuthorized(core.clone(),a.clone(),"edge".into(),"pairing".into(),dispatch::Authorization::SessionReturn).await;
            if let Err(e)=&result { a.duplex().unwrap().fail(e.clone()); }
            a.close().await; result
        };
        let receiving=async {
            let result=dispatch::serve(edge.clone(),b.clone(),"core".into(),"pairing".into(),false).await;
            b.close().await;result
        };
        tokio::time::timeout(std::time::Duration::from_secs(3),async { let (_, incoming_result, receiving_result) = tokio::join!(scenario,incoming,receiving);
            assert!(incoming_result.is_err(), "Scope revocation closes the original serial return session");
            assert!(receiving_result.is_ok()); }).await.unwrap();
        assert_eq!(edgeRouter.calls.load(Ordering::SeqCst),0);
        ap.await.unwrap();bp.await.unwrap();
    }

}
