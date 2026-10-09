use operit_host_api::{
    HostError, HostJavaScriptAsyncJsonCallback, HostJavaScriptExecutionInterrupt,
    HostJavaScriptInterruptHandler, HostJavaScriptJsonCallback, HostJavaScriptRuntime,
    HostJavaScriptRuntimeHost, HostJavaScriptRuntimeState, HostJavaScriptRuntimeStateAsyncTask,
    HostJavaScriptRuntimeStateFactory, HostJavaScriptRuntimeStateHandle,
    HostJavaScriptRuntimeStateOutput, HostJavaScriptRuntimeStateOutputFuture,
    HostJavaScriptRuntimeStateTask, HostResult,
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

    /// Converts an owned result using the shared bounded Web host value converter.
    fn callbackValue<'a>(
        &'a self,
        value: &serde_json::Value,
        depth: usize,
        nodes: &mut usize,
    ) -> anyhow::Result<QuickJsValueRef<'a>> {
        structuredResult(&self.context, value, depth, nodes)
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

    /// Calls an engine lifecycle function directly through the browser QuickJS API.
    fn callHostJavaScriptFunction(
        &mut self,
        name: &str,
        arguments: &[serde_json::Value],
    ) -> HostResult<serde_json::Value> {
        self.ensureExecutionActive()?;
        let global = self
            .context
            .global_object()
            .map_err(|e| HostError::new(e.to_string()))?;
        let function = global
            .get_property(name)
            .map_err(|e| HostError::new(e.to_string()))?;
        if !function.is_function() {
            return Err(HostError::new(format!(
                "JavaScript lifecycle function is unavailable: {name}"
            )));
        }
        let mut nodes = 0;
        let args = arguments
            .iter()
            .map(|value| self.callbackValue(value, 0, &mut nodes))
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(|e| HostError::new(e.to_string()))?;
        let receiver = self
            .context
            .undefined_value()
            .map_err(|e| HostError::new(e.to_string()))?;
        let result = function
            .call(&receiver, &args)
            .map_err(|e| HostError::new(e.to_string()))?;
        self.ensureExecutionActive()?;
        let registry = global
            .get_property("__operitHostPromiseRegistry")
            .map_err(|error| HostError::new(error.to_string()))?;
        let normalize = registry
            .get_property("snapshotResult")
            .map_err(|error| HostError::new(error.to_string()))?;
        let result = normalize
            .call(&registry, &[result])
            .map_err(|error| HostError::new(error.to_string()))?;
        structuredArgument(result, 0, &mut 0).map_err(|error| HostError::new(error.to_string()))
    }

    /// Installs a Web host Promise binding with Rust-owned structured request arguments.
    fn registerHostJavaScriptAsyncJsonFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptAsyncJsonCallback,
    ) -> HostResult<()> {
        self.ensureExecutionActive()?;
        let submit = self
            .context
            .wrap_callback(move |_, _, args| {
                let id = args
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("Host request id is missing"))?;
                let id = id.as_f64()?;
                if !id.is_finite() || id.fract() != 0.0 || id <= 0.0 || id > 9_007_199_254_740_991.0
                {
                    anyhow::bail!("Host request id must be a positive safe integer");
                }
                let mut nodes = 0;
                let values = args[1..]
                    .iter()
                    .map(|arg| structuredArgument(*arg, 0, &mut nodes))
                    .collect::<anyhow::Result<Vec<_>>>()?;
                callback(id as u64, values).map_err(|error| anyhow::anyhow!(error.to_string()))?;
                Ok(QuickJsValue::Undefined)
            })
            .map_err(|error| HostError::new(error.to_string()))?;
        let global = self
            .context
            .global_object()
            .map_err(|error| HostError::new(error.to_string()))?;
        let registry = global
            .get_property("__operitHostPromiseRegistry")
            .map_err(|error| HostError::new(error.to_string()))?;
        let factory = registry
            .get_property("binding")
            .map_err(|error| HostError::new(error.to_string()))?;
        let function = factory
            .call(&registry, &[submit])
            .map_err(|error| HostError::new(error.to_string()))?;
        global
            .set_property(name, function)
            .map_err(|error| HostError::new(error.to_string()))
    }

    /// Settles a scoped Promise without encoding values or evaluating callback source.
    fn settleHostJavaScriptPromise(
        &mut self,
        id: u64,
        value: &serde_json::Value,
        reject: bool,
    ) -> HostResult<()> {
        self.ensureExecutionActive()?;
        let mut nodes = 0;
        let (value, reject) = match self.callbackValue(value, 0, &mut nodes) {
            Ok(value) => (value, reject),
            Err(error) => (
                self.context
                    .value_from_str(&error.to_string())
                    .map_err(|error| HostError::new(error.to_string()))?,
                true,
            ),
        };
        let registry = self
            .context
            .global_object()
            .and_then(|global| global.get_property("__operitHostPromiseRegistry"))
            .map_err(|error| HostError::new(error.to_string()))?;
        let function = registry
            .get_property("settle")
            .map_err(|error| HostError::new(error.to_string()))?;
        let id = self
            .context
            .value_from_f64(id as f64)
            .map_err(|error| HostError::new(error.to_string()))?;
        let reject = self
            .context
            .value_from_bool(reject)
            .map_err(|error| HostError::new(error.to_string()))?;
        function
            .call(&registry, &[id, value, reject])
            .map_err(|error| HostError::new(error.to_string()))?;
        self.ensureExecutionActive()
    }

    /// Releases the Web host's Promise handles for one cancelled execution scope.
    fn cancelHostJavaScriptPromises(&mut self, scope: &str) -> HostResult<()> {
        let registry = self
            .context
            .global_object()
            .and_then(|global| global.get_property("__operitHostPromiseRegistry"))
            .map_err(|error| HostError::new(error.to_string()))?;
        let function = registry
            .get_property("cancel")
            .map_err(|error| HostError::new(error.to_string()))?;
        let scope = self
            .context
            .value_from_str(scope)
            .map_err(|error| HostError::new(error.to_string()))?;
        function
            .call(&registry, &[scope])
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(())
    }

    /// Installs a synchronous Web host binding with the same structured value contract.
    fn registerHostJavaScriptJsonFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptJsonCallback,
    ) -> HostResult<()> {
        self.ensureExecutionActive()?;
        let submit = self
            .context
            .wrap_callback(move |_, _, args| {
                let mut nodes = 0;
                let values = args
                    .iter()
                    .map(|arg| structuredArgument(*arg, 0, &mut nodes))
                    .collect::<anyhow::Result<Vec<_>>>()?;
                let result =
                    callback(values).map_err(|error| anyhow::anyhow!(error.to_string()))?;
                let mut nodes = 0;
                ownedStructuredResult(&result, 0, &mut nodes)
            })
            .map_err(|error| HostError::new(error.to_string()))?;
        let global = self
            .context
            .global_object()
            .map_err(|error| HostError::new(error.to_string()))?;
        let registry = global
            .get_property("__operitHostPromiseRegistry")
            .map_err(|error| HostError::new(error.to_string()))?;
        let factory = registry
            .get_property("syncBinding")
            .map_err(|error| HostError::new(error.to_string()))?;
        let function = factory
            .call(&registry, &[submit])
            .map_err(|error| HostError::new(error.to_string()))?;
        global
            .set_property(name, function)
            .map_err(|error| HostError::new(error.to_string()))
    }
}

