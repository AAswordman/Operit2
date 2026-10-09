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

use crate::javascript::JsJavaBridgeDelegates::{
    nativeJavaCallInstanceStrings, nativeJavaCallStaticString, nativeJavaClassExistsString,
    nativeJavaGetApplicationContextString, nativeJavaNewInstanceString,
};
use crate::javascript::JsLibraries::buildRuntimeBootstrapScript;
use crate::javascript::JsNativeInterfaceDelegates;
use operit_host_api::HostManager::{
    defaultHostJavaScriptRuntimeHost, defaultHostRuntimeTaskSchedulerHost,
};
use operit_host_api::TimeUtils::currentTimeMillisU128;
use operit_host_api::{
    HostError, HostErrorKind, HostJavaScriptExecutionInterrupt, HostJavaScriptInterruptHandler,
    HostJavaScriptRuntime, HostJavaScriptRuntimeHost, HostJavaScriptRuntimeStateHandle,
    HostJavaScriptRuntimeStateOutput, HostJavaScriptStringCallback, HostJavaScriptVoidCallback,
    HostResult,
};
use operit_plugin_sdk::execution_result::{
    build_js_execution_error_payload as buildJsExecutionErrorPayload,
    extract_js_execution_error_message as extractJsExecutionErrorMessage, JsExecutionError,
    JsExecutionResult,
};
use operit_plugin_sdk::javascript::{
    JsExecutionCompletion, JsExecutionEngine, JsExecutionFuture, JsExecutionHost, JsToolNameResolutionRequest,
    JsToolPkgIpcRequest, JsToolPkgResourceRequest, JsToolPkgWasmArg, JsToolPkgWasmRequest,
    ToolPkgConfigScope, ToolPkgExecutionContext, ToolPkgMainRegistrationCapture, ToolPkgTextResourceHost,
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
    static CURRENT_INTERMEDIATE_CALLBACK: RefCell<Option<Arc<dyn Fn(String) + Send + Sync>>> = RefCell::new(None);
    static CURRENT_EXECUTION_LISTENER: RefCell<Option<JsExecutionListenerRef>> = RefCell::new(None);
    static CURRENT_DETACHED_INTERMEDIATE_CALLBACKS: RefCell<BTreeMap<String, Arc<dyn Fn(String) + Send + Sync>>> =
        RefCell::new(BTreeMap::new());
    static CURRENT_ENV_OVERRIDES: RefCell<BTreeMap<String, String>> = RefCell::new(BTreeMap::new());
    static CURRENT_CALL_RESULTS: RefCell<BTreeMap<String, String>> = RefCell::new(BTreeMap::new());
    static CURRENT_TOOLPKG_TEXT_RESOURCES: RefCell<Option<Arc<ToolPkgTextResources>>> = RefCell::new(None);
    static CURRENT_TOOLPKG_TEXT_RESOURCE_HOST: RefCell<Option<Arc<dyn ToolPkgTextResourceHost>>> = RefCell::new(None);
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
    Legacy {
        callbackId: String,
        result: String,
        isError: bool,
    },
    Structured {
        requestId: u64,
        result: Value,
    },
}

