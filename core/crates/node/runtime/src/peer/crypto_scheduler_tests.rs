use super::*;
use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use operit_peer_link::{PeerEndpoint, PeerTransport};

#[derive(Default)]
struct RecordingScheduler {
    tasks: StdMutex<Vec<(String, HostRuntimeAsyncTask)>>,
    reject: std::sync::atomic::AtomicBool,
}
impl HostRuntimeTaskSchedulerHost for RecordingScheduler {
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(100)
    }
    fn scheduleHostRuntimeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        if self.reject.load(Ordering::Relaxed) {
            return Err(HostError::new("scheduler unavailable"));
        }
        self.tasks.lock().unwrap().push((name.into(), task));
        Ok(())
    }
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.scheduleHostRuntimeAsyncTask(name, task)
    }
    fn scheduleHostRuntimeTask(&self, _: &str, _: HostRuntimeTask) -> HostResult<()> {
        unreachable!()
    }
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: HostRuntimeTask,
    ) -> HostResult<()> {
        unreachable!()
    }
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        unreachable!()
    }
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
        unreachable!()
    }
}

struct UnusedPeer(PeerEndpoint);
#[async_trait::async_trait]
impl PeerConnection for UnusedPeer {
    fn source(&self) -> &PeerEndpoint {
        &self.0
    }
    fn target(&self) -> &PeerEndpoint {
        &self.0
    }
    fn transport(&self) -> PeerTransport {
        PeerTransport::WebSocket
    }
    async fn send(&self, _: PeerMessage) -> Result<(), String> {
        unreachable!()
    }
    async fn receive(&self) -> Result<Option<PeerMessage>, String> {
        unreachable!()
    }
    async fn close(&self) {
        unreachable!()
    }
}

/// Creates a bidirectional channel whose Host tasks are recorded without running.
fn channel(scheduler: Arc<RecordingScheduler>) -> Arc<MultiplexedChannel> {
    let raw = Arc::new(UnusedPeer(PeerEndpoint {
        nodeId: "test".into(),
        address: "unused".into(),
    }));
    let encrypted = Channel::new(raw, &[7; 32], b"test", true).unwrap();
    let live = super::super::LiveChannel::start(encrypted, scheduler.clone(), || {}).unwrap();
    let side = super::super::dispatch::InboundSide::new(
        super::super::single_channel_tests::service(), "test".into(), "test".into(),
        super::super::dispatch::Authorization::Inbound);
    MultiplexedChannel::start(live, scheduler, side).unwrap()
}

/// No Tokio runtime exists on this thread: a dropped watch must use its own Host.
#[test]
fn watchCloseUsesTheOwningSchedulerWithoutAmbientTokio() {
    let scheduler = Arc::new(RecordingScheduler::default());
    let channel = channel(scheduler.clone());
    let (_sender, receiver) = mpsc::unbounded_channel();
    let stream =
        CoreEventStream::new(receiver).withOnClose(channel.watchCloseCallback("watch-1".into()));
    drop(stream);
    let names = scheduler
        .tasks
        .lock()
        .unwrap()
        .drain(..)
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "peer-channel-reader",
            "peer-multiplexed-reader",
            "peer-multiplexed-dispatch",
            "peer-multiplexed-watch-close"
        ]
    );
    assert!(!channel.isFailed());
}

#[test]
fn failedWatchCloseSchedulingIsReportedWithoutFallback() {
    let scheduler = Arc::new(RecordingScheduler::default());
    let channel = channel(scheduler.clone());
    scheduler.reject.store(true, Ordering::Relaxed);
    let (_sender, receiver) = mpsc::unbounded_channel();
    let stream =
        CoreEventStream::new(receiver).withOnClose(channel.watchCloseCallback("watch-1".into()));
    drop(stream);
    assert!(channel.isFailed());
    assert!(channel
        .failureOr("missing")
        .message
        .contains("scheduler unavailable"));
    scheduler.tasks.lock().unwrap().clear();
}


