pub struct LlmRetryPolicy;

impl LlmRetryPolicy {
    pub const MAX_RETRY_ATTEMPTS: i32 = 5;
    const RETRY_BASE_DELAY_MS: i64 = 1000;

    pub fn nextDelayMs(retryAttempt: i32) -> i64 {
        let normalizedAttempt = retryAttempt.max(1);
        Self::RETRY_BASE_DELAY_MS * (1_i64 << (normalizedAttempt - 1))
    }
}
