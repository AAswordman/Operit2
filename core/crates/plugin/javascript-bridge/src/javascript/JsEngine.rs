use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use uuid::Uuid;

use crate::javascript::JsHostOperations;
use crate::javascript::JsJavaBridgeDelegates::{
    javaCallInstance, javaCallStatic, javaClassExists, javaGetApplicationContext, javaNewInstance,
};
use crate::javascript::JsLibraries::buildRuntimeBootstrapScript;
use operit_host_api::HostManager::{
    defaultHostJavaScriptRuntimeHost, defaultHostRuntimeTaskSchedulerHost,
};
use operit_host_api::TimeUtils::currentTimeMillisU128;
use operit_host_api::{
    HostError, HostErrorKind, HostJavaScriptAsyncJsonCallback, HostJavaScriptExecutionInterrupt,
    HostJavaScriptInterruptHandler, HostJavaScriptJsonCallback, HostJavaScriptRuntime,
    HostJavaScriptRuntimeHost, HostJavaScriptRuntimeStateHandle, HostJavaScriptRuntimeStateOutput,
    HostResult, HostJavaScriptValueCallback,
};
use operit_plugin_sdk::execution_result::{
    build_js_execution_error_payload as buildJsExecutionErrorPayload,
    extract_js_execution_error_message as extractJsExecutionErrorMessage, JsExecutionError,
    JsExecutionResult,
};
use operit_plugin_sdk::javascript::{
    JsExecutionCompletion, JsExecutionEngine, JsExecutionFuture, JsExecutionHost,
    JsToolNameResolutionRequest, JsToolPkgIpcRequest, JsToolPkgResourceRequest, JsToolPkgWasmArg,
    JsToolPkgWasmRequest, ToolPkgConfigScope, ToolPkgExecutionContext,
    ToolPkgMainRegistrationCapture, ToolPkgTextResourceHost,
};
use operit_plugin_sdk::toolpkg::ToolPkgApiRuntimeScript::buildToolPkgApiRuntimeScript;
use operit_plugin_sdk::toolpkg::ToolPkgComposeDslRuntimeScript::buildComposeDslRuntimeWrappedScript;
use operit_plugin_sdk::toolpkg::ToolPkgRegistrationBridge::buildToolPkgRegistrationBridgeScript;
use operit_util::stream::Stream::{CollectFuture, Stream};
use operit_util::AppLogger::AppLogger;

const TAG: &str = "OperitQuickJsEngine";
const TOOLPKG_SCRIPT_TIMEOUT_SECONDS: u64 = 60;
type ToolPkgTextResources = BTreeMap<String, String>;

#[allow(non_snake_case)]
pub trait JsExecutionListener {
    fn on_intermediate_result(&self, callId: &str, result: &str);
    fn on_failed(&self, callId: &str, reason: &str);
}

type JsExecutionListenerRef = Arc<dyn JsExecutionListener + Send + Sync>;

thread_local! {
    static CURRENT_ACTIVE_CALL_CONTEXTS: RefCell<BTreeMap<String, JsCallContext>> = RefCell::new(BTreeMap::new());
    static CURRENT_EXECUTION_HOST: RefCell<Option<Arc<dyn JsExecutionHost>>> = RefCell::new(None);
    static CURRENT_REGISTRATION_CONFIG_PARAMS: RefCell<Option<BTreeMap<String, Value>>> = RefCell::new(None);
    static CURRENT_INTERMEDIATE_CALLBACK: RefCell<Option<Arc<dyn Fn(Value) + Send + Sync>>> = RefCell::new(None);
    static CURRENT_EXECUTION_LISTENER: RefCell<Option<JsExecutionListenerRef>> = RefCell::new(None);
    static CURRENT_DETACHED_INTERMEDIATE_CALLBACKS: RefCell<BTreeMap<String, Arc<dyn Fn(Value) + Send + Sync>>> =
        RefCell::new(BTreeMap::new());
    static CURRENT_ENV_OVERRIDES: RefCell<BTreeMap<String, String>> = RefCell::new(BTreeMap::new());
    static CURRENT_CALL_RESULTS: RefCell<BTreeMap<String, JsExecutionResult<Value>>> = RefCell::new(BTreeMap::new());
    static CURRENT_TEXT_RESOURCE_SOURCE: RefCell<Option<JsTextResourceSource>> = RefCell::new(None);
}

#[derive(Clone)]
pub struct JsEngine {
    worker: JsEngineWorker,
}

#[derive(Clone)]
#[allow(non_snake_case)]
pub struct JsComposeDslActionEventStream {
    engine: JsEngine,
    actionId: String,
    payload: Option<Value>,
    runtimeOptions: BTreeMap<String, Value>,
    envOverrides: BTreeMap<String, String>,
}

#[derive(Clone)]
struct JsEngineWorker {
    runtimeHost: Arc<dyn HostJavaScriptRuntimeHost>,
    stateHandle: HostJavaScriptRuntimeStateHandle,
    alive: Arc<AtomicBool>,
    executionHost: Option<Arc<dyn JsExecutionHost>>,
}

enum JsAsyncCallback {
    Promise {
        requestId: u64,
        result: Value,
        reject: bool,
    },
    CancelScope {
        scope: String,
    },
}

type JsAsyncCallbackSink = Arc<dyn Fn(JsAsyncCallback) + Send + Sync>;
type JsBackgroundWake = Arc<dyn Fn() + Send + Sync>;

/// Identifies the authoritative module-resource owner for a single execution mode.
#[derive(Clone)]
enum JsTextResourceSource {
    Snapshot(Arc<ToolPkgTextResources>),
    Package(Arc<dyn ToolPkgTextResourceHost>),
    ExecutionHost,
}

#[derive(Clone)]
struct JsCallContext {
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    intermediateCallback: Option<Arc<dyn Fn(Value) + Send + Sync>>,
    executionListener: Option<JsExecutionListenerRef>,
    envOverrides: Arc<Mutex<BTreeMap<String, String>>>,
    textResourceSource: JsTextResourceSource,
}

struct JsPendingScriptExecution {
    callId: String,
    context: JsCallContext,
    deadlineMillis: u128,
    timeout: Duration,
    completion: JsScriptCompletion,
}

impl Drop for JsPendingScriptExecution {
    /// Clears the native call session whenever cooperative execution leaves scope.
    fn drop(&mut self) {
        clearNativeExecutionSession(&self.callId);
        completeScriptExecution(
            &self.completion,
            Err(JsExecutionError::worker_unavailable(
                "JavaScript execution session ended before completion",
            )),
        );
    }
}

/// Owns the result sink independently from the lifetime of detached script work.
type JsScriptCompletion =
    Arc<Mutex<Option<tokio::sync::oneshot::Sender<JsExecutionResult<Option<Value>>>>>>;

/// Completes a call once without holding the result lock while waking its caller.
fn completeScriptExecution(
    completion: &JsScriptCompletion,
    result: JsExecutionResult<Option<Value>>,
) {
    let sender = completion
        .lock()
        .expect("JavaScript completion mutex poisoned")
        .take();
    if let Some(sender) = sender {
        let _ = sender.send(result);
    }
}

/// Owns the arguments only until a call has been started on its affine JS thread.
struct JsScriptRequest {
    callId: String,
    script: String,
    functionName: String,
    params: BTreeMap<String, Value>,
    envOverrides: BTreeMap<String, String>,
    onIntermediateResult: Option<Arc<dyn Fn(Value) + Send + Sync>>,
    timeout: Duration,
    timeoutSec: u64,
    executionListener: Option<JsExecutionListenerRef>,
    textResources: Option<Arc<ToolPkgTextResources>>,
    useComposeDslTextResources: bool,
    completion: JsScriptCompletion,
    cancelled: Arc<AtomicBool>,
    deadlineMillis: u128,
}

enum JsScriptRequestStep {
    Start(JsScriptRequest),
    Cancel(String),
}

/// Cancels an abandoned call on its owning JS thread, never on the waiting caller's thread.
struct JsScriptRequestLease {
    worker: JsEngineWorker,
    callId: Option<String>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for JsScriptRequestLease {
    /// Cancels an unfinished request before its queued Start can evaluate any script.
    fn drop(&mut self) {
        let Some(callId) = self.callId.take() else {
            return;
        };
        self.worker.cancelScriptRequest(callId, &self.cancelled);
    }
}

struct JsEngineState {
    runtime: Box<dyn HostJavaScriptRuntime>,
    asyncCallbackSender: mpsc::Sender<JsAsyncCallback>,
    asyncCallbackReceiver: mpsc::Receiver<JsAsyncCallback>,
    backgroundWake: Arc<Mutex<Option<JsBackgroundWake>>>,
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    toolPkgContext: Option<ToolPkgExecutionContext>,
    composeDslTextResources: Option<Arc<ToolPkgTextResources>>,
    detachedCallContexts: BTreeMap<String, JsCallContext>,
    pendingScriptExecutions: BTreeMap<String, JsPendingScriptExecution>,
    jsEnvironmentInitialized: bool,
}

impl JsEngineWorker {
    /// Creates one host-owned JavaScript runtime state.
    fn new(
        executionHost: Option<Arc<dyn JsExecutionHost>>,
        toolPkgContext: Option<ToolPkgExecutionContext>,
    ) -> Self {
        let runtimeHost = defaultHostJavaScriptRuntimeHost();
        let runtimeHostForState = runtimeHost.clone();
        let workerExecutionHost = executionHost.clone();
        let backgroundWake = Arc::new(Mutex::new(None));
        let backgroundWakeForState = backgroundWake.clone();
        let stateHandle = runtimeHost
            .createHostJavaScriptRuntimeState(
                "OperitQuickJsEngine",
                Box::new(move || {
                    let runtime = runtimeHostForState.createHostJavaScriptRuntime()?;
                    let state = JsEngineState::newWithRuntime(
                        runtime,
                        executionHost,
                        toolPkgContext,
                        backgroundWakeForState,
                    )
                    .map_err(HostError::new)?;
                    Ok(Box::new(state))
                }),
            )
            .expect("JavaScript runtime state must be created by the Host");
        let scheduled = Arc::new(AtomicBool::new(false));
        let requested = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(AtomicBool::new(true));
        let wakeRuntimeHost = runtimeHost.clone();
        let wakeScheduler = defaultHostRuntimeTaskSchedulerHost();
        let wakeSlot = backgroundWake.clone();
        let wakeAlive = alive.clone();
        let wake: JsBackgroundWake = Arc::new(move || {
            if !wakeAlive.load(Ordering::Acquire) {
                return;
            }
            requested.store(true, Ordering::Release);
            if scheduled.swap(true, Ordering::AcqRel) {
                return;
            }
            let taskRuntimeHost = wakeRuntimeHost.clone();
            let taskScheduler = wakeScheduler.clone();
            let taskScheduled = scheduled.clone();
            let taskRequested = requested.clone();
            let taskWakeSlot = wakeSlot.clone();
            let taskAlive = wakeAlive.clone();
            let task: operit_host_api::HostRuntimeAsyncTask = Box::new(move || {
                Box::pin(async move {
                    if !taskAlive.load(Ordering::Acquire) {
                        return;
                    }
                    taskRequested.store(false, Ordering::Release);
                    let result = taskRuntimeHost
                        .executeHostJavaScriptRuntimeStateAsyncTask(
                            stateHandle,
                            60_000,
                            Box::new(move |state, interrupt| {
                                Box::pin(async move {
                                    let state =
                                        state.downcast_mut::<JsEngineState>().ok_or_else(|| {
                                            HostError::new(
                                                "JavaScript runtime state type does not match",
                                            )
                                        })?;
                                    state
                                        .advanceJavaScriptExecutions(interrupt)
                                        .map_err(|error| HostError::new(error.to_string()))?;
                                    Ok(Box::new(()) as HostJavaScriptRuntimeStateOutput)
                                })
                            }),
                        )
                        .await;
                    if let Err(error) = result {
                        if !taskAlive.load(Ordering::Acquire) {
                            return;
                        }
                        AppLogger::e(
                            TAG,
                            &format!("detached JavaScript execution failed: {error}"),
                        );
                    }
                    taskScheduled.store(false, Ordering::Release);
                    if taskRequested.swap(false, Ordering::AcqRel) {
                        if let Some(wake) = taskWakeSlot
                            .lock()
                            .expect("background wake mutex poisoned")
                            .clone()
                        {
                            wake();
                        }
                    }
                })
            });
            if let Err(error) =
                taskScheduler.scheduleHostRuntimeCooperativeAsyncTask("operit-js-detached", task)
            {
                scheduled.store(false, Ordering::Release);
                AppLogger::e(
                    TAG,
                    &format!("schedule detached JavaScript execution failed: {error}"),
                );
            }
        });
        *backgroundWake
            .lock()
            .expect("background wake mutex poisoned") = Some(wake);
        Self {
            runtimeHost,
            stateHandle,
            alive,
            executionHost: workerExecutionHost,
        }
    }

    /// Cancels a submitted call independently of whether its completion future is polled.
    #[allow(non_snake_case)]
    fn cancelScriptRequest(&self, callId: String, cancelled: &AtomicBool) {
        cancelled.store(true, Ordering::Release);
        let worker = self.clone();
        let result = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "operit-js-cancel",
            Box::new(move || {
                Box::pin(async move {
                    if worker.alive.load(Ordering::Acquire) {
                        let _ = worker
                            .executeScriptStepAsync(JsScriptRequestStep::Cancel(callId), 60_000, 60)
                            .await;
                    }
                })
            }),
        );
        if let Err(error) = result {
            AppLogger::e(
                TAG,
                &format!("schedule JavaScript cancellation failed: {error}"),
            );
        }
    }

    /// The async state task is deliberately one short step, not the lifetime of a JS call.
    #[allow(non_snake_case)]
    async fn executeScriptStepAsync(
        &self,
        step: JsScriptRequestStep,
        timeoutMillis: u64,
        timeoutSec: u64,
    ) -> JsExecutionResult<()> {
        let task = scriptRequestStepTask(step, timeoutSec);
        let output = self
            .runtimeHost
            .executeHostJavaScriptRuntimeStateAsyncTask(
                self.stateHandle,
                timeoutMillis,
                Box::new(move |state, interrupt| Box::pin(async move { task(state, interrupt) })),
            )
            .await
            .map_err(javaScriptExecutionErrorFromHost)?;
        downcastJavaScriptStateOutput(output)
    }