/// Converts synchronous callback results to the Web engine's owned value ABI without JSON text.
fn ownedStructuredResult(
    value: &serde_json::Value,
    depth: usize,
    nodes: &mut usize,
) -> anyhow::Result<QuickJsValue> {
    *nodes += 1;
    if depth > 128 || *nodes > 1_000_000 {
        anyhow::bail!("Host result exceeds structured bridge depth/node limit");
    }
    Ok(match value {
        serde_json::Value::Null => QuickJsValue::Null,
        serde_json::Value::Bool(value) => QuickJsValue::Bool(*value),
        serde_json::Value::Number(value) => {
            QuickJsValue::Float(value.as_f64().ok_or_else(|| {
                anyhow::anyhow!("Host number cannot be represented in JavaScript")
            })?)
        }
        serde_json::Value::String(value) => QuickJsValue::String(value.clone()),
        serde_json::Value::Array(values) => QuickJsValue::Array(
            values
                .iter()
                .map(|value| ownedStructuredResult(value, depth + 1, nodes))
                .collect::<anyhow::Result<_>>()?,
        ),
        serde_json::Value::Object(values) => QuickJsValue::Object(
            values
                .iter()
                .map(|(key, value)| {
                    std::ffi::CString::new(key.as_str())?;
                    Ok((key.clone(), ownedStructuredResult(value, depth + 1, nodes)?))
                })
                .collect::<anyhow::Result<_>>()?,
        ),
    })
}

