use js_sys::{Array, Function, Object, Reflect};
use operit_host_api::{
    HostError, HostJavaScriptAsyncJsonCallback, HostJavaScriptExecutionInterrupt,
    HostJavaScriptInterruptController, HostJavaScriptInterruptSubscription, HostJavaScriptCallback, HostJavaScriptRuntime,
    HostJavaScriptValue, HostJavaScriptRuntimeHost, HostJavaScriptRuntimeState, HostJavaScriptRuntimeStateAsyncTask,
    HostJavaScriptRuntimeStateFactory, HostJavaScriptRuntimeStateHandle,
    HostJavaScriptRuntimeStateOutput, HostJavaScriptRuntimeStateOutputFuture,
    HostJavaScriptRuntimeStateTask, HostResult,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

thread_local! {
    static JAVASCRIPT_STATES: RefCell<BTreeMap<u64, HostJavaScriptRuntimeState>> = RefCell::new(BTreeMap::new());
    static JAVASCRIPT_STATE_LOCKS: RefCell<BTreeMap<u64, Rc<tokio::sync::Mutex<()>>>> = RefCell::new(BTreeMap::new());
    static DESTROYED_JAVASCRIPT_STATES: RefCell<BTreeSet<u64>> = RefCell::new(BTreeSet::new());
}

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

static NEXT_JAVASCRIPT_STATE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
enum WebJavaScriptBinding {
    Synchronous(HostJavaScriptCallback),
    Asynchronous(HostJavaScriptAsyncJsonCallback),
    Binary(operit_host_api::HostJavaScriptBinaryCallback),
}

#[derive(Default)]
struct WebExecution {
    generation: u64,
    subscriptions: Vec<HostJavaScriptInterruptSubscription>,
}

/// Cancellation and transport messages notify the same shared atomic wait word.
struct WebInterruptController {
    signal: [AtomicI32; 4],
    execution: Mutex<WebExecution>,
    owner: Weak<Self>,
}

impl WebInterruptController {
    fn new() -> Arc<Self> {
        Arc::new_cyclic(|owner| Self {
            signal: std::array::from_fn(|_| AtomicI32::new(0)),
            execution: Mutex::new(WebExecution::default()), owner: owner.clone(),
        })
    }

    fn signalView(&self) -> js_sys::Int32Array {
        let memory = wasm_bindgen::memory().unchecked_into::<js_sys::WebAssembly::Memory>();
        js_sys::Int32Array::new_with_byte_offset_and_length(
            &memory.buffer(), self.signal.as_ptr() as u32, 4,
        )
    }

    fn notify(&self) {
        self.signal[0].fetch_add(1, Ordering::AcqRel);
        let _ = js_sys::Atomics::notify(&self.signalView(), 0);
    }

    fn cancel(&self, generation: u64) {
        let execution = self.execution.lock().expect("Web JavaScript execution scope poisoned");
        if execution.generation == generation {
            self.signal[1].store(1, Ordering::Release);
            self.notify();
        }
    }
}

impl HostJavaScriptInterruptController for WebInterruptController {
    fn replace(&self, tokens: Vec<HostJavaScriptExecutionInterrupt>) {
        let deadline = tokens.iter().filter_map(HostJavaScriptExecutionInterrupt::deadlineMillis)
            .min().unwrap_or(0) as u64;
        let (generation, previous) = {
            let mut execution = self.execution.lock().expect("Web JavaScript execution scope poisoned");
            execution.generation += 1;
            self.signal[1].store(0, Ordering::Release);
            self.signal[2].store(deadline as i32, Ordering::Release);
            self.signal[3].store((deadline >> 32) as i32, Ordering::Release);
            (execution.generation, std::mem::take(&mut execution.subscriptions))
        };
        drop(previous);
        let subscriptions = tokens.iter().map(|token| {
            let owner = self.owner.clone();
            token.onInterrupt(Arc::new(move || {
                if let Some(owner) = owner.upgrade() { owner.cancel(generation); }
            }))
        }).collect();
        {
            let mut execution = self.execution.lock().expect("Web JavaScript execution scope poisoned");
            if execution.generation != generation { return; }
            execution.subscriptions = subscriptions;
        }
        for token in &tokens { token.shouldInterrupt(); }
        self.notify();
    }
}

struct WebHostJavaScriptRuntime {
    runtime: JsValue,
    bindings: Rc<RefCell<BTreeMap<String, WebJavaScriptBinding>>>,
    interrupts: Arc<WebInterruptController>,
}

impl WebHostJavaScriptRuntime {
    /// Executes one structured command while servicing callbacks on the owning Core executor.
    fn execute(&self, command: serde_json::Value) -> HostResult<JsValue> {
        let bindings = self.bindings.clone();
        let callback = Closure::wrap(Box::new(
            move |name: String, arguments: JsValue| -> Result<JsValue, JsValue> {
                let arguments = Array::from(&arguments).iter().map(|argument| {
                    if argument.is_instance_of::<js_sys::Uint8Array>() {
                        return Ok(HostJavaScriptValue::Buffer(js_sys::Uint8Array::new(&argument).to_vec().into()));
                    }
                    structuredArgument(&argument, 0, &mut 0).map(HostJavaScriptValue::Json)
                }).collect::<HostResult<Vec<_>>>().map_err(|error| JsValue::from_str(&error.to_string()))?;
                let binding = bindings.borrow().get(&name).cloned().ok_or_else(|| {
                    JsValue::from_str(&format!("JavaScript host binding is unavailable: {name}"))
                })?;
                let value = match binding {
                    WebJavaScriptBinding::Binary(callback) => {
                        let arguments = arguments.into_iter().map(HostJavaScriptValue::into_json)
                            .collect::<HostResult<Vec<_>>>().map_err(|error| JsValue::from_str(&error.to_string()))?;
                        let (metadata, bytes) = callback(arguments).map_err(|error| JsValue::from_str(&error.to_string()))?;
                        let result = Object::new();
                        let metadata = structuredValue(&metadata).map_err(|error| JsValue::from_str(&error.to_string()))?;
                        Reflect::set(&result, &JsValue::from_str("metadata"), &metadata)?;
                        Reflect::set(&result, &JsValue::from_str("bytes"), &js_sys::Uint8Array::from(bytes.as_slice()))?;
                        return Ok(result.into());
                    }
                    WebJavaScriptBinding::Synchronous(callback) => callback(arguments),
                    WebJavaScriptBinding::Asynchronous(callback) => {
                        let arguments = arguments.into_iter().map(HostJavaScriptValue::into_json).collect::<HostResult<Vec<_>>>()
                            .map_err(|error| JsValue::from_str(&error.to_string()))?;
                        let mut arguments = arguments.into_iter();
                        let id = arguments
                            .next()
                            .and_then(|id| id.as_u64())
                            .filter(|id| *id > 0 && *id <= 9_007_199_254_740_991)
                            .ok_or_else(|| {
                                JsValue::from_str("Host request id must be a positive safe integer")
                            })?;
                        callback(id, arguments.collect()).map(|()| serde_json::Value::Null)
                    }
                }
                .map_err(|error| JsValue::from_str(&error.to_string()))?;
                structuredValue(&value).map_err(|error| JsValue::from_str(&error.to_string()))
            },
        )
            as Box<dyn FnMut(String, JsValue) -> Result<JsValue, JsValue>>);
        let function = Reflect::get(&self.runtime, &JsValue::from_str("execute"))
            .map_err(crate::common::js_error)?
            .dyn_into::<Function>()
            .map_err(|_| HostError::new("Browser JavaScript runtime execute is unavailable"))?;
        function
            .call3(
                &self.runtime,
                &structuredValue(&command)?,
                callback.as_ref(),
                self.interrupts.signalView().as_ref(),
            )
            .map_err(crate::common::js_error)
    }
}

impl Drop for WebHostJavaScriptRuntime {
    /// Releases the dedicated browser execution realm when the host runtime is dropped.
    fn drop(&mut self) {
        if let Ok(function) = Reflect::get(&self.runtime, &JsValue::from_str("dispose")) {
            if let Some(function) = function.dyn_ref::<Function>() {
                let _ = function.call0(&self.runtime);
            }
        }
    }
}

impl HostJavaScriptRuntime for WebHostJavaScriptRuntime {
    fn registerHostJavaScriptBinaryFunction(&mut self, name: &str, callback: operit_host_api::HostJavaScriptBinaryCallback) -> HostResult<()> {
        self.execute(serde_json::json!({"operation":"binding","name":name,"factory":"syncBinding"}))?;
        self.bindings.borrow_mut().insert(name.to_string(), WebJavaScriptBinding::Binary(callback));
        Ok(())
    }

    /// Executes the shared bootstrap source in this browser runtime's independent realm.
    fn evaluateHostJavaScriptBootstrap(&mut self, source: &'static str) -> HostResult<()> {
        self.evaluateHostJavaScriptVoid("operit.bootstrap.js", source)
    }

    /// Installs a browser Function constructor with the fixed host module parameters.
    fn registerHostJavaScriptFunctionConstructor(
        &mut self,
        name: &str,
        parameters: &[&str],
    ) -> HostResult<()> {
        self.execute(
            serde_json::json!({"operation":"constructor","name":name,"parameters":parameters}),
        )
        .map(|_| ())
    }

    /// Evaluates one named script without retaining its return value.
    fn evaluateHostJavaScriptVoid(&mut self, scriptName: &str, script: &str) -> HostResult<()> {
        self.execute(
            serde_json::json!({"operation":"evaluateVoid","name":scriptName,"source":script}),
        )
        .map(|_| ())
    }

    /// Evaluates one named script using the browser engine's string conversion.
    fn evaluateHostJavaScriptString(
        &mut self,
        scriptName: &str,
        script: &str,
    ) -> HostResult<String> {
        let value = self.execute(
            serde_json::json!({"operation":"evaluateString","name":scriptName,"source":script}),
        )?;
        value
            .as_string()
            .ok_or_else(|| HostError::new("Browser JavaScript result is not a string"))
    }

    /// Waits for the browser to drain native Promise jobs before releasing the Core execution scope.
    fn executePendingHostJavaScriptJobs(&mut self) -> HostResult<()> {
        self.execute(serde_json::json!({"operation":"jobs"}))
            .map(|_| ())
    }

    /// Shares event notifications with the browser transport through Wasm memory.
    fn interruptController(&self) -> Arc<dyn HostJavaScriptInterruptController> {
        self.interrupts.clone()
    }

    /// Invokes a lifecycle function and reads its trusted structured snapshot.
    fn callHostJavaScriptFunction(
        &mut self,
        name: &str,
        arguments: &[serde_json::Value],
    ) -> HostResult<serde_json::Value> {
        let value =
            self.execute(serde_json::json!({"operation":"call","name":name,"args":arguments}))?;
        structuredArgument(&value, 0, &mut 0)
    }

    /// Registers one scoped asynchronous callback without retaining guest Promise handles in Rust.
    fn registerHostJavaScriptAsyncJsonFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptAsyncJsonCallback,
    ) -> HostResult<()> {
        self.execute(serde_json::json!({"operation":"binding","name":name,"factory":"binding"}))?;
        self.bindings.borrow_mut().insert(
            name.to_string(),
            WebJavaScriptBinding::Asynchronous(callback),
        );
        Ok(())
    }

    /// Settles a pending request directly in the browser realm that owns its Promise.
    fn settleHostJavaScriptPromise(
        &mut self,
        id: u64,
        value: serde_json::Value,
        reject: bool,
    ) -> HostResult<()> {
        self.execute(serde_json::json!({"operation":"settle","args":[id,value,reject]}))
            .map(|_| ())
    }

    /// Rejects the cancelled scope's pending Promises so JavaScript cleanup can run.
    fn cancelHostJavaScriptPromises(&mut self, scope: &str) -> HostResult<()> {
        self.execute(serde_json::json!({"operation":"cancel","args":[scope]}))
            .map(|_| ())
    }

    /// Registers a synchronous binding using the same structured snapshot contract.
    fn registerHostJavaScriptFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptCallback,
    ) -> HostResult<()> {
        self.execute(
            serde_json::json!({"operation":"binding","name":name,"factory":"syncBinding"}),
        )?;
        self.bindings.borrow_mut().insert(
            name.to_string(),
            WebJavaScriptBinding::Synchronous(callback),
        );
        Ok(())
    }
}

