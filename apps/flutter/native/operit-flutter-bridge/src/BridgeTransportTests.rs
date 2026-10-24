//! Exercises the production stream owner with controllable Core sources.

use super::*;
use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEventStream, CoreLinkPushSession, CoreValue,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

#[derive(Default)]
struct TestCore {
    watches: Mutex<HashMap<String, oneshot::Receiver<CoreEventStream>>>,
    pushes: Mutex<HashMap<String, Box<dyn CoreLinkPushSession>>>,
    opening: Arc<AtomicUsize>,
}

impl TestCore {
    /// Installs a source whose opening can be completed or cancelled by the test.
    fn watch_source(&self, id: &str) -> oneshot::Sender<CoreEventStream> {
        let (sender, receiver) = oneshot::channel();
        self.watches.lock().unwrap().insert(id.into(), receiver);
        sender
    }
}

struct Opening(Arc<AtomicUsize>);

impl Drop for Opening {
    /// Counts pending source opens released by either completion or cancellation.
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for TestCore {
    /// Rejects calls outside this stream-owner fixture.
    async fn call(&self, _: CoreCallRequest) -> CoreCallResponse {
        unreachable!()
    }
    /// Rejects snapshots outside this stream-owner fixture.
    async fn watchSnapshot(&self, _: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        unreachable!()
    }
    /// Waits for the test to provide a source while exposing cancellation ownership.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.opening.fetch_add(1, Ordering::SeqCst);
        let _opening = Opening(self.opening.clone());
        let receiver = self
            .watches
            .lock()
            .unwrap()
            .remove(&request.propertyName)
            .unwrap();
        receiver
            .await
            .map_err(|_| CoreLinkError::new("TEST_CLOSED", "Test source was closed"))
    }
    /// Transfers the installed push session to its stream-local FIFO owner.
    async fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Ok(self
            .pushes
            .lock()
            .unwrap()
            .remove(&request.requestId.0)
            .unwrap())
    }
}

struct TestPush {
    values: Arc<Mutex<Vec<CoreValue>>>,
    gate: Option<oneshot::Receiver<()>>,
}

#[async_trait]
impl CoreLinkPushSession for TestPush {
    /// Blocks only the first input when a gate was installed.
    async fn send(&mut self, value: CoreValue) -> Result<(), CoreLinkError> {
        if let Some(gate) = self.gate.take() {
            gate.await.unwrap();
        }
        self.values.lock().unwrap().push(value);
        Ok(())
    }
    /// Completes after all preceding items in this session have been accepted.
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
        Ok(())
    }
}

/// Creates one logical watch whose source key is independent of its generation.
fn open_watch(id: &str, source: &str, tag: i64) -> StreamAction {
    StreamAction::OpenWatch {
        id: id.into(),
        tag,
        request: CoreWatchRequest::new(id, "test", source, CoreValue::Null),
    }
}

/// Creates one sequenced push item with a recognizable payload.
fn item(id: &str, sequence: u64) -> StreamAction {
    StreamAction::PushItem(CorePushItem {
        pushId: id.into(),
        sequence,
        args: CoreValue::Unsigned(sequence),
    })
}

/// Constructs one event without adding timers or producer tasks.
fn event() -> CoreEvent {
    CoreEvent {
        requestId: None,
        target: "test".into(),
        propertyName: "value".into(),
        kind: CoreEventKind::Changed,
        value: CoreValue::Null,
    }
}

/// Drives one production owner alongside a bounded test scenario.
async fn drive(owner: StreamOwner, scenario: impl Future<Output = ()>) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(owner.run(), scenario);
    })
    .await
    .expect("Stream owner scenario stalled");
}

/// Verifies that a blocked push cannot serialize another push or an unrelated watch.
#[tokio::test]
async fn blocked_push_preserves_per_stream_order_without_stalling_other_streams() {
    let core = Arc::new(TestCore::default());
    let slow = Arc::new(Mutex::new(Vec::new()));
    let fast = Arc::new(Mutex::new(Vec::new()));
    let (release, gate) = oneshot::channel();
    core.pushes.lock().unwrap().insert(
        "slow".into(),
        Box::new(TestPush {
            values: slow.clone(),
            gate: Some(gate),
        }),
    );
    core.pushes.lock().unwrap().insert(
        "fast".into(),
        Box::new(TestPush {
            values: fast.clone(),
            gate: None,
        }),
    );
    let source = core.watch_source("source");
    let (sender, stream) = CoreEventStream::channel();
    source.send(stream).ok().unwrap();
    let (client, mut events, owner) = BridgeStreams::channel(core);
    drive(owner, async {
        client
            .request(StreamAction::OpenPush(CorePushRequest::new(
                "slow", "test", "input",
            )))
            .await
            .unwrap();
        client
            .request(StreamAction::OpenPush(CorePushRequest::new(
                "fast", "test", "input",
            )))
            .await
            .unwrap();
        let first = client.submit(item("slow", 0)).unwrap();
        let second = client.submit(item("slow", 1)).unwrap();
        client.request(item("fast", 0)).await.unwrap();
        client
            .request(open_watch("watch", "source", 1))
            .await
            .unwrap();
        sender.send(event()).unwrap();
        assert!(events.recv().await.unwrap().event.unwrap().is_some());
        assert!(slow.lock().unwrap().is_empty());
        assert_eq!(fast.lock().unwrap().len(), 1);
        release.send(()).unwrap();
        first.await.unwrap().unwrap();
        second.await.unwrap().unwrap();
        assert_eq!(
            *slow.lock().unwrap(),
            [CoreValue::Unsigned(0), CoreValue::Unsigned(1)]
        );
        let error = client.request(item("slow", 4)).await.unwrap_err();
        assert_eq!(error.code, "PUSH_SEQUENCE_MISMATCH");
        client.request(item("slow", 2)).await.unwrap();
        client.close();
    })
    .await;
}