type JsAsyncCallbackSink = Arc<dyn Fn(JsAsyncCallback) + Send + Sync>;
type JsBackgroundWake = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
struct JsCallContext {
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    intermediateCallback: Option<Arc<dyn Fn(String) + Send + Sync>>,
    executionListener: Option<JsExecutionListenerRef>,
    envOverrides: Arc<Mutex<BTreeMap<String, String>>>,
    textResources: Option<Arc<ToolPkgTextResources>>,
    textResourceHost: Option<Arc<dyn ToolPkgTextResourceHost>>,
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
    Arc<Mutex<Option<tokio::sync::oneshot::Sender<JsExecutionResult<Option<String>>>>>>;

/// Completes a call once without holding the result lock while waking its caller.
fn completeScriptExecution(
    completion: &JsScriptCompletion,
    result: JsExecutionResult<Option<String>>,
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
    onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
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
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        _dispatchIntermediateOnMain: bool,
        timeout: Duration,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        useComposeDslTextResources: bool,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
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
        self.worker.execute_script_function_async(
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
        self.worker.execute_script_function_async(
            script.to_owned(),
            functionName.to_owned(),
            params.clone(),
            envOverrides.clone(),
            on_intermediate_result,
            dispatchIntermediateOnMain,
            timeout,
            timeoutSec,
            executionListener,
            textResources,
            useComposeDslTextResources,
        )
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
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
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
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
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
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
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
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
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
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
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
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        self.execute_script_function_with_timeout(
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

    /// Executes one Compose DSL operation through the asynchronous engine contract.
    #[allow(non_snake_case)]
    fn executeComposeDslFunctionAsync(
        &self,
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
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
    type Item = String;

    /// Collects Compose DSL action events without blocking the collector task.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        Box::pin(async move {
            let intermediateEvents = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
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
fn composeDslActionEvent(phase: &str, error: Option<&str>, result: Option<&str>) -> String {
    let mut object = serde_json::Map::new();
    object.insert("phase".to_string(), Value::String(phase.to_string()));
    if let Some(error) = error {
        object.insert("error".to_string(), Value::String(error.to_string()));
    }
    if let Some(result) = result {
        object.insert("result".to_string(), Value::String(result.to_string()));
    }
    Value::Object(object).to_string()
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
        state.registerNativeInterface()?;
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
        let textResourceHost = self
            .toolPkgContext
            .as_ref()
            .map(|context| context.text_resource_host.clone());
        let context = JsCallContext {
            executionHost: self.executionHost.clone(),
            intermediateCallback: onIntermediateResult,
            executionListener,
            envOverrides: Arc::new(Mutex::new(envOverrides)),
            textResources,
            textResourceHost,
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
            let paramsJson = serde_json::to_string(&effectiveParams)
                .map_err(|error| JsExecutionError::serialization(error.to_string()))?;
            let scriptJson = serde_json::to_string(&script)
                .map_err(|error| JsExecutionError::serialization(error.to_string()))?;
            let functionNameJson = serde_json::to_string(&functionName)
                .map_err(|error| JsExecutionError::serialization(error.to_string()))?;
            let callIdJson = serde_json::to_string(&callId)
                .map_err(|error| JsExecutionError::serialization(error.to_string()))?;
            clearNativeExecutionSession(&callId);
            let executionScript = format!(
                "__operitExecuteScriptFunction({callIdJson}, {paramsJson}, {scriptJson}, {functionNameJson}, {timeoutSec}, 10000);"
            );
            if let Err(error) = self.evalJavaScriptVoid(&executionScript) {
                self.cancelJavaScriptExecution(&callId);
                return Err(JsExecutionError::runtime(error));
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
    fn execute_script_function_on_current_thread(
        &mut self,
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
            *callback.borrow_mut() = on_intermediate_result;
        });
        CURRENT_EXECUTION_LISTENER.with(|listener| {
            *listener.borrow_mut() = executionListener;
        });
        CURRENT_ENV_OVERRIDES.with(|overrides| {
            *overrides.borrow_mut() = envOverrides.clone();
        });
        CURRENT_TOOLPKG_TEXT_RESOURCE_HOST.with(|host| {
            *host.borrow_mut() = self
                .toolPkgContext
                .as_ref()
                .map(|context| context.text_resource_host.clone());
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

        let paramsJson = match serde_json::to_string(&effectiveParams) {
            Ok(value) => value,
            Err(error) => {
                clearThreadLocalCallState();
                return Err(JsExecutionError::serialization(error.to_string()));
            }
        };
        let scriptJson = serde_json::to_string(script).map_err(|error| {
            clearThreadLocalCallState();
            JsExecutionError::serialization(error.to_string())
        })?;
        let functionNameJson = serde_json::to_string(functionName).map_err(|error| {
            clearThreadLocalCallState();
            JsExecutionError::serialization(error.to_string())
        })?;
        let callId = format!(
            "operit_call_{}",
            Uuid::new_v4().to_string().replace('-', "")
        );
        let callIdJson = serde_json::to_string(&callId).map_err(|error| {
            clearThreadLocalCallState();
            JsExecutionError::serialization(error.to_string())
        })?;
        clearNativeExecutionSession(&callId);
        let executionScript = format!(
            "__operitExecuteScriptFunction({callIdJson}, {paramsJson}, {scriptJson}, {functionNameJson}, {timeoutSec}, 10000);"
        );
        let output = match self.evalJavaScriptVoid(&executionScript) {
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
        CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
            *resources.borrow_mut() = textResources;
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
            let paramsJson = serde_json::to_string(&registrationParams)
                .map_err(|error| JsExecutionError::serialization(error.to_string()))?;
            let scriptJson = serde_json::to_string(script)
                .map_err(|error| JsExecutionError::serialization(error.to_string()))?;
            let functionNameJson = serde_json::to_string(functionName)
                .map_err(|error| JsExecutionError::serialization(error.to_string()))?;
            let callId = format!(
                "operit_registration_{}",
                Uuid::new_v4().to_string().replace('-', "")
            );
            let callIdJson = serde_json::to_string(&callId)
                .map_err(|error| JsExecutionError::serialization(error.to_string()))?;
            clearNativeExecutionSession(&callId);
            let executionScript = format!(
                "__operitExecuteScriptFunction({callIdJson}, {paramsJson}, {scriptJson}, {functionNameJson}, 60, 10000);"
            );
            if let Err(error) = self.evalJavaScriptVoid(&executionScript) {
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

            let captureScript = r#"
            (function() {
                return JSON.stringify(globalThis.__operitToolPkgRegistrationCapture);
            })()
            "#;
            let captureJson = self
                .evalJavaScriptString(captureScript)
                .map_err(JsExecutionError::runtime)?;
            serde_json::from_str::<ToolPkgMainRegistrationCapture>(&captureJson)
                .map_err(|error| JsExecutionError::protocol(error.to_string()))
        })();
        CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
            *resources.borrow_mut() = None;
        });
        CURRENT_EXECUTION_HOST.with(|host| {
            *host.borrow_mut() = None;
        });
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
                        readNativeExecutionSession(&callId).map(|output| {
                            match extractJsExecutionErrorMessage(Some(&output)) {
                                Some(message) => Err(JsExecutionError::runtime(message)),
                                None => Ok(Some(output)),
                            }
                        })
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
            .evalJavaScriptString("JSON.stringify(typeof __operitGetDetachedCallIds === 'function' ? __operitGetDetachedCallIds() : [])")?;
        let ids: Vec<String> = serde_json::from_str(&ids).map_err(|error| error.to_string())?;
        let callbacks = ids
            .iter()
            .filter_map(|callId| {
                self.detachedCallContexts
                    .get(callId)
                    .and_then(|context| context.intermediateCallback.clone())
                    .map(|callback| (callId.clone(), callback))
            })
            .collect::<BTreeMap<_, _>>();
        CURRENT_DETACHED_INTERMEDIATE_CALLBACKS.with(|current| {
            *current.borrow_mut() = callbacks;
        });
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
            let callIdJson = serde_json::to_string(&callId).map_err(|error| error.to_string())?;
            let prepared = self.evalJavaScriptString(&format!(
                "JSON.stringify(typeof __operitPrepareDetachedCall === 'function' && __operitPrepareDetachedCall({callIdJson}))"
            ))?;
            if prepared == "true" {
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
            .evalJavaScriptString("JSON.stringify(typeof __operitGetDetachedCallIds === 'function' ? __operitGetDetachedCallIds() : [])")?;
        let activeIds: Vec<String> =
            serde_json::from_str(&activeIds).map_err(|error| error.to_string())?;
        let completedCallIds = self
            .detachedCallContexts
            .keys()
            .filter(|callId| !activeIds.iter().any(|activeId| activeId == *callId))
            .cloned()
            .collect::<Vec<_>>();
        for callId in completedCallIds {
            let callIdJson = serde_json::to_string(&callId).map_err(|error| error.to_string())?;
            self.evalJavaScriptVoid(&format!(
                "if (typeof __operitFinalizeDetachedCall === 'function') {{ __operitFinalizeDetachedCall({callIdJson}); }}"
            ))?;
        }
        self.detachedCallContexts
            .retain(|callId, _| activeIds.iter().any(|activeId| activeId == callId));
        CURRENT_DETACHED_INTERMEDIATE_CALLBACKS.with(|current| {
            current.borrow_mut().clear();
        });
        Ok(())
    }

    /// Retains the callback context while a JavaScript call owns detached timers.
    fn rememberDetachedCallContext(
        &mut self,
        pending: &JsPendingScriptExecution,
    ) -> Result<(), String> {
        let callIdJson =
            serde_json::to_string(&pending.callId).map_err(|error| error.to_string())?;
        let detached = self.evalJavaScriptString(&format!(
            "JSON.stringify((typeof __operitGetCallState === 'function' && __operitGetCallState({callIdJson}))?.detached === true)"
        ))?;
        if detached == "true" {
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
                return Ok(Some(output));
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

    /// Delivers one asynchronous host result to its JavaScript callback.
    #[allow(non_snake_case)]
    fn deliverAsyncCallback(&mut self, callback: JsAsyncCallback) -> Result<(), String> {
        match callback {
            JsAsyncCallback::Legacy {
                callbackId,
                result,
                isError,
            } => self
                .runtime
                .callHostJavaScriptFunction(
                    &callbackId,
                    &[Value::String(result), Value::Bool(isError)],
                )
                .map_err(|error| error.to_string()),
            // Business failures remain fulfilled result envelopes. The public
            // __operitParseToolResult still decides whether to throw, unchanged.
            JsAsyncCallback::Structured { requestId, result } => self
                .runtime
                .settleHostJavaScriptPromise(requestId, &result, false)
                .map_err(|error| error.to_string()),
        }
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

    #[allow(non_snake_case)]
    fn registerNativeInterface(&mut self) -> Result<(), String> {
        let stringFunctions: Vec<(&str, HostJavaScriptStringCallback)> = vec![
            (
                "__operitNativeReadToolPkgTextResource",
                Arc::new(|arguments| {
                    let [packageNameOrSubpackageId, resourcePath] = exactHostJavaScriptArguments(
                        "__operitNativeReadToolPkgTextResource",
                        arguments,
                    )?;
                    Ok(nativeReadToolPkgTextResourceStrings(
                        packageNameOrSubpackageId,
                        resourcePath,
                    ))
                }),
            ),
            (
                "__operitNativeReadToolPkgResource",
                Arc::new(|arguments| {
                    let [packageNameOrSubpackageId, resourceKey, outputFileName, internal] =
                        exactHostJavaScriptArguments(
                            "__operitNativeReadToolPkgResource",
                            arguments,
                        )?;
                    Ok(nativeReadToolPkgResourceStrings(
                        packageNameOrSubpackageId,
                        resourceKey,
                        outputFileName,
                        internal,
                    ))
                }),
            ),
            (
                "__operitNativeCallToolPkgWasm",
                Arc::new(|arguments| {
                    let [packageTarget, moduleId, exportName, argsJson] =
                        exactHostJavaScriptArguments("__operitNativeCallToolPkgWasm", arguments)?;
                    Ok(nativeCallToolPkgWasmStrings(
                        packageTarget,
                        moduleId,
                        exportName,
                        argsJson,
                    ))
                }),
            ),
            (
                "__operitNativeComposeWebViewControllerCommand",
                Arc::new(|arguments| {
                    let [payloadJson] = exactHostJavaScriptArguments(
                        "__operitNativeComposeWebViewControllerCommand",
                        arguments,
                    )?;
                    Ok(nativeComposeWebViewControllerCommandString(payloadJson))
                }),
            ),
            (
                "__operitNativeComposeFilePickerCommand",
                Arc::new(|arguments| {
                    let [payloadJson] = exactHostJavaScriptArguments(
                        "__operitNativeComposeFilePickerCommand",
                        arguments,
                    )?;
                    Ok(nativeComposeFilePickerCommandString(payloadJson))
                }),
            ),
            (
                "__operitNativeGetEnvForCall",
                Arc::new(|arguments| {
                    let [callId, key] =
                        exactHostJavaScriptArguments("__operitNativeGetEnvForCall", arguments)?;
                    let context = CURRENT_ACTIVE_CALL_CONTEXTS
                        .with(|contexts| contexts.borrow().get(&callId).cloned());
                    if let Some(context) = context {
                        if let Some(value) = context
                            .envOverrides
                            .lock()
                            .expect("JavaScript call environment mutex poisoned")
                            .get(key.trim())
                            .filter(|value| !value.is_empty())
                        {
                            return Ok(value.clone());
                        }
                        return Ok(context
                            .executionHost
                            .ok_or_else(|| "JavaScript execution host is unavailable".to_string())
                            .and_then(|host| host.read_environment_variable(&key))
                            .map(|value| value.unwrap_or_default())
                            .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error)));
                    }
                    Ok(nativeGetEnvForCallStrings(key))
                }),
            ),
            (
                "__operitNativeLog",
                Arc::new(|arguments| {
                    let [level, call_id, message] =
                        exactHostJavaScriptArguments("__operitNativeLog", arguments)?;
                    let message = if call_id.is_empty() {
                        message
                    } else {
                        format!("[{call_id}] {message}")
                    };
                    match level.as_str() {
                        "info" => AppLogger::i("ToolPkg", &message),
                        "warn" => AppLogger::w("ToolPkg", &message),
                        "error" => AppLogger::e("ToolPkg", &message),
                        _ => {
                            return Err(HostError::new(format!(
                                "Unknown plugin log level: {level}"
                            )))
                        }
                    };
                    Ok(String::new())
                }),
            ),
            (
                "__operitNativeSetEnv",
                Arc::new(|arguments| {
                    let [callId, key, value] =
                        exactHostJavaScriptArguments("__operitNativeSetEnv", arguments)?;
                    Ok(nativeSetEnvStrings(callId, key, value))
                }),
            ),
            (
                "__operitNativeSetEnvs",
                Arc::new(|arguments| {
                    let [callId, valuesJson] =
                        exactHostJavaScriptArguments("__operitNativeSetEnvs", arguments)?;
                    Ok(nativeSetEnvsStrings(callId, valuesJson))
                }),
            ),
            (
                "__operitNativeGetPluginConfigDir",
                Arc::new(|arguments| {
                    let [pluginId] = exactHostJavaScriptArguments(
                        "__operitNativeGetPluginConfigDir",
                        arguments,
                    )?;
                    Ok(nativeGetPluginConfigDirString(pluginId))
                }),
            ),
            (
                "__operitNativeGetScopedPluginConfigDir",
                Arc::new(|arguments| {
                    let [ownerId, pluginId] = exactHostJavaScriptArguments(
                        "__operitNativeGetScopedPluginConfigDir",
                        arguments,
                    )?;
                    Ok(nativeGetScopedPluginConfigDirString(ownerId, pluginId))
                }),
            ),
            (
                "__operitNativeIsPackageImported",
                Arc::new(|arguments| {
                    let [packageName] =
                        exactHostJavaScriptArguments("__operitNativeIsPackageImported", arguments)?;
                    Ok(nativeIsPackageImportedString(packageName))
                }),
            ),
            (
                "__operitNativeImportPackage",
                Arc::new(|arguments| {
                    let [packageName] =
                        exactHostJavaScriptArguments("__operitNativeImportPackage", arguments)?;
                    Ok(nativeImportPackageString(packageName))
                }),
            ),
            (
                "__operitNativeRemovePackage",
                Arc::new(|arguments| {
                    let [packageName] =
                        exactHostJavaScriptArguments("__operitNativeRemovePackage", arguments)?;
                    Ok(nativeRemovePackageString(packageName))
                }),
            ),
            (
                "__operitNativeUsePackage",
                Arc::new(|arguments| {
                    let [packageName] =
                        exactHostJavaScriptArguments("__operitNativeUsePackage", arguments)?;
                    Ok(nativeUsePackageString(packageName))
                }),
            ),
            (
                "__operitNativeListImportedPackagesJson",
                Arc::new(|arguments| {
                    let [] = exactHostJavaScriptArguments(
                        "__operitNativeListImportedPackagesJson",
                        arguments,
                    )?;
                    Ok(nativeListImportedPackagesJsonString())
                }),
            ),
            (
                "__operitNativeGetToolCatalogJson",
                Arc::new(|arguments| {
                    let [] = exactHostJavaScriptArguments(
                        "__operitNativeGetToolCatalogJson",
                        arguments,
                    )?;
                    Ok(nativeGetToolCatalogJsonString())
                }),
            ),
            (
                "__operitNativeResolveToolName",
                Arc::new(|arguments| {
                    let [packageName, subpackageId, toolName, preferImported] =
                        exactHostJavaScriptArguments("__operitNativeResolveToolName", arguments)?;
                    Ok(nativeResolveToolNameString(
                        packageName,
                        subpackageId,
                        toolName,
                        preferImported,
                    ))
                }),
            ),
            (
                "__operitNativeDecompress",
                Arc::new(|arguments| {
                    let [data, algorithm] =
                        exactHostJavaScriptArguments("__operitNativeDecompress", arguments)?;
                    Ok(nativeDecompressStrings(data, algorithm))
                }),
            ),
            (
                "__operitNativeCrypto",
                Arc::new(|arguments| {
                    let [algorithm, operation, argsJson] =
                        exactHostJavaScriptArguments("__operitNativeCrypto", arguments)?;
                    Ok(nativeCryptoStrings(algorithm, operation, argsJson))
                }),
            ),
            (
                "__operitNativeImageProcessing",
                Arc::new(|arguments| {
                    let [callbackId, operation, argsJson] =
                        exactHostJavaScriptArguments("__operitNativeImageProcessing", arguments)?;
                    Ok(nativeImageProcessingStrings(
                        callbackId, operation, argsJson,
                    ))
                }),
            ),
            (
                "__operitNativeJavaClassExists",
                Arc::new(|arguments| {
                    let [className] =
                        exactHostJavaScriptArguments("__operitNativeJavaClassExists", arguments)?;
                    Ok(nativeJavaClassExistsString(className))
                }),
            ),
            (
                "__operitNativeJavaGetApplicationContext",
                Arc::new(|arguments| {
                    let [] = exactHostJavaScriptArguments(
                        "__operitNativeJavaGetApplicationContext",
                        arguments,
                    )?;
                    Ok(nativeJavaGetApplicationContextString())
                }),
            ),
            (
                "__operitNativeJavaCallInstance",
                Arc::new(|arguments| {
                    let [instanceHandle, methodName, argsJson] =
                        exactHostJavaScriptArguments("__operitNativeJavaCallInstance", arguments)?;
                    Ok(nativeJavaCallInstanceStrings(
                        instanceHandle,
                        methodName,
                        argsJson,
                    ))
                }),
            ),
            (
                "__operitNativeJavaNewInstance",
                Arc::new(|arguments| {
                    let [className, _argsJson] =
                        exactHostJavaScriptArguments("__operitNativeJavaNewInstance", arguments)?;
                    Ok(nativeJavaNewInstanceString(className))
                }),
            ),
            (
                "__operitNativeJavaCallStatic",
                Arc::new(|arguments| {
                    let [className, methodName, _argsJson] =
                        exactHostJavaScriptArguments("__operitNativeJavaCallStatic", arguments)?;
                    Ok(nativeJavaCallStaticString(className, methodName))
                }),
            ),
        ];
        for (name, callback) in stringFunctions {
            self.runtime
                .registerHostJavaScriptStringFunction(name, callback)
                .map_err(|error| error.to_string())?;
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
        let structuredHost = self.executionHost.clone();
        let structuredSink = asyncCallbackSink.clone();
        self.runtime
            .registerHostJavaScriptAsyncJsonFunction(
                "__operitNativeCallToolStructured",
                Arc::new(move |requestId, arguments| {
                    dispatchStructuredToolCall(
                        structuredHost.clone(),
                        structuredSink.clone(),
                        requestId,
                        arguments,
                    );
                    Ok(())
                }),
            )
            .map_err(|error| error.to_string())?;
        let toolExecutionHost = self.executionHost.clone();
        let toolAsyncCallbackSink = asyncCallbackSink.clone();
        let timerCallbackSink = asyncCallbackSink.clone();
        let dependencyHost = self.executionHost.clone();
        let dependencySink = asyncCallbackSink.clone();
        let dependencyContext = self.toolPkgContext.clone();
        let ipcContext = self.toolPkgContext.clone();
        let voidFunctions: Vec<(&str, HostJavaScriptVoidCallback)> = vec![
            (
                "__operitSendIntermediateResult",
                Arc::new(|arguments| {
                    let [callId, result] =
                        exactHostJavaScriptArguments("__operitSendIntermediateResult", arguments)?;
                    nativeSendIntermediateResultString(callId, result);
                    Ok(())
                }),
            ),
            (
                "__operitNativeSetCallResult",
                Arc::new(move |arguments| {
                    let [callId, result] =
                        exactHostJavaScriptArguments("__operitNativeSetCallResult", arguments)?;
                    nativeSetCallResultStrings(callId, result);
                    Ok(())
                }),
            ),
            (
                "__operitNativeSetCallError",
                Arc::new(move |arguments| {
                    let [callId, error] =
                        exactHostJavaScriptArguments("__operitNativeSetCallError", arguments)?;
                    nativeSetCallErrorStrings(callId, error);
                    Ok(())
                }),
            ),
            (
                "__operitNativeNotifyDetachedCall",
                Arc::new(|arguments| {
                    let [_callId] = exactHostJavaScriptArguments(
                        "__operitNativeNotifyDetachedCall",
                        arguments,
                    )?;
                    Ok(())
                }),
            ),
            (
                "__operitNativeCallToolAsync",
                Arc::new(move |arguments| {
                    let [callbackId, toolType, toolName, paramsJson] =
                        exactHostJavaScriptArguments("__operitNativeCallToolAsync", arguments)?;
                    dispatchToolCallAsync(
                        toolExecutionHost.clone(),
                        toolAsyncCallbackSink.clone(),
                        callbackId,
                        toolType,
                        toolName,
                        paramsJson,
                    );
                    Ok(())
                }),
            ),
            (
                "__operitNativeScheduleJavaScriptTimer",
                Arc::new(move |arguments| {
                    let [callbackId, delayMs] = exactHostJavaScriptArguments(
                        "__operitNativeScheduleJavaScriptTimer",
                        arguments,
                    )?;
                    dispatchJavaScriptTimer(timerCallbackSink.clone(), callbackId, delayMs)
                        .map_err(HostError::new)
                }),
            ),
            (
                "__operitNativeInvokeToolPkgIpcAsync",
                Arc::new(move |arguments| {
                    let [callbackId, packageTarget, callerContextKey, targetContextKey, targetRuntime, channel, payloadJson] =
                        exactHostJavaScriptArguments(
                            "__operitNativeInvokeToolPkgIpcAsync",
                            arguments,
                        )?;
                    let context = ipcContext.as_ref().ok_or_else(|| {
                        HostError::new("ToolPkg IPC requires a bound package context")
                    })?;
                    if packageTarget != context.container_package_name {
                        return Err(HostError::new(
                            "Package-private IPC cannot target another package",
                        ));
                    }
                    dispatchToolPkgIpcAsync(
                        None,
                        executionHost.clone(),
                        asyncCallbackSink.clone(),
                        callbackId,
                        packageTarget,
                        callerContextKey,
                        targetContextKey,
                        targetRuntime,
                        channel,
                        payloadJson,
                    );
                    Ok(())
                }),
            ),
            (
                "__operitNativeCallDependencyAsync",
                Arc::new(move |arguments| {
                    let [callbackId, packageTarget, methodName, payloadJson] =
                        exactHostJavaScriptArguments(
                            "__operitNativeCallDependencyAsync",
                            arguments,
                        )?;
                    let context = dependencyContext.as_ref().ok_or_else(|| {
                        HostError::new("Dependency calls require a bound ToolPkg context")
                    })?;
                    dispatchToolPkgIpcAsync(
                        Some(context.container_package_name.clone()),
                        dependencyHost.clone(),
                        dependencySink.clone(),
                        callbackId,
                        packageTarget,
                        context.context_key.clone(),
                        String::new(),
                        "main".to_string(),
                        methodName,
                        payloadJson,
                    );
                    Ok(())
                }),
            ),
            (
                "__operitNativeLogJsExecutionTrace",
                Arc::new(|arguments| {
                    let [callId, message] = exactHostJavaScriptArguments(
                        "__operitNativeLogJsExecutionTrace",
                        arguments,
                    )?;
                    nativeLogJsExecutionTraceStrings(callId, message);
                    Ok(())
                }),
            ),
        ];
        for (name, callback) in voidFunctions {
            self.runtime
                .registerHostJavaScriptVoidFunction(name, callback)
                .map_err(|error| error.to_string())?;
        }
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

/// Submits one tool call through the Host task scheduler and reports completion to QuickJS.
#[allow(non_snake_case)]
fn dispatchToolCallAsync(
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    callbackSink: JsAsyncCallbackSink,
    callbackId: String,
    toolType: String,
    toolName: String,
    paramsJson: String,
) {
    let normalizedCallbackId = callbackId.trim().to_string();
    if normalizedCallbackId.is_empty() {
        return;
    }
    let Some(executionHost) = executionHost else {
        callbackSink(JsAsyncCallback::Legacy {
            callbackId: normalizedCallbackId,
            result: serde_json::json!({
                "success": false,
                "message": "JavaScript execution host is unavailable"
            })
            .to_string(),
            isError: true,
        });
        return;
    };
    let callbackSinkForTask = callbackSink.clone();
    let callbackIdForTask = normalizedCallbackId.clone();
    let scheduleResult = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
        "operit-js-tool-call",
        Box::new(move || {
            Box::pin(async move {
                let (result, isError) = JsNativeInterfaceDelegates::callToolSerialized(
                    executionHost.as_ref(),
                    &toolType,
                    &toolName,
                    &paramsJson,
                )
                .await;
                callbackSinkForTask(JsAsyncCallback::Legacy {
                    callbackId: callbackIdForTask,
                    result,
                    isError,
                });
            })
        }),
    );
    if let Err(error) = scheduleResult {
        callbackSink(JsAsyncCallback::Legacy {
            callbackId: normalizedCallbackId,
            result: serde_json::json!({
                "success": false,
                "message": format!("Schedule JavaScript tool call failed: {error}")
            })
            .to_string(),
            isError: true,
        });
    }
}

/// Executes a structured request with the same SDK validation and result envelope
/// as the legacy path; only owned Rust values cross task/thread boundaries.
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
        callbackSink(JsAsyncCallback::Structured {
            requestId,
            result: serde_json::json!({"success": false, "message": message}),
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
    let request = match JsNativeInterfaceDelegates::parseToolCallValue(toolType, toolName, params) {
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
                sink(JsAsyncCallback::Structured {
                    requestId,
                    result: JsNativeInterfaceDelegates::toolExecutionResultValue(result),
                });
            })
        }),
    );
    if let Err(error) = scheduled {
        fail(format!("Schedule JavaScript tool call failed: {error}"));
    }
}

/// Schedules one JavaScript timer through the platform Host task scheduler.
#[allow(non_snake_case)]
fn dispatchJavaScriptTimer(
    callbackSink: JsAsyncCallbackSink,
    callbackId: String,
    delayMs: String,
) -> Result<(), String> {
    let normalizedCallbackId = callbackId.trim().to_string();
    if normalizedCallbackId.is_empty() {
        return Err("JavaScript timer callback id is empty".to_string());
    }
    let delayMillis = delayMs
        .trim()
        .parse::<u64>()
        .map_err(|error| format!("JavaScript timer delay is invalid: {error}"))?;
    let callbackIdForTask = normalizedCallbackId.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleDelayedHostRuntimeTask(
            "operit-js-timer",
            delayMillis,
            Box::new(move || {
                callbackSink(JsAsyncCallback::Legacy {
                    callbackId: callbackIdForTask,
                    result: String::new(),
                    isError: false,
                });
            }),
        )
        .map_err(|error| format!("Schedule JavaScript timer failed: {error}"))
}

#[allow(non_snake_case)]
fn buildToolPkgIpcFailure(message: &str) -> String {
    serde_json::json!({
        "success": false,
        "message": message.trim()
    })
    .to_string()
}

/// Submits ToolPkg IPC through the execution host and reports completion to the source engine.
#[allow(non_snake_case)]
fn dispatchToolPkgIpcAsync(
    dependencyCaller: Option<String>,
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    callbackSink: JsAsyncCallbackSink,
    callbackId: String,
    packageTarget: String,
    callerContextKey: String,
    targetContextKey: String,
    targetRuntime: String,
    channel: String,
    payloadJson: String,
) {
    let normalizedCallbackId = callbackId.trim().to_string();
    if normalizedCallbackId.is_empty() {
        return;
    }
    let mut request = match buildToolPkgIpcRequest(
        packageTarget,
        callerContextKey,
        targetContextKey,
        targetRuntime,
        channel,
        payloadJson,
    ) {
        Ok(request) => request,
        Err(error) => {
            callbackSink(JsAsyncCallback::Legacy {
                callbackId: normalizedCallbackId,
                result: error,
                isError: false,
            });
            return;
        }
    };
    request.dependency_caller = dependencyCaller;
    let Some(executionHost) = executionHost else {
        callbackSink(JsAsyncCallback::Legacy {
            callbackId: normalizedCallbackId,
            result: buildToolPkgIpcFailure("JavaScript execution host is unavailable"),
            isError: false,
        });
        return;
    };
    AppLogger::d(
        TAG,
        &format!(
            "toolpkg-ipc-submit package={} channel={} targetContext={} targetRuntime={}",
            request.package_target,
            request.channel,
            request.target_context_key.as_deref().unwrap_or_default(),
            request.target_runtime.as_deref().unwrap_or_default(),
        ),
    );
    let callbackSinkForCompletion = callbackSink.clone();
    let callbackIdForCompletion = normalizedCallbackId.clone();
    let submitResult = executionHost.invoke_toolpkg_ipc_async(
        request,
        Box::new(move |result| {
            let (result, isError) = match result {
                Ok(value) => (
                    serde_json::json!({
                        "success": true,
                        "value": value
                    })
                    .to_string(),
                    false,
                ),
                Err(error) => (buildToolPkgIpcFailure(&error), false),
            };
            AppLogger::d(
                TAG,
                &format!(
                    "toolpkg-ipc-finish callback={} isError={}",
                    callbackIdForCompletion, isError
                ),
            );
            callbackSinkForCompletion(JsAsyncCallback::Legacy {
                callbackId: callbackIdForCompletion,
                result,
                isError,
            });
        }),
    );
    if let Err(error) = submitResult {
        callbackSink(JsAsyncCallback::Legacy {
            callbackId: normalizedCallbackId,
            result: buildToolPkgIpcFailure(&format!("ToolPkg.ipc async dispatch failed: {error}")),
            isError: false,
        });
    }
}

/// Parses one serialized ToolPkg IPC request into the host contract.
#[allow(non_snake_case)]
fn buildToolPkgIpcRequest(
    packageTarget: String,
    callerContextKey: String,
    targetContextKey: String,
    targetRuntime: String,
    channel: String,
    payloadJson: String,
) -> Result<JsToolPkgIpcRequest, String> {
    let normalizedTarget = packageTarget.trim().to_string();
    if normalizedTarget.is_empty() {
        return Err(buildToolPkgIpcFailure(
            "ToolPkg.ipc package target is empty",
        ));
    }
    let normalizedChannel = channel.trim().to_string();
    if normalizedChannel.is_empty() {
        return Err(buildToolPkgIpcFailure("ToolPkg.ipc channel is required"));
    }
    let requestedRuntime = targetRuntime.trim().to_ascii_lowercase();
    if !requestedRuntime.is_empty()
        && requestedRuntime != "main"
        && requestedRuntime != "ui"
        && requestedRuntime != "sandbox"
        && requestedRuntime != "provider"
    {
        return Err(buildToolPkgIpcFailure(&format!(
            "ToolPkg.ipc targetRuntime is invalid: {requestedRuntime}"
        )));
    }
    let payload = if payloadJson.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str::<Value>(payloadJson.trim()).map_err(|error| {
            buildToolPkgIpcFailure(&format!("ToolPkg.ipc payload JSON is invalid: {error}"))
        })?
    };
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
    CURRENT_DETACHED_INTERMEDIATE_CALLBACKS.with(|callbacks| {
        callbacks.borrow_mut().clear();
    });
    CURRENT_ENV_OVERRIDES.with(|overrides| {
        overrides.borrow_mut().clear();
    });
    CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
        *resources.borrow_mut() = None;
    });
    CURRENT_TOOLPKG_TEXT_RESOURCE_HOST.with(|host| {
        *host.borrow_mut() = None;
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
    CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
        *resources.borrow_mut() = context.textResources.clone();
    });
    CURRENT_TOOLPKG_TEXT_RESOURCE_HOST.with(|host| {
        *host.borrow_mut() = context.textResourceHost.clone();
    });
}

/// Executes one operation while exposing its immutable ToolPkg text resources to native module reads.
#[allow(non_snake_case)]
fn executeWithToolPkgTextResources<T>(
    textResources: Arc<ToolPkgTextResources>,
    operation: impl FnOnce() -> JsExecutionResult<T>,
) -> JsExecutionResult<T> {
    let previousResources =
        CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| resources.replace(Some(textResources)));
    let output = operation();
    CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
        *resources.borrow_mut() = previousResources;
    });
    output
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
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
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
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
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
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
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
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
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

#[allow(non_snake_case)]
fn nativeSendIntermediateResultString(callId: String, result: String) {
    if let Some(context) =
        CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow().get(&callId).cloned())
    {
        if let Some(listener) = context.executionListener {
            listener.on_intermediate_result(&callId, &result);
        }
        if let Some(callback) = context.intermediateCallback {
            callback(result);
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
            callback(result);
            return;
        }
        if let Some(callback) = detachedCallback {
            callback(result);
        }
    });
}

