use operit_host_api::{
    HostError, HostJavaScriptExecutionInterrupt, HostJavaScriptInterruptHandler,
    HostJavaScriptRuntime, HostJavaScriptRuntimeHost, HostJavaScriptRuntimeState,
    HostJavaScriptRuntimeStateAsyncTask, HostJavaScriptRuntimeStateFactory,
    HostJavaScriptRuntimeStateHandle, HostJavaScriptRuntimeStateOutput,
    HostJavaScriptRuntimeStateOutputFuture, HostJavaScriptRuntimeStateTask,
    HostJavaScriptStringCallback, HostJavaScriptVoidCallback, HostResult,
};
use quickjs_wasm_rs::{
    JSContextRef as QuickJsContext, JSValue as QuickJsValue, JSValueRef as QuickJsValueRef,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

thread_local! {
    static JAVASCRIPT_STATES: RefCell<BTreeMap<u64, HostJavaScriptRuntimeState>> = RefCell::new(BTreeMap::new());
    static JAVASCRIPT_STATE_LOCKS: RefCell<BTreeMap<u64, Rc<tokio::sync::Mutex<()>>>> = RefCell::new(BTreeMap::new());
    static DESTROYED_JAVASCRIPT_STATES: RefCell<BTreeSet<u64>> = RefCell::new(BTreeSet::new());
}

static NEXT_JAVASCRIPT_STATE_ID: AtomicU64 = AtomicU64::new(1);

struct WebHostJavaScriptRuntime {
    context: QuickJsContext,
    interruptHandler: Option<HostJavaScriptInterruptHandler>,
}

impl WebHostJavaScriptRuntime {
    /// Rejects work after the active execution interrupt has fired.
    fn ensureExecutionActive(&self) -> HostResult<()> {
        if self
            .interruptHandler
            .as_ref()
            .is_some_and(|handler| handler())
        {
            return Err(HostError::new("JavaScript execution was interrupted"));
        }
        Ok(())
    }

    /// Converts one QuickJS callback argument to its string representation.
    fn callbackArgument(args: &[QuickJsValueRef], index: usize) -> String {
        args[index].to_string()
    }
}

impl HostJavaScriptRuntime for WebHostJavaScriptRuntime {
    /// Evaluates one browser QuickJS script without reading its return value.
    fn evaluateHostJavaScriptVoid(&mut self, scriptName: &str, script: &str) -> HostResult<()> {
        self.ensureExecutionActive()?;
        self.context
            .eval_global(scriptName, script)
            .map(|_| ())
            .map_err(|error| HostError::new(error.to_string()))?;
        self.ensureExecutionActive()
    }

    /// Evaluates one browser QuickJS script and converts its return value to a string.
    fn evaluateHostJavaScriptString(
        &mut self,
        scriptName: &str,
        script: &str,
    ) -> HostResult<String> {
        self.ensureExecutionActive()?;
        let value = self
            .context
            .eval_global(scriptName, script)
            .map_err(|error| HostError::new(error.to_string()))?;
        self.ensureExecutionActive()?;
        Ok(value.to_string())
    }

    /// Executes every browser QuickJS job currently ready in this runtime.
    fn executePendingHostJavaScriptJobs(&mut self) -> HostResult<()> {
        self.ensureExecutionActive()?;
        self.context
            .execute_pending()
            .map_err(|error| HostError::new(error.to_string()))?;
        self.ensureExecutionActive()
    }

    /// Replaces the browser execution interrupt predicate.
    fn setHostJavaScriptInterruptHandler(
        &mut self,
        handler: Option<HostJavaScriptInterruptHandler>,
    ) -> HostResult<()> {
        self.interruptHandler = handler;
        Ok(())
    }

    /// Registers one browser QuickJS global function returning a string.
    fn registerHostJavaScriptStringFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptStringCallback,
    ) -> HostResult<()> {
        let function = self
            .context
            .wrap_callback(move |_, _, args| {
                let values = (0..args.len())
                    .map(|index| Self::callbackArgument(args, index))
                    .collect::<Vec<_>>();
                let output =
                    callback(values).map_err(|error| anyhow::anyhow!(error.to_string()))?;
                Ok(QuickJsValue::String(output))
            })
            .map_err(|error| HostError::new(error.to_string()))?;
        self.context
            .global_object()
            .map_err(|error| HostError::new(error.to_string()))?
            .set_property(name, function)
            .map_err(|error| HostError::new(error.to_string()))
    }

    /// Registers one browser QuickJS global function returning `undefined`.
    fn registerHostJavaScriptVoidFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptVoidCallback,
    ) -> HostResult<()> {
        let function = self
            .context
            .wrap_callback(move |_, _, args| {
                let values = (0..args.len())
                    .map(|index| Self::callbackArgument(args, index))
                    .collect::<Vec<_>>();
                callback(values).map_err(|error| anyhow::anyhow!(error.to_string()))?;
                Ok(QuickJsValue::Undefined)
            })
            .map_err(|error| HostError::new(error.to_string()))?;
        self.context
            .global_object()
            .map_err(|error| HostError::new(error.to_string()))?
            .set_property(name, function)
            .map_err(|error| HostError::new(error.to_string()))
    }
}