/// Creates safe, bounded structured results directly in the owning Web host context.
fn structuredResult<'a>(
    context: &'a QuickJsContext,
    value: &serde_json::Value,
    depth: usize,
    nodes: &mut usize,
) -> anyhow::Result<QuickJsValueRef<'a>> {
    *nodes += 1;
    if depth > 128 || *nodes > 1_000_000 {
        anyhow::bail!("JavaScript callback exceeds structured bridge depth/node limit");
    }
    match value {
        serde_json::Value::Null => context.null_value(),
        serde_json::Value::Bool(value) => context.value_from_bool(*value),
        serde_json::Value::Number(value) => {
            context.value_from_f64(value.as_f64().ok_or_else(|| {
                anyhow::anyhow!("Host number cannot be represented in JavaScript")
            })?)
        }
        serde_json::Value::String(value) => context.value_from_str(value),
        serde_json::Value::Array(values) => {
            let array = context.array_value()?;
            for (index, value) in values.iter().enumerate() {
                array.set_property(
                    index.to_string(),
                    structuredResult(context, value, depth + 1, nodes)?,
                )?;
            }
            Ok(array)
        }
        serde_json::Value::Object(values) => {
            let object = context.object_value()?;
            for (key, value) in values {
                object.set_property(
                    key.as_str(),
                    structuredResult(context, value, depth + 1, nodes)?,
                )?;
            }
            Ok(object)
        }
    }
}

/// Converts an already-normalized host snapshot into owned Rust values with bounded recursion.
fn structuredArgument(
    value: QuickJsValueRef<'_>,
    depth: usize,
    nodes: &mut usize,
) -> anyhow::Result<serde_json::Value> {
    use serde_json::{Number, Value};
    *nodes += 1;
    if depth > 128 || *nodes > 1_000_000 {
        anyhow::bail!("Host arguments exceed structured bridge depth/node limit");
    }
    if value.is_null() {
        return Ok(Value::Null);
    }
    if value.is_bool() {
        return Ok(Value::Bool(value.as_bool()?));
    }
    if value.is_str() {
        return Ok(Value::String(value.as_str_lossy().into_owned()));
    }
    if value.is_number() {
        let number = value.as_f64()?;
        if !number.is_finite() {
            anyhow::bail!("Host snapshot contains a non-finite number");
        }
        let number = if number.fract() == 0.0 {
            if number.abs() <= 9_007_199_254_740_991.0 {
                if number < 0.0 {
                    Number::from(number as i64)
                } else {
                    Number::from(number as u64)
                }
            } else {
                value.to_string().parse::<Number>()?
            }
        } else {
            Number::from_f64(number).ok_or_else(|| anyhow::anyhow!("Host number is invalid"))?
        };
        return Ok(Value::Number(number));
    }
    if value.is_array() {
        let length = value.get_property("length")?.as_f64()?;
        if length < 0.0 || length.fract() != 0.0 || length > (1_000_000 - *nodes) as f64 {
            anyhow::bail!("Host array exceeds structured bridge node limit");
        }
        let values = (0..length as u32)
            .map(|index| structuredArgument(value.get_indexed_property(index)?, depth + 1, nodes))
            .collect::<anyhow::Result<Vec<_>>>()?;
        return Ok(Value::Array(values));
    }
    if value.is_object() {
        let mut object = serde_json::Map::new();
        let mut properties = value.properties()?;
        while let Some(key) = properties.next_key()? {
            let key = key.to_string();
            object.insert(
                key.clone(),
                structuredArgument(value.get_property(key)?, depth + 1, nodes)?,
            );
        }
        return Ok(Value::Object(object));
    }
    anyhow::bail!("Host snapshot contains a non-JSON value")
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
        let context = QuickJsContext::default();
        context
            .eval_global("host-promises.js", include_str!("javascript_promises.js"))
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(WebHostJavaScriptRuntime {
            context,
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