#[allow(non_snake_case)]
fn nativeReadToolPkgTextResourceStrings(
    packageNameOrSubpackageId: String,
    resourcePath: String,
) -> String {
    let resourceKey = normalizeToolPkgTextResourcePath(&resourcePath);
    if let Some(textResources) =
        CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| resources.borrow().clone())
    {
        return textResources.get(&resourceKey).cloned().unwrap_or_default();
    }
    if let Some(host) = CURRENT_TOOLPKG_TEXT_RESOURCE_HOST.with(|host| host.borrow().clone()) {
        return host
            .read_toolpkg_text_resource(&packageNameOrSubpackageId, &resourcePath)
            .unwrap_or_default();
    }
    currentExecutionHost()
        .and_then(|host| host.read_toolpkg_text_resource(&packageNameOrSubpackageId, &resourcePath))
        .unwrap_or_default()
}

#[allow(non_snake_case)]
fn nativeReadToolPkgResourceStrings(
    packageNameOrSubpackageId: String,
    resourceKey: String,
    outputFileName: String,
    internal: String,
) -> String {
    let request = JsToolPkgResourceRequest {
        package_name_or_subpackage_id: packageNameOrSubpackageId,
        resource_key: resourceKey,
        output_file_name: normalizeOptionalString(&outputFileName),
        internal: parseBooleanFlag(&internal),
    };
    currentExecutionHost()
        .and_then(|host| host.materialize_toolpkg_resource(request))
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
}

