//! Direct cancellation events and one-shot deadlines for a V8 execution scope.
use operit_host_api::{
    HostJavaScriptExecutionInterrupt, HostJavaScriptInterruptController,
    HostJavaScriptInterruptSubscription, HostResult, HostRuntimeTaskSchedulerHost,
};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

#[derive(Default)]
struct Execution {
    generation: u64,
    tokens: Vec<HostJavaScriptExecutionInterrupt>,
    subscriptions: Vec<HostJavaScriptInterruptSubscription>,
}

pub(super) struct JavaScriptInterrupt {
    isolate: v8::IsolateHandle,
    execution: Mutex<Execution>,
    changed: tokio::sync::watch::Sender<Option<u128>>,
    owner: Weak<Self>,
    panic: Mutex<Option<Box<dyn std::any::Any + Send>>>,
}

impl JavaScriptInterrupt {
    pub(super) fn new(isolate: v8::IsolateHandle) -> HostResult<Arc<Self>> {
        let (changed, updates) = tokio::sync::watch::channel(None);
        let interrupt = Arc::new_cyclic(|owner| Self {
            isolate,
            execution: Mutex::new(Execution::default()),
            changed,
            owner: owner.clone(),
            panic: Mutex::new(None),
        });
        let owner = Arc::downgrade(&interrupt);
        crate::NativeHostRuntimeTaskSchedulerHost::new().scheduleHostRuntimeCooperativeAsyncTask(
            "operit-js-execution-deadline",
            Box::new(move || Box::pin(deadlines(owner, updates))),
        )?;
        Ok(interrupt)
    }

    /// Serializes delivery with scope replacement so a late event cannot stop the next call.
    fn terminate(&self, generation: u64) {
        let execution = self.execution.lock().expect("V8 execution scope poisoned");
        if execution.generation == generation {
            self.isolate.terminate_execution();
        }
    }

    pub(super) fn capture_panic(&self, panic: Box<dyn std::any::Any + Send>) {
        *self.panic.lock().expect("V8 panic state mutex poisoned") = Some(panic);
    }

    pub(super) fn take_panic(&self) -> Option<Box<dyn std::any::Any + Send>> {
        self.panic
            .lock()
            .expect("V8 panic state mutex poisoned")
            .take()
    }
}

impl HostJavaScriptInterruptController for JavaScriptInterrupt {
    fn replace(&self, tokens: Vec<HostJavaScriptExecutionInterrupt>) {
        let (generation, previous) = {
            let mut execution = self.execution.lock().expect("V8 execution scope poisoned");
            execution.generation += 1;
            execution.tokens = tokens.clone();
            self.isolate.cancel_terminate_execution();
            (
                execution.generation,
                std::mem::take(&mut execution.subscriptions),
            )
        };
        drop(previous);
        let subscriptions = tokens
            .iter()
            .map(|token| {
                let owner = self.owner.clone();
                token.onInterrupt(Arc::new(move || {
                    if let Some(owner) = owner.upgrade() {
                        owner.terminate(generation);
                    }
                }))
            })
            .collect();
        {
            let mut execution = self.execution.lock().expect("V8 execution scope poisoned");
            if execution.generation != generation {
                return;
            }
            execution.subscriptions = subscriptions;
        }
        // Past deadlines and pre-cancelled tokens are applied before entering JavaScript.
        let cancelled = tokens
            .iter()
            .any(HostJavaScriptExecutionInterrupt::shouldInterrupt);
        let deadline = if cancelled {
            None
        } else {
            tokens
                .iter()
                .filter_map(HostJavaScriptExecutionInterrupt::deadlineMillis)
                .min()
        };
        self.changed.send_if_modified(|current| {
            if *current == deadline {
                false
            } else {
                *current = deadline;
                true
            }
        });
    }
}

/// Sleeps until this scope changes or its exact deadline arrives; unlimited work has no timer.
async fn deadlines(
    owner: Weak<JavaScriptInterrupt>,
    mut updates: tokio::sync::watch::Receiver<Option<u128>>,
) {
    loop {
        let deadline = *updates.borrow_and_update();
        match deadline {
            None => {
                if updates.changed().await.is_err() {
                    return;
                }
            }
            Some(deadline) => {
                let remaining =
                    deadline.saturating_sub(operit_host_api::TimeUtils::currentTimeMillisU128());
                tokio::select! {
                    update = updates.changed() => if update.is_err() { return; },
                    _ = tokio::time::sleep(Duration::from_millis(remaining.min(u128::from(u64::MAX)) as u64)) => {
                        let Some(interrupt) = owner.upgrade() else { return; };
                        let tokens = interrupt.execution.lock().expect("V8 execution scope poisoned").tokens.clone();
                        drop(interrupt);
                        for token in &tokens { token.shouldInterrupt(); }
                        if updates.changed().await.is_err() { return; }
                    }
                }
            }
        }
    }
}
