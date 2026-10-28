use super::*;
use crate::{CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CorePushItem,
    CorePushRequest, CoreValue, CoreWatchRequest, decodeLink, encodeLink};
use std::{future::Future, sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}}, task::{Context, Wake, Waker}};
use async_trait::async_trait;

#[derive(Default)]
struct State {
    events: Mutex<BTreeMap<String, tokio::sync::mpsc::UnboundedSender<CoreEvent>>>,
    items: Mutex<Vec<CoreValue>>,
    watchDrops: AtomicUsize,
    pushDrops: AtomicUsize,
    pushCloses: AtomicUsize,
}
struct Client(Arc<State>);
struct Input(Arc<State>);
impl Drop for Input {
    fn drop(&mut self) { self.0.pushDrops.fetch_add(1, Ordering::SeqCst); }
}
#[async_trait]
impl CoreLinkPushSession for Input {
    async fn send(&mut self, value: CoreValue) -> Result<(), CoreLinkError> {
        if value == CoreValue::String("panic".into()) { panic!("broken push item"); }
        if value == CoreValue::String("fail".into()) {
            return Err(CoreLinkError::new("SERVICE_FAILED", "Test service failure"));
        }
        self.0.items.lock().unwrap().push(value);
        Ok(())
    }
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
        self.0.pushCloses.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
fn event(id: &str) -> CoreEvent {
    CoreEvent { requestId: Some(CoreRequestId::new(id)), target: "test".into(),
        propertyName: "state".into(), kind: CoreEventKind::Changed, value: CoreValue::Null }
}
fn watch(id: &str) -> CoreWatchRequest {
    CoreWatchRequest::new(id, "test", "state", CoreValue::emptyMap())
}
#[async_trait(?Send)]
impl CoreLinkClient for Client {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        if request.methodName == "panic" {
            tokio::task::yield_now().await;
            panic!("broken call after await");
        }
        CoreCallResponse::ok(request.requestId, request.args)
    }
    async fn watchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        if request.target == "denied" { return Err(CoreLinkError::new("DENIED", "Test denial")); }
        Ok(event(&request.requestId.0))
    }
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let (sender, stream) = CoreEventStream::channel();
        self.0.events.lock().unwrap().insert(request.requestId.0, sender);
        let state = self.0.clone();
        Ok(stream.withOnClose(move || { state.watchDrops.fetch_add(1, Ordering::SeqCst); }))
    }
    async fn openPush(&mut self, _: CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Ok(Box::new(Input(self.0.clone())))
    }
}
fn session(limit: usize) -> (CoreLinkSession<Client>, Arc<State>) {
    let state = Arc::new(State::default());
    (CoreLinkSession::new(Client(state.clone()), limit), state)
}
fn open_watch(id: &str) -> CoreLinkRequest { CoreLinkRequest::Watch(CoreLinkWatchRequest::Open(watch(id))) }
fn open_push(id: &str) -> CoreLinkRequest {
    CoreLinkRequest::Push(CoreLinkPushRequestMessage::Open(CorePushRequest::new(id, "test", "input")))
}
fn item(id: &str, sequence: u64, value: CoreValue) -> CoreLinkRequest {
    CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(CorePushItem { pushId: id.into(), sequence, args: value }))
}
fn assert_push_error(response: CoreLinkResponse, id: &str, code: &str) {
    match response {
        CoreLinkResponse::Push { pushId, result: Err(error) } => {
            assert_eq!(pushId, id); assert_eq!(error.code, code);
        },
        other => panic!("Expected push error: {other:?}"),
    }
}

#[tokio::test]
async fn wire_call_dispatches_standard_request() {
    let (mut session, _) = session(8);
    let request = CoreLinkRequest::Call(CoreCallRequest::new("call", "test", "echo", CoreValue::String("value".into())));
    let response = session.dispatch(decodeLink(&encodeLink(&request).unwrap()).unwrap()).await;
    let response: CoreLinkResponse = decodeLink(&encodeLink(&response).unwrap()).unwrap();
    assert!(matches!(response, CoreLinkResponse::Call(CoreCallResponse { requestId, result: Ok(CoreValue::String(value)) })
        if requestId.0 == "call" && value == "value"));
}