#[allow(non_snake_case)]
/// Builds the stable failure envelope for ToolPkg WASM calls.
fn buildToolPkgWasmFailure(message: &str) -> String {
    serde_json::json!({
        "success": false,
        "message": message.trim()
    })
    .to_string()
}

#[allow(non_snake_case)]
/// Calls one ToolPkg WASM export through the current execution host.
fn nativeCallToolPkgWasmStrings(
    packageTarget: String,
    moduleId: String,
    exportName: String,
    argsJson: String,
) -> String {
    let normalizedTarget = packageTarget.trim().to_string();
    if normalizedTarget.is_empty() {
        return buildToolPkgWasmFailure("ToolPkg.wasm package target is empty");
    }
    let normalizedModuleId = moduleId.trim().to_string();
    if normalizedModuleId.is_empty() {
        return buildToolPkgWasmFailure("ToolPkg.wasm module id is required");
    }
    let normalizedExportName = exportName.trim().to_string();
    if normalizedExportName.is_empty() {
        return buildToolPkgWasmFailure("ToolPkg.wasm export name is required");
    }
    let args = if argsJson.trim().is_empty() {
        Vec::new()
    } else {
        match serde_json::from_str::<Vec<JsToolPkgWasmArg>>(argsJson.trim()) {
            Ok(value) => value,
            Err(error) => {
                return buildToolPkgWasmFailure(&format!(
                    "ToolPkg.wasm args JSON is invalid: {error}"
                ))
            }
        }
    };
    let request = JsToolPkgWasmRequest {
        package_target: normalizedTarget,
        module_id: normalizedModuleId,
        export_name: normalizedExportName,
        args,
    };
    match currentExecutionHost().and_then(|host| host.call_toolpkg_wasm(request)) {
        Ok(result) => serde_json::json!({
            "success": true,
            "valueType": result.value_type,
            "value": result.value
        })
        .to_string(),
        Err(error) => buildToolPkgWasmFailure(&error),
    }
}

