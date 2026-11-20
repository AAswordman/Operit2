use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};

use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostResult, HostRuntimeTaskSchedulerHost};
use tokio::sync::Notify;

use super::AIService::AiServiceError;

/// Bounds one model attempt until its first decoded content, reasoning or tool output.
#[derive(Default)]
pub(super) struct FirstOutputDeadline {
    output: Notify,
    received_output: AtomicBool,
}

impl FirstOutputDeadline {
    const TIMEOUT_MS: u64 = 120_000;

    /// Releases the deadline only after a provider decodes actual model output.
    pub(super) fn mark_output(&self) {
        if !self.received_output.swap(true, Ordering::AcqRel) {
            self.output.notify_one();
        }
    }

    /// Starts one Host-owned deadline covering request dispatch, headers and model decoding.
    pub(super) async fn run<T>(
        &self,
        operation: impl Future<Output = Result<T, AiServiceError>>,
    ) -> Result<T, AiServiceError> {
        self.run_with_delay(
            operation,
            defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(Self::TIMEOUT_MS),
        )
        .await
    }

    /// Drops the pending HTTP operation on expiry and the timer on first model output.
    async fn run_with_delay<T>(
        &self,
        operation: impl Future<Output = Result<T, AiServiceError>>,
        elapsed: impl Future<Output = HostResult<()>>,
    ) -> Result<T, AiServiceError> {
        tokio::pin!(operation);
        tokio::select! {
            biased;
            _ = self.output.notified() => operation.await,
            result = &mut operation => result,
            result = elapsed => {
                // Output decoded by this poll takes precedence over the still-ready timer.
                if self.received_output.load(Ordering::Acquire) {
                    return operation.await;
                }
                result.map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
                Err(AiServiceError::FirstOutputTimeout)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Poll;

    /// A first delta decoded in the timer's ready poll must release the deadline.
    #[tokio::test]
    async fn output_and_deadline_ready_together_preserve_the_response() {
        let deadline = FirstOutputDeadline::default();
        let mut polled = false;
        let operation = std::future::poll_fn(|_| {
            if polled {
                Poll::Ready(Ok("continued response"))
            } else {
                polled = true;
                deadline.mark_output();
                Poll::Pending
            }
        });
        assert_eq!(
            deadline
                .run_with_delay(operation, async { Ok(()) })
                .await
                .unwrap(),
            "continued response"
        );
    }

    /// A failed Host timer reports its own failure instead of inventing a model timeout.
    #[tokio::test]
    async fn host_timer_failure_is_not_a_first_output_timeout() {
        let result: Result<(), _> = FirstOutputDeadline::default()
            .run_with_delay(std::future::pending(), async {
                Err(operit_host_api::HostError::new("timer unavailable"))
            })
            .await;
        assert_eq!(
            result,
            Err(AiServiceError::RequestFailed("timer unavailable".into()))
        );
    }
}