/// Materializes structured values without serializing the host bridge to JSON text.
fn structuredValue(value: &serde_json::Value) -> HostResult<JsValue> {
    structuredResult(value, 0, &mut 0)
}

/// Creates host-owned JavaScript values within the shared depth and node budget.
fn structuredResult(
    value: &serde_json::Value,
    depth: usize,
    nodes: &mut usize,
) -> HostResult<JsValue> {
    *nodes += 1;
    if depth > 128 || *nodes > 1_000_000 {
        return Err(HostError::new(
            "Host result exceeds structured bridge depth/node limit",
        ));
    }
    match value {
        serde_json::Value::Null => Ok(JsValue::NULL),
        serde_json::Value::Bool(value) => Ok(JsValue::from_bool(*value)),
        serde_json::Value::Number(value) => value
            .as_f64()
            .map(JsValue::from_f64)
            .ok_or_else(|| HostError::new("Host number cannot be represented in JavaScript")),
        serde_json::Value::String(value) => Ok(JsValue::from_str(value)),
        serde_json::Value::Array(values) => {
            let array = Array::new();
            for value in values {
                array.push(&structuredResult(value, depth + 1, nodes)?);
            }
            Ok(array.into())
        }
        serde_json::Value::Object(values) => {
            let object = Object::create(&Object::from(JsValue::NULL));
            for (key, value) in values {
                Reflect::set(
                    &object,
                    &JsValue::from_str(key),
                    &structuredResult(value, depth + 1, nodes)?,
                )
                .map_err(crate::common::js_error)?;
            }
            Ok(object.into())
        }
    }
}