#[allow(non_snake_case)]
fn nativeComposeWebViewControllerCommandString(payloadJson: String) -> String {
    currentExecutionHost()
        .and_then(|host| host.handle_compose_webview_controller_command(&payloadJson))
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
}

/// Runs one Compose DSL file-picker request through the current execution host.
#[allow(non_snake_case)]
fn nativeComposeFilePickerCommandString(payloadJson: String) -> String {
    currentExecutionHost()
        .and_then(|host| host.open_compose_file_picker(&payloadJson))
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
}

#[allow(non_snake_case)]
fn normalizeToolPkgTextResourcePath(path: &str) -> String {
    path.replace('\\', "/")
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

#[allow(non_snake_case)]
fn nativeSetCallResultStrings(callId: String, result: String) {
    CURRENT_CALL_RESULTS.with(|results| {
        results.borrow_mut().insert(callId, result);
    });
}

#[allow(non_snake_case)]
fn nativeSetCallErrorStrings(callId: String, error: String) {
    let owner =
        CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow().get(&callId).cloned());
    if let Some(context) = owner {
        if let Some(listener) = context.executionListener {
            listener.on_failed(&callId, &error);
        }
    } else {
        CURRENT_EXECUTION_LISTENER.with(|listener| {
            if let Some(listener) = listener.borrow().as_ref() {
                listener.on_failed(&callId, &error);
            }
        });
    }
    CURRENT_CALL_RESULTS.with(|results| {
        results.borrow_mut().insert(callId, error);
    });
}