    /// Submits an owned request to the Host and awaits only its terminal completion event.
    #[allow(non_snake_case)]
    fn execute_script_function_async(
        &self,
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(Value) + Send + Sync>>,
        _dispatchIntermediateOnMain: bool,
        timeout: Duration,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        useComposeDslTextResources: bool,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<Value>>> {
        let worker = self.clone();
        Box::pin(async move {
            if !worker.alive.load(Ordering::Acquire) {
                return Err(JsExecutionError::worker_unavailable(
                    "JavaScript runtime was destroyed",
                ));
            }
            let deadlineMillis = scriptRequestDeadline(timeout)?;
            let callId = format!("operit_call_{}", Uuid::new_v4().simple());
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let completion = Arc::new(Mutex::new(Some(sender)));
            let cancelled = Arc::new(AtomicBool::new(false));
            let mut lease = JsScriptRequestLease {
                worker: worker.clone(),
                callId: Some(callId.clone()),
                cancelled: cancelled.clone(),
            };
            let request = JsScriptRequest {
                callId,
                script,
                functionName,
                params,
                envOverrides,
                onIntermediateResult: on_intermediate_result,
                timeout,
                timeoutSec,
                executionListener,
                textResources,
                useComposeDslTextResources,
                completion: completion.clone(),
                cancelled: cancelled.clone(),
                deadlineMillis,
            };
            // A deadline is one Host event, not a recurring polling interval.
            let weakCompletion = Arc::downgrade(&completion);
            let deadlineWorker = worker.clone();
            let deadlineCallId = request.callId.clone();
            defaultHostRuntimeTaskSchedulerHost()
                .scheduleDelayedHostRuntimeTask(
                    "operit-js-deadline",
                    remainingScriptRequestMillis(deadlineMillis, timeoutSec)?,
                    Box::new(move || {
                        if let Some(completion) = weakCompletion.upgrade() {
                            let sender = completion
                                .lock()
                                .expect("JavaScript completion mutex poisoned")
                                .take();
                            if let Some(sender) = sender {
                                deadlineWorker.cancelScriptRequest(deadlineCallId, &cancelled);
                                let _ = sender.send(Err(JsExecutionError::timeout(format!(
                                    "Script execution timed out after {} milliseconds",
                                    timeout.as_millis(),
                                ))));
                            }
                        }
                    }),
                )
                .map_err(javaScriptExecutionErrorFromHost)?;
            let completionForStart = completion.clone();
            defaultHostRuntimeTaskSchedulerHost()
                .scheduleHostRuntimeAsyncTask(
                    "operit-js-request",
                    Box::new(move || {
                        Box::pin(async move {
                            let result = async {
                                if !worker.alive.load(Ordering::Acquire) {
                                    return Err(JsExecutionError::worker_unavailable(
                                        "JavaScript runtime was destroyed",
                                    ));
                                }
                                worker
                                    .executeScriptStepAsync(
                                        JsScriptRequestStep::Start(request),
                                        remainingScriptRequestMillis(deadlineMillis, timeoutSec)?,
                                        timeoutSec,
                                    )
                                    .await
                            }
                            .await;
                            if let Err(error) = result {
                                completeScriptExecution(&completionForStart, Err(error));
                            }
                        })
                    }),
                )
                .map_err(javaScriptExecutionErrorFromHost)?;
            let result = receiver.await.map_err(|_| {
                JsExecutionError::worker_unavailable(
                    "JavaScript execution completion channel disconnected",
                )
            })?;
            if result.is_ok() {
                lease.callId = None;
            }
            result
        })
    }

    /// Executes one ToolPkg registration request through the host-owned state executor.
    #[allow(non_snake_case)]
    fn execute_toolpkg_main_registration_function(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        timeoutSec: u64,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        let result = self.runtimeHost.executeHostJavaScriptRuntimeStateTask(
            self.stateHandle,
            timeoutSec
                .checked_mul(1_000)
                .expect("ToolPkg registration timeout must fit in milliseconds"),
            Box::new({
                let script = script.to_string();
                let functionName = functionName.to_string();
                let params = params.clone();
                move |state, interrupt| {
                    let state = state.downcast_mut::<JsEngineState>().ok_or_else(|| {
                        HostError::new("JavaScript runtime state type does not match")
                    })?;
                    let output = executeWithInterrupt(
                        state,
                        interrupt,
                        timeoutSec,
                        "ToolPkg registration",
                        |state| {
                            state.execute_toolpkg_main_registration_function_on_current_thread(
                                &script,
                                &functionName,
                                &params,
                                textResources,
                            )
                        },
                    );
                    Ok(Box::new(output))
                }
            }),
        );
        match result {
            Ok(output) => downcastJavaScriptStateOutput(output),
            Err(error) => Err(javaScriptExecutionErrorFromHost(error)),
        }
    }

    /// Destroys the host-owned JavaScript runtime state.
    fn destroy(&self) {
        if !self.alive.swap(false, Ordering::AcqRel) {
            return;
        }
        self.runtimeHost
            .destroyHostJavaScriptRuntimeState(self.stateHandle)
            .expect("JavaScript runtime state must be destroyed by the Host");
    }
}

/// Builds an interrupt-bounded phase; no borrow of the state escapes this operation.
#[allow(non_snake_case)]
fn scriptRequestStepTask(
    step: JsScriptRequestStep,
    timeoutSec: u64,
) -> operit_host_api::HostJavaScriptRuntimeStateTask {
    Box::new(move |state, interrupt| {
        let state = state
            .downcast_mut::<JsEngineState>()
            .ok_or_else(|| HostError::new("JavaScript runtime state type does not match"))?;
        let stepInterrupt = interrupt.clone();
        let output =
            executeWithInterrupt(state, interrupt, timeoutSec, "Script execution", |state| {
                state.executeScriptRequestStep(step, stepInterrupt)
            });
        clearThreadLocalCallState();
        Ok(Box::new(output) as HostJavaScriptRuntimeStateOutput)
    })
}

/// Includes time spent queued behind other callers in the request deadline.
fn scriptRequestDeadline(timeout: Duration) -> JsExecutionResult<u128> {
    currentTimeMillisU128()
        .checked_add(timeout.as_millis())
        .ok_or_else(|| {
            JsExecutionError::timeout("Script execution deadline exceeds host clock range")
        })
}

#[allow(non_snake_case)]
fn remainingScriptRequestMillis(deadline: u128, timeoutSec: u64) -> JsExecutionResult<u64> {
    let remaining = deadline.saturating_sub(currentTimeMillisU128());
    if remaining == 0 {
        return Err(JsExecutionError::timeout(format!(
            "Script execution timed out after {timeoutSec} seconds"
        )));
    }
    u64::try_from(remaining).map_err(|_| {
        JsExecutionError::timeout("JavaScript execution timeout exceeds the host range")
    })
}

/// Converts one structured Host failure into the JavaScript execution error contract.
#[allow(non_snake_case)]
fn javaScriptExecutionErrorFromHost(error: HostError) -> JsExecutionError {
    match error.kind {
        HostErrorKind::Timeout => JsExecutionError::timeout(error.message),
        HostErrorKind::General => JsExecutionError::worker_unavailable(error.message),
    }
}

/// Converts one host state output into the exact JavaScript execution result type.
fn downcastJavaScriptStateOutput<T: 'static>(
    output: HostJavaScriptRuntimeStateOutput,
) -> JsExecutionResult<T> {
    let output = output.downcast::<JsExecutionResult<T>>().map_err(|_| {
        JsExecutionError::worker_unavailable("JavaScript runtime state result type does not match")
    })?;
    *output
}

/// Executes one JavaScript operation under a host-owned interrupt token.
#[allow(non_snake_case)]
fn executeWithInterrupt<T>(
    state: &mut JsEngineState,
    interrupt: HostJavaScriptExecutionInterrupt,
    timeoutSec: u64,
    timeoutLabel: &str,
    operation: impl FnOnce(&mut JsEngineState) -> JsExecutionResult<T>,
) -> JsExecutionResult<T> {
    let interruptForHandler = interrupt.clone();
    let handler: HostJavaScriptInterruptHandler =
        Arc::new(move || interruptForHandler.shouldInterrupt());
    if let Err(error) = state
        .runtime
        .setHostJavaScriptInterruptHandler(Some(handler))
    {
        return Err(JsExecutionError::initialization(error.to_string()));
    }
    let output = operation(state);
    let clearResult = state.runtime.setHostJavaScriptInterruptHandler(None);
    if let Err(error) = clearResult {
        return Err(JsExecutionError::runtime(error.to_string()));
    }
    if interrupt.didTimeOut() {
        return Err(JsExecutionError::timeout(format!(
            "{timeoutLabel} timed out after {timeoutSec} seconds"
        )));
    }
    output
}

impl JsEngine {
    /// Creates a JavaScript execution engine backed by a caller-supplied execution host.
    pub fn new(executionHost: Arc<dyn JsExecutionHost>) -> Self {
        Self {
            worker: JsEngineWorker::new(Some(executionHost), None),
        }
    }

    /// Creates a JavaScript execution engine bound to one ToolPkg package environment.
    pub fn new_toolpkg_execution_engine(
        executionHost: Arc<dyn JsExecutionHost>,
        context: ToolPkgExecutionContext,
    ) -> Self {
        Self {
            worker: JsEngineWorker::new(Some(executionHost), Some(context)),
        }
    }

    /// Creates a JavaScript engine used only for ToolPkg registration.
    #[allow(non_snake_case)]
    pub fn new_toolpkg_registration_engine() -> Self {
        Self {
            worker: JsEngineWorker::new(None, None),
        }
    }

    /// Executes a named JavaScript function with serialized parameters.
    #[allow(non_snake_case)]
    pub fn execute_script_function(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        if timeoutSec == 0 {
            let reason = "Script execution timed out after 0 seconds";
            if let Some(listener) = executionListener.as_ref() {
                listener.on_failed("", reason);
            }
            return Box::pin(async move { Err(JsExecutionError::timeout(reason)) });
        }
        self.execute_script_function_with_timeout(
            script,
            functionName,
            params,
            envOverrides,
            on_intermediate_result,
            dispatchIntermediateOnMain,
            Duration::from_secs(timeoutSec),
            timeoutSec,
            executionListener,
            None,
            false,
        )
    }

    /// Executes a named JavaScript function with an exact millisecond deadline.
    #[allow(non_snake_case)]
    pub fn execute_script_function_with_timeout_millis(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutMillis: u64,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        if timeoutMillis == 0 {
            let reason = "Script execution timed out after 0 milliseconds";
            if let Some(listener) = executionListener.as_ref() {
                listener.on_failed("", reason);
            }
            return Box::pin(async move { Err(JsExecutionError::timeout(reason)) });
        }
        let timeoutSec = (timeoutMillis - 1) / 1_000 + 1;
        self.execute_script_function_with_timeout(
            script,
            functionName,
            params,
            envOverrides,
            on_intermediate_result,
            dispatchIntermediateOnMain,
            Duration::from_millis(timeoutMillis),
            timeoutSec,
            executionListener,
            None,
            false,
        )
    }

    /// Executes a named JavaScript function while allowing the host runtime to keep advancing.
    #[allow(non_snake_case)]
    pub fn execute_script_function_async(
        &self,
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutMillis: u64,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
        if timeoutMillis == 0 {
            let reason = "Script execution timed out after 0 milliseconds";
            if let Some(listener) = executionListener.as_ref() {
                listener.on_failed("", reason);
            }
            return Box::pin(async move { Err(JsExecutionError::timeout(reason)) });
        }
        let timeoutSec = (timeoutMillis - 1) / 1_000 + 1;
        textScriptCompletion(self.worker.execute_script_function_async(
            script,
            functionName,
            params,
            envOverrides,
            textIntermediateCallback(on_intermediate_result),
            dispatchIntermediateOnMain,
            Duration::from_millis(timeoutMillis),
            timeoutSec,
            executionListener,
            None,
            false,
        ))
    }

    /// Executes JavaScript with the supplied native deadline and whole-second script metadata.
    #[allow(non_snake_case)]
    fn execute_script_function_with_timeout(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeout: Duration,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        useComposeDslTextResources: bool,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        textScriptCompletion(self.worker.execute_script_function_async(
            script.to_owned(),
            functionName.to_owned(),
            params.clone(),
            envOverrides.clone(),
            textIntermediateCallback(on_intermediate_result),
            dispatchIntermediateOnMain,
            timeout,
            timeoutSec,
            executionListener,
            textResources,
            useComposeDslTextResources,
        ))
    }

    /// Executes a ToolPkg registration function and captures its declaration.
    #[allow(non_snake_case)]
    pub fn execute_toolpkg_main_registration_function(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        self.execute_toolpkg_main_registration_function_with_text_resources(
            script,
            functionName,
            params,
            None,
        )
    }

    /// Executes ToolPkg registration with archive text resources and the standard deadline.
    #[allow(non_snake_case)]
    pub(crate) fn execute_toolpkg_main_registration_function_with_text_resources(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        self.executeToolPkgMainRegistrationWithTimeout(
            script,
            functionName,
            params,
            textResources,
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
        )
    }

    /// Executes ToolPkg registration with one explicit native deadline.
    #[allow(non_snake_case)]
    fn executeToolPkgMainRegistrationWithTimeout(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        timeoutSec: u64,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        if timeoutSec == 0 {
            return Err(JsExecutionError::timeout(
                "ToolPkg registration timed out after 0 seconds",
            ));
        }
        self.worker.execute_toolpkg_main_registration_function(
            script,
            functionName,
            params,
            textResources,
            timeoutSec,
        )
    }