/// Converts trusted snapshots into owned JSON values without losing safe integer types.
fn structuredArgument(
    value: &JsValue,
    depth: usize,
    nodes: &mut usize,
) -> HostResult<serde_json::Value> {
    use serde_json::{Number, Value};
    *nodes += 1;
    if depth > 128 || *nodes > 1_000_000 {
        return Err(HostError::new(
            "Host arguments exceed structured bridge depth/node limit",
        ));
    }
    if value.is_null() {
        return Ok(Value::Null);
    }
    if let Some(value) = value.as_bool() {
        return Ok(Value::Bool(value));
    }
    if let Some(value) = value.as_string() {
        return Ok(Value::String(value));
    }
    if let Some(number) = value.as_f64() {
        if !number.is_finite() {
            return Err(HostError::new("Host snapshot contains a non-finite number"));
        }
        let number = if number.fract() == 0.0 && number.abs() <= 9_007_199_254_740_991.0 {
            if number < 0.0 {
                Number::from(number as i64)
            } else {
                Number::from(number as u64)
            }
        } else {
            Number::from_f64(number).ok_or_else(|| HostError::new("Host number is invalid"))?
        };
        return Ok(Value::Number(number));
    }
    if Array::is_array(value) {
        let array = Array::from(value);
        if array.length() as usize > 1_000_000 - *nodes {
            return Err(HostError::new(
                "Host array exceeds structured bridge node limit",
            ));
        }
        let values = array
            .iter()
            .map(|value| structuredArgument(&value, depth + 1, nodes))
            .collect::<HostResult<Vec<_>>>()?;
        return Ok(Value::Array(values));
    }
    if value.is_object() {
        let mut object = serde_json::Map::new();
        for key in Object::keys(&Object::from(value.clone())).iter() {
            let name = key
                .as_string()
                .ok_or_else(|| HostError::new("Host object key must be a string"))?;
            let child = Reflect::get(value, &key).map_err(crate::common::js_error)?;
            object.insert(name, structuredArgument(&child, depth + 1, nodes)?);
        }
        return Ok(Value::Object(object));
    }
    Err(HostError::new("Host snapshot contains a non-JSON value"))
}

