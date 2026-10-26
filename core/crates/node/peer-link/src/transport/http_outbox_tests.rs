//! Deterministic sending turns without wall-clock sleeps or real sockets.
use super::*;
use futures_util::poll;
use operit_host_api::{HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTurnFuture};
use std::task::Poll;
use tokio::sync::Semaphore;

struct Clock(Arc<Semaphore>);

impl Default for Clock {
    /// Starts with no elapsed sending turns.
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(0)))
    }
}

impl HostRuntimeTaskSchedulerHost for Clock {
    /// Supplies a fixed monotonic origin for this manually advanced test clock.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        Ok(0)
    }
    /// Rejects unrelated blocking tasks in the sending-turn tests.
    fn scheduleHostRuntimeTask(&self, _: &str, _: HostRuntimeTask) -> HostResult<()> {
        unreachable!()
    }
    /// Rejects unrelated asynchronous tasks in the sending-turn tests.
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, _: HostRuntimeAsyncTask) -> HostResult<()> {
        unreachable!()
    }
    /// Rejects unrelated cooperative tasks in the sending-turn tests.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        _: &str,
        _: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        unreachable!()
    }
    /// Rejects unrelated delayed tasks in the sending-turn tests.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        _: u64,
        _: HostRuntimeTask,
    ) -> HostResult<()> {
        unreachable!()
    }
    /// Rejects an unexpected executor-turn dependency.
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        unreachable!()
    }
    /// Releases exactly one sending turn when the test explicitly advances its clock.
    fn waitForHostRuntimeDelay(&self, delay: u64) -> HostRuntimeTurnFuture {
        assert_eq!(delay, FLUSH_INTERVAL_MS);
        let permit = self.0.clone().acquire_owned();
        Box::pin(async move {
            permit.await.unwrap().forget();
            Ok(())
        })
    }
}

/// An idle burst coalesces once; frames queued during I/O drain without another clock tick.
#[tokio::test]
async fn idle_burst_coalesces_and_in_flight_backlog_drains_without_another_tick() {
    let clock = Clock::default();
    let outbox = HttpOutbox::new(32, Inbox::new(28));
    let batch = outbox.nextBatch(&clock);
    tokio::pin!(batch);
    assert!(poll!(&mut batch).is_pending());
    outbox.write(b"A").await.unwrap();
    assert!(poll!(&mut batch).is_pending());
    outbox.write(b"BC").await.unwrap();
    assert!(poll!(&mut batch).is_pending());
    clock.0.add_permits(1);
    let in_flight = batch.await.unwrap().unwrap();
    assert_eq!(in_flight, b"ABC");
    outbox.write(b"D").await.unwrap();
    outbox.write(b"EF").await.unwrap();
    assert_eq!(outbox.nextBatch(&clock).await.unwrap().unwrap(), b"DEF");
    assert_eq!(clock.0.available_permits(), 0);
    assert_eq!(in_flight, b"ABC");
}

/// Byte pressure blocks a writer until a batch is detached, preserving whole-frame order.
#[tokio::test]
async fn pending_bytes_are_bounded_and_do_not_split_a_frame() {
    let clock = Clock::default();
    let outbox = HttpOutbox::new(8, Inbox::new(4));
    outbox.write(b"ABCDEF").await.unwrap();
    let write = outbox.write(b"GHI");
    tokio::pin!(write);
    assert!(poll!(&mut write).is_pending());
    assert_eq!(outbox.nextBatch(&clock).await.unwrap().unwrap(), b"ABCDEF");
    write.await.unwrap();
    assert_eq!(outbox.nextBatch(&clock).await.unwrap().unwrap(), b"GHI");
    assert_eq!(
        outbox.write(b"123456789").await.unwrap_err(),
        crate::MESSAGE_TOO_LARGE_ERROR
    );
}

/// A failed POST wakes a blocked writer and reader with the same first failure.
#[tokio::test]
async fn send_failure_releases_pending_io_without_retrying() {
    let inbox = Inbox::new(4);
    let outbox = HttpOutbox::new(8, inbox.clone());
    outbox.write(b"12345678").await.unwrap();
    let write = outbox.write(b"9");
    let read = inbox.read();
    tokio::pin!(write, read);
    assert!(poll!(&mut write).is_pending());
    assert!(poll!(&mut read).is_pending());
    outbox.finish(Err("POST rejected".into()));
    outbox.finish(Ok(()));
    assert_eq!(write.await.unwrap_err(), "POST rejected");
    assert_eq!(read.await.unwrap_err(), "POST rejected");
    assert!(outbox.nextBatch(&Clock::default()).await.unwrap().is_none());
}

/// Closing an idle or clock-waiting sender ends it without scheduling another turn.
#[tokio::test]
async fn close_wakes_idle_and_clock_waits() {
    for queued in [false, true] {
        let clock = Clock::default();
        let outbox = HttpOutbox::new(8, Inbox::new(4));
        let batch = outbox.nextBatch(&clock);
        tokio::pin!(batch);
        assert!(matches!(poll!(&mut batch), Poll::Pending));
        if queued {
            outbox.write(b"data").await.unwrap();
            assert!(matches!(poll!(&mut batch), Poll::Pending));
        }
        outbox.finish(Ok(()));
        assert!(batch.await.unwrap().is_none());
        assert!(outbox.write(b"late").await.is_err());
    }
}