    /// Executes a Compose DSL script and returns its rendered event stream.
    #[allow(non_snake_case)]
    pub fn execute_compose_dsl_script(
        &self,
        script: &str,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        textResources: Arc<ToolPkgTextResources>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<Value>>> {
        self.executeComposeDslFunction(
            &buildComposeDslRuntimeWrappedScript(script),
            "__operit_render_compose_dsl",
            runtimeOptions,
            envOverrides,
            None,
            Some(textResources),
        )
    }

    /// Executes a Compose DSL render while allowing the host runtime to keep advancing.
    #[allow(non_snake_case)]
    pub fn execute_compose_dsl_script_async(
        &self,
        script: String,
        runtimeOptions: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        textResources: Arc<ToolPkgTextResources>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
        self.executeComposeDslFunctionAsync(
            buildComposeDslRuntimeWrappedScript(&script),
            "__operit_render_compose_dsl".to_string(),
            runtimeOptions,
            envOverrides,
            None,
            Some(textResources),
        )
    }

    #[allow(non_snake_case)]
    pub fn execute_compose_dsl_action(
        &self,
        actionId: &str,
        payload: Option<Value>,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(Value) + Send + Sync>>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<Value>>> {
        let normalizedActionId = actionId.trim();
        if normalizedActionId.is_empty() {
            return Box::pin(async {
                Err(JsExecutionError::invalid_request(
                    "compose action id is required",
                ))
            });
        }
        let mut params = runtimeOptions.clone();
        params.insert(
            "__action_id".to_string(),
            Value::String(normalizedActionId.to_string()),
        );
        if let Some(payload) = payload {
            params.insert("__action_payload".to_string(), payload);
        }
        self.executeComposeDslFunction(
            "",
            "__operit_dispatch_compose_dsl_action",
            &params,
            envOverrides,
            on_intermediate_result,
            None,
        )
    }

    /// Dispatches a Compose DSL action while allowing the host runtime to keep advancing.
    #[allow(non_snake_case)]
    pub fn dispatch_compose_dsl_action_result_async(
        &self,
        actionId: String,
        payload: Option<Value>,
        runtimeOptions: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(Value) + Send + Sync>>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
        let normalizedActionId = actionId.trim().to_string();
        if normalizedActionId.is_empty() {
            return Box::pin(async {
                Err(JsExecutionError::invalid_request(
                    "compose action id is required",
                ))
            });
        }
        let mut params = runtimeOptions;
        params.insert("__action_id".to_string(), Value::String(normalizedActionId));
        if let Some(payload) = payload {
            params.insert("__action_payload".to_string(), payload);
        }
        self.executeComposeDslFunctionAsync(
            String::new(),
            "__operit_dispatch_compose_dsl_action".to_string(),
            params,
            envOverrides,
            on_intermediate_result,
            None,
        )
    }

    #[allow(non_snake_case)]
    pub fn rerender_compose_dsl_tree(
        &self,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<Value>>> {
        self.executeComposeDslFunction(
            "",
            "__operit_rerender_compose_dsl",
            runtimeOptions,
            envOverrides,
            None,
            None,
        )
    }

    /// Executes a Compose DSL operation with the resource snapshot owned by its page runtime.
    #[allow(non_snake_case)]
    fn executeComposeDslFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(Value) + Send + Sync>>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<Value>>> {
        self.worker.execute_script_function_async(
            script.to_owned(),
            functionName.to_owned(),
            params.clone(),
            envOverrides.clone(),
            onIntermediateResult,
            true,
            Duration::from_secs(TOOLPKG_SCRIPT_TIMEOUT_SECONDS),
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
            None,
            textResources,
            true,
        )
    }

    /// Executes one Compose DSL operation through the asynchronous engine contract.
    #[allow(non_snake_case)]
    fn executeComposeDslFunctionAsync(
        &self,
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(Value) + Send + Sync>>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
        self.worker.execute_script_function_async(
            script,
            functionName,
            params,
            envOverrides,
            onIntermediateResult,
            true,
            Duration::from_secs(TOOLPKG_SCRIPT_TIMEOUT_SECONDS),
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
            None,
            textResources,
            true,
        )
    }

    #[allow(non_snake_case)]
    pub fn dispatch_compose_dsl_action_async(
        &self,
        actionId: &str,
        payload: Option<Value>,
        runtimeOptions: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
    ) -> JsComposeDslActionEventStream {
        JsComposeDslActionEventStream {
            engine: self.clone(),
            actionId: actionId.to_string(),
            payload,
            runtimeOptions,
            envOverrides,
        }
    }
}

impl Stream for JsComposeDslActionEventStream {
    type Item = Value;

    /// Collects Compose DSL action events without blocking the collector task.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        Box::pin(async move {
            let intermediateEvents = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
            let intermediateEventsForCallback = intermediateEvents.clone();
            let result = self
                .engine
                .dispatch_compose_dsl_action_result_async(
                    self.actionId.clone(),
                    self.payload.clone(),
                    self.runtimeOptions.clone(),
                    self.envOverrides.clone(),
                    Some(Arc::new(move |intermediate| {
                        intermediateEventsForCallback
                            .lock()
                            .expect("compose dsl intermediate event mutex poisoned")
                            .push(composeDslActionEvent(
                                "intermediate",
                                None,
                                Some(&intermediate),
                            ));
                    })),
                )
                .await;
            for event in intermediateEvents
                .lock()
                .expect("compose dsl intermediate event mutex poisoned")
                .iter()
                .cloned()
            {
                collector(event);
            }
            match result {
                Ok(Some(result)) => collector(composeDslActionEvent("final", None, Some(&result))),
                Ok(None) => {}
                Err(error) => collector(composeDslActionEvent("error", Some(&error.message), None)),
            }
            collector(composeDslActionEvent("complete", None, None));
        })
    }
}

#[allow(non_snake_case)]
fn composeDslActionEvent(phase: &str, error: Option<&str>, result: Option<&Value>) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("phase".to_string(), Value::String(phase.to_string()));
    if let Some(error) = error {
        object.insert("error".to_string(), Value::String(error.to_string()));
    }
    if let Some(result) = result {
        object.insert("result".to_string(), result.clone());
    }
    Value::Object(object)
}

/// Validates and converts one Host JavaScript callback argument list.
#[allow(non_snake_case)]
fn exactHostJavaScriptArguments<const N: usize>(
    functionName: &str,
    arguments: Vec<String>,
) -> HostResult<[String; N]> {
    let argumentCount = arguments.len();
    arguments.try_into().map_err(|_| {
        HostError::new(format!(
            "{functionName} requires {N} arguments, received {argumentCount}"
        ))
    })
}

impl JsEngineState {
    /// Creates a JavaScript runtime state from a Host-owned runtime instance.
    fn newWithRuntime(
        runtime: Box<dyn HostJavaScriptRuntime>,
        executionHost: Option<Arc<dyn JsExecutionHost>>,
        toolPkgContext: Option<ToolPkgExecutionContext>,
        backgroundWake: Arc<Mutex<Option<JsBackgroundWake>>>,
    ) -> Result<Self, String> {
        let executionHost = match (&executionHost, &toolPkgContext) {
            (Some(host), Some(context)) => Some(host.for_toolpkg_execution_context(context)?),
            (None, Some(_)) => {
                return Err("ToolPkg execution context requires a real host".to_string())
            }
            (_, None) => executionHost,
        };
        let (asyncCallbackSender, asyncCallbackReceiver) = mpsc::channel();
        let mut state = Self {
            runtime,
            asyncCallbackSender,
            asyncCallbackReceiver,
            backgroundWake,
            executionHost,
            toolPkgContext,
            composeDslTextResources: None,
            detachedCallContexts: BTreeMap::new(),
            pendingScriptExecutions: BTreeMap::new(),
            jsEnvironmentInitialized: false,
        };
        state.registerHostBindings()?;
        Ok(state)
    }

