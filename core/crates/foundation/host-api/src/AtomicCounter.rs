//! Portable 64-bit counters for hosts without native 64-bit atomics.
use std::sync::atomic::Ordering;
use std::sync::Mutex;

#[cfg(target_has_atomic = "64")]
pub use std::sync::atomic::AtomicU64;
#[cfg(not(target_has_atomic = "64"))]
pub type AtomicU64 = LockedU64;

/// A mutex provides stronger ordering than requested and avoids narrowing
/// timestamps, byte counts or revisions to 32 bits on embedded hosts.
pub struct LockedU64(Mutex<u64>);
impl LockedU64 {
    pub const fn new(value: u64) -> Self {
        Self(Mutex::new(value))
    }
    pub fn load(&self, _ordering: Ordering) -> u64 {
        *self.0.lock().expect("counter lock poisoned")
    }
    pub fn fetch_add(&self, value: u64, _ordering: Ordering) -> u64 {
        let mut current = self.0.lock().expect("counter lock poisoned");
        let previous = *current;
        *current = previous.wrapping_add(value);
        previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fallback_preserves_values_above_32_bits_and_wraps() {
        let counter = LockedU64::new(u32::MAX as u64);
        assert_eq!(counter.fetch_add(2, Ordering::Relaxed), u32::MAX as u64);
        assert_eq!(counter.load(Ordering::Acquire), u32::MAX as u64 + 2);
        let counter = LockedU64::new(u64::MAX);
        assert_eq!(counter.fetch_add(1, Ordering::SeqCst), u64::MAX);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn fallback_serializes_concurrent_mutations() {
        let counter = std::sync::Arc::new(LockedU64::new(0));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let counter = counter.clone();
                std::thread::spawn(move || {
                    for _ in 0..1000 {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::SeqCst), 4000);
    }
}
