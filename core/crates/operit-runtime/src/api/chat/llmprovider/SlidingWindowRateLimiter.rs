use std::collections::VecDeque;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct SlidingWindowRateLimiter {
    pub maxRequestsPerMinute: i32,
    windowMs: i64,
    timestamps: Mutex<VecDeque<i64>>,
}

impl SlidingWindowRateLimiter {
    pub fn new(maxRequestsPerMinute: i32) -> Self {
        Self {
            maxRequestsPerMinute,
            windowMs: 60_000,
            timestamps: Mutex::new(VecDeque::new()),
        }
    }

    pub fn withWindow(maxRequestsPerMinute: i32, windowMs: i64) -> Self {
        Self {
            maxRequestsPerMinute,
            windowMs,
            timestamps: Mutex::new(VecDeque::new()),
        }
    }

    pub fn tryAcquire(&self, nowMs: i64) -> i64 {
        if self.maxRequestsPerMinute <= 0 {
            return 0;
        }

        let mut timestamps = self.timestamps.lock().expect("SlidingWindowRateLimiter mutex poisoned");
        while timestamps
            .front()
            .map(|oldest| nowMs - *oldest >= self.windowMs)
            .unwrap_or(false)
        {
            timestamps.pop_front();
        }

        if timestamps.len() >= self.maxRequestsPerMinute as usize {
            let oldest = *timestamps.front().expect("timestamps not empty");
            (self.windowMs - (nowMs - oldest)).max(1)
        } else {
            timestamps.push_back(nowMs);
            0
        }
    }

    pub fn acquire(&self) {
        loop {
            let retryAfterMs = self.tryAcquire(now_ms());
            if retryAfterMs <= 0 {
                return;
            }
            thread::sleep(Duration::from_millis(retryAfterMs as u64));
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