/// Owns browser JavaScript realms and their event-loop-affine state.
#[derive(Clone, Debug, Default)]
pub struct WebHostJavaScriptRuntimeHost;

impl WebHostJavaScriptRuntimeHost {
    /// Creates the browser JavaScript runtime host.
    pub fn new() -> Self {
        Self::default()
    }
}

impl HostJavaScriptRuntimeHost for WebHostJavaScriptRuntimeHost {
    /// Creates one browser engine realm from the Web worker host bridge.
    fn createHostJavaScriptRuntime(&self) -> HostResult<Box<dyn HostJavaScriptRuntime>> {
        let global = js_sys::global();
        let factory = Reflect::get(
            global.as_ref(),
            &JsValue::from_str("__operitCreateJavaScriptRuntime"),
        )
        .map_err(crate::common::js_error)?
        .dyn_into::<Function>()
        .map_err(|_| HostError::new("Browser JavaScript runtime factory is not installed"))?;
        let runtime = WebHostJavaScriptRuntime {
            runtime: factory
                .call0(&JsValue::UNDEFINED)
                .map_err(crate::common::js_error)?,
            bindings: Rc::new(RefCell::new(BTreeMap::new())),
            interrupts: WebInterruptController::new(),
        };
        runtime.execute(serde_json::json!({"operation":"initialize","source":include_str!("javascript_promises.js")}))?;
        Ok(Box::new(runtime))
    }

