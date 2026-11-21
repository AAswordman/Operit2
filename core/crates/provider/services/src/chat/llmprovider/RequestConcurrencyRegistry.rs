use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::{Semaphore, SemaphorePermit};

#[derive(Debug)]
pub struct RequestSemaphore {
    maxConcurrentRequests: i32,
    permits: Semaphore,
}

impl RequestSemaphore {
    /// Creates an asynchronous request budget whose permits are returned on drop.
    pub fn new(maxConcurrentRequests: i32) -> Self {
        assert!(maxConcurrentRequests > 0, "maxConcurrentRequests must be > 0");
        Self {
            maxConcurrentRequests,
            permits: Semaphore::new(maxConcurrentRequests as usize),
        }
    }

    /// Yields while occupied and returns a permit owned by the awaiting request.
    pub async fn acquire(&self) -> SemaphorePermit<'_> {
        self.permits.acquire().await.expect("Request semaphore is never closed")
    }

    /// Returns the configured request budget.
    pub fn maxConcurrentRequests(&self) -> i32 {
        self.maxConcurrentRequests
    }
}

struct Entry {
    maxConcurrentRequests: i32,
    semaphore: Arc<RequestSemaphore>,
}

pub struct RequestConcurrencyRegistry;

static SEMAPHORES: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();

impl RequestConcurrencyRegistry {
    /// Shares a request budget for the same configured model and limit.
    pub fn getOrCreate(key: &str, maxConcurrentRequests: i32) -> Arc<RequestSemaphore> {
        assert!(
            maxConcurrentRequests > 0,
            "maxConcurrentRequests must be > 0"
        );
        let map = SEMAPHORES.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = map
            .lock()
            .expect("RequestConcurrencyRegistry mutex poisoned");
        let shouldCreate = guard
            .get(key)
            .map(|existing| existing.maxConcurrentRequests != maxConcurrentRequests)
            .unwrap_or(true);
        if shouldCreate {
            guard.insert(
                key.to_string(),
                Entry {
                    maxConcurrentRequests,
                    semaphore: Arc::new(RequestSemaphore::new(maxConcurrentRequests)),
                },
            );
        }
        guard
            .get(key)
            .expect("semaphore must exist")
            .semaphore
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::RequestSemaphore;

    /// Waiting requests yield, and dropping an acquired guard wakes the next request.
    #[tokio::test]
    async fn permits_yield_and_return_on_drop() {
        let semaphore = RequestSemaphore::new(1);
        let permit = semaphore.acquire().await;
        let mut waiting = Box::pin(semaphore.acquire());
        assert!(futures_util::poll!(&mut waiting).is_pending());
        drop(permit);
        let next = waiting.await;
        assert_eq!(semaphore.permits.available_permits(), 0);
        drop(next);
        assert_eq!(semaphore.permits.available_permits(), 1);
    }

    /// Cancelling an enqueued request neither consumes nor strands the remaining budget.
    #[tokio::test]
    async fn dropped_waiter_does_not_lose_a_permit() {
        let semaphore = RequestSemaphore::new(1);
        let permit = semaphore.acquire().await;
        let mut cancelled = Box::pin(semaphore.acquire());
        assert!(futures_util::poll!(&mut cancelled).is_pending());
        drop(cancelled);
        drop(permit);
        assert!(futures_util::poll!(Box::pin(semaphore.acquire())).is_ready());
        assert_eq!(semaphore.permits.available_permits(), 1);
    }
}
