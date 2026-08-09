use std::time::{Duration, Instant};

use crate::data::preferences::ApiPreferences::ApiPreferences;

/// Tracks the single total deadline shared by one ToolPkg pre-hook dispatch chain.
pub struct ToolPkgPreHookTimeout {
    deadline: Instant,
}

impl ToolPkgPreHookTimeout {
    /// Creates a deadline from the persisted ToolPkg pre-hook timeout preference.
    pub fn fromPreferences() -> Self {
        let seconds = ApiPreferences::getInstance()
            .getToolPkgPreHookTimeoutSeconds()
            .expect("ToolPkg pre-hook timeout preference must be readable");
        Self::fromSeconds(seconds)
    }

    /// Creates a deadline with the supplied whole-second duration.
    pub fn fromSeconds(seconds: i32) -> Self {
        let duration = Duration::from_secs(seconds.clamp(1, 60) as u64);
        Self {
            deadline: Instant::now() + duration,
        }
    }

    /// Returns the timeout milliseconds derived from the remaining shared deadline.
    #[allow(non_snake_case)]
    pub fn remainingTimeoutMillis(&self) -> Option<u64> {
        let remaining = self.deadline.checked_duration_since(Instant::now())?;
        let millis = u64::try_from(remaining.as_millis())
            .expect("ToolPkg pre-hook remaining timeout must fit into u64 milliseconds");
        Some(millis.max(1))
    }

    /// Identifies the timeout error emitted by the JavaScript hook executor.
    #[allow(non_snake_case)]
    pub fn isTimeoutError(error: &str) -> bool {
        error.to_ascii_lowercase().contains("timed out")
    }

    /// Reports whether the shared pre-hook deadline has elapsed.
    #[allow(non_snake_case)]
    pub fn hasExpired(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::ToolPkgPreHookTimeout;

    /// Verifies a shared deadline cannot be renewed between hook invocations.
    #[test]
    fn shared_deadline_expires_for_all_remaining_hooks() {
        let budget = ToolPkgPreHookTimeout::fromSeconds(1);
        assert!(budget.remainingTimeoutMillis().is_some());

        thread::sleep(Duration::from_millis(1100));

        assert!(budget.hasExpired());
        assert_eq!(budget.remainingTimeoutMillis(), None);
    }
}
