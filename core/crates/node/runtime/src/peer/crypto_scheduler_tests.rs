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

fn channel(scheduler: Arc<RecordingScheduler>) -> Arc<MultiplexedChannel> {
    let raw = Arc::new(UnusedPeer(PeerEndpoint {
        nodeId: "test".into(),
        address: "unused".into(),
    }));
    let encrypted = Channel::new(raw, &[7; 32], b"test", true).unwrap();
    let live = super::super::LiveChannel::start(encrypted, scheduler.clone(), || {}).unwrap();
    MultiplexedChannel::start(live, scheduler).unwrap()
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
