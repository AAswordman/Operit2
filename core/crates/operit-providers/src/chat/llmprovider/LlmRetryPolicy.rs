use std::time::Duration;

pub struct LlmRetryPolicy;

impl LlmRetryPolicy {
    pub const MAX_RETRY_ATTEMPTS: i32 = 5;
    const RETRY_BASE_DELAY_MS: i64 = 1000;

    /// Calculates the exponential retry delay for an attempt.
    #[allow(non_snake_case)]
    pub fn nextDelayMs(retryAttempt: i32) -> i64 {
        let normalizedAttempt = retryAttempt.max(1);
        Self::RETRY_BASE_DELAY_MS * (1_i64 << (normalizedAttempt - 1))
    }
}

/// Waits for the retry delay selected by the provider implementation policy.
pub async fn delay_retry_ms(retry_attempt: i32) {
    let delay_ms = LlmRetryPolicy::nextDelayMs(retry_attempt);
    tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
}