    /// Creates one browser-affine JavaScript state.
    fn createHostJavaScriptRuntimeState(
        &self,
        _taskName: &str,
        _kind: operit_host_api::HostJavaScriptRuntimeStateKind,
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
        interrupt: HostJavaScriptExecutionInterrupt,
        task: HostJavaScriptRuntimeStateTask,
    ) -> HostResult<HostJavaScriptRuntimeStateOutput> {
        let state = JAVASCRIPT_STATES
            .with(|states| states.borrow_mut().remove(&handle.id))
            .ok_or_else(|| {
                HostError::new(format!(
                    "JavaScript runtime state is unavailable: {}",
                    handle.id
                ))
            })?;
        let mut lease = JavaScriptStateLease {
            stateId: handle.id,
            state: Some(state),
        };
        task(
            lease
                .state
                .as_mut()
                .expect("JavaScript state lease must own its state")
                .as_mut(),
            interrupt,
        )
    }

    /// Posts one finite continuation through the browser event loop and the existing state lease.
    fn enqueueHostJavaScriptRuntimeStateTask(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
        interrupt: HostJavaScriptExecutionInterrupt,
        task: HostJavaScriptRuntimeStateTask,
    ) -> HostResult<()> {
        JAVASCRIPT_STATE_LOCKS.with(|locks| {
            if locks.borrow().contains_key(&handle.id) {
                Ok(())
            } else {
                Err(HostError::new(format!(
                    "JavaScript runtime state does not exist: {}",
                    handle.id
                )))
            }
        })?;
        let pending = self.executeHostJavaScriptRuntimeStateAsyncTask(
            handle,
            interrupt,
            Box::new(move |state, interrupt| Box::pin(std::future::ready(task(state, interrupt)))),
        );
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(error) = pending.await {
                operit_host_api::logHostError("JavaScriptRuntime", &error.to_string());
            }
        });
        Ok(())
    }

    /// Executes one asynchronous operation against a browser-affine JavaScript state.
    fn executeHostJavaScriptRuntimeStateAsyncTask(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
        interrupt: HostJavaScriptExecutionInterrupt,
        task: HostJavaScriptRuntimeStateAsyncTask,
    ) -> HostJavaScriptRuntimeStateOutputFuture {
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