/// Verifies immediate cancellation while Core has not yet returned a watch source.
#[tokio::test]
async fn cancellation_drops_an_inflight_source_open() {
    let core = Arc::new(TestCore::default());
    let source = core.watch_source("pending");
    let (client, mut events, owner) = BridgeStreams::channel(core.clone());
    drive(owner, async {
        let opening = client.submit(open_watch("watch", "pending", 7)).unwrap();
        while core.opening.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        client
            .request(StreamAction::CloseWatch {
                id: "watch".into(),
                tag: 7,
            })
            .await
            .unwrap();
        assert_eq!(opening.await.unwrap().unwrap_err().code, "WATCH_CLOSED");
        assert!(source.is_closed());
        assert_eq!(core.opening.load(Ordering::SeqCst), 0);
        assert!(events.recv().await.unwrap().event.unwrap().is_none());
        client.close();
    })
    .await;
}

/// Verifies that bounded delivery cannot retain a cancelled Core source.
#[tokio::test]
async fn cancellation_releases_a_source_even_with_a_full_outbox() {
    let core = Arc::new(TestCore::default());
    let source = core.watch_source("full");
    let (sender, stream) = CoreEventStream::channel();
    let dropped = Arc::new(AtomicUsize::new(0));
    let source_dropped = dropped.clone();
    source
        .send(stream.withOnClose(move || {
            source_dropped.fetch_add(1, Ordering::SeqCst);
        }))
        .ok()
        .unwrap();
    for _ in 0..WATCH_BATCH_CAPACITY + 2 {
        sender.send(event()).unwrap();
    }
    let (client, mut events, owner) = BridgeStreams::channel(core);
    drive(owner, async {
        client
            .request(open_watch("watch", "full", 9))
            .await
            .unwrap();
        while events.len() != WATCH_BATCH_CAPACITY {
            tokio::task::yield_now().await;
        }
        client
            .request(StreamAction::CloseWatch {
                id: "watch".into(),
                tag: 9,
            })
            .await
            .unwrap();
        while dropped.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        assert!(sender.is_closed());
        for _ in 0..WATCH_BATCH_CAPACITY {
            assert!(events.recv().await.unwrap().event.unwrap().is_some());
        }
        assert!(events.recv().await.unwrap().event.unwrap().is_none());
        client.close();
    })
    .await;
}

/// Verifies stale closes and old completion cannot cancel a reused watch id.
#[tokio::test]
async fn duplicate_and_stale_watch_generations_do_not_replace_the_live_source() {
    let core = Arc::new(TestCore::default());
    let first = core.watch_source("first");
    let second = core.watch_source("second");
    let (first_sender, first_stream) = CoreEventStream::channel();
    let (second_sender, second_stream) = CoreEventStream::channel();
    first.send(first_stream).ok().unwrap();
    second.send(second_stream).ok().unwrap();
    let (client, mut events, owner) = BridgeStreams::channel(core);
    drive(owner, async {
        client
            .request(open_watch("same", "first", 10))
            .await
            .unwrap();
        assert_eq!(
            client
                .request(open_watch("same", "unused", 11))
                .await
                .unwrap_err()
                .code,
            "WATCH_ALREADY_EXISTS"
        );
        client
            .request(StreamAction::CloseWatch {
                id: "same".into(),
                tag: 10,
            })
            .await
            .unwrap();
        client
            .request(open_watch("same", "second", 12))
            .await
            .unwrap();
        client
            .request(StreamAction::CloseWatch {
                id: "same".into(),
                tag: 10,
            })
            .await
            .unwrap();
        second_sender.send(event()).unwrap();
        loop {
            let delivery = events.recv().await.unwrap();
            if delivery.tag == 12 {
                assert!(delivery.event.unwrap().is_some());
                break;
            }
        }
        assert!(first_sender.is_closed());
        assert!(!second_sender.is_closed());
        client.close();
    })
    .await;
}

/// Verifies bounded ingress and connection shutdown resolve retained requests.
#[tokio::test]
async fn bounded_ingress_and_shutdown_release_queued_operations() {
    let core = Arc::new(TestCore::default());
    let (client, _events, owner) = BridgeStreams::channel(core);
    let mut replies = Vec::new();
    for index in 0..COMMAND_CAPACITY {
        replies.push(
            client
                .submit(StreamAction::CloseWatch {
                    id: index.to_string(),
                    tag: 1,
                })
                .unwrap(),
        );
    }
    let error = client
        .submit(StreamAction::CloseWatch {
            id: "overflow".into(),
            tag: 1,
        })
        .unwrap_err();
    assert_eq!(error.code, "STREAM_BACKPRESSURE");
    client.close();
    owner.run().await;
    for reply in replies {
        assert!(reply.await.is_err());
    }
    assert_eq!(
        client
            .request(StreamAction::ClosePush("closed".into()))
            .await
            .unwrap_err()
            .code,
        "STREAM_CLOSED"
    );
}
