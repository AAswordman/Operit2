use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

#[derive(Debug)]
pub struct RequestSemaphore {
    maxConcurrentRequests: i32,
    state: Mutex<i32>,
    condvar: Condvar,
}

impl RequestSemaphore {
    pub fn new(maxConcurrentRequests: i32) -> Self {
        Self {
            maxConcurrentRequests,
            state: Mutex::new(maxConcurrentRequests),
            condvar: Condvar::new(),
        }
    }

    pub fn acquire(&self) {
        let mut permits = self.state.lock().expect("RequestSemaphore mutex poisoned");
        while *permits <= 0 {
            permits = self
                .condvar
                .wait(permits)
                .expect("RequestSemaphore condvar wait failed");
        }
        *permits -= 1;
    }

    pub fn release(&self) {
        let mut permits = self.state.lock().expect("RequestSemaphore mutex poisoned");
        *permits = (*permits + 1).min(self.maxConcurrentRequests);
        self.condvar.notify_one();
    }

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
    pub fn getOrCreate(key: &str, maxConcurrentRequests: i32) -> Arc<RequestSemaphore> {
        assert!(maxConcurrentRequests > 0, "maxConcurrentRequests must be > 0");
        let map = SEMAPHORES.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = map.lock().expect("RequestConcurrencyRegistry mutex poisoned");
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
        guard.get(key).expect("semaphore must exist").semaphore.clone()
    }
}