    /// Installs every active owner so callbacks advanced by another call retain their context.
    #[allow(non_snake_case)]
    fn installActiveCallContexts(&self) {
        CURRENT_EXECUTION_HOST.with(|host| {
            *host.borrow_mut() = self.executionHost.clone();
        });
        CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| {
            *contexts.borrow_mut() = self
                .detachedCallContexts
                .iter()
                .map(|(id, context)| (id.clone(), context.clone()))
                .chain(
                    self.pendingScriptExecutions
                        .iter()
                        .map(|(id, pending)| (id.clone(), pending.context.clone())),
                )
                .collect();
        });
    }

    /// Starts or cancels a session only on its owning JS state executor.
    #[allow(non_snake_case)]
    fn executeScriptRequestStep(
        &mut self,
        step: JsScriptRequestStep,
        interrupt: HostJavaScriptExecutionInterrupt,
    ) -> JsExecutionResult<()> {
        self.installActiveCallContexts();
        match step {
            JsScriptRequestStep::Start(request) => {
                if request.cancelled.load(Ordering::Acquire) {
                    return Err(JsExecutionError::worker_unavailable(
                        "JavaScript execution was cancelled",
                    ));
                }
                let pending = self.startScriptFunctionForRequest(request)?;
                self.pendingScriptExecutions
                    .insert(pending.callId.clone(), pending);
                self.advanceJavaScriptExecutions(interrupt)
            }
            JsScriptRequestStep::Cancel(callId) => {
                if let Some(pending) = self.pendingScriptExecutions.remove(&callId) {
                    installThreadLocalCallContext(&pending.context);
                    self.cancelJavaScriptExecution(&callId);
                }
                Ok(())
            }
        }
    }

    /// Loads the module and starts its function, without retaining the state while awaiting it.
    #[allow(non_snake_case)]
    fn startScriptFunctionForRequest(
        &mut self,
        request: JsScriptRequest,
    ) -> JsExecutionResult<JsPendingScriptExecution> {
        let JsScriptRequest {
            callId,
            script,
            functionName,
            params,
            envOverrides,
            onIntermediateResult,
            timeout,
            timeoutSec,
            executionListener,
            textResources,
            useComposeDslTextResources,
            completion,
            cancelled: _,
            deadlineMillis,
        } = request;
        let textResources = if useComposeDslTextResources {
            if let Some(textResources) = textResources {
                self.composeDslTextResources = Some(textResources);
            }
            Some(self.composeDslTextResources.clone().ok_or_else(|| {
                JsExecutionError::invalid_request(
                    "Compose DSL action requires a rendered page resource snapshot",
                )
            })?)
        } else {
            None
        };
        let textResourceSource = if useComposeDslTextResources {
            JsTextResourceSource::Snapshot(
                textResources.expect("Compose page snapshot was validated"),
            )
        } else {
            packageTextResourceSource(self.toolPkgContext.as_ref())
        };
        let context = JsCallContext {
            executionHost: self.executionHost.clone(),
            intermediateCallback: onIntermediateResult,
            executionListener,
            envOverrides: Arc::new(Mutex::new(envOverrides)),
            textResourceSource,
        };
        installThreadLocalCallContext(&context);
        CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| {
            contexts
                .borrow_mut()
                .insert(callId.clone(), context.clone())
        });
        let started = (|| {
            self.initJavaScriptEnvironment()
                .map_err(JsExecutionError::initialization)?;
            let mut effectiveParams = params;
            // Async Compose renders and actions need the same API contract as synchronous calls.
            let apiVersion = match self.toolPkgContext.as_ref() {
                Some(context) => context.api_version.clone(),
                None => operit_plugin_sdk::toolpkg::ToolPkgApiVersion::CURRENT_TOOLPKG_API_VERSION
                    .to_string(),
            };
            effectiveParams.insert(
                "__operit_toolpkg_api_version".to_string(),
                Value::String(apiVersion),
            );
            let explicitLanguage = effectiveParams
                .get("__operit_package_lang")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if explicitLanguage.is_empty() {
                let language = self
                    .resolveCurrentPackageLanguage()
                    .map_err(JsExecutionError::runtime)?;
                effectiveParams
                    .insert("__operit_package_lang".to_string(), Value::String(language));
            }
            clearNativeExecutionSession(&callId);
            if let Err(error) = self.runtime.callHostJavaScriptFunction(
                "__operitExecuteScriptFunction",
                &[
                    Value::String(callId.clone()),
                    Value::Object(effectiveParams.into_iter().collect()),
                    Value::String(script),
                    Value::String(functionName),
                    Value::from(timeoutSec),
                    Value::from(10000),
                    Value::Bool(useComposeDslTextResources),
                ],
            ) {
                self.cancelJavaScriptExecution(&callId);
                return Err(JsExecutionError::runtime(error.to_string()));
            }
            Ok(JsPendingScriptExecution {
                callId,
                context: context.clone(),
                deadlineMillis,
                timeout,
                completion,
            })
        })();
        clearThreadLocalCallState();
        started
    }

    /// Executes a direct-state fixture without exposing a production blocking entry point.
    #[cfg(test)]
    #[allow(non_snake_case)]
    /// Executes a test script against its explicitly supplied module-resource owner.
    fn execute_script_function_on_current_thread(
        &mut self,
        textResourceSource: JsTextResourceSource,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        _dispatchIntermediateOnMain: bool,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> JsExecutionResult<Option<String>> {
        if let Err(error) = self.initJavaScriptEnvironment() {
            return Err(JsExecutionError::initialization(error));
        }
        CURRENT_EXECUTION_HOST.with(|host| {
            *host.borrow_mut() = self.executionHost.clone();
        });
        CURRENT_INTERMEDIATE_CALLBACK.with(|callback| {
            *callback.borrow_mut() = textIntermediateCallback(on_intermediate_result);
        });
        CURRENT_EXECUTION_LISTENER.with(|listener| {
            *listener.borrow_mut() = executionListener;
        });
        CURRENT_ENV_OVERRIDES.with(|overrides| {
            *overrides.borrow_mut() = envOverrides.clone();
        });
        CURRENT_TEXT_RESOURCE_SOURCE.with(|source| {
            *source.borrow_mut() = Some(textResourceSource.clone());
        });

        let mut effectiveParams = params.clone();
        // Standalone scripts use the current SDK contract; ToolPkg engines retain their manifest contract.
        let apiVersion = match self.toolPkgContext.as_ref() {
            Some(context) => context.api_version.clone(),
            None => operit_plugin_sdk::toolpkg::ToolPkgApiVersion::CURRENT_TOOLPKG_API_VERSION
                .to_string(),
        };
        effectiveParams.insert(
            "__operit_toolpkg_api_version".to_string(),
            Value::String(apiVersion),
        );
        let explicitLanguage = effectiveParams
            .get("__operit_package_lang")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if explicitLanguage.is_empty() {
            let language = match self.resolveCurrentPackageLanguage() {
                Ok(language) => language,
                Err(error) => {
                    clearThreadLocalCallState();
                    return Err(JsExecutionError::runtime(error));
                }
            };
            effectiveParams.insert("__operit_package_lang".to_string(), Value::String(language));
        }

        let callId = format!(
            "operit_call_{}",
            Uuid::new_v4().to_string().replace('-', "")
        );
        CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| {
            contexts.borrow_mut().insert(
                callId.clone(),
                JsCallContext {
                    executionHost: self.executionHost.clone(),
                    intermediateCallback: CURRENT_INTERMEDIATE_CALLBACK
                        .with(|value| value.borrow().clone()),
                    executionListener: CURRENT_EXECUTION_LISTENER
                        .with(|value| value.borrow().clone()),
                    envOverrides: Arc::new(Mutex::new(envOverrides.clone())),
                    textResourceSource,
                },
            );
        });
        clearNativeExecutionSession(&callId);
        let output = match self.invokeExecutionFunction(
            &callId,
            effectiveParams,
            script,
            functionName,
            timeoutSec,
        ) {
            Ok(_) => match self.waitForExecutionResult(&callId, timeoutSec) {
                Ok(output) => output,
                Err(error) => {
                    clearNativeExecutionSession(&callId);
                    clearThreadLocalCallState();
                    return Err(error);
                }
            },
            Err(error) => {
                AppLogger::e(
                    TAG,
                    &format!(
                        "execute-eval-error callId={} function={} error={}",
                        callId, functionName, error
                    ),
                );
                self.cancelJavaScriptExecution(&callId);
                clearNativeExecutionSession(&callId);
                clearThreadLocalCallState();
                return Err(JsExecutionError::runtime(error.to_string()));
            }
        };
        clearNativeExecutionSession(&callId);
        clearThreadLocalCallState();
        if let Some(message) = extractJsExecutionErrorMessage(output.as_deref()) {
            Err(JsExecutionError::runtime(message))
        } else {
            Ok(output)
        }
    }

    /// Registers a package while keeping its host services bound for module evaluation.
    #[allow(non_snake_case)]
    fn execute_toolpkg_main_registration_function_on_current_thread(
        &mut self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        self.initJavaScriptEnvironment()
            .map_err(JsExecutionError::initialization)?;
        let apiRuntime = buildToolPkgApiRuntimeScript();
        self.evalJavaScriptVoid(&apiRuntime)
            .map_err(JsExecutionError::runtime)?;
        let bridge = buildToolPkgRegistrationBridgeScript(true);
        self.evalJavaScriptVoid(&bridge)
            .map_err(JsExecutionError::runtime)?;
        let source = match textResources {
            Some(resources) => JsTextResourceSource::Snapshot(resources),
            None => packageTextResourceSource(self.toolPkgContext.as_ref()),
        };
        CURRENT_TEXT_RESOURCE_SOURCE.with(|current| {
            *current.borrow_mut() = Some(source);
        });
        CURRENT_EXECUTION_HOST.with(|host| {
            *host.borrow_mut() = self.executionHost.clone();
        });
        // Bind trusted scope/owner parameters before any top-level module code can query paths.
        CURRENT_REGISTRATION_CONFIG_PARAMS.with(|current| {
            *current.borrow_mut() = Some(params.clone());
        });
        let registrationResult = (|| {
            let mut registrationParams = params.clone();
            registrationParams.insert("__operit_registration_mode".to_string(), Value::Bool(true));
            let explicitLanguage = registrationParams
                .get("__operit_package_lang")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if explicitLanguage.is_empty() {
                let language = self
                    .resolveCurrentPackageLanguage()
                    .map_err(JsExecutionError::runtime)?;
                registrationParams
                    .insert("__operit_package_lang".to_string(), Value::String(language));
            }
            let callId = format!(
                "operit_registration_{}",
                Uuid::new_v4().to_string().replace('-', "")
            );
            let textResourceSource = CURRENT_TEXT_RESOURCE_SOURCE
                .with(|source| source.borrow().clone())
                .ok_or_else(|| {
                    JsExecutionError::invalid_request("Registration resource owner is unavailable")
                })?;
            CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| {
                contexts.borrow_mut().insert(
                    callId.clone(),
                    JsCallContext {
                        executionHost: self.executionHost.clone(),
                        intermediateCallback: None,
                        executionListener: None,
                        envOverrides: Arc::new(Mutex::new(BTreeMap::new())),
                        textResourceSource,
                    },
                );
            });
            clearNativeExecutionSession(&callId);
            if let Err(error) =
                self.invokeExecutionFunction(&callId, registrationParams, script, functionName, 60)
            {
                self.cancelJavaScriptExecution(&callId);
                return Err(JsExecutionError::runtime(error));
            }
            if let Err(error) = self.runJavaScriptJobs() {
                self.cancelJavaScriptExecution(&callId);
                return Err(JsExecutionError::runtime(error));
            }
            let output = match readNativeExecutionSession(&callId) {
                Some(output) => output,
                None => {
                    self.cancelJavaScriptExecution(&callId);
                    return Err(JsExecutionError::runtime(
                        "ToolPkg registration JavaScript did not complete",
                    ));
                }
            };
            clearNativeExecutionSession(&callId);
            ensureRegistrationExecutionSucceeded(&output).map_err(JsExecutionError::runtime)?;

            let capture = self
                .runtime
                .callHostJavaScriptFunction("__operitReadRegistrationCapture", &[])
                .map_err(|error| JsExecutionError::runtime(error.to_string()))?;
            serde_json::from_value::<ToolPkgMainRegistrationCapture>(capture)
                .map_err(|error| JsExecutionError::protocol(error.to_string()))
        })();
        clearThreadLocalCallState();
        CURRENT_REGISTRATION_CONFIG_PARAMS.with(|current| {
            *current.borrow_mut() = None;
        });
        // Registration temporarily installs a restricted bridge. Restore the runtime bridge
        // before any hook can evaluate a package main module again.
        let runtimeBridge = buildToolPkgRegistrationBridgeScript(false);
        self.evalJavaScriptVoid(&runtimeBridge)
            .map_err(JsExecutionError::runtime)?;
        registrationResult
    }

    #[allow(non_snake_case)]
    fn evalJavaScriptVoid(&mut self, script: &str) -> Result<(), String> {
        self.runtime
            .evaluateHostJavaScriptVoid("operit.js", script)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    #[cfg(test)]
    fn evalJavaScriptString(&mut self, script: &str) -> Result<String, String> {
        self.runtime
            .evaluateHostJavaScriptString("operit.js", script)
            .map_err(|error| error.to_string())
    }

    /// Cancels one JavaScript call and releases its call-scoped callbacks and timers.
    #[allow(non_snake_case)]
    fn cancelJavaScriptExecution(&mut self, callId: &str) {
        if let Err(error) = self.runtime.cancelHostJavaScriptPromises(callId) {
            AppLogger::e(
                TAG,
                &format!("cancel structured JavaScript promises failed: {error}"),
            );
        }
        if let Err(error) = self.runtime.callHostJavaScriptFunction(
            "__operitCancelCallSession",
            &[Value::String(callId.to_string())],
        ) {
            AppLogger::e(
                TAG,
                &format!("cancel JavaScript execution failed callId={callId}: {error}"),
            );
        }
    }

    /// Drives active and detached sessions once after a Host callback, never on a timer loop.
    #[allow(non_snake_case)]
    fn advanceJavaScriptExecutions(
        &mut self,
        hostInterrupt: HostJavaScriptExecutionInterrupt,
    ) -> JsExecutionResult<()> {
        // Bound continuations by active request deadlines, including CPU-bound Promise jobs.
        let now = currentTimeMillisU128();
        let timeoutMillis =
            self.pendingScriptExecutions
                .values()
                .fold(60_000u64, |bound, pending| {
                    bound.min(
                        pending
                            .deadlineMillis
                            .saturating_sub(now)
                            .max(1)
                            .min(60_000) as u64,
                    )
                });
        let interrupt = HostJavaScriptExecutionInterrupt::new(timeoutMillis)
            .map_err(javaScriptExecutionErrorFromHost)?;
        let hostInterruptForHandler = hostInterrupt.clone();
        let interruptForHandler = interrupt.clone();
        self.runtime
            .setHostJavaScriptInterruptHandler(Some(Arc::new(move || {
                hostInterruptForHandler.shouldInterrupt() || interruptForHandler.shouldInterrupt()
            })))
            .map_err(|error| JsExecutionError::initialization(error.to_string()))?;
        let jobs = self.advanceDetachedJavaScriptExecution();
        self.runtime
            .setHostJavaScriptInterruptHandler(None)
            .map_err(|error| JsExecutionError::runtime(error.to_string()))?;
        let advanced = if interrupt.didTimeOut() || hostInterrupt.didTimeOut() {
            Err(JsExecutionError::timeout(
                "JavaScript continuation exceeded its execution deadline",
            ))
        } else {
            jobs.map_err(JsExecutionError::runtime)
        };
        let ids = self
            .pendingScriptExecutions
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for callId in ids {
            let terminal = match &advanced {
                Err(error) => Some(Err(error.clone())),
                Ok(()) => {
                    let pending = &self.pendingScriptExecutions[&callId];
                    if currentTimeMillisU128() >= pending.deadlineMillis {
                        Some(Err(JsExecutionError::timeout(format!(
                            "Script execution timed out after {} milliseconds",
                            pending.timeout.as_millis(),
                        ))))
                    } else {
                        readNativeExecutionSession(&callId).map(|output| output.map(Some))
                    }
                }
            };
            if let Some(mut result) = terminal {
                let pending = self
                    .pendingScriptExecutions
                    .remove(&callId)
                    .expect("terminal JavaScript session must remain registered");
                installThreadLocalCallContext(&pending.context);
                if result.is_ok() {
                    if let Err(error) = self.rememberDetachedCallContext(&pending) {
                        result = Err(JsExecutionError::runtime(error));
                    }
                }
                if result.is_err() {
                    self.cancelJavaScriptExecution(&callId);
                }
                // Retain detached context before publishing the primary result.
                completeScriptExecution(&pending.completion, result);
            }
        }
        clearThreadLocalCallState();
        advanced
    }

    /// Advances detached JavaScript calls after their original request has returned.
    fn advanceDetachedJavaScriptExecution(&mut self) -> Result<(), String> {
        self.installActiveCallContexts();
        let ids = self
            .runtime
            .callHostJavaScriptFunction("__operitGetDetachedCallIds", &[])
            .map_err(|error| error.to_string())?;
        let ids: Vec<String> = serde_json::from_value(ids).map_err(|error| error.to_string())?;
        self.runJavaScriptJobs()?;
        loop {
            match self.asyncCallbackReceiver.try_recv() {
                Ok(callback) => {
                    self.deliverAsyncCallback(callback)?;
                    self.runJavaScriptJobs()?;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("JavaScript asynchronous callback queue disconnected".to_string())
                }
            }
        }
        for callId in ids {
            let prepared = self
                .runtime
                .callHostJavaScriptFunction("__operitPrepareDetachedCall", &[Value::String(callId)])
                .map_err(|error| error.to_string())?;
            let prepared = prepared
                .as_bool()
                .ok_or("Detached-call preparation must return a boolean")?;
            if prepared {
                self.runJavaScriptJobs()?;
            }
        }
        loop {
            match self.asyncCallbackReceiver.try_recv() {
                Ok(callback) => {
                    self.deliverAsyncCallback(callback)?;
                    self.runJavaScriptJobs()?;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("JavaScript asynchronous callback queue disconnected".to_string())
                }
            }
        }
        let activeIds = self
            .runtime
            .callHostJavaScriptFunction("__operitGetDetachedCallIds", &[])
            .map_err(|error| error.to_string())?;
        let activeIds: Vec<String> =
            serde_json::from_value(activeIds).map_err(|error| error.to_string())?;
        let completedCallIds = self
            .detachedCallContexts
            .keys()
            .filter(|callId| !activeIds.iter().any(|activeId| activeId == *callId))
            .cloned()
            .collect::<Vec<_>>();
        for callId in completedCallIds {
            self.runtime
                .callHostJavaScriptFunction(
                    "__operitFinalizeDetachedCall",
                    &[Value::String(callId)],
                )
                .map_err(|error| error.to_string())?;
        }
        self.detachedCallContexts
            .retain(|callId, _| activeIds.iter().any(|activeId| activeId == callId));
        Ok(())
    }

    /// Retains the callback context while a JavaScript call owns detached timers.
    fn rememberDetachedCallContext(
        &mut self,
        pending: &JsPendingScriptExecution,
    ) -> Result<(), String> {
        let detached = self
            .runtime
            .callHostJavaScriptFunction(
                "__operitIsCallDetached",
                &[Value::String(pending.callId.clone())],
            )
            .map_err(|error| error.to_string())?;
        let detached = detached
            .as_bool()
            .ok_or("Detached-call predicate must return a boolean")?;
        if detached {
            self.detachedCallContexts
                .insert(pending.callId.clone(), pending.context.clone());
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn runJavaScriptJobs(&mut self) -> Result<(), String> {
        self.runtime
            .executePendingHostJavaScriptJobs()
            .map_err(|error| error.to_string())
    }

    /// Waits for one JavaScript call to complete while delivering queued host callbacks.
    #[cfg(test)]
    #[allow(non_snake_case)]
    fn waitForExecutionResult(
        &mut self,
        callId: &str,
        timeoutSec: u64,
    ) -> JsExecutionResult<Option<String>> {
        let timeout = Duration::from_secs(timeoutSec);
        let deadlineMillis = currentTimeMillisU128()
            .checked_add(timeout.as_millis())
            .ok_or_else(|| {
                JsExecutionError::timeout("Script execution deadline exceeds host clock range")
            })?;
        loop {
            if let Err(error) = self.runJavaScriptJobs() {
                self.cancelJavaScriptExecution(callId);
                return Err(JsExecutionError::runtime(error));
            }
            if let Some(output) = readNativeExecutionSession(callId) {
                return textScriptResult(output.map(Some));
            }
            let nowMillis = currentTimeMillisU128();
            if nowMillis >= deadlineMillis {
                self.cancelJavaScriptExecution(callId);
                return Err(JsExecutionError::timeout(format!(
                    "Script execution timed out after {} milliseconds",
                    timeout.as_millis()
                )));
            }
            let waitDuration = Duration::from_millis(
                deadlineMillis
                    .saturating_sub(nowMillis)
                    .min(10)
                    .try_into()
                    .expect("bounded JavaScript wait duration must fit in milliseconds"),
            );
            match self.asyncCallbackReceiver.recv_timeout(waitDuration) {
                Ok(callback) => {
                    if let Err(error) = self.deliverAsyncCallback(callback) {
                        self.cancelJavaScriptExecution(callId);
                        return Err(JsExecutionError::runtime(error));
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.cancelJavaScriptExecution(callId);
                    return Err(JsExecutionError::worker_unavailable(
                        "JavaScript asynchronous callback queue disconnected",
                    ));
                }
            }
        }
    }

    /// Settles one asynchronous result through the owning host Promise registry.
    #[allow(non_snake_case)]
    fn deliverAsyncCallback(&mut self, callback: JsAsyncCallback) -> Result<(), String> {
        match callback {
            JsAsyncCallback::Promise {
                requestId,
                result,
                reject,
            } => self
                .runtime
                .settleHostJavaScriptPromise(requestId, &result, reject),
            JsAsyncCallback::CancelScope { scope } => {
                self.runtime.cancelHostJavaScriptPromises(&scope)
            }
        }
        .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn resolveCurrentPackageLanguage(&self) -> Result<String, String> {
        let executionHost = self.executionHost.as_ref().ok_or_else(|| {
            "JavaScript execution host is required to resolve package language".to_string()
        })?;
        let language = executionHost.package_language()?;
        let trimmed = language.trim();
        if trimmed.is_empty() {
            return Err("JavaScript execution host returned an empty package language".to_string());
        }
        Ok(trimmed.to_string())
    }

    /// Invokes the script lifecycle entry with structured arguments instead of generated call source.
    #[allow(non_snake_case)]
    fn invokeExecutionFunction(
        &mut self,
        callId: &str,
        params: BTreeMap<String, Value>,
        script: &str,
        functionName: &str,
        timeoutSec: u64,
    ) -> Result<(), String> {
        self.runtime
            .callHostJavaScriptFunction(
                "__operitExecuteScriptFunction",
                &[
                    Value::String(callId.to_string()),
                    Value::Object(params.into_iter().collect()),
                    Value::String(script.to_string()),
                    Value::String(functionName.to_string()),
                    Value::from(timeoutSec),
                    Value::from(10_000u64),
                ],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Installs the single structured host transport for synchronous and Promise-backed operations.
    #[allow(non_snake_case)]
    fn registerHostBindings(&mut self) -> Result<(), String> {
        let syncNames = [
            "__operitNativeReadToolPkgTextResource",
            "__operitNativeGetEnvForCall",
            "__operitNativeSetEnv",
            "__operitNativeSetEnvs",
            "__operitNativeGetScopedPluginConfigDir",
            "__operitNativeGetToolCatalog",
            "__operitNativeComposeWebViewControllerCommand",
            "__operitNativeLog",
            "__operitNativeDecompress",
            "__operitNativeCrypto",
            "__operitNativeJavaClassExists",
            "__operitNativeJavaGetApplicationContext",
            "__operitNativeJavaGetCurrentActivity",
            "__operitNativeJavaCallInstance",
            "__operitNativeJavaNewInstance",
            "__operitNativeJavaCallStatic",
            "__operitNativeJavaCallStaticSuspend",
            "__operitNativeJavaGetStaticField",
            "__operitNativeJavaSetStaticField",
            "__operitSendIntermediateResult",
            "__operitNativeSetCallResult",
            "__operitNativeSetCallError",
            "__operitNativeNotifyDetachedCall",
            "__operitNativeLogJsExecutionTrace",
        ];
        for name in syncNames {
            self.runtime
                .registerHostJavaScriptJsonFunction(
                    name,
                    Arc::new(move |arguments| executeHostOperation(name, arguments)),
                )
                .map_err(|error| error.to_string())?;
        }

        let structuredFunctions: Vec<(&str, HostJavaScriptValueCallback)> = vec![
            ("__operitNativeSetCallStructuredResult", Arc::new(|arguments| {
                let [callId, value]: [Value; 2] = arguments.try_into().map_err(|_| HostError::new("Structured result requires call id and value"))?;
                let callId = callId.as_str().ok_or_else(|| HostError::new("Structured call id must be a string"))?;
                CURRENT_CALL_RESULTS.with(|results| { results.borrow_mut().insert(callId.to_string(), Ok(value)); });
                Ok(())
            })),
            ("__operitNativeSendStructuredIntermediate", Arc::new(|arguments| {
                let [callId, value]: [Value; 2] = arguments.try_into().map_err(|_| HostError::new("Structured intermediate requires call id and value"))?;
                let callId = callId.as_str().ok_or_else(|| HostError::new("Structured call id must be a string"))?;
                sendStructuredIntermediate(callId, value);
                Ok(())
            })),
        ];
        for (name, callback) in structuredFunctions {
            self.runtime.registerHostJavaScriptValueFunction(name, callback).map_err(|error| error.to_string())?;
        }

        let executionHost = self.executionHost.clone();
        let asyncCallbackSender = self.asyncCallbackSender.clone();
        let backgroundWake = self.backgroundWake.clone();
        let asyncCallbackSink: JsAsyncCallbackSink = Arc::new(move |callback| {
            let _ = asyncCallbackSender.send(callback);
            if let Some(wake) = backgroundWake
                .lock()
                .expect("background wake mutex poisoned")
                .clone()
            {
                wake();
            }
        });
        let toolHost = self.executionHost.clone();
        let toolSink = asyncCallbackSink.clone();
        let timerSink = asyncCallbackSink.clone();
        let ipcHost = self.executionHost.clone();
        let ipcSink = asyncCallbackSink.clone();
        let ipcContext = self.toolPkgContext.clone();
        let dependencyHost = self.executionHost.clone();
        let dependencySink = asyncCallbackSink.clone();
        let dependencyContext = self.toolPkgContext.clone();
        let asyncFunctions: Vec<(&str, HostJavaScriptAsyncJsonCallback)> = vec![
            (
                "__operitNativeCallToolStructured",
                Arc::new(move |requestId, arguments| {
                    dispatchStructuredToolCall(
                        toolHost.clone(),
                        toolSink.clone(),
                        requestId,
                        arguments,
                    );
                    Ok(())
                }),
            ),
            (
                "__operitNativeScheduleJavaScriptTimer",
                Arc::new(move |requestId, arguments| {
                    let [delay] = exactHostJavaScriptJsonArguments(
                        "__operitNativeScheduleJavaScriptTimer",
                        arguments,
                    )?;
                    let delayMs = delay.as_u64().ok_or_else(|| {
                        HostError::new("JavaScript timer delay must be a non-negative integer")
                    })?;
                    dispatchJavaScriptTimer(timerSink.clone(), requestId, delayMs)
                        .map_err(HostError::new)
                }),
            ),
            (
                "__operitNativeInvokeToolPkgIpc",
                Arc::new(move |requestId, arguments| {
                    let [packageTarget, callerContextKey, targetContextKey, targetRuntime, channel, payload] =
                        exactHostJavaScriptJsonArguments(
                            "__operitNativeInvokeToolPkgIpc",
                            arguments,
                        )?;
                    let packageTarget = hostJavaScriptStringArgument(packageTarget)?;
                    let context = ipcContext.as_ref().ok_or_else(|| {
                        HostError::new("ToolPkg IPC requires a bound package context")
                    })?;
                    if packageTarget != context.container_package_name {
                        return Err(HostError::new(
                            "Package-private IPC cannot target another package",
                        ));
                    }
                    let request = buildToolPkgIpcRequest(
                        packageTarget,
                        hostJavaScriptStringArgument(callerContextKey)?,
                        hostJavaScriptStringArgument(targetContextKey)?,
                        hostJavaScriptStringArgument(targetRuntime)?,
                        hostJavaScriptStringArgument(channel)?,
                        payload,
                    )
                    .map_err(HostError::new)?;
                    dispatchToolPkgIpcAsync(ipcHost.clone(), ipcSink.clone(), requestId, request);
                    Ok(())
                }),
            ),
            (
                "__operitNativeCallDependency",
                Arc::new(move |requestId, arguments| {
                    let [packageTarget, methodName, payload] = exactHostJavaScriptJsonArguments(
                        "__operitNativeCallDependency",
                        arguments,
                    )?;
                    let context = dependencyContext.as_ref().ok_or_else(|| {
                        HostError::new("Dependency calls require a bound ToolPkg context")
                    })?;
                    let mut request = buildToolPkgIpcRequest(
                        hostJavaScriptStringArgument(packageTarget)?,
                        context.context_key.clone(),
                        String::new(),
                        "main".to_string(),
                        hostJavaScriptStringArgument(methodName)?,
                        payload,
                    )
                    .map_err(HostError::new)?;
                    request.dependency_caller = Some(context.container_package_name.clone());
                    dispatchToolPkgIpcAsync(
                        dependencyHost.clone(),
                        dependencySink.clone(),
                        requestId,
                        request,
                    );
                    Ok(())
                }),
            ),
        ];
        for (name, callback) in asyncFunctions {
            self.runtime
                .registerHostJavaScriptAsyncJsonFunction(name, callback)
                .map_err(|error| error.to_string())?;
        }
        let asyncNames = [
            "__operitNativeReadToolPkgResource",
            "__operitNativeCallToolPkgWasm",
            "__operitNativeImageProcessing",
            "__operitNativeComposeWebViewControllerCommandSuspend",
            "__operitNativeComposeFilePickerCommand",
            "__operitNativeIsPackageImported",
            "__operitNativeImportPackage",
            "__operitNativeRemovePackage",
            "__operitNativeUsePackage",
            "__operitNativeListImportedPackages",
            "__operitNativeResolveToolName",
        ];
        for name in asyncNames {
            let sink = asyncCallbackSink.clone();
            self.runtime
                .registerHostJavaScriptAsyncJsonFunction(
                    name,
                    Arc::new(move |requestId, arguments| {
                        let (result, reject) = match executeHostOperation(name, arguments) {
                            Ok(result) => (result, false),
                            Err(error) => (Value::String(error.to_string()), true),
                        };
                        sink(JsAsyncCallback::Promise {
                            requestId,
                            result,
                            reject,
                        });
                        Ok(())
                    }),
                )
                .map_err(|error| error.to_string())?;
        }
        let cancellationSink = asyncCallbackSink.clone();
        self.runtime
            .registerHostJavaScriptJsonFunction(
                "__operitNativeCancelJavaScriptPromises",
                Arc::new(move |arguments| {
                    let [scope] = exactHostJavaScriptJsonArguments(
                        "__operitNativeCancelJavaScriptPromises",
                        arguments,
                    )?;
                    cancellationSink(JsAsyncCallback::CancelScope {
                        scope: hostJavaScriptStringArgument(scope)?,
                    });
                    Ok(Value::Null)
                }),
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[allow(non_snake_case)]
    fn initJavaScriptEnvironment(&mut self) -> Result<(), String> {
        if self.jsEnvironmentInitialized {
            return Ok(());
        }
        let bootstrap = buildRuntimeBootstrapScript();
        self.evalJavaScriptVoid(&bootstrap)?;
        self.jsEnvironmentInitialized = true;
        Ok(())
    }
}

/// Validates the exact argument count of a structured host binding.
#[allow(non_snake_case)]
fn exactHostJavaScriptJsonArguments<const N: usize>(
    name: &str,
    arguments: Vec<Value>,
) -> HostResult<[Value; N]> {
    let count = arguments.len();
    arguments
        .try_into()
        .map_err(|_| HostError::new(format!("{name} requires {N} arguments, received {count}")))
}

/// Moves one structured string argument into the host request contract.
#[allow(non_snake_case)]
fn hostJavaScriptStringArgument(value: Value) -> HostResult<String> {
    match value {
        Value::String(value) => Ok(value),
        _ => Err(HostError::new("Host identity arguments must be strings")),
    }
}

/// Executes a validated structured request; only owned Rust values cross task boundaries.
/// Tools retain the ordinary executor: they may block or use exclusive TLS.
/// Only the JS continuation wake uses the cooperative reusable executor.
#[allow(non_snake_case)]
fn dispatchStructuredToolCall(
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    callbackSink: JsAsyncCallbackSink,
    requestId: u64,
    arguments: Vec<Value>,
) {
    let fail = |message: String| {
        callbackSink(JsAsyncCallback::Promise {
            requestId,
            result: serde_json::json!({"success": false, "message": message}),
            reject: false,
        })
    };
    let [toolType, toolName, params]: [Value; 3] = match arguments.try_into() {
        Ok(arguments) => arguments,
        Err(_) => {
            fail("Structured tool call requires three arguments".to_string());
            return;
        }
    };
    let (Some(toolType), Some(toolName)) = (toolType.as_str(), toolName.as_str()) else {
        fail("Tool type and name must be strings".to_string());
        return;
    };
    let request = match JsHostOperations::parseToolCallValue(toolType, toolName, params) {
        Ok(request) => request,
        Err(error) => {
            fail(error);
            return;
        }
    };
    let Some(executionHost) = executionHost else {
        fail("JavaScript execution host is unavailable".to_string());
        return;
    };
    let sink = callbackSink.clone();
    let scheduled = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
        "operit-js-tool-call",
        Box::new(move || {
            Box::pin(async move {
                let result = executionHost.execute_tool_call(request).await;
                sink(JsAsyncCallback::Promise {
                    requestId,
                    result: JsHostOperations::toolExecutionResultValue(result),
                    reject: false,
                });
            })
        }),
    );
    if let Err(error) = scheduled {
        fail(format!("Schedule JavaScript tool call failed: {error}"));
    }
}

/// Schedules one scoped JavaScript timer and resolves its host-owned Promise.
#[allow(non_snake_case)]
fn dispatchJavaScriptTimer(
    callbackSink: JsAsyncCallbackSink,
    requestId: u64,
    delayMs: u64,
) -> Result<(), String> {
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleDelayedHostRuntimeTask(
            "operit-js-timer",
            delayMs,
            Box::new(move || {
                callbackSink(JsAsyncCallback::Promise {
                    requestId,
                    result: Value::Null,
                    reject: false,
                });
            }),
        )
        .map_err(|error| format!("Schedule JavaScript timer failed: {error}"))
}

/// Builds one structured failure envelope without encoding JSON text.
#[allow(non_snake_case)]
fn buildToolPkgIpcFailure(message: &str) -> Value {
    serde_json::json!({"success": false, "message": message.trim()})
}

/// Submits owned IPC data and settles the source engine's scoped Promise.
#[allow(non_snake_case)]
fn dispatchToolPkgIpcAsync(
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    callbackSink: JsAsyncCallbackSink,
    requestId: u64,
    request: JsToolPkgIpcRequest,
) {
    let Some(executionHost) = executionHost else {
        callbackSink(JsAsyncCallback::Promise {
            requestId,
            result: buildToolPkgIpcFailure("JavaScript execution host is unavailable"),
            reject: false,
        });
        return;
    };
    let completionSink = callbackSink.clone();
    let submitted = executionHost.invoke_toolpkg_ipc_async(
        request,
        Box::new(move |result| {
            let result = match result {
                Ok(value) => serde_json::json!({"success": true, "value": value}),
                Err(error) => buildToolPkgIpcFailure(&error),
            };
            completionSink(JsAsyncCallback::Promise {
                requestId,
                result,
                reject: false,
            });
        }),
    );
    if let Err(error) = submitted {
        callbackSink(JsAsyncCallback::Promise {
            requestId,
            result: buildToolPkgIpcFailure(&format!("ToolPkg.ipc async dispatch failed: {error}")),
            reject: false,
        });
    }
}

/// Validates owned structured IPC data against the host routing contract.
#[allow(non_snake_case)]
fn buildToolPkgIpcRequest(
    packageTarget: String,
    callerContextKey: String,
    targetContextKey: String,
    targetRuntime: String,
    channel: String,
    payload: Value,
) -> Result<JsToolPkgIpcRequest, String> {
    let normalizedTarget = packageTarget.trim().to_string();
    if normalizedTarget.is_empty() {
        return Err("ToolPkg.ipc package target is empty".to_string());
    }
    let normalizedChannel = channel.trim().to_string();
    if normalizedChannel.is_empty() {
        return Err("ToolPkg.ipc channel is required".to_string());
    }
    let requestedRuntime = targetRuntime.trim().to_ascii_lowercase();
    if !requestedRuntime.is_empty()
        && requestedRuntime != "main"
        && requestedRuntime != "ui"
        && requestedRuntime != "sandbox"
        && requestedRuntime != "provider"
    {
        return Err(format!(
            "ToolPkg.ipc targetRuntime is invalid: {requestedRuntime}"
        ));
    }
    Ok(JsToolPkgIpcRequest {
        dependency_caller: None,
        package_target: normalizedTarget,
        caller_context_key: callerContextKey.trim().to_string(),
        target_context_key: normalizeOptionalString(&targetContextKey),
        target_runtime: normalizeOptionalString(&requestedRuntime),
        channel: normalizedChannel,
        payload,
    })
}

#[allow(non_snake_case)]
fn clearThreadLocalCallState() {
    CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow_mut().clear());
    CURRENT_EXECUTION_HOST.with(|host| {
        *host.borrow_mut() = None;
    });
    CURRENT_INTERMEDIATE_CALLBACK.with(|callback| {
        *callback.borrow_mut() = None;
    });
    CURRENT_EXECUTION_LISTENER.with(|listener| {
        *listener.borrow_mut() = None;
    });
    CURRENT_ENV_OVERRIDES.with(|overrides| {
        overrides.borrow_mut().clear();
    });
    CURRENT_TEXT_RESOURCE_SOURCE.with(|source| {
        *source.borrow_mut() = None;
    });
}

/// Installs one engine-owned call context for the next QuickJS execution step.
#[allow(non_snake_case)]
fn installThreadLocalCallContext(context: &JsCallContext) {
    CURRENT_EXECUTION_HOST.with(|host| {
        *host.borrow_mut() = context.executionHost.clone();
    });
    CURRENT_INTERMEDIATE_CALLBACK.with(|callback| {
        *callback.borrow_mut() = context.intermediateCallback.clone();
    });
    CURRENT_EXECUTION_LISTENER.with(|listener| {
        *listener.borrow_mut() = context.executionListener.clone();
    });
    CURRENT_ENV_OVERRIDES.with(|overrides| {
        *overrides.borrow_mut() = context
            .envOverrides
            .lock()
            .expect("JavaScript call environment mutex poisoned")
            .clone();
    });
    CURRENT_TEXT_RESOURCE_SOURCE.with(|source| {
        *source.borrow_mut() = Some(context.textResourceSource.clone());
    });
}

/// Selects the module owner from the engine's explicit bound package identity.
#[allow(non_snake_case)]
fn packageTextResourceSource(context: Option<&ToolPkgExecutionContext>) -> JsTextResourceSource {
    match context {
        Some(context) => JsTextResourceSource::Package(context.text_resource_host.clone()),
        None => JsTextResourceSource::ExecutionHost,
    }
}

#[allow(non_snake_case)]
fn hashText(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[allow(non_snake_case)]
fn summarizeText(value: &str) -> String {
    let preview = value.chars().take(240).collect::<String>();
    let escaped = preview.replace('\n', "\\n").replace('\r', "\\r");
    format!("len={} preview={}", value.len(), escaped)
}

#[allow(non_snake_case)]
fn summarizeOptionText(value: Option<&str>) -> String {
    match value {
        Some(value) => summarizeText(value),
        None => "none".to_string(),
    }
}

#[allow(non_snake_case)]
fn summarizeRegistrationResult(result: &Result<ToolPkgMainRegistrationCapture, String>) -> String {
    match result {
        Ok(capture) => format!(
            "ok toolboxUiModules={} routes={} hooks={} menus={}",
            capture.toolboxUiModules.len(),
            capture.uiRoutes.len(),
            capture.systemPromptComposeHooks.len(),
            capture.inputMenuTogglePlugins.len()
        ),
        Err(error) => format!("err {}", summarizeText(error)),
    }
}

#[allow(non_snake_case)]
fn summarizeParams(params: &BTreeMap<String, Value>) -> String {
    let keys = params.keys().cloned().collect::<Vec<_>>().join(",");
    let mut important = Vec::new();
    for key in [
        "__operit_execution_context_key",
        "__operit_toolpkg_subpackage_id",
        "containerPackageName",
        "toolPkgId",
        "__operit_ui_package_name",
        "__operit_script_screen",
        "__operit_inline_function_name",
        "__operit_toolpkg_runtime_kind",
        "__operit_registration_mode",
        "event",
        "eventName",
        "functionName",
    ] {
        if let Some(value) = params.get(key) {
            important.push(format!("{key}={}", summarizeJsonValue(value)));
        }
    }
    format!(
        "count={} keys=[{}] important=[{}]",
        params.len(),
        keys,
        important.join(";")
    )
}

#[allow(non_snake_case)]
fn summarizeJsonValue(value: &Value) -> String {
    match value {
        Value::String(text) => {
            let preview = text.chars().take(120).collect::<String>();
            format!(
                "str(len={},value={})",
                text.len(),
                preview.replace('\n', "\\n")
            )
        }
        _ => value.to_string(),
    }
}

impl JsEngine {
    /// Releases the engine worker and associated JavaScript runtime state.
    pub fn destroy(&self) {
        self.worker.destroy();
    }
}

impl JsExecutionEngine for JsEngine {
    /// Executes a named JavaScript function through this engine.
    #[allow(non_snake_case)]
    fn execute_script_function(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutSec: u64,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        JsEngine::execute_script_function(
            self,
            script,
            functionName,
            params,
            envOverrides,
            on_intermediate_result,
            dispatchIntermediateOnMain,
            timeoutSec,
            None,
        )
    }

    /// Executes a named JavaScript function through this engine with an exact millisecond deadline.
    #[allow(non_snake_case)]
    fn execute_script_function_with_timeout_millis(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutMillis: u64,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        JsEngine::execute_script_function_with_timeout_millis(
            self,
            script,
            functionName,
            params,
            envOverrides,
            on_intermediate_result,
            dispatchIntermediateOnMain,
            timeoutMillis,
            None,
        )
    }

    /// Executes a named JavaScript function through the asynchronous engine contract.
    #[allow(non_snake_case)]
    fn execute_script_function_async(
        &self,
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutMillis: u64,
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
        JsEngine::execute_script_function_async(
            self,
            script,
            functionName,
            params,
            envOverrides,
            on_intermediate_result,
            dispatchIntermediateOnMain,
            timeoutMillis,
            None,
        )
    }

    /// Executes a ToolPkg registration function through this engine.
    #[allow(non_snake_case)]
    fn execute_toolpkg_main_registration_function_with_text_resources(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<BTreeMap<String, String>>>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        JsEngine::execute_toolpkg_main_registration_function_with_text_resources(
            self,
            script,
            functionName,
            params,
            textResources,
        )
    }

    /// Executes one Compose DSL render script through this engine.
    #[allow(non_snake_case)]
    fn execute_compose_dsl_script(
        &self,
        script: &str,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        textResources: Arc<BTreeMap<String, String>>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<Value>>> {
        JsEngine::execute_compose_dsl_script(
            self,
            script,
            runtimeOptions,
            envOverrides,
            textResources,
        )
    }

    /// Executes one Compose DSL render through the asynchronous engine contract.
    #[allow(non_snake_case)]
    fn execute_compose_dsl_script_async(
        &self,
        script: String,
        runtimeOptions: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        textResources: Arc<BTreeMap<String, String>>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
        JsEngine::execute_compose_dsl_script_async(
            self,
            script,
            runtimeOptions,
            envOverrides,
            textResources,
        )
    }

    /// Dispatches one Compose DSL action through this engine.
    #[allow(non_snake_case)]
    fn dispatch_compose_dsl_action(
        &self,
        actionId: &str,
        payload: Option<Value>,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(Value) + Send + Sync>>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<Value>>> {
        JsEngine::execute_compose_dsl_action(
            self,
            actionId,
            payload,
            runtimeOptions,
            envOverrides,
            on_intermediate_result,
        )
    }

    /// Dispatches one Compose DSL action through the asynchronous engine contract.
    #[allow(non_snake_case)]
    fn dispatch_compose_dsl_action_result_async(
        &self,
        actionId: String,
        payload: Option<Value>,
        runtimeOptions: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(Value) + Send + Sync>>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
        JsEngine::dispatch_compose_dsl_action_result_async(
            self,
            actionId,
            payload,
            runtimeOptions,
            envOverrides,
            on_intermediate_result,
        )
    }

    /// Releases this engine's JavaScript resources.
    fn destroy(&self) {
        JsEngine::destroy(self);
    }
}

#[cfg(test)]
#[path = "tests/JsEngineTests.rs"]
mod JsEngineTests;
#[cfg(test)]
#[path = "tests/PluginConfigTests.rs"]
mod PluginConfigTests;

/// Delivers an external intermediate value only to the explicitly owning execution session.
#[allow(non_snake_case)]
fn deliverIntermediateResult(callId: String, result: String) -> HostResult<()> {
    let context = CURRENT_ACTIVE_CALL_CONTEXTS
        .with(|contexts| contexts.borrow().get(&callId).cloned())
        .ok_or_else(|| HostError::new("Intermediate result requires an active owning call"))?;
    if let Some(listener) = context.executionListener {
        listener.on_intermediate_result(&callId, &result);
    }
    if let Some(callback) = context.intermediateCallback {
        callback(Value::String(result));
    }
    Ok(())
}

/// Adapts the ordinary-script result contract without interpreting DSL objects as text.
fn textScriptCompletion(completion: JsExecutionCompletion<JsExecutionResult<Option<Value>>>) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
    Box::pin(async move { textScriptResult(completion.await) })
}

/// Extracts the explicit text variant used by existing non-DSL callers.
fn textScriptResult(result: JsExecutionResult<Option<Value>>) -> JsExecutionResult<Option<String>> {
    match result? {
        None => Ok(None),
        Some(Value::String(text)) => match extractJsExecutionErrorMessage(Some(&text)) {
            Some(message) => Err(JsExecutionError::runtime(message)),
            None => Ok(Some(text)),
        },
        Some(_) => Err(JsExecutionError::serialization("Ordinary script returned a structured DSL result")),
    }
}

/// Preserves text callbacks for ordinary scripts while keeping the internal channel structured.
fn textIntermediateCallback(callback: Option<Arc<dyn Fn(String) + Send + Sync>>) -> Option<Arc<dyn Fn(Value) + Send + Sync>> {
    callback.map(|callback| Arc::new(move |value| {
        let Value::String(text) = value else { panic!("Text intermediate callback received a structured value"); };
        callback(text);
    }) as Arc<dyn Fn(Value) + Send + Sync>)
}

/// Delivers DSL values to their active or detached execution owner without JSON text.
fn sendStructuredIntermediate(callId: &str, value: Value) {
    let active = CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow().get(callId).and_then(|context| context.intermediateCallback.clone()));
    let detached = CURRENT_DETACHED_INTERMEDIATE_CALLBACKS.with(|callbacks| callbacks.borrow().get(callId).cloned());
    match (active, detached) {
        (Some(callback), _) => callback(value),
        (None, Some(callback)) => callback(value),
        (None, None) => {},
    }
}

#[allow(non_snake_case)]
fn nativeSendIntermediateResultString(callId: String, result: String) {
    if let Some(context) =
        CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow().get(&callId).cloned())
    {
        if let Some(listener) = context.executionListener {
            listener.on_intermediate_result(&callId, &result);
        }
        if let Some(callback) = context.intermediateCallback {
            callback(Value::String(result));
        }
        return;
    }

    let detachedCallback = CURRENT_DETACHED_INTERMEDIATE_CALLBACKS
        .with(|callbacks| callbacks.borrow().get(&callId).cloned());
    CURRENT_EXECUTION_LISTENER.with(|listener| {
        if let Some(listener) = listener.borrow().as_ref() {
            listener.on_intermediate_result(&callId, &result);
        }
    });
    CURRENT_INTERMEDIATE_CALLBACK.with(|callback| {
        if let Some(callback) = callback.borrow().as_ref() {
            callback(Value::String(result));
            return;
        }
        if let Some(callback) = detachedCallback {
            callback(Value::String(result));
        }
    });
}

/// Executes one named host capability using typed structured arguments and results.
#[allow(non_snake_case)]
fn executeHostOperation(name: &str, arguments: Vec<Value>) -> HostResult<Value> {
    match name {
        "__operitNativeReadToolPkgTextResource" => {
            let [package, path] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let package = hostJavaScriptStringArgument(package)?;
            let path = hostJavaScriptStringArgument(path)?;
            let source = CURRENT_TEXT_RESOURCE_SOURCE
                .with(|current| current.borrow().clone())
                .ok_or_else(|| HostError::new("Module read requires an active resource owner"))?;
            let text = match source {
                JsTextResourceSource::Snapshot(resources) => Ok(resources
                    .get(&normalizeToolPkgTextResourcePath(&path))
                    .cloned()),
                JsTextResourceSource::Package(provider) => {
                    provider.read_toolpkg_text_resource(&package, &path)
                }
                JsTextResourceSource::ExecutionHost => currentExecutionHost()
                    .and_then(|host| host.read_toolpkg_text_resource(&package, &path)),
            }
            .map_err(HostError::new)?;
            Ok(text.map(Value::String).unwrap_or(Value::Null))
        }
        "__operitNativeReadToolPkgResource" => {
            let [package, key, output, internal] =
                exactHostJavaScriptJsonArguments(name, arguments)?;
            let request = JsToolPkgResourceRequest {
                package_name_or_subpackage_id: hostJavaScriptStringArgument(package)?,
                resource_key: hostJavaScriptStringArgument(key)?,
                output_file_name: normalizeOptionalString(&hostJavaScriptStringArgument(output)?),
                internal: hostJavaScriptBoolArgument(internal)?,
            };
            currentExecutionHost()
                .and_then(|host| host.materialize_toolpkg_resource(request))
                .map(Value::String)
                .map_err(HostError::new)
        }
        "__operitNativeCallToolPkgWasm" => {
            let [package, module, export, args] =
                exactHostJavaScriptJsonArguments(name, arguments)?;
            let request = JsToolPkgWasmRequest {
                package_target: hostJavaScriptStringArgument(package)?,
                module_id: hostJavaScriptStringArgument(module)?,
                export_name: hostJavaScriptStringArgument(export)?,
                args: serde_json::from_value::<Vec<JsToolPkgWasmArg>>(args)
                    .map_err(|error| HostError::new(error.to_string()))?,
            };
            if request.package_target.trim().is_empty()
                || request.module_id.trim().is_empty()
                || request.export_name.trim().is_empty()
            {
                return Err(HostError::new(
                    "ToolPkg WASM package, module and export are required",
                ));
            }
            currentExecutionHost()
                .and_then(|host| host.call_toolpkg_wasm(request))
                .map(|result| result.value)
                .map_err(HostError::new)
        }
        "__operitNativeComposeWebViewControllerCommand"
        | "__operitNativeComposeWebViewControllerCommandSuspend" => {
            let [payload] = exactHostJavaScriptJsonArguments(name, arguments)?;
            requireHostObject(&payload)?;
            currentExecutionHost()
                .and_then(|host| host.handle_compose_webview_controller_command(&payload))
                .map_err(HostError::new)
        }
        "__operitNativeComposeFilePickerCommand" => {
            let [payload] = exactHostJavaScriptJsonArguments(name, arguments)?;
            requireHostObject(&payload)?;
            currentExecutionHost()
                .and_then(|host| host.open_compose_file_picker(&payload))
                .map_err(HostError::new)
        }
        "__operitNativeGetEnvForCall" => {
            let [callId, key] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let callId = hostJavaScriptStringArgument(callId)?;
            let key = hostJavaScriptStringArgument(key)?;
            let context = CURRENT_ACTIVE_CALL_CONTEXTS
                .with(|contexts| contexts.borrow().get(&callId).cloned())
                .ok_or_else(|| {
                    HostError::new("Environment request requires an active owning call")
                })?;
            let overrides = context
                .envOverrides
                .lock()
                .expect("JavaScript call environment mutex poisoned");
            if let Some(value) = overrides.get(key.trim()) {
                return Ok(Value::String(value.clone()));
            }
            drop(overrides);
            context
                .executionHost
                .ok_or_else(|| HostError::new("JavaScript execution host is unavailable"))?
                .read_environment_variable(&key)
                .map(|value| value.map(Value::String).unwrap_or(Value::Null))
                .map_err(HostError::new)
        }
        "__operitNativeSetEnv" => {
            let [callId, key, value] = exactHostJavaScriptJsonArguments(name, arguments)?;
            setHostEnvironmentValue(
                &hostJavaScriptStringArgument(callId)?,
                &hostJavaScriptStringArgument(key)?,
                &hostJavaScriptStringArgument(value)?,
            )?;
            Ok(Value::Null)
        }
        "__operitNativeSetEnvs" => {
            let [callId, values] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let callId = hostJavaScriptStringArgument(callId)?;
            let values = values
                .as_object()
                .ok_or_else(|| HostError::new("setEnvs requires an object"))?;
            for (key, value) in values {
                let value = match value {
                    Value::String(value) => value.clone(),
                    Value::Null => String::new(),
                    value => value.to_string(),
                };
                setHostEnvironmentValue(&callId, key, &value)?;
            }
            Ok(Value::Null)
        }
        "__operitNativeGetScopedPluginConfigDir" => {
            let [owner, plugin] = exactHostJavaScriptJsonArguments(name, arguments)?;
            scopedPluginConfigDirectory(
                hostJavaScriptStringArgument(owner)?,
                hostJavaScriptStringArgument(plugin)?,
            )
            .map(Value::String)
            .map_err(HostError::new)
        }
        "__operitNativeIsPackageImported"
        | "__operitNativeImportPackage"
        | "__operitNativeRemovePackage"
        | "__operitNativeUsePackage" => {
            let [package] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let package = hostJavaScriptStringArgument(package)?;
            let host = currentExecutionHost().map_err(HostError::new)?;
            match name {
                "__operitNativeIsPackageImported" => {
                    host.is_package_imported(package.trim()).map(Value::Bool)
                }
                "__operitNativeImportPackage" => {
                    host.import_package(package.trim()).map(Value::String)
                }
                "__operitNativeRemovePackage" => {
                    host.remove_package(package.trim()).map(Value::String)
                }
                "__operitNativeUsePackage" => host.use_package(package.trim()).map(Value::String),
                _ => unreachable!(),
            }
            .map_err(HostError::new)
        }
        "__operitNativeListImportedPackages" => {
            let [] = exactHostJavaScriptJsonArguments(name, arguments)?;
            currentExecutionHost()
                .and_then(|host| host.list_imported_packages())
                .map(|values| Value::Array(values.into_iter().map(Value::String).collect()))
                .map_err(HostError::new)
        }
        "__operitNativeGetToolCatalog" => {
            let [] = exactHostJavaScriptJsonArguments(name, arguments)?;
            currentExecutionHost()
                .and_then(|host| host.get_tool_catalog())
                .map_err(HostError::new)
        }
        "__operitNativeResolveToolName" => {
            let [package, subpackage, tool, preferImported] =
                exactHostJavaScriptJsonArguments(name, arguments)?;
            let request = JsToolNameResolutionRequest {
                package_name: normalizeOptionalString(&hostJavaScriptStringArgument(package)?),
                subpackage_id: normalizeOptionalString(&hostJavaScriptStringArgument(subpackage)?),
                tool_name: hostJavaScriptStringArgument(tool)?,
                prefer_imported: hostJavaScriptBoolArgument(preferImported)?,
            };
            currentExecutionHost()
                .and_then(|host| host.resolve_tool_name(request))
                .map(Value::String)
                .map_err(HostError::new)
        }
        "__operitNativeDecompress" => {
            let [data, algorithm] = exactHostJavaScriptJsonArguments(name, arguments)?;
            JsHostOperations::decompress(
                &hostJavaScriptStringArgument(data)?,
                &hostJavaScriptStringArgument(algorithm)?,
            )
            .map(Value::String)
            .map_err(HostError::new)
        }
        "__operitNativeCrypto" => {
            let [algorithm, operation, args] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let args: Vec<String> =
                serde_json::from_value(args).map_err(|error| HostError::new(error.to_string()))?;
            JsHostOperations::crypto(
                &hostJavaScriptStringArgument(algorithm)?,
                &hostJavaScriptStringArgument(operation)?,
                &args,
            )
            .map(Value::String)
            .map_err(HostError::new)
        }
        "__operitNativeImageProcessing" => {
            let [operation, args] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let args = hostJavaScriptArrayArgument(args)?;
            JsHostOperations::imageProcessing(&hostJavaScriptStringArgument(operation)?, &args)
                .map_err(HostError::new)
        }
        "__operitNativeJavaClassExists" => {
            let [class] = exactHostJavaScriptJsonArguments(name, arguments)?;
            Ok(Value::Bool(javaClassExists(&hostJavaScriptStringArgument(
                class,
            )?)))
        }
        "__operitNativeJavaGetApplicationContext" => {
            let [] = exactHostJavaScriptJsonArguments(name, arguments)?;
            Ok(javaGetApplicationContext())
        }
        "__operitNativeJavaGetCurrentActivity" => {
            let [] = exactHostJavaScriptJsonArguments(name, arguments)?;
            Err(HostError::new("current activity is null"))
        }
        "__operitNativeJavaNewInstance" => {
            let [class, args] = exactHostJavaScriptJsonArguments(name, arguments)?;
            javaNewInstance(
                &hostJavaScriptStringArgument(class)?,
                &hostJavaScriptArrayArgument(args)?,
            )
            .map_err(HostError::new)
        }
        "__operitNativeJavaCallStatic" => {
            let [class, method, args] = exactHostJavaScriptJsonArguments(name, arguments)?;
            javaCallStatic(
                &hostJavaScriptStringArgument(class)?,
                &hostJavaScriptStringArgument(method)?,
                &hostJavaScriptArrayArgument(args)?,
            )
            .map_err(HostError::new)
        }
        "__operitNativeJavaCallInstance" => {
            let [handle, method, args] = exactHostJavaScriptJsonArguments(name, arguments)?;
            javaCallInstance(
                &hostJavaScriptStringArgument(handle)?,
                &hostJavaScriptStringArgument(method)?,
                &hostJavaScriptArrayArgument(args)?,
            )
            .map_err(HostError::new)
        }
        "__operitNativeJavaCallStaticSuspend"
        | "__operitNativeJavaGetStaticField"
        | "__operitNativeJavaSetStaticField" => Err(HostError::new(format!(
            "Java host operation is not implemented: {name}"
        ))),
        "__operitNativeLog" => {
            let [level, callId, message] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let level = hostJavaScriptStringArgument(level)?;
            let callId = hostJavaScriptStringArgument(callId)?;
            let message = hostJavaScriptStringArgument(message)?;
            let message = if callId.is_empty() {
                message
            } else {
                format!("[{callId}] {message}")
            };
            match level.as_str() {
                "info" => AppLogger::i("ToolPkg", &message),
                "warn" => AppLogger::w("ToolPkg", &message),
                "error" => AppLogger::e("ToolPkg", &message),
                _ => return Err(HostError::new(format!("Unknown plugin log level: {level}"))),
            };
            Ok(Value::Null)
        }
        "__operitSendIntermediateResult"
        | "__operitNativeSetCallResult"
        | "__operitNativeSetCallError" => {
            let [callId, value] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let callId = hostJavaScriptStringArgument(callId)?;
            // Execution listeners and the external script result contract own JSON serialization.
            let serialized = value.to_string();
            match name {
                "__operitSendIntermediateResult" => deliverIntermediateResult(
                    callId,
                    match value {
                        Value::String(value) => value,
                        _ => serialized,
                    },
                )?,
                "__operitNativeSetCallResult" => storeExecutionResult(callId, serialized)?,
                "__operitNativeSetCallError" => storeExecutionError(callId, serialized)?,
                _ => unreachable!(),
            }
            Ok(Value::Null)
        }
        "__operitNativeNotifyDetachedCall" => {
            let [callId] = exactHostJavaScriptJsonArguments(name, arguments)?;
            hostJavaScriptStringArgument(callId)?;
            Ok(Value::Null)
        }
        "__operitNativeLogJsExecutionTrace" => {
            let [callId, message] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let callId = hostJavaScriptStringArgument(callId)?;
            let message = hostJavaScriptStringArgument(message)?;
            AppLogger::d(TAG, &format!("[{callId}] {message}"));
            Ok(Value::Null)
        }
        _ => Err(HostError::new(format!(
            "Unknown structured host operation: {name}"
        ))),
    }
}

/// Requires a structured object without accepting serialized JSON text.
#[allow(non_snake_case)]
fn requireHostObject(value: &Value) -> HostResult<()> {
    if value.is_object() {
        Ok(())
    } else {
        Err(HostError::new("Host payload must be an object"))
    }
}

/// Extracts an exact boolean value from a structured request.
#[allow(non_snake_case)]
fn hostJavaScriptBoolArgument(value: Value) -> HostResult<bool> {
    match value {
        Value::Bool(value) => Ok(value),
        _ => Err(HostError::new("Host argument must be a boolean")),
    }
}

/// Moves an owned array from the structured request into the host operation.
#[allow(non_snake_case)]
fn hostJavaScriptArrayArgument(value: Value) -> HostResult<Vec<Value>> {
    match value {
        Value::Array(values) => Ok(values),
        _ => Err(HostError::new("Host argument must be an array")),
    }
}

/// Normalizes a package resource key according to the package resource contract.
#[allow(non_snake_case)]
fn normalizeToolPkgTextResourcePath(path: &str) -> String {
    path.replace('\\', "/")
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

/// Stores an externally serialized execution result for the owning call.
#[allow(non_snake_case)]
fn storeExecutionResult(callId: String, result: String) -> HostResult<()> {
    if !CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow().contains_key(&callId)) {
        return Err(HostError::new("Completion requires an active owning call"));
    }
    CURRENT_CALL_RESULTS.with(|results| {
        results.borrow_mut().insert(callId, Ok(Value::String(result)));
    });
    Ok(())
}

/// Reports and stores an execution error through the external listener contract.
#[allow(non_snake_case)]
fn storeExecutionError(callId: String, error: String) -> HostResult<()> {
    let context = CURRENT_ACTIVE_CALL_CONTEXTS
        .with(|contexts| contexts.borrow().get(&callId).cloned())
        .ok_or_else(|| HostError::new("Execution error requires an active owning call"))?;
    if let Some(listener) = context.executionListener {
        listener.on_failed(&callId, &error);
    }
    CURRENT_CALL_RESULTS.with(|results| {
        results.borrow_mut().insert(callId, Err(JsExecutionError::runtime(extractJsExecutionErrorMessage(Some(&error)).expect("Native error must contain a structured error message"))));
    });
    Ok(())
}

/// Writes one environment value and records it only after the host accepts the write.
#[allow(non_snake_case)]
fn setHostEnvironmentValue(callId: &str, key: &str, value: &str) -> HostResult<()> {
    let key = key.trim();
    if key.is_empty() {
        return Err(HostError::new("Environment variable name is required"));
    }
    let context = CURRENT_ACTIVE_CALL_CONTEXTS
        .with(|contexts| contexts.borrow().get(callId).cloned())
        .ok_or_else(|| HostError::new("Environment write requires an active owning call"))?;
    context
        .executionHost
        .ok_or_else(|| HostError::new("JavaScript execution host is unavailable"))?
        .write_environment_variable(key, value)
        .map_err(HostError::new)?;
    context
        .envOverrides
        .lock()
        .expect("JavaScript call environment mutex poisoned")
        .insert(key.to_string(), value.to_string());
    CURRENT_ENV_OVERRIDES.with(|overrides| {
        overrides
            .borrow_mut()
            .insert(key.to_string(), value.to_string());
    });
    Ok(())
}

/// Resolves and validates a package-owned configuration directory through its explicit scope.
#[allow(non_snake_case)]
fn scopedPluginConfigDirectory(ownerId: String, pluginId: String) -> Result<String, String> {
    let path = currentExecutionHost().and_then(|host| {
        CURRENT_REGISTRATION_CONFIG_PARAMS.with(|current| match current.borrow().as_ref() {
            Some(params) => {
                let owner = params
                    .get("toolPkgId")
                    .and_then(Value::as_str)
                    .ok_or("ToolPkg registration configuration owner is missing")?;
                if owner != ownerId {
                    return Err(
                        "ToolPkg registration configuration owner does not match its context"
                            .to_string(),
                    );
                }
                let scope = params
                    .get("__operit_registration_config_scope")
                    .ok_or("ToolPkg registration configuration scope is missing")?;
                let scope: ToolPkgConfigScope =
                    serde_json::from_value(scope.clone()).map_err(|error| {
                        format!("Invalid ToolPkg registration configuration scope: {error}")
                    })?;
                host.registration_plugin_config_dir(owner, &pluginId, scope)
            }
            None => host.scoped_plugin_config_dir(&ownerId, &pluginId),
        })
    })?;
    if !path.starts_with('/') {
        return Err("Plugin configuration directory must be an absolute VFS path".to_string());
    }
    Ok(path)
}

/// Returns the execution host bound to the active JavaScript call.
#[allow(non_snake_case)]
fn currentExecutionHost() -> Result<Arc<dyn JsExecutionHost>, String> {
    CURRENT_EXECUTION_HOST.with(|host| {
        host.borrow()
            .clone()
            .ok_or_else(|| "JavaScript execution host is unavailable".to_string())
    })
}

/// Converts a trimmed non-empty string into an optional contract value.
#[allow(non_snake_case)]
fn normalizeOptionalString(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[allow(non_snake_case)]
fn normalizeNonBlankString(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[allow(non_snake_case)]
fn parseBooleanFlag(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

#[allow(non_snake_case)]
fn readNativeExecutionSession(callId: &str) -> Option<JsExecutionResult<Value>> {
    CURRENT_CALL_RESULTS.with(|results| results.borrow().get(callId).cloned())
}

#[allow(non_snake_case)]
fn clearNativeExecutionSession(callId: &str) {
    CURRENT_CALL_RESULTS.with(|results| {
        results.borrow_mut().remove(callId);
    });
}

#[allow(non_snake_case)]
fn ensureRegistrationExecutionSucceeded(output: &str) -> Result<(), String> {
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "undefined" {
        return Ok(());
    }
    let value = serde_json::from_str::<Value>(trimmed).map_err(|error| error.to_string())?;
    if value
        .get("success")
        .and_then(Value::as_bool)
        .is_some_and(|success| !success)
    {
        let message = value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("ToolPkg registration failed");
        return Err(message.to_string());
    }
    Ok(())
}

/// Reads the owner exclusively from the immutable native engine context, never from tool parameters.
#[allow(non_snake_case)]
fn chatExtensionExecutionOwner(context: Option<&ToolPkgExecutionContext>) -> Result<&str, String> {
    let context = context.ok_or("Chat extensions require a real ToolPkg execution owner")?;
    if context.context_key.trim().is_empty() || context.container_package_name.trim().is_empty() {
        return Err("Chat extension execution context is incomplete".to_string());
    }
    Ok(&context.container_package_name)
}

/// Stores private pull handles inside one authenticated engine; authors never receive execution identities.
type ChatSendStreamRegistry = Arc<Mutex<BTreeMap<String, operit_plugin_sdk::js_sdk::JsAsyncIterable<operit_plugin_sdk::js_sdk::chat::ChatSendEvent>>>>;

/// Dispatches the current send contract and private iterator pulls through the existing host callback transport.
#[allow(non_snake_case)]
fn dispatchChatSend(
    executionHost: Option<Arc<dyn JsExecutionHost>>, callbackSink: JsAsyncCallbackSink,
    streams: ChatSendStreamRegistry, callbackId: String, method: String, payload: String,
) -> Result<(), String> {
    use operit_plugin_sdk::js_sdk::{JsFuture, JsHostError};
    use operit_plugin_sdk::js_sdk::chat::ChatSendRequest;
    if callbackId.trim().is_empty() { return Err("Chat callback identity is empty".to_string()); }
    let host = executionHost.ok_or("Chat execution host is unavailable")?;
    // Invoke host admission/cancellation now, not inside a queued task that might target a newer send.
    let operation: JsFuture<Value> = match method.as_str() {
        "send" => {
            let request: ChatSendRequest = serde_json::from_str(&payload).map_err(|error| error.to_string())?;
            request.validate()?;
            let send = host.sendMessage(request);
            Box::pin(async move { send.await.and_then(|result| serde_json::to_value(result).map_err(|error| JsHostError::new(error.to_string()))) })
        }
        "open" => {
            let request: ChatSendRequest = serde_json::from_str(&payload).map_err(|error| error.to_string())?;
            request.validate()?;
            let stream = host.sendMessageStreaming(request);
            let streamId = Uuid::new_v4().to_string();
            streams.lock().map_err(|_| "Chat observation registry mutex poisoned".to_string())?.insert(streamId.clone(), stream);
            Box::pin(async move { Ok(Value::String(streamId)) })
        }
        "next" => {
            let streamId: String = serde_json::from_str(&payload).map_err(|error| error.to_string())?;
            let stream = streams.lock().map_err(|_| "Chat observation registry mutex poisoned".to_string())?
                .get(&streamId).cloned().ok_or_else(|| "Unknown Chat observation".to_string())?;
            Box::pin(async move { stream.next().await.and_then(|event| serde_json::to_value(event).map_err(|error| JsHostError::new(error.to_string()))) })
        }
        "close" => {
            let streamId: String = serde_json::from_str(&payload).map_err(|error| error.to_string())?;
            let stream = streams.lock().map_err(|_| "Chat observation registry mutex poisoned".to_string())?
                .remove(&streamId).ok_or_else(|| "Unknown Chat observation".to_string())?;
            let close = stream.close();
            Box::pin(async move { close.await.map(|()| Value::Null) })
        }
        "cancel" => {
            let chatId: String = serde_json::from_str(&payload).map_err(|error| error.to_string())?;
            let cancel = host.cancel(chatId);
            Box::pin(async move { cancel.await.and_then(|result| serde_json::to_value(result).map_err(|error| JsHostError::new(error.to_string()))) })
        }
        _ => return Err(format!("Unknown native Chat operation: {method}")),
    };
    defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask("operit-chat-send-bridge", Box::new(move || Box::pin(async move {
        let (result, isError) = match operation.await {
            Ok(value) => (value.to_string(), false),
            Err(error) => (serde_json::json!({"message": error.message}).to_string(), true),
        };
        callbackSink(JsAsyncCallback { callbackId, result, isError });
    }))).map_err(|error| error.to_string())
}

/// Dispatches one typed owner-authenticated record operation through the already-bound execution host.
#[allow(non_snake_case)]
fn dispatchChatExtension(
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    callbackSink: JsAsyncCallbackSink,
    callbackId: String,
    method: String,
    targetJson: String,
    valueJson: String,
) -> Result<(), String> {
    use operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget;
    use operit_plugin_sdk::js_sdk::core::JsonObject;
    if callbackId.trim().is_empty() {
        return Err("Chat extension callback ID is empty".to_string());
    }
    let executionHost = executionHost.ok_or("Chat extension execution host is unavailable")?;
    let target: ChatExtensionTarget =
        serde_json::from_str(&targetJson).map_err(|error| error.to_string())?;
    target.validate()?;
    let value = match method.as_str() {
        "writeExtension" => Some(
            serde_json::from_str::<JsonObject>(&valueJson).map_err(|error| error.to_string())?,
        ),
        "readExtension" | "deleteExtension" => {
            if serde_json::from_str::<Value>(&valueJson).map_err(|error| error.to_string())?
                != Value::Null
            {
                return Err(
                    "Chat extension read/delete does not accept a value or namespace".to_string(),
                );
            }
            None
        }
        _ => return Err(format!("Unknown Chat extension method: {method}")),
    };
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "operit-chat-extension",
            Box::new(move || {
                Box::pin(async move {
                    let result = match method.as_str() {
                        "readExtension" => {
                            executionHost.readExtension(target).await.and_then(|value| {
                                serde_json::to_value(value).map_err(|error| {
                                    operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string())
                                })
                            })
                        }
                        "writeExtension" => executionHost
                            .writeExtension(
                                target,
                                value.expect("validated write operation has an object"),
                            )
                            .await
                            .and_then(|value| {
                                serde_json::to_value(value).map_err(|error| {
                                    operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string())
                                })
                            }),
                        "deleteExtension" => {
                            executionHost.deleteExtension(target).await.map(Value::Bool)
                        }
                        _ => unreachable!("method was validated before scheduling"),
                    };
                    let (result, isError) = match result {
                        Ok(value) => (value.to_string(), false),
                        Err(error) => (
                            serde_json::json!({"message": error.message}).to_string(),
                            true,
                        ),
                    };
                    callbackSink(JsAsyncCallback {
                        callbackId,
                        result,
                        isError,
                    });
                })
            }),
        )
        .map_err(|error| error.to_string())
}

/// Reads only a declared configuration directory through the typed SoftwareSettingsHost compatibility layer.
#[allow(non_snake_case)]
fn dispatchSoftwareSettingsDirectoryRead(
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    callbackSink: JsAsyncCallbackSink,
    callbackId: String,
    method: String,
) -> Result<(), String> {
    if callbackId.trim().is_empty() {
        return Err("SoftwareSettings callback ID is empty".to_string());
    }
    let executionHost =
        executionHost.ok_or_else(|| "JavaScript execution host is unavailable".to_string())?;
    if !matches!(
        method.as_str(),
        "listModelSummaries" | "listTtsConfigs" | "readToolSourceCatalog" | "listThemeConfigs" | "getCurrentTtsConfigId"
    ) {
        return Err(format!(
            "Unknown SoftwareSettings directory method: {method}"
        ));
    }
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "operit-software-settings-directory",
            Box::new(move || {
                Box::pin(async move {
                    let result = match method.as_str() {
                        "listModelSummaries" => {
                            executionHost
                                .listModelSummaries()
                                .await
                                .and_then(|records| {
                                    serde_json::to_value(records).map_err(|error| {
                                        operit_plugin_sdk::js_sdk::JsHostError::new(
                                            error.to_string(),
                                        )
                                    })
                                })
                        }
                        "listTtsConfigs" => {
                            executionHost.listTtsConfigs().await.and_then(|records| {
                                serde_json::to_value(records).map_err(|error| {
                                    operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string())
                                })
                            })
                        }
                        "listThemeConfigs" => executionHost.listThemeConfigs().await.and_then(|records| {
                            serde_json::to_value(records).map_err(|error| operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string()))
                        }),
                        "getCurrentTtsConfigId" => executionHost.getCurrentTtsConfigId().await.and_then(|id| {
                            serde_json::to_value(id).map_err(|error| operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string()))
                        }),
                        "readToolSourceCatalog" => executionHost
                            .readToolSourceCatalog()
                            .await
                            .and_then(|records| {
                                serde_json::to_value(records).map_err(|error| {
                                    operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string())
                                })
                            }),
                        _ => Err(operit_plugin_sdk::js_sdk::JsHostError::new(format!(
                            "Unknown SoftwareSettings directory method: {method}"
                        ))),
                    };
                    let (result, isError) = match result {
                        Ok(value) => (value.to_string(), false),
                        Err(error) => (
                            serde_json::json!({ "message": error.message }).to_string(),
                            true,
                        ),
                    };
                    callbackSink(JsAsyncCallback {
                        callbackId,
                        result,
                        isError,
                    });
                })
            }),
        )
        .map_err(|error| error.to_string())
}

/// Dispatches only the declared ordinary Theme or TTS configuration setter and returns its single original result.
#[allow(non_snake_case)]
fn dispatchSoftwareSettingsConfigApply(
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    callbackSink: JsAsyncCallbackSink,
    callbackId: String,
    method: String,
    id: String,
) -> Result<(), String> {
    if callbackId.trim().is_empty() {
        return Err("SoftwareSettings callback ID is empty".to_string());
    }
    if id.trim().is_empty() || id.trim() != id {
        return Err("Configuration ID must be exact nonblank text".to_string());
    }
    if !matches!(method.as_str(), "applyThemeConfig" | "setCurrentTtsConfigId") {
        return Err(format!("Unknown SoftwareSettings configuration method: {method}"));
    }
    let executionHost = executionHost.ok_or_else(|| "JavaScript execution host is unavailable".to_string())?;
    defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
        "operit-software-settings-config",
        Box::new(move || Box::pin(async move {
            let result = match method.as_str() {
                "applyThemeConfig" => executionHost.applyThemeConfig(id).await.and_then(|config| {
                    serde_json::to_value(config).map_err(|error| operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string()))
                }),
                "setCurrentTtsConfigId" => executionHost.setCurrentTtsConfigId(id).await.and_then(|current| {
                    serde_json::to_value(current).map_err(|error| operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string()))
                }),
                _ => Err(operit_plugin_sdk::js_sdk::JsHostError::new(format!("Unknown SoftwareSettings configuration method: {method}"))),
            };
            let (result, isError) = match result {
                Ok(value) => (value.to_string(), false),
                Err(error) => (serde_json::json!({"message": error.message}).to_string(), true),
            };
            callbackSink(JsAsyncCallback {callbackId, result, isError});
        })),
    ).map_err(|error| error.to_string())
}
