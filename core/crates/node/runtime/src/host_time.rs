//! Peer operations use Host timers on both native executors and browser workers.
use operit_host_api::{HostError, HostResult, HostRuntimeTurnFuture};
use std::{future::Future, pin::Pin, task::{Context, Poll}, time::Duration};

pub(crate) async fn timeout<F: Future>(duration: Duration, operation: F) -> HostResult<F::Output> {
    let scheduler = operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost();
    withScheduler(scheduler.as_ref(), duration, operation).await
}

pub(crate) async fn withScheduler<F: Future>(
    scheduler: &dyn operit_host_api::HostRuntimeTaskSchedulerHost,
    duration: Duration,
    operation: F,
) -> HostResult<F::Output> {
    let delay = scheduler.waitForHostRuntimeDelay(
        duration
            .as_millis()
            .try_into()
            .map_err(|_| HostError::new("Peer timeout exceeds u64 milliseconds"))?,
    );
    timeoutWithDelay(operation, delay).await
}

async fn timeoutWithDelay<F: Future>(
    operation: F,
    mut delay: HostRuntimeTurnFuture,
) -> HostResult<F::Output> {
    let mut operation = std::pin::pin!(operation);
    std::future::poll_fn(|context| pollTimeout(operation.as_mut(), &mut delay, context)).await
}

// Erase only during a poll; storing a dyn Future across await loses F's Send.
#[inline(never)]
fn pollTimeout<T>(
    mut operation: Pin<&mut dyn Future<Output = T>>,
    delay: &mut HostRuntimeTurnFuture,
    context: &mut Context<'_>,
) -> Poll<HostResult<T>> {
    if let Poll::Ready(value) = operation.as_mut().poll(context) {
        return Poll::Ready(Ok(value));
    }
    match delay.as_mut().poll(context) {
        Poll::Ready(result) => Poll::Ready(
            result.and_then(|()| Err(HostError::timeout("Peer operation timed out")))),
        Poll::Pending => Poll::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::{pending, ready};

    #[test]
    fn movable_operation_keeps_timeout_movable() {
        fn assert_send(_: impl Send) {}
        assert_send(timeout(Duration::ZERO, ready(42)));
    }

    #[tokio::test]
    async fn completed_operation_wins_over_elapsed_deadline() {
        assert_eq!(
            timeoutWithDelay(ready(42), Box::pin(async { Ok(()) }))
                .await
                .unwrap(),
            42
        );
    }

    #[tokio::test]
    async fn host_delay_ends_stalled_operation() {
        assert!(
            timeoutWithDelay(pending::<()>(), Box::pin(async { Ok(()) }))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn host_timer_errors_are_preserved() {
        let error = timeoutWithDelay(
            pending::<()>(),
            Box::pin(async { Err(HostError::new("timer unavailable")) }),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("timer unavailable"));
    }
}
