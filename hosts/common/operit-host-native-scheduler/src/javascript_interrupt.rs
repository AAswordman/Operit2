//! Thread-safe cancellation for V8 execution, including loops without Host calls.
use operit_host_api::{HostError, HostJavaScriptInterruptHandler, HostResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

/// Serializes predicate replacement with termination requests from the shared monitor.
pub(super) struct JavaScriptInterrupt {
    isolate: v8::IsolateHandle,
    handler: Mutex<Option<HostJavaScriptInterruptHandler>>,
    queued: AtomicBool,
    panic: Mutex<Option<Box<dyn std::any::Any + Send>>>,
}

impl JavaScriptInterrupt {
    /// Registers an isolate with the process-wide cancellation monitor.
    pub(super) fn new(isolate: v8::IsolateHandle) -> HostResult<Arc<Self>> {
        static MONITOR: OnceLock<HostResult<mpsc::Sender<Weak<JavaScriptInterrupt>>>> =
            OnceLock::new();
        let sender = MONITOR
            .get_or_init(|| {
                let (sender, receiver) = mpsc::channel();
                std::thread::Builder::new()
                    .name("operit-v8-interrupts".into())
                    .spawn(move || monitor(receiver))
                    .map_err(|error| HostError::new(error.to_string()))?;
                Ok(sender)
            })
            .as_ref()
            .map_err(Clone::clone)?;
        let interrupt = Arc::new(Self {
            isolate,
            handler: Mutex::new(None),
            queued: AtomicBool::new(false),
            panic: Mutex::new(None),
        });
        sender
            .send(Arc::downgrade(&interrupt))
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(interrupt)
    }

    /// Replaces the active execution predicate after synchronizing with the monitor.
    pub(super) fn set(&self, handler: Option<HostJavaScriptInterruptHandler>) {
        let mut current = self
            .handler
            .lock()
            .expect("V8 interrupt handler mutex poisoned");
        *current = handler;
        self.isolate.cancel_terminate_execution();
    }

    /// Retains the original Rust panic until execution returns across the V8 ABI.
    pub(super) fn capture_panic(&self, panic: Box<dyn std::any::Any + Send>) {
        *self.panic.lock().expect("V8 panic state mutex poisoned") = Some(panic);
    }

    /// Takes a callback panic for resumption after the engine has stopped executing.
    pub(super) fn take_panic(&self) -> Option<Box<dyn std::any::Any + Send>> {
        self.panic
            .lock()
            .expect("V8 panic state mutex poisoned")
            .take()
    }

    /// Requests at most one safepoint check while an execution predicate is installed.
    fn check(&self) {
        let active = self
            .handler
            .lock()
            .expect("V8 interrupt handler mutex poisoned")
            .is_some();
        if active && !self.queued.swap(true, Ordering::AcqRel) {
            if !self
                .isolate
                .request_interrupt(check_on_owner, std::ptr::null_mut())
            {
                self.queued.store(false, Ordering::Release);
            }
        }
    }
}

/// Evaluates cancellation on the isolate owner so execution-local call state remains valid.
unsafe extern "C" fn check_on_owner(isolate: v8::UnsafeRawIsolatePtr, _: *mut std::ffi::c_void) {
    // SAFETY: V8 invokes this callback on the interrupted isolate's owning thread.
    let isolate = unsafe { v8::Isolate::ref_from_raw_isolate_ptr(&isolate) };
    let interrupt = isolate
        .get_slot::<Arc<JavaScriptInterrupt>>()
        .expect("V8 interrupt state");
    interrupt.queued.store(false, Ordering::Release);
    let handler = interrupt
        .handler
        .lock()
        .expect("V8 interrupt handler mutex poisoned");
    if let Some(handler) = handler.as_ref() {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler())) {
            Ok(true) => {
                isolate.terminate_execution();
            }
            Ok(false) => {}
            Err(panic) => {
                interrupt.capture_panic(panic);
                isolate.terminate_execution();
            }
        }
    }
}

/// Polls live isolates from one thread without retaining a runtime or its JS handles.
fn monitor(receiver: mpsc::Receiver<Weak<JavaScriptInterrupt>>) {
    let mut isolates: Vec<Weak<JavaScriptInterrupt>> = Vec::new();
    loop {
        let request = if isolates.is_empty() {
            match receiver.recv() {
                Ok(isolate) => Some(isolate),
                Err(_) => return,
            }
        } else {
            match receiver.recv_timeout(Duration::from_millis(2)) {
                Ok(isolate) => Some(isolate),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        };
        if let Some(isolate) = request {
            isolates.push(isolate);
        }
        isolates.retain(|isolate| match isolate.upgrade() {
            Some(isolate) => {
                isolate.check();
                true
            }
            None => false,
        });
    }
}