#[tokio::test]
async fn snapshot_errors_keep_request_identity_without_opening_watch() {
    let (mut session, _) = session(0);
    let mut request = watch("snapshot");
    request.target = "denied".into();
    let response = session.dispatch(CoreLinkRequest::Watch(CoreLinkWatchRequest::Snapshot(request))).await;
    assert!(matches!(response, CoreLinkResponse::Watch { requestId, result: Err(error) }
        if requestId.0 == "snapshot" && error.code == "DENIED"));
    assert!(!session.hasWatches());
    assert!(session.nextWatchEvent().await.is_none());
}

#[tokio::test]
async fn watch_duplicate_does_not_replace_live_stream_and_close_releases_it() {
    let (mut session, state) = session(8);
    assert!(matches!(session.dispatch(open_watch("w")).await,
        CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Opened), .. }));
    assert!(matches!(session.dispatch(open_watch("w")).await,
        CoreLinkResponse::Watch { result: Err(error), .. } if error.code == "LINK_DUPLICATE_WATCH"));
    state.events.lock().unwrap()["w"].send(event("w")).unwrap();
    assert!(matches!(session.nextWatchEvent().await,
        Some(CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Event(_)) }) if requestId.0 == "w"));
    for _ in 0..2 {
        session.dispatch(CoreLinkRequest::Watch(CoreLinkWatchRequest::Close { requestId: CoreRequestId::new("w") })).await;
    }
    assert_eq!(state.watchDrops.load(Ordering::SeqCst), 1);
    assert!(state.events.lock().unwrap()["w"].is_closed());
}

struct WakeFlag(AtomicUsize);
impl Wake for WakeFlag {
    fn wake(self: Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
}
#[tokio::test]
async fn cancelled_watch_wait_is_lossless_and_woken_by_source() {
    let (mut session, state) = session(8);
    session.dispatch(open_watch("w")).await;
    let flag = Arc::new(WakeFlag(AtomicUsize::new(0)));
    let waker = Waker::from(flag.clone());
    let mut cx = Context::from_waker(&waker);
    {
        let mut waiting = Box::pin(session.nextWatchEvent());
        assert!(waiting.as_mut().poll(&mut cx).is_pending());
    }
    state.events.lock().unwrap()["w"].send(event("w")).unwrap();
    assert_eq!(flag.0.load(Ordering::SeqCst), 1);
    assert!(matches!(session.nextWatchEvent().await,
        Some(CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Event(_)), .. })));
}

#[tokio::test]
async fn watches_are_fair_and_source_end_is_reported() {
    let (mut session, state) = session(8);
    for id in ["a", "b"] {
        session.dispatch(open_watch(id)).await;
        for _ in 0..2 { state.events.lock().unwrap()[id].send(event(id)).unwrap(); }
    }
    for expected in ["a", "b", "a", "b"] {
        assert!(matches!(session.nextWatchEvent().await,
            Some(CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Event(_)) }) if requestId.0 == expected));
    }
    state.events.lock().unwrap().clear();
    for _ in 0..2 {
        assert!(matches!(session.nextWatchEvent().await,
            Some(CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Closed), .. })));
    }
    assert!(!session.hasWatches());
    assert_eq!(state.watchDrops.load(Ordering::SeqCst), 2);
}

