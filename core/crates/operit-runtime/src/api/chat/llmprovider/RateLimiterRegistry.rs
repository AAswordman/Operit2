use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::api::chat::llmprovider::SlidingWindowRateLimiter::SlidingWindowRateLimiter;

pub struct RateLimiterRegistry;

static LIMITERS: OnceLock<Mutex<HashMap<String, Arc<SlidingWindowRateLimiter>>>> = OnceLock::new();

impl RateLimiterRegistry {
    pub fn getOrCreate(key: &str, maxRequestsPerMinute: i32) -> Arc<SlidingWindowRateLimiter> {
        assert!(maxRequestsPerMinute > 0, "maxRequestsPerMinute must be > 0");
        let map = LIMITERS.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = map.lock().expect("RateLimiterRegistry mutex poisoned");
        let shouldCreate = guard
            .get(key)
            .map(|existing| existing.maxRequestsPerMinute != maxRequestsPerMinute)
            .unwrap_or(true);
        if shouldCreate {
            guard.insert(
                key.to_string(),
                Arc::new(SlidingWindowRateLimiter::new(maxRequestsPerMinute)),
            );
        }
        guard.get(key).expect("limiter must exist").clone()
    }
}
