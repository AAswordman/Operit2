//! Bounds online pairing guesses across sessions and sources.
//!
//! A six-digit code is only safe while an attacker gets a handful of online
//! attempts. Per-pending attempt counters alone are not enough: every new
//! session gets a new code, so an attacker could rotate sessions until one
//! guess lands. These buckets cap the total confirmation work and the rate at
//! which a single source can create new pairing transactions.
//!
//! The state is intentionally in memory. Restarting a CoreNode clears it, but
//! an attacker cannot restart the receiver, and persisting every guess would
//! add a disk-write amplification path to an unauthenticated endpoint.

use operit_peer_link::{PeerConnection, PeerTransport};
use std::{
    collections::BTreeMap,
    sync::Arc,
};

/// Failed or attempted confirmations allowed across every pending pairing.
const CONFIRMATION_GLOBAL_CAPACITY: u32 = 10;
/// One global confirmation token returns per hour. Pairing is rare, so a slow
/// sustained guess rate matters more than a large hourly budget.
const CONFIRMATION_GLOBAL_REFILL_MS: i64 = 60 * 60 * 1_000;
/// Confirmation attempts allowed from one source before it must wait.
const CONFIRMATION_SOURCE_CAPACITY: u32 = 5;
/// One per-source confirmation token returns every ten minutes.
const CONFIRMATION_SOURCE_REFILL_MS: i64 = 10 * 60 * 1_000;
/// Pairing transactions one source may create in a burst.
const CREATION_SOURCE_CAPACITY: u32 = 4;
/// One per-source creation token returns every minute.
const CREATION_SOURCE_REFILL_MS: i64 = 60 * 1_000;
/// Pairing transactions every source together may create in a burst.
const CREATION_GLOBAL_CAPACITY: u32 = 6;
/// One global creation token returns every minute.
const CREATION_GLOBAL_REFILL_MS: i64 = 60 * 1_000;
/// Caps memory under a distributed source rotation.
const MAX_SOURCE_BUCKETS: usize = 512;

/// A fixed-capacity token bucket that refills one token per interval.
struct TokenBucket {
    tokens: u32,
    capacity: u32,
    refillMs: i64,
    lastRefill: i64,
}

impl TokenBucket {
    fn new(capacity: u32, refillMs: i64, now: i64) -> Self {
        Self { tokens: capacity, capacity, refillMs: refillMs.max(1), lastRefill: now }
    }

    /// Refills from elapsed wall-clock time, tolerating a backwards clock.
    fn refill(&mut self, now: i64) {
        let elapsed = now.saturating_sub(self.lastRefill).max(0);
        if elapsed < self.refillMs {
            return;
        }
        let gained = (elapsed / self.refillMs).min(i64::from(u32::MAX)) as u32;
        self.tokens = self.tokens.saturating_add(gained).min(self.capacity);
        self.lastRefill = now.saturating_sub(elapsed % self.refillMs);
    }

    fn available(&mut self, now: i64) -> bool {
        self.refill(now);
        self.tokens > 0
    }

    fn take(&mut self, now: i64) {
        self.refill(now);
        self.tokens = self.tokens.saturating_sub(1);
    }
}

/// Counts pairing work without retaining per-address state forever.
#[derive(Default)]
pub(super) struct PairingThrottle {
    globalConfirmations: Option<TokenBucket>,
    sourceConfirmations: BTreeMap<String, TokenBucket>,
    globalCreations: Option<TokenBucket>,
    sourceCreations: BTreeMap<String, TokenBucket>,
}

impl PairingThrottle {
    /// Consumes one confirmation attempt when both budgets allow it.
    pub(super) fn allowConfirmation(&mut self, source: &str, now: i64) -> bool {
        let global = self
            .globalConfirmations
            .get_or_insert_with(|| {
                TokenBucket::new(CONFIRMATION_GLOBAL_CAPACITY, CONFIRMATION_GLOBAL_REFILL_MS, now)
            });
        let sourceBucket = self.sourceConfirmations.entry(source.into()).or_insert_with(|| {
            TokenBucket::new(CONFIRMATION_SOURCE_CAPACITY, CONFIRMATION_SOURCE_REFILL_MS, now)
        });
        // Check both before consuming either: a denied global budget must not
        // silently drain a legitimate source's allowance.
        if !sourceBucket.available(now) || !global.available(now) {
            return false;
        }
        sourceBucket.take(now);
        global.take(now);
        prune(&mut self.sourceConfirmations);
        true
    }