/// A delayed consumer receives the whole backlog in order, then waits for a new burst.
#[tokio::test]
async fn watch_batch_drains_all_ready_streams_beyond_the_cooperative_budget() {
    let (mut session, state) = session(8);
    for id in ["a", "b"] {
        session.dispatch(open_watch(id)).await;
        for value in 0..256 {
            let mut update = event(id);
            update.value = CoreValue::Signed(value);
            state.events.lock().unwrap()[id].send(update).unwrap();
        }
    }
    let batch = session.nextWatchBatch().await.unwrap();
    assert_eq!(batch.len(), 512);
    for id in ["a", "b"] {
        let values = batch.iter().filter_map(|response| match response {
            CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Event(event)) }
                if requestId.0 == id => Some(event.value.clone()),
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(values, (0..256).map(CoreValue::Signed).collect::<Vec<_>>());
    }
    let flag = Arc::new(WakeFlag(AtomicUsize::new(0)));
    let waker = Waker::from(flag.clone());
    let mut cx = Context::from_waker(&waker);
    assert!(Box::pin(session.nextWatchBatch()).as_mut().poll(&mut cx).is_pending());
    state.events.lock().unwrap()["a"].send(event("a")).unwrap();
    assert!(flag.0.load(Ordering::SeqCst) > 0);
    assert_eq!(session.nextWatchBatch().await.unwrap().len(), 1);
    state.events.lock().unwrap().clear();
    let closed = session.nextWatchBatch().await.unwrap();
    assert_eq!(closed.len(), 2);
    assert!(closed.iter().all(|response| matches!(response,
        CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Closed), .. })));
    assert!(!session.hasWatches());
}

#[tokio::test]
async fn push_is_ordered_and_close_finishes_service() {
    let (mut session, state) = session(8);
    assert!(matches!(session.dispatch(open_push("p")).await,
        CoreLinkResponse::Push { pushId, result: Ok(CoreLinkPushResponse::Opened) } if pushId == "p"));
    assert_push_error(session.dispatch(open_push("p")).await, "p", "LINK_DUPLICATE_PUSH");
    assert_push_error(session.dispatch(item("p", 1, CoreValue::Null)).await, "p", "LINK_PUSH_SEQUENCE");
    assert!(state.items.lock().unwrap().is_empty());
    assert!(matches!(session.dispatch(item("p", 0, CoreValue::Null)).await,
        CoreLinkResponse::Push { result: Ok(CoreLinkPushResponse::ItemAccepted { sequence: 0 }), .. }));
    assert_push_error(session.dispatch(item("p", 0, CoreValue::Null)).await, "p", "LINK_PUSH_SEQUENCE");
    let response = session.dispatch(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close { pushId: "p".into() })).await;
    assert!(matches!(response, CoreLinkResponse::Push { result: Ok(CoreLinkPushResponse::Closed), .. }));
    assert_eq!(state.pushCloses.load(Ordering::SeqCst), 1);
    assert_eq!(state.pushDrops.load(Ordering::SeqCst), 1);
    assert_eq!(state.items.lock().unwrap().len(), 1);
    assert_push_error(session.dispatch(item("p", 1, CoreValue::Null)).await, "p", "LINK_PUSH_NOT_FOUND");
}

