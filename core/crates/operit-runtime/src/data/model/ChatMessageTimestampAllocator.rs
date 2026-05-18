use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static LAST_ISSUED_TIMESTAMP: AtomicI64 = AtomicI64::new(0);

pub struct ChatMessageTimestampAllocator;

impl ChatMessageTimestampAllocator {
    pub fn next() -> i64 {
        Self::next_with_baseTimestamp(Self::currentTimeMillis())
    }

    pub fn next_with_baseTimestamp(baseTimestamp: i64) -> i64 {
        loop {
            let previous = LAST_ISSUED_TIMESTAMP.load(Ordering::SeqCst);
            let candidate = baseTimestamp.max(previous + 1);
            if LAST_ISSUED_TIMESTAMP
                .compare_exchange(previous, candidate, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return candidate;
            }
        }
    }

    pub fn observe(timestamp: i64) {
        loop {
            let previous = LAST_ISSUED_TIMESTAMP.load(Ordering::SeqCst);
            if timestamp <= previous {
                return;
            }
            if LAST_ISSUED_TIMESTAMP
                .compare_exchange(previous, timestamp, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return;
            }
        }
    }

    pub fn currentTimeMillis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time is before UNIX_EPOCH")
            .as_millis() as i64
    }
}