#[tokio::test]
async fn duplicate_correlation_does_not_cancel_the_original_request() {
    let channel = channel(Arc::new(RecordingScheduler::default()));
    let (sender, receiver) = oneshot::channel();
    channel.pending.lock().await.insert("call:same-id".into(), sender);
    let duplicate = channel.exchange(CoreLinkRequest::Call(CoreCallRequest::new(
        "same-id", "test", "read", CoreValue::Null,
    ))).await.unwrap_err();
    assert!(duplicate.message.contains("already in flight"));
    channel.dispatchResponse(CoreLinkResponse::Call(CoreCallResponse::ok(
        CoreRequestId("same-id".into()), CoreValue::Null,
    ))).await;
    assert!(matches!(receiver.await.unwrap().unwrap(), CoreLinkResponse::Call(CoreCallResponse { result: Ok(CoreValue::Null), .. })));
}

/// Records ciphertext while the first write deliberately holds the authenticated writer.
struct PausedPeer {
    endpoint: PeerEndpoint,
    sent: StdMutex<std::collections::VecDeque<PeerMessage>>,
    entered: tokio::sync::Notify,
    release: tokio::sync::Semaphore,
}
#[async_trait::async_trait]
impl PeerConnection for PausedPeer {
    /// Supplies the test carrier identity.
    fn source(&self) -> &PeerEndpoint { &self.endpoint }
    /// Supplies the test carrier destination.
    fn target(&self) -> &PeerEndpoint { &self.endpoint }
    /// Uses the same authenticated writer as a serial Host carrier.
    fn transport(&self) -> PeerTransport { PeerTransport::Serial }
    /// Pauses only the first frame so a competing operation must wait behind the burst.
    async fn send(&self, message: PeerMessage) -> Result<(), String> {
        let first = {
            let mut sent = self.sent.lock().unwrap();
            sent.push_back(message);
            sent.len() == 1
        };
        if first {
            self.entered.notify_one();
            self.release.acquire().await.unwrap().forget();
        }
        Ok(())
    }
    /// Replays the recorded ciphertext for verification by the peer's decrypting reader.
    async fn receive(&self) -> Result<Option<PeerMessage>, String> { Ok(self.sent.lock().unwrap().pop_front()) }
    /// Closes this in-memory carrier without further work.
    async fn close(&self) {}
}

/// Backpressure delays a whole burst; a concurrent call cannot split its ordered events.
#[tokio::test]
async fn delayed_watch_burst_is_sent_without_interleaving_a_concurrent_call() {
    let raw = Arc::new(PausedPeer {
        endpoint: PeerEndpoint { nodeId: "peer".into(), address: "memory".into() },
        sent: StdMutex::default(), entered: tokio::sync::Notify::new(), release: tokio::sync::Semaphore::new(0),
    });
    let sender = Channel::new(raw.clone(), &[7; 32], b"burst", true).unwrap();
    let receiver = Channel::new(raw.clone(), &[7; 32], b"burst", false).unwrap();
    let event = |id: &str| PeerMessage::Response(CoreLinkResponse::Watch {
        requestId: CoreRequestId::new("watch"), result: Ok(CoreLinkWatchResponse::Event(CoreEvent {
            requestId: None, target: "test".into(), propertyName: "state".into(),
            kind: CoreEventKind::Changed, value: CoreValue::String(id.into()),
        })),
    });
    let burst = sender.sendBatch(vec![event("first"), event("second")]);
    let competing = async {
        raw.entered.notified().await;
        let call = sender.send(PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new("call", "test", "read", CoreValue::Null))));
        tokio::pin!(call);
        assert!(futures_util::poll!(&mut call).is_pending());
        raw.release.add_permits(1);
        call.await.unwrap();
    };
    let (result, ()) = tokio::join!(burst, competing);
    result.unwrap();
    for expected in ["first", "second"] {
        assert!(matches!(receiver.receive().await.unwrap(), Some(PeerMessage::Response(CoreLinkResponse::Watch {
            result: Ok(CoreLinkWatchResponse::Event(CoreEvent { value: CoreValue::String(value), .. })), ..
        })) if value == expected));
    }
    assert!(matches!(receiver.receive().await.unwrap(), Some(PeerMessage::Request(CoreLinkRequest::Call(request))) if request.requestId.0 == "call"));
}
