//! Peer operations use Host timers on both native executors and browser workers.
use operit_host_api::{HostError, HostResult, HostRuntimeTurnFuture};
use std::{future::Future, time::Duration};

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
    delay: HostRuntimeTurnFuture,
) -> HostResult<F::Output> {
    tokio::select! {
        biased;
        result = operation => Ok(result),
        result = delay => {
            result?;
            Err(HostError::timeout("Peer operation timed out"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::{pending, ready};

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
