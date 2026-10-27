//! Event-driven cancellation shared by JavaScript callers and platform engines.
use crate::{HostError, HostResult};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

struct Listeners {
    nextId: u64,
    callbacks: BTreeMap<u64, Arc<dyn Fn() + Send + Sync>>,
}

struct InterruptState {
    deadlineMillis: Option<u128>,
    cancelled: AtomicBool,
    timedOut: AtomicBool,
    listeners: Mutex<Listeners>,
}

/// Detaches one cancellation notification when its execution scope ends.
pub struct HostJavaScriptInterruptSubscription {
    state: Weak<InterruptState>,
    id: u64,
}

impl Drop for HostJavaScriptInterruptSubscription {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            state
                .listeners
                .lock()
                .expect("JavaScript interrupt listeners poisoned")
                .callbacks
                .remove(&self.id);
        }
    }
}

/// Controls cancellation and an optional absolute execution deadline.
#[derive(Clone)]
pub struct HostJavaScriptExecutionInterrupt {
    state: Arc<InterruptState>,
}

impl HostJavaScriptExecutionInterrupt {
    pub fn new(timeoutMillis: u64) -> HostResult<Self> {
        if timeoutMillis == 0 {
            return Err(HostError::new(
                "JavaScript execution timeout must be greater than zero",
            ));
        }
        let deadline = crate::TimeUtils::currentTimeMillisU128()
            .checked_add(u128::from(timeoutMillis))
            .ok_or_else(|| HostError::new("JavaScript execution deadline exceeds clock range"))?;
        Ok(Self::withDeadline(Some(deadline)))
    }

    pub fn unlimited() -> Self {
        Self::withDeadline(None)
    }

    fn withDeadline(deadlineMillis: Option<u128>) -> Self {
        Self {
            state: Arc::new(InterruptState {
                deadlineMillis,
                cancelled: AtomicBool::new(false),
                timedOut: AtomicBool::new(false),
                listeners: Mutex::new(Listeners {
                    nextId: 0,
                    callbacks: BTreeMap::new(),
                }),
            }),
        }
    }

    /// Notifies attached engines once, without a scanning thread or caller polling.
    pub fn interrupt(&self) {
        if self.state.cancelled.swap(true, Ordering::AcqRel) {
            return;
        }
        let callbacks = std::mem::take(
            &mut self
                .state
                .listeners
                .lock()
                .expect("JavaScript interrupt listeners poisoned")
                .callbacks,
        );
        for callback in callbacks.into_values() {
            callback();
        }
    }

    /// Subscribes atomically with cancellation; already cancelled tokens notify immediately.
    pub fn onInterrupt(
        &self,
        callback: Arc<dyn Fn() + Send + Sync>,
    ) -> HostJavaScriptInterruptSubscription {
        let mut listeners = self
            .state
            .listeners
            .lock()
            .expect("JavaScript interrupt listeners poisoned");
        let id = listeners.nextId;
        listeners.nextId += 1;
        if self.state.cancelled.load(Ordering::Acquire) {
            drop(listeners);
            callback();
        } else {
            listeners.callbacks.insert(id, callback);
        }
        HostJavaScriptInterruptSubscription {
            state: Arc::downgrade(&self.state),
            id,
        }
    }

    /// Exposes the single deadline for the platform's asynchronous timer.
    pub fn deadlineMillis(&self) -> Option<u128> {
        self.state.deadlineMillis
    }

    /// Reads terminal state, expiring the token when a deadline event has fired.
    pub fn shouldInterrupt(&self) -> bool {
        if self.state.cancelled.load(Ordering::Acquire) {
            return true;
        }
        if self
            .state
            .deadlineMillis
            .is_some_and(|deadline| crate::TimeUtils::currentTimeMillisU128() >= deadline)
        {
            self.state.timedOut.store(true, Ordering::Release);
            self.interrupt();
            return true;
        }
        false
    }

    pub fn didTimeOut(&self) -> bool {
        self.state.timedOut.load(Ordering::Acquire)
    }
}

/// Selects the lifetime and call signals currently executing in one JavaScript realm.
/// Replacing a scope detaches its old signals before the next call can run.
pub trait HostJavaScriptInterruptController: Send + Sync {
    fn replace(&self, interrupts: Vec<HostJavaScriptExecutionInterrupt>);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn cancellation_notifies_once_and_late_subscribers_are_not_lost() {
        let token = HostJavaScriptExecutionInterrupt::unlimited();
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let _subscription = token.onInterrupt(Arc::new(move || {
            count.fetch_add(1, Ordering::SeqCst);
        }));
        token.interrupt();
        token.interrupt();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let count = calls.clone();
        let _late = token.onInterrupt(Arc::new(move || {
            count.fetch_add(1, Ordering::SeqCst);
        }));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn leaving_an_execution_detaches_its_notification() {
        let token = HostJavaScriptExecutionInterrupt::unlimited();
        let subscription =
            token.onInterrupt(Arc::new(|| panic!("retired execution was interrupted")));
        drop(subscription);
        token.interrupt();
    }

    #[test]
    fn deadline_expiration_emits_the_same_terminal_notification() {
        let token = HostJavaScriptExecutionInterrupt::withDeadline(Some(0));
        let notified = Arc::new(AtomicBool::new(false));
        let signal = notified.clone();
        let _subscription =
            token.onInterrupt(Arc::new(move || signal.store(true, Ordering::Release)));
        assert!(token.shouldInterrupt());
        assert!(token.didTimeOut());
        assert!(notified.load(Ordering::Acquire));
    }
}