#[allow(non_snake_case)]
fn nativeGetEnvForCallStrings(key: String) -> String {
    if let Some(value) = CURRENT_ENV_OVERRIDES.with(|overrides| {
        overrides
            .borrow()
            .get(key.trim())
            .filter(|value| !value.is_empty())
            .cloned()
    }) {
        return value;
    }
    currentExecutionHost()
        .and_then(|host| host.read_environment_variable(&key))
        .map(|value| value.unwrap_or_default())
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
}

/// Writes one environment value for later Compose screens and tool calls.
fn nativeSetEnvStrings(callId: String, key: String, value: String) -> String {
    let name = key.trim().to_string();
    if name.is_empty() {
        return String::new();
    }
    if let Some(context) =
        CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow().get(&callId).cloned())
    {
        context
            .envOverrides
            .lock()
            .expect("JavaScript call environment mutex poisoned")
            .insert(name.clone(), value.clone());
    }
    CURRENT_ENV_OVERRIDES.with(|overrides| {
        overrides.borrow_mut().insert(name.clone(), value.clone());
    });
    match currentExecutionHost().and_then(|host| host.write_environment_variable(&name, &value)) {
        Ok(()) => String::new(),
        Err(error) => buildJsExecutionErrorPayload(&error),
    }
}

/// Writes a JSON object of environment values for later Compose screens.
fn nativeSetEnvsStrings(callId: String, valuesJson: String) -> String {
    let parsed = match serde_json::from_str::<Value>(&valuesJson) {
        Ok(value) => value,
        Err(error) => return buildJsExecutionErrorPayload(&error.to_string()),
    };
    let object = match parsed.as_object() {
        Some(object) => object,
        None => {
            return buildJsExecutionErrorPayload("setEnvs requires a JSON object");
        }
    };
    for (key, value) in object {
        let serialized = match value {
            Value::String(text) => text.clone(),
            Value::Null => String::new(),
            other => other.to_string(),
        };
        let result = nativeSetEnvStrings(callId.clone(), key.clone(), serialized);
        if !result.trim().is_empty() {
            return result;
        }
    }
    String::new()
}