    /// Consumes one new-pairing attempt when both budgets allow it.
    pub(super) fn allowCreation(&mut self, source: &str, now: i64) -> bool {
        let global = self
            .globalCreations
            .get_or_insert_with(|| {
                TokenBucket::new(CREATION_GLOBAL_CAPACITY, CREATION_GLOBAL_REFILL_MS, now)
            });
        let sourceBucket = self.sourceCreations.entry(source.into()).or_insert_with(|| {
            TokenBucket::new(CREATION_SOURCE_CAPACITY, CREATION_SOURCE_REFILL_MS, now)
        });
        if !sourceBucket.available(now) || !global.available(now) {
            return false;
        }
        sourceBucket.take(now);
        global.take(now);
        prune(&mut self.sourceCreations);
        true
    }
}

/// Drops fully refilled buckets so a source-rotating attacker cannot grow memory.
fn prune(buckets: &mut BTreeMap<String, TokenBucket>) {
    if buckets.len() <= MAX_SOURCE_BUCKETS {
        return;
    }
    buckets.retain(|_, bucket| bucket.tokens < bucket.capacity);
    while buckets.len() > MAX_SOURCE_BUCKETS {
        let Some(key) = buckets.keys().next().cloned() else { break; };
        buckets.remove(&key);
    }
}

/// Identifies the observed pairing source, never a client-claimed identity.
///
/// Network transports expose the real peer address. Exclusive local carriers
/// (serial, Bluetooth) share one bucket per transport, which is enough because
/// an attacker on those carriers already needs physical proximity.
pub(super) fn pairingSource(raw: &Arc<dyn PeerConnection>) -> String {
    raw.remoteAddress()
        .map(|address| address.ip().to_string())
        .unwrap_or_else(|| match raw.transport() {
            PeerTransport::Serial => "serial".into(),
            PeerTransport::Bluetooth => "bluetooth".into(),
            transport => format!("{transport:?}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source exhausts its own bucket without draining the global one.
    #[test]
    fn source_budget_is_independent_of_the_global_budget() {
        let mut throttle = PairingThrottle::default();
        for _ in 0..CONFIRMATION_SOURCE_CAPACITY {
            assert!(throttle.allowConfirmation("192.0.2.1", 0));
        }
        assert!(!throttle.allowConfirmation("192.0.2.1", 0));
        // Another source still has its own allowance.
        assert!(throttle.allowConfirmation("192.0.2.2", 0));
    }

    /// Tokens return on the configured schedule and never exceed capacity.
    #[test]
    fn refill_restores_tokens_after_the_interval() {
        let mut throttle = PairingThrottle::default();
        for _ in 0..CONFIRMATION_SOURCE_CAPACITY {
            assert!(throttle.allowConfirmation("192.0.2.1", 0));
        }
        assert!(!throttle.allowConfirmation("192.0.2.1", CONFIRMATION_SOURCE_REFILL_MS - 1));
        assert!(throttle.allowConfirmation("192.0.2.1", CONFIRMATION_SOURCE_REFILL_MS));
        // The global bucket was also refilled; repeated long waits never exceed capacity.
        assert!(throttle.allowConfirmation("192.0.2.1", CONFIRMATION_SOURCE_REFILL_MS * 1_000));
    }

    /// Creation and confirmation budgets are separate, so pairing traffic cannot
    /// consume a source's confirmation allowance or the reverse.
    #[test]
    fn creation_and_confirmation_budgets_do_not_share_tokens() {
        let mut throttle = PairingThrottle::default();
        for _ in 0..CREATION_SOURCE_CAPACITY {
            assert!(throttle.allowCreation("192.0.2.1", 0));
        }
        assert!(!throttle.allowCreation("192.0.2.1", 0));
        assert!(throttle.allowConfirmation("192.0.2.1", 0));
    }

    /// A backwards clock cannot mint tokens.
    #[test]
    fn backwards_clock_does_not_refill() {
        let mut throttle = PairingThrottle::default();
        for _ in 0..CONFIRMATION_SOURCE_CAPACITY {
            assert!(throttle.allowConfirmation("192.0.2.1", 1_000));
        }
        assert!(!throttle.allowConfirmation("192.0.2.1", 0));
    }
}