/// Owns browser QuickJS runtimes and their event-loop-affine state.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebHostJavaScriptRuntimeHost;

impl WebHostJavaScriptRuntimeHost {
    /// Creates the browser JavaScript runtime host.
    pub fn new() -> Self {
        Self
    }
}

impl HostJavaScriptRuntimeHost for WebHostJavaScriptRuntimeHost {
    /// Creates one browser QuickJS runtime on the current event-loop executor.
    fn createHostJavaScriptRuntime(&self) -> HostResult<Box<dyn HostJavaScriptRuntime>> {
        Ok(Box::new(WebHostJavaScriptRuntime {
            context: QuickJsContext::default(),
            interruptHandler: None,
        }))
    }

    /// Creates one browser-affine JavaScript state.
    fn createHostJavaScriptRuntimeState(
        &self,
        _taskName: &str,
        factory: HostJavaScriptRuntimeStateFactory,
    ) -> HostResult<HostJavaScriptRuntimeStateHandle> {
        let stateId = NEXT_JAVASCRIPT_STATE_ID.fetch_add(1, Ordering::Relaxed);
        let state = factory()?;
        JAVASCRIPT_STATES.with(|states| {
            states.borrow_mut().insert(stateId, state);
        });
        JAVASCRIPT_STATE_LOCKS.with(|locks| {
            locks
                .borrow_mut()
                .insert(stateId, Rc::new(tokio::sync::Mutex::new(())));
        });
        Ok(HostJavaScriptRuntimeStateHandle { id: stateId })
    }

    /// Executes one blocking operation against a browser-affine JavaScript state.
    fn executeHostJavaScriptRuntimeStateTask(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
        timeoutMillis: u64,
        task: HostJavaScriptRuntimeStateTask,
    ) -> HostResult<HostJavaScriptRuntimeStateOutput> {
        let interrupt = HostJavaScriptExecutionInterrupt::new(timeoutMillis)?;
        JAVASCRIPT_STATES.with(|states| {
            let mut states = states.borrow_mut();
            let state = states.get_mut(&handle.id).ok_or_else(|| {
                HostError::new(format!(
                    "JavaScript runtime state is unavailable: {}",
                    handle.id
                ))
            })?;
            task(state.as_mut(), interrupt)
        })
    }

    /// Executes one asynchronous operation against a browser-affine JavaScript state.
    fn executeHostJavaScriptRuntimeStateAsyncTask(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
        timeoutMillis: u64,
        task: HostJavaScriptRuntimeStateAsyncTask,
    ) -> HostJavaScriptRuntimeStateOutputFuture {
        struct JavaScriptStateLease {
            stateId: u64,
            state: Option<HostJavaScriptRuntimeState>,
        }

        impl Drop for JavaScriptStateLease {
            /// Restores the browser-affine state unless it was destroyed during execution.
            fn drop(&mut self) {
                let Some(state) = self.state.take() else {
                    return;
                };
                let destroyed = DESTROYED_JAVASCRIPT_STATES
                    .with(|destroyedStates| destroyedStates.borrow_mut().remove(&self.stateId));
                if !destroyed {
                    JAVASCRIPT_STATES.with(|states| {
                        states.borrow_mut().insert(self.stateId, state);
                    });
                }
            }
        }

        let stateLock = JAVASCRIPT_STATE_LOCKS.with(|locks| {
            locks.borrow().get(&handle.id).cloned().ok_or_else(|| {
                HostError::new(format!(
                    "JavaScript runtime state does not exist: {}",
                    handle.id
                ))
            })
        });
        Box::pin(async move {
            let stateLock = stateLock?;
            let _executionGuard = stateLock.lock().await;
            let destroyed = DESTROYED_JAVASCRIPT_STATES
                .with(|destroyedStates| destroyedStates.borrow().contains(&handle.id));
            if destroyed {
                return Err(HostError::new(format!(
                    "JavaScript runtime state was destroyed: {}",
                    handle.id
                )));
            }
            let state = JAVASCRIPT_STATES.with(|states| states.borrow_mut().remove(&handle.id));
            let state = state.ok_or_else(|| {
                HostError::new(format!(
                    "JavaScript runtime state is unavailable: {}",
                    handle.id
                ))
            })?;
            let mut lease = JavaScriptStateLease {
                stateId: handle.id,
                state: Some(state),
            };
            let interrupt = HostJavaScriptExecutionInterrupt::new(timeoutMillis)?;
            task(
                lease
                    .state
                    .as_mut()
                    .expect("JavaScript state lease must own its state")
                    .as_mut(),
                interrupt,
            )
            .await
        })
    }

    /// Destroys one browser-affine JavaScript state.
    fn destroyHostJavaScriptRuntimeState(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
    ) -> HostResult<()> {
        DESTROYED_JAVASCRIPT_STATES.with(|destroyedStates| {
            destroyedStates.borrow_mut().insert(handle.id);
        });
        JAVASCRIPT_STATES.with(|states| {
            states.borrow_mut().remove(&handle.id);
        });
        JAVASCRIPT_STATE_LOCKS.with(|locks| {
            locks.borrow_mut().remove(&handle.id);
        });
        Ok(())
    }
}