/// Resolves package-owned configuration without altering the public one-argument API.
#[allow(non_snake_case)]
fn nativeGetScopedPluginConfigDirString(ownerId: String, pluginId: String) -> String {
    let result = currentExecutionHost().and_then(|host| {
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
    });
    match result {
        Ok(path) => serde_json::json!({"success": true, "path": path}).to_string(),
        Err(error) => buildJsExecutionErrorPayload(&error),
    }
}

/// Resolves explicit device configuration for the original native entry point.
#[allow(non_snake_case)]
fn nativeGetPluginConfigDirString(pluginId: String) -> String {
    currentExecutionHost()
        .and_then(|host| host.plugin_config_dir(&pluginId))
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
}

#[allow(non_snake_case)]
fn nativeIsPackageImportedString(packageName: String) -> String {
    currentExecutionHost()
        .and_then(|host| host.is_package_imported(packageName.trim()))
        .map(|value| value.to_string())
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
}

#[allow(non_snake_case)]
fn nativeImportPackageString(packageName: String) -> String {
    currentExecutionHost()
        .and_then(|host| host.import_package(packageName.trim()))
        .unwrap_or_else(|error| error)
}

#[allow(non_snake_case)]
fn nativeRemovePackageString(packageName: String) -> String {
    currentExecutionHost()
        .and_then(|host| host.remove_package(packageName.trim()))
        .unwrap_or_else(|error| error)
}