#[tokio::test]
async fn failed_push_cannot_retry_side_effects() {
    let (mut session, state) = session(8);
    session.dispatch(open_push("p")).await;
    assert_push_error(session.dispatch(item("p", 0, CoreValue::String("fail".into()))).await, "p", "SERVICE_FAILED");
    assert_push_error(session.dispatch(item("p", 0, CoreValue::Null)).await, "p", "LINK_PUSH_NOT_FOUND");
    assert_eq!(state.pushDrops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn stream_limit_is_shared_and_disconnect_drops_resources() {
    let (mut session, state) = session(2);
    session.dispatch(open_watch("w")).await;
    session.dispatch(open_push("p")).await;
    assert_push_error(session.dispatch(open_push("other")).await, "other", "LINK_STREAM_LIMIT");
    assert!(matches!(session.dispatch(open_watch("other")).await,
        CoreLinkResponse::Watch { result: Err(error), .. } if error.code == "LINK_STREAM_LIMIT"));
    drop(session);
    assert_eq!(state.watchDrops.load(Ordering::SeqCst), 1);
    assert_eq!(state.pushDrops.load(Ordering::SeqCst), 1);
    // Disconnect aborts inputs; it does not pretend the caller completed them.
    assert_eq!(state.pushCloses.load(Ordering::SeqCst), 0);
}

/// Round-trips standard operations and rejects tags outside the Link operation families.
#[test]
fn protocol_has_only_three_operation_families() {
    for request in [open_watch("w"), open_push("p"), CoreLinkRequest::Call(CoreCallRequest::new("c", "t", "m", CoreValue::Null))] {
        assert_eq!(decodeLink::<CoreLinkRequest>(&encodeLink(&request).unwrap()).unwrap(), request);
    }
    for tag in ["Snapshot", "Open", "Item", "Close", "UnknownOperation"] {
        let value = serde_json::json!({"type": tag, "body": {}});
        assert!(decodeLink::<CoreLinkRequest>(&encodeLink(&value).unwrap()).is_err(), "{tag}");
    }
}

#[tokio::test]
async fn a_call_panic_returns_its_correlation_and_the_next_call_still_runs() {
    let (mut session, _) = session(8);
    let failed = session.dispatch(CoreLinkRequest::Call(CoreCallRequest::new("bad-call", "test", "panic", CoreValue::Null))).await;
    match failed {
        CoreLinkResponse::Call(CoreCallResponse { requestId, result: Err(error) }) => {
            assert_eq!(requestId.0, "bad-call");
            assert_eq!(error.code, "LINK_OPERATION_PANIC");
            assert!(error.message.contains("broken call after await"));
        },
        other => panic!("Unexpected response: {other:?}"),
    }
    let good = session.dispatch(CoreLinkRequest::Call(CoreCallRequest::new("next-call", "test", "echo", CoreValue::String("ok".into())))).await;
    assert!(matches!(good, CoreLinkResponse::Call(CoreCallResponse { result: Ok(CoreValue::String(value)), .. }) if value == "ok"));
}

#[tokio::test]
async fn a_push_panic_retires_only_that_stream_and_other_pushes_still_work() {
    let (mut session, _) = session(8);
    session.dispatch(open_push("broken")).await;
    session.dispatch(open_push("healthy")).await;
    assert_push_error(session.dispatch(item("broken", 0, CoreValue::String("panic".into()))).await, "broken", "LINK_OPERATION_PANIC");
    assert_push_error(session.dispatch(item("broken", 0, CoreValue::Null)).await, "broken", "LINK_PUSH_NOT_FOUND");
    assert!(matches!(session.dispatch(item("healthy", 0, CoreValue::Null)).await,
        CoreLinkResponse::Push { result: Ok(CoreLinkPushResponse::ItemAccepted { sequence: 0 }), .. }));
    assert!(matches!(session.dispatch(open_push("broken")).await,
        CoreLinkResponse::Push { result: Ok(CoreLinkPushResponse::Opened), .. }));
}

#[tokio::test]
async fn a_watch_mapping_panic_keeps_other_watches_and_allows_reopening() {
    let (mut session, state) = session(8);
    session.dispatch(open_watch("healthy")).await;
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<CoreEvent>();
    session.watches.insert(CoreRequestId::new("broken"), CoreEventStream::fromMappedReceiver(receiver, |_| panic!("broken watch value")));
    sender.send(event("broken")).unwrap();
    match session.nextWatchEvent().await.unwrap() {
        CoreLinkResponse::Watch { requestId, result: Err(error) } => {
            assert_eq!(requestId.0, "broken");
            assert_eq!(error.code, "LINK_OPERATION_PANIC");
            assert!(error.message.contains("broken watch value"));
        },
        other => panic!("Unexpected response: {other:?}"),
    }
    state.events.lock().unwrap()["healthy"].send(event("healthy")).unwrap();
    assert!(matches!(session.nextWatchEvent().await.unwrap(),
        CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Event(_)) } if requestId.0 == "healthy"));
    assert!(matches!(session.dispatch(open_watch("broken")).await,
        CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Opened), .. }));
}