#[allow(non_snake_case)]
fn nativeUsePackageString(packageName: String) -> String {
    currentExecutionHost()
        .and_then(|host| host.use_package(packageName.trim()))
        .unwrap_or_else(|error| error)
}

#[allow(non_snake_case)]
fn nativeListImportedPackagesJsonString() -> String {
    currentExecutionHost()
        .and_then(|host| host.list_imported_packages())
        .and_then(|packages| serde_json::to_string(&packages).map_err(|error| error.to_string()))
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
}

/// Returns the executable tool catalog exposed by the active host.
#[allow(non_snake_case)]
fn nativeGetToolCatalogJsonString() -> String {
    currentExecutionHost()
        .and_then(|host| host.get_tool_catalog())
        .and_then(|catalog| serde_json::to_string(&catalog).map_err(|error| error.to_string()))
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
}

#[allow(non_snake_case)]
fn nativeResolveToolNameString(
    packageName: String,
    subpackageId: String,
    toolName: String,
    preferImported: String,
) -> String {
    let request = JsToolNameResolutionRequest {
        package_name: normalizeOptionalString(&packageName),
        subpackage_id: normalizeOptionalString(&subpackageId),
        tool_name: toolName,
        prefer_imported: !preferImported.eq_ignore_ascii_case("false"),
    };
    currentExecutionHost()
        .and_then(|host| host.resolve_tool_name(request))
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
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
fn nativeLogJsExecutionTraceStrings(callId: String, message: String) {
    let _ = (callId, message);
}

#[allow(non_snake_case)]
fn nativeDecompressStrings(data: String, algorithm: String) -> String {
    JsNativeInterfaceDelegates::decompress(&data, &algorithm)
}

#[allow(non_snake_case)]
fn nativeCryptoStrings(algorithm: String, operation: String, argsJson: String) -> String {
    JsNativeInterfaceDelegates::crypto(&algorithm, &operation, &argsJson)
}

#[allow(non_snake_case)]
fn nativeImageProcessingStrings(
    _callbackId: String,
    operation: String,
    argsJson: String,
) -> String {
    match JsNativeInterfaceDelegates::imageProcessing(&operation, &argsJson) {
        Ok(result) => serde_json::json!({
            "success": true,
            "result": result
        })
        .to_string(),
        Err(error) => serde_json::json!({
            "success": false,
            "error": error
        })
        .to_string(),
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
fn readNativeExecutionSession(callId: &str) -> Option<String> {
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
