use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde_json::Value;
use uuid::Uuid;

use crate::javascript::JsHostOperations;
use crate::javascript::JsJavaBridgeDelegates::{
    javaCallInstance, javaCallStatic, javaClassExists, javaGetApplicationContext, javaNewInstance,
};
use crate::javascript::JsLibraries::{buildRuntimeBootstrapScript, loadBootstrapLibrary};
use operit_host_api::HostManager::{
    defaultHostJavaScriptRuntimeHost, defaultHostRuntimeTaskSchedulerHost,
};
use operit_host_api::TimeUtils::currentTimeMillisU128;
use operit_host_api::{
    HostError, HostErrorKind, HostJavaScriptAsyncJsonCallback, HostJavaScriptExecutionInterrupt,
    HostJavaScriptInterruptController, HostJavaScriptCallback, HostJavaScriptRuntime,
    HostJavaScriptRuntimeHost, HostJavaScriptRuntimeStateHandle, HostJavaScriptRuntimeStateOutput,
    HostResult, HostRuntimeTaskSchedulerHost,
};
use operit_plugin_sdk::execution_result::{
    build_js_execution_error_payload as buildJsExecutionErrorPayload,
    extract_js_execution_error_message as extractJsExecutionErrorMessage, JsExecutionError,
    JsExecutionResult,
};
use operit_plugin_sdk::javascript::{
    JsEnvironmentOwner, JsExecutionCompletion, JsExecutionEngine, JsExecutionFuture, JsExecutionHost,
    JsToolNameResolutionRequest, JsToolPkgIpcRequest, JsToolPkgResourceRequest, JsToolPkgWasmArg,
    JsToolPkgWasmRequest, ToolPkgConfigScope, ToolPkgExecutionContext,
    ToolPkgMainRegistrationCapture, ToolPkgTextResourceHost,
};
use operit_plugin_sdk::toolpkg::ToolPkgApiRuntimeScript::buildToolPkgApiRuntimeScript;
use operit_plugin_sdk::toolpkg::ToolPkgRegistrationBridge::buildToolPkgRegistrationBridgeScript;
use operit_util::AppLogger::AppLogger;

const TAG: &str = "OperitJsEngine";
type ToolPkgTextResources = BTreeMap<String, String>;

#[allow(non_snake_case)]
pub trait JsExecutionListener {
    fn on_intermediate_result(&self, callId: &str, result: &str);
    fn on_failed(&self, callId: &str, reason: &str);
}

type JsExecutionListenerRef = Arc<dyn JsExecutionListener + Send + Sync>;

thread_local! {
    static CURRENT_SCRIPT_OWNER: RefCell<Option<(String, HostJavaScriptExecutionInterrupt)>> = RefCell::new(None);
    static CURRENT_CLEANUP_OWNER: RefCell<Option<(String, HostJavaScriptExecutionInterrupt)>> = RefCell::new(None);
    static CURRENT_INTERRUPT_SCOPE: RefCell<Option<InterruptScopeState>> = RefCell::new(None);
    static CURRENT_ACTIVE_CALL_CONTEXTS: RefCell<BTreeMap<String, JsCallContext>> = RefCell::new(BTreeMap::new());
    static CURRENT_EXECUTION_HOST: RefCell<Option<Arc<dyn JsExecutionHost>>> = RefCell::new(None);
    static CURRENT_REGISTRATION_CONFIG_PARAMS: RefCell<Option<BTreeMap<String, Value>>> = RefCell::new(None);
    static CURRENT_INTERMEDIATE_CALLBACK: RefCell<Option<Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>>> = RefCell::new(None);
    static CURRENT_EXECUTION_LISTENER: RefCell<Option<JsExecutionListenerRef>> = RefCell::new(None);
    static CURRENT_DETACHED_INTERMEDIATE_CALLBACKS: RefCell<BTreeMap<String, Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>>> =
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
struct JsEngineWorker {
    lifetimeInterrupt: HostJavaScriptExecutionInterrupt,
    runtimeHost: Arc<dyn HostJavaScriptRuntimeHost>,
    stateHandle: HostJavaScriptRuntimeStateHandle,
    alive: Arc<AtomicBool>,
    executionHost: Option<Arc<dyn JsExecutionHost>>,
}

enum JsAsyncCallback {
    ProfiledToolResult {
        requestId: u64,
        result: Value,
        timing: JsIpcTiming,
        successful: bool,
    },
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
    interrupt: HostJavaScriptExecutionInterrupt,
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    intermediateCallback: Option<Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>>,
    executionListener: Option<JsExecutionListenerRef>,
    envOverrides: Arc<Mutex<BTreeMap<String, String>>>,
    environmentOwner: Option<JsEnvironmentOwner>,
    textResourceSource: JsTextResourceSource,
    ipcTiming: Option<JsIpcTiming>,
}

/// Correlates short engine steps and asynchronous tool work with their originating IPC.
#[derive(Clone)]
struct JsIpcTiming {
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    traceId: String,
    callId: String,
    kind: &'static str,
    operation: String,
    stages: Arc<Mutex<Vec<(&'static str, u64)>>>,
}

impl JsIpcTiming {
    /// Starts one monotonic timeline without copying script or payload data.
    fn new(traceId: String, callId: String, kind: &'static str, operation: String) -> HostResult<Self> {
        let scheduler = defaultHostRuntimeTaskSchedulerHost();
        let started = scheduler.monotonicTimeMillis()?;
        Ok(Self {
            scheduler, traceId, callId, kind, operation,
            stages: Arc::new(Mutex::new(vec![("submitted", started)])),
        })
    }

    /// Records one boundary with no log formatting on the measured path.
    fn mark(&self, stage: &'static str) -> HostResult<()> {
        let now = self.scheduler.monotonicTimeMillis()?;
        self.stages.lock().expect("IPC timing mutex poisoned").push((stage, now));
        Ok(())
    }

    /// Reports failures normally and successful stage timings only in an explicit performance run.
    fn log(&self, successful: bool) {
        if successful && std::env::var_os("OPERIT_PROFILE_COMPOSE_STAGES").is_none() {
            return;
        }
        let stages = self.stages.lock().expect("IPC timing mutex poisoned");
        let phases = stages.windows(2)
            .map(|pair| format!("{}={}", pair[1].0, pair[1].1.saturating_sub(pair[0].1)))
            .collect::<Vec<_>>().join(" ");
        if successful {
            eprintln!(
                "event=compose_ipc_timing kind={} traceId={} callId={} operation={} wallMillis={} {}",
                self.kind, self.traceId, self.callId, self.operation, currentTimeMillisU128(), phases,
            );
            return;
        }
        AppLogger::e("ToolPkg", &format!(
            "event=toolpkg_ipc_{} traceId={} callId={} operation={} outcome={} {}",
            self.kind, self.traceId, self.callId, self.operation,
            "error", phases,
        ));
    }
}

struct JsPendingScriptExecution {
    callId: String,
    context: JsCallContext,
    deadlineMillis: Option<u128>,
    timeout: Option<Duration>,
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
    submittedAtMs: u128,
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    callId: String,
    script: String,
    functionName: String,
    params: BTreeMap<String, Value>,
    envOverrides: BTreeMap<String, String>,
    onIntermediateResult: Option<Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>>,
    timeout: Option<Duration>,
    interrupt: HostJavaScriptExecutionInterrupt,
    executionListener: Option<JsExecutionListenerRef>,
    textResources: Option<Arc<ToolPkgTextResources>>,
    useComposeDslTextResources: bool,
    completion: JsScriptCompletion,
    cancelled: Arc<AtomicBool>,
    deadlineMillis: Option<u128>,
    ipcTiming: Option<JsIpcTiming>,
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
    interrupt: HostJavaScriptExecutionInterrupt,
}

impl Drop for JsScriptRequestLease {
    /// Cancels an unfinished request before its queued Start can evaluate any script.
    fn drop(&mut self) {
        let Some(callId) = self.callId.take() else {
            return;
        };
        self.worker.cancelScriptRequest(callId, &self.cancelled, &self.interrupt);
    }
}

/// Holds the immutable package identity installed when a prepared engine is claimed.
#[derive(Clone)]
struct JsPackageBinding {
    executionHost: Arc<dyn JsExecutionHost>,
    context: ToolPkgExecutionContext,
}

struct JsEngineState {
    lifetimeInterrupt: HostJavaScriptExecutionInterrupt,
    routeLifetime: tokio::sync::watch::Sender<()>,
    runtime: Box<dyn HostJavaScriptRuntime>,
    asyncCallbackSender: mpsc::Sender<JsAsyncCallback>,
    asyncCallbackReceiver: mpsc::Receiver<JsAsyncCallback>,
    backgroundWake: Arc<Mutex<Option<JsBackgroundWake>>>,
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    toolPkgContext: Option<ToolPkgExecutionContext>,
    composeDslTextResources: Option<Arc<ToolPkgTextResources>>,
    detachedCallContexts: BTreeMap<String, JsCallContext>,
    pendingScriptExecutions: BTreeMap<String, JsPendingScriptExecution>,
    registeredScriptSources: BTreeMap<String, String>,
    jsEnvironmentInitialized: bool,
    packageBinding: Arc<OnceLock<JsPackageBinding>>,
}

impl JsEngineWorker {
    /// Creates one host-owned JavaScript runtime state.
    fn new(
        executionHost: Option<Arc<dyn JsExecutionHost>>,
        toolPkgContext: Option<ToolPkgExecutionContext>,
    ) -> JsExecutionResult<Self> {
        let executionHost = match (&executionHost, &toolPkgContext) {
            (Some(host), Some(context)) => {
                context.validate().map_err(JsExecutionError::invalid_request)?;
                Some(host.for_toolpkg_execution_context(context).map_err(JsExecutionError::invalid_request)?)
            }
            (None, Some(_)) => return Err(JsExecutionError::invalid_request("ToolPkg execution context requires a real host")),
            (_, None) => executionHost,
        };
        let runtimeHost = defaultHostJavaScriptRuntimeHost();
        let runtimeHostForState = runtimeHost.clone();
        let workerExecutionHost = executionHost.clone();
        let lifetimeInterrupt = HostJavaScriptExecutionInterrupt::unlimited();
        let stateLifetimeInterrupt = lifetimeInterrupt.clone();
        let backgroundWake = Arc::new(Mutex::new(None));
        let backgroundWakeForState = backgroundWake.clone();
        let stateHandle = runtimeHost
            .createHostJavaScriptRuntimeState(
                "OperitJsEngine",
                operit_host_api::HostJavaScriptRuntimeStateKind::Execution,
                Box::new(move || {
                    let runtime = runtimeHostForState.createHostJavaScriptRuntime()?;
                    let state = JsEngineState::newWithRuntime(
                        runtime,
                        executionHost,
                        toolPkgContext,
                        backgroundWakeForState,
                        stateLifetimeInterrupt,
                    )
                    .map_err(HostError::new)?;
                    Ok(Box::new(state))
                }),
            )
            .map_err(|error| JsExecutionError::initialization(error.to_string()))?;
        let scheduled = Arc::new(AtomicBool::new(false));
        let requested = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(AtomicBool::new(true));
        let wakeRuntimeHost = runtimeHost.clone();
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
            let taskScheduled = scheduled.clone();
            let taskRequested = requested.clone();
            let taskWakeSlot = wakeSlot.clone();
            let taskAlive = wakeAlive.clone();
            let task: operit_host_api::HostJavaScriptRuntimeStateTask = Box::new(move |state, interrupt| {
                if !taskAlive.load(Ordering::Acquire) {
                    return Ok(Box::new(()));
                }
                taskRequested.store(false, Ordering::Release);
                let result = state.downcast_mut::<JsEngineState>()
                    .ok_or_else(|| HostError::new("Invalid JavaScript runtime state"))
                    .and_then(|state| state.advanceJavaScriptExecutions(interrupt)
                        .map_err(|error| HostError::new(error.to_string())));
                if let Err(error) = result {
                    if taskAlive.load(Ordering::Acquire) {
                        AppLogger::e(TAG, &format!("detached JavaScript execution failed: {error}"));
                    }
                }
                taskScheduled.store(false, Ordering::Release);
                if taskRequested.swap(false, Ordering::AcqRel) {
                    if let Some(wake) = taskWakeSlot.lock()
                        .expect("background wake mutex poisoned").clone()
                    {
                        wake();
                    }
                }
                Ok(Box::new(()))
            });
            if let Err(error) = taskRuntimeHost.enqueueHostJavaScriptRuntimeStateTask(
                stateHandle, HostJavaScriptExecutionInterrupt::unlimited(), task,
            ) {
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
        Ok(Self {
            lifetimeInterrupt,
            runtimeHost,
            stateHandle,
            alive,
            executionHost: workerExecutionHost,
        })
    }

    /// Cancels a submitted call independently of whether its completion future is polled.
    #[allow(non_snake_case)]
    fn cancelScriptRequest(&self, callId: String, cancelled: &AtomicBool, interrupt: &HostJavaScriptExecutionInterrupt) {
        interrupt.interrupt();
        cancelled.store(true, Ordering::Release);
        let worker = self.clone();
        let result = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "operit-js-cancel",
            Box::new(move || {
                Box::pin(async move {
                    if worker.alive.load(Ordering::Acquire) {
                        let _ = worker
                            .executeScriptStepAsync(JsScriptRequestStep::Cancel(callId))
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
    ) -> JsExecutionResult<()> {
        let task = scriptRequestStepTask(step);
        let output = self
            .runtimeHost
            .executeHostJavaScriptRuntimeStateAsyncTask(
                self.stateHandle,
                HostJavaScriptExecutionInterrupt::unlimited(),
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
        on_intermediate_result: Option<Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>>,
        _dispatchIntermediateOnMain: bool,
        timeout: Option<Duration>,
        executionListener: Option<JsExecutionListenerRef>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        useComposeDslTextResources: bool,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<Value>>> {
        let executionHost = self.executionHost.as_ref().map(|host| host.capture_tool_invocation_host());
        let worker = self.clone();
        Box::pin(async move {
            if !worker.alive.load(Ordering::Acquire) {
                return Err(JsExecutionError::worker_unavailable(
                    "JavaScript runtime was destroyed",
                ));
            }
            let deadlineMillis = timeout.map(scriptRequestDeadline).transpose()?;
            let interrupt = match timeout {
                Some(duration) => HostJavaScriptExecutionInterrupt::new(
                    u64::try_from(duration.as_millis()).map_err(|_| JsExecutionError::invalid_request("Before hook timeout exceeds the host range"))?,
                ).map_err(javaScriptExecutionErrorFromHost)?,
                None => HostJavaScriptExecutionInterrupt::unlimited(),
            };
            let callId = format!("operit_call_{}", Uuid::new_v4().simple());
            let ipcTiming = params.get("__operit_ipc_trace_id").and_then(Value::as_str)
                .map(|traceId| JsIpcTiming::new(traceId.to_string(), callId.clone(), "execution", functionName.clone()))
                .transpose().map_err(javaScriptExecutionErrorFromHost)?;
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let completion = Arc::new(Mutex::new(Some(sender)));
            let cancelled = Arc::new(AtomicBool::new(false));
            let mut lease = JsScriptRequestLease {
                worker: worker.clone(),
                callId: Some(callId.clone()),
                cancelled: cancelled.clone(),
                interrupt: interrupt.clone(),
            };
            let request = JsScriptRequest {
                submittedAtMs: currentTimeMillisU128(),
                executionHost,
                callId,
                script,
                functionName,
                params,
                envOverrides,
                onIntermediateResult: on_intermediate_result,
                timeout,
                interrupt: interrupt.clone(),
                executionListener,
                textResources,
                useComposeDslTextResources,
                completion: completion.clone(),
                cancelled: cancelled.clone(),
                deadlineMillis,
                ipcTiming: ipcTiming.clone(),
            };
            // Only finite requests schedule a wall-clock completion deadline.
            if let Some(deadlineMillis) = deadlineMillis {
                let weakCompletion = Arc::downgrade(&completion);
                let deadlineWorker = worker.clone();
                let deadlineCallId = request.callId.clone();
                let deadlineInterrupt = interrupt.clone();
                defaultHostRuntimeTaskSchedulerHost()
                    .scheduleDelayedHostRuntimeTask(
                        "operit-js-deadline",
                        remainingScriptRequestMillis(deadlineMillis)?,
                        Box::new(move || {
                            if let Some(completion) = weakCompletion.upgrade() {
                                let sender = completion
                                    .lock()
                                    .expect("JavaScript completion mutex poisoned")
                                    .take();
                                if let Some(sender) = sender {
                                    deadlineWorker.cancelScriptRequest(deadlineCallId, &cancelled, &deadlineInterrupt);
                                    let _ = sender.send(Err(JsExecutionError::timeout(format!(
                                        "Script execution timed out after {} milliseconds",
                                        timeout.expect("Timed before hook must own a duration").as_millis(),
                                    ))));
                                }
                            }
                        }),
                    )
                    .map_err(javaScriptExecutionErrorFromHost)?;
            }
            let completionForStart = completion.clone();
            let timingForStart = request.ipcTiming.clone();
            let aliveForStart = worker.alive.clone();
            let task = scriptRequestStepTask(JsScriptRequestStep::Start(request));
            worker.runtimeHost.enqueueHostJavaScriptRuntimeStateTask(
                worker.stateHandle, interrupt,
                Box::new(move |state, interrupt| {
                    let result = (|| {
                        if let Some(timing) = &timingForStart {
                            timing.mark("schedulerWaitMs").map_err(javaScriptExecutionErrorFromHost)?;
                        }
                        if !aliveForStart.load(Ordering::Acquire) {
                            return Err(JsExecutionError::worker_unavailable("JavaScript runtime was destroyed"));
                        }
                        let output = task(state, interrupt).map_err(javaScriptExecutionErrorFromHost)?;
                        downcastJavaScriptStateOutput::<()>(output)
                    })();
                    if let Err(error) = result {
                        completeScriptExecution(&completionForStart, Err(error));
                    }
                    Ok(Box::new(()))
                }),
            ).map_err(javaScriptExecutionErrorFromHost)?;
            // The queued task owns completion; dropping an unstarted task must disconnect its caller.
            drop(completion);
            let result = receiver.await.map_err(|_| {
                JsExecutionError::worker_unavailable(
                    "JavaScript execution completion channel disconnected",
                )
            })?;
            if let Some(timing) = ipcTiming {
                timing.mark("resultReturnMs").map_err(javaScriptExecutionErrorFromHost)?;
                timing.log(result.is_ok());
            }
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
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        let result = self.runtimeHost.executeHostJavaScriptRuntimeStateTask(
            self.stateHandle,
            HostJavaScriptExecutionInterrupt::unlimited(),
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
        self.lifetimeInterrupt.interrupt();
        if !self.alive.swap(false, Ordering::AcqRel) {
            return;
        }
        self.runtimeHost
            .destroyHostJavaScriptRuntimeState(self.stateHandle)
            .expect("JavaScript runtime state must be destroyed by the Host");
    }
}

/// Builds one cancellable state step without retaining its borrow across an await.
#[allow(non_snake_case)]
fn scriptRequestStepTask(
    step: JsScriptRequestStep,
) -> operit_host_api::HostJavaScriptRuntimeStateTask {
    Box::new(move |state, interrupt| {
        let state = state
            .downcast_mut::<JsEngineState>()
            .ok_or_else(|| HostError::new("JavaScript runtime state type does not match"))?;
        let stepInterrupt = interrupt.clone();
        let output =
            executeWithInterrupt(state, interrupt, "Before hook execution", |state| {
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
fn remainingScriptRequestMillis(deadline: u128) -> JsExecutionResult<u64> {
    let remaining = deadline.saturating_sub(currentTimeMillisU128());
    if remaining == 0 {
        return Err(JsExecutionError::timeout(format!(
            "Before hook timed out before execution"
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

/// Switches cancellation ownership at the same host boundary as JavaScript call context.
fn selectScriptExecution(owner: Option<(String, HostJavaScriptExecutionInterrupt)>) {
    let cleanup = CURRENT_CLEANUP_OWNER.with(|current| current.borrow().clone());
    let owner = owner.map(|(id, token)| {
        let token = cleanup.as_ref().filter(|(cleanupId, _)| cleanupId == &id)
            .map(|(_, token)| token.clone()).unwrap_or(token);
        (id, token)
    });
    CURRENT_INTERRUPT_SCOPE.with(|scope| {
        if let Some(scope) = scope.borrow().as_ref() {
            // A ready continuation from another call keeps its own execution budget.
            let otherCall = cleanup.as_ref().zip(owner.as_ref())
                .is_some_and(|((cleanupId, _), (ownerId, _))| cleanupId != ownerId);
            let mut tokens = vec![scope.lifetime.clone()];
            if !otherCall { tokens.extend(scope.request.clone()); }
            if let Some((_, token)) = &owner { tokens.push(token.clone()); }
            scope.controller.replace(tokens);
        }
    });
    CURRENT_SCRIPT_OWNER.with(|current| *current.borrow_mut() = owner);
}

/// Allows only the explicitly bounded cancellation cleanup to resume a retired call.
fn activateCancelledScriptCall(callId: String) {
    let interrupt = CURRENT_CLEANUP_OWNER.with(|owner| {
        owner.borrow().as_ref().filter(|(id, _)| id == &callId).map(|(_, token)| token.clone())
    }).unwrap_or_else(|| {
        let token = HostJavaScriptExecutionInterrupt::unlimited();
        token.interrupt();
        token
    });
    selectScriptExecution(Some((callId, interrupt)));
}

struct InterruptScopeState {
    controller: Arc<dyn HostJavaScriptInterruptController>,
    lifetime: HostJavaScriptExecutionInterrupt,
    request: Option<HostJavaScriptExecutionInterrupt>,
}

/// Restores the enclosing execution scope even if a host callback unwinds.
struct ExecutionScope {
    previous: Option<InterruptScopeState>,
    owner: Option<(String, HostJavaScriptExecutionInterrupt)>,
    cleanupOwner: Option<(String, HostJavaScriptExecutionInterrupt)>,
}

impl ExecutionScope {
    fn enter(state: &JsEngineState, interrupt: Option<HostJavaScriptExecutionInterrupt>) -> Self {
        let scope = InterruptScopeState {
            controller: state.runtime.interruptController(),
            lifetime: state.lifetimeInterrupt.clone(),
            request: interrupt,
        };
        let mut tokens = vec![scope.lifetime.clone()];
        tokens.extend(scope.request.clone());
        scope.controller.replace(tokens);
        Self {
            previous: CURRENT_INTERRUPT_SCOPE.with(|current| current.replace(Some(scope))),
            owner: CURRENT_SCRIPT_OWNER.with(|owner| owner.take()),
            cleanupOwner: CURRENT_CLEANUP_OWNER.with(|owner| owner.take()),
        }
    }
}

impl Drop for ExecutionScope {
    fn drop(&mut self) {
        let previous = self.previous.take();
        CURRENT_INTERRUPT_SCOPE.with(|scope| {
            if let Some(scope) = scope.replace(previous) {
                scope.controller.replace(Vec::new());
            }
        });
        CURRENT_CLEANUP_OWNER.with(|owner| *owner.borrow_mut() = self.cleanupOwner.take());
        selectScriptExecution(self.owner.take());
    }
}

/// Executes one operation with directly subscribed lifetime and call cancellation signals.
#[allow(non_snake_case)]
fn executeWithInterrupt<T>(
    state: &mut JsEngineState,
    interrupt: HostJavaScriptExecutionInterrupt,
    timeoutLabel: &str,
    operation: impl FnOnce(&mut JsEngineState) -> JsExecutionResult<T>,
) -> JsExecutionResult<T> {
    let scope = ExecutionScope::enter(state, Some(interrupt.clone()));
    let output = operation(state);
    drop(scope);
    if interrupt.shouldInterrupt() && interrupt.didTimeOut() {
        return Err(JsExecutionError::timeout(format!("{timeoutLabel} timed out")));
    }
    output
}

/// Owns a pristine initialized engine until binding succeeds or its request is cancelled.
pub(crate) struct PreparedJsEngine {
    engine: Option<JsEngine>,
}

impl PreparedJsEngine {
    /// Initializes only the shared runtime, native interfaces and SDK bootstrap.
    pub(crate) fn new() -> Result<Self, String> {
        let worker = JsEngineWorker::new(None, None).map_err(|error| error.to_string())?;
        Ok(Self {
            engine: Some(JsEngine { worker }),
        })
    }

    /// Authenticates and binds one package exactly once on the runtime's Host executor.
    pub(crate) async fn bind(
        mut self,
        executionHost: Arc<dyn JsExecutionHost>,
        context: ToolPkgExecutionContext,
    ) -> Result<JsEngine, String> {
        context.validate()?;
        let host = executionHost.for_toolpkg_execution_context(&context)?;
        let engine = self
            .engine
            .as_mut()
            .expect("Prepared engine must be owned until binding completes");
        let boundHost = host.clone();
        engine
            .worker
            .runtimeHost
            .executeHostJavaScriptRuntimeStateAsyncTask(
                engine.worker.stateHandle,
                HostJavaScriptExecutionInterrupt::unlimited(),
                Box::new(move |state, _interrupt| {
                    Box::pin(async move {
                        let state = state.downcast_mut::<JsEngineState>().ok_or_else(|| {
                            HostError::new("JavaScript runtime state type does not match")
                        })?;
                        if state.executionHost.is_some() || state.toolPkgContext.is_some() {
                            return Err(HostError::new(
                                "Prepared engine already has an execution owner",
                            ));
                        }
                        state
                            .packageBinding
                            .set(JsPackageBinding {
                                executionHost: boundHost.clone(),
                                context: context.clone(),
                            })
                            .map_err(|_| {
                                HostError::new("ToolPkg execution context is already bound")
                            })?;
                        state.executionHost = Some(boundHost);
                        state.toolPkgContext = Some(context);
                        Ok(Box::new(()) as HostJavaScriptRuntimeStateOutput)
                    })
                }),
            )
            .await
            .map_err(|error| error.to_string())?;
        engine.worker.executionHost = Some(host);
        Ok(self
            .engine
            .take()
            .expect("Bound engine must still be owned"))
    }
}

impl Drop for PreparedJsEngine {
    /// Destroys unused, rejected or cancelled prepared engines through their Host.
    fn drop(&mut self) {
        if let Some(engine) = self.engine.take() {
            engine.worker.destroy();
        }
    }
}

impl JsEngine {
    /// Creates a JavaScript execution engine backed by a caller-supplied execution host.
    pub fn new(executionHost: Arc<dyn JsExecutionHost>) -> JsExecutionResult<Self> {
        Ok(Self {
            worker: JsEngineWorker::new(Some(executionHost), None)?,
        })
    }

    /// Creates a JavaScript execution engine bound to one ToolPkg package environment.
    pub fn new_toolpkg_execution_engine(
        executionHost: Arc<dyn JsExecutionHost>,
        context: ToolPkgExecutionContext,
    ) -> JsExecutionResult<Self> {
        Ok(Self {
            worker: JsEngineWorker::new(Some(executionHost), Some(context))?,
        })
    }

    /// Creates a JavaScript engine used only for ToolPkg registration.
    #[allow(non_snake_case)]
    pub fn new_toolpkg_registration_engine() -> JsExecutionResult<Self> {
        Ok(Self {
            worker: JsEngineWorker::new(None, None)?,
        })
    }

    /// Executes a script until completion or explicit caller cancellation.
    #[allow(non_snake_case)]
    pub fn execute_script_function(
        &self, script: &str, functionName: &str, params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        self.executeScriptFunction(script, functionName, params, envOverrides,
            on_intermediate_result, dispatchIntermediateOnMain, None, executionListener)
    }

    /// Applies the caller's explicit before-hook completion deadline.
    #[allow(non_snake_case)]
    pub fn execute_script_function_with_timeout_millis(
        &self, script: &str, functionName: &str, params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool, timeoutMillis: u64,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        if timeoutMillis == 0 {
            return Box::pin(async { Err(JsExecutionError::timeout("Before hook timed out before execution")) });
        }
        self.executeScriptFunction(script, functionName, params, envOverrides,
            on_intermediate_result, dispatchIntermediateOnMain,
            Some(Duration::from_millis(timeoutMillis)), executionListener)
    }

    /// Executes asynchronous script work without counting suspended Promise time.
    #[allow(non_snake_case)]
    pub fn execute_script_function_async(
        &self, script: String, functionName: String, params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
        textScriptCompletion(self.worker.execute_script_function_async(
            script, functionName, params, envOverrides,
            textIntermediateCallback(on_intermediate_result), dispatchIntermediateOnMain,
            None, executionListener, None, false,
        ))
    }

    /// Shares lifecycle and cancellation handling between ordinary scripts and before hooks.
    #[allow(non_snake_case)]
    fn executeScriptFunction(
        &self, script: &str, functionName: &str, params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool, timeout: Option<Duration>,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        textScriptCompletion(self.worker.execute_script_function_async(
            script.to_owned(), functionName.to_owned(), params.clone(), envOverrides.clone(),
            textIntermediateCallback(on_intermediate_result), dispatchIntermediateOnMain,
            timeout, executionListener, None, false,
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

    /// Executes ToolPkg registration with archive text resources until completion.
    #[allow(non_snake_case)]
    pub(crate) fn execute_toolpkg_main_registration_function_with_text_resources(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        self.worker.execute_toolpkg_main_registration_function(
            script,
            functionName,
            params,
            textResources,
        )
    }

    /// Dispatches a Compose DSL action while allowing the host runtime to keep advancing.
    #[allow(non_snake_case)]
    pub fn dispatch_compose_dsl_action_stream_async(
        &self,
        actionId: String,
        payload: Option<Value>,
        runtimeOptions: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_response: Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>,
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
            Some(on_response),
            None,
        )
    }

    /// Sends render commits on the JS thread before asynchronous command completion can be overtaken.
    pub fn execute_compose_dsl_script_stream_async(
        &self,
        script: String,
        runtimeOptions: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        textResources: Arc<ToolPkgTextResources>,
        onResponse: Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
        self.executeComposeDslFunctionAsync(
            script, "__operit_render_compose_dsl".into(),
            runtimeOptions, envOverrides, Some(onResponse), Some(textResources),
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
        onIntermediateResult: Option<Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
        self.worker.execute_script_function_async(
            script,
            functionName,
            params,
            envOverrides,
            onIntermediateResult,
            true,
            None,
            None,
            textResources,
            true,
        )
    }

}

/// Preserves the JavaScript UTF-16 hash and wrapping arithmetic without interpreter work per character.
fn javaScriptSourceFingerprint(source: &str) -> String {
    let hash = source.encode_utf16().fold(0_u32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(u32::from(unit))
    });
    format!("{hash:x}")
}

impl JsEngineState {
    /// Creates a JavaScript runtime state from a Host-owned runtime instance.
    fn newWithRuntime(
        runtime: Box<dyn HostJavaScriptRuntime>,
        executionHost: Option<Arc<dyn JsExecutionHost>>,
        toolPkgContext: Option<ToolPkgExecutionContext>,
        backgroundWake: Arc<Mutex<Option<JsBackgroundWake>>>,
        lifetimeInterrupt: HostJavaScriptExecutionInterrupt,
    ) -> Result<Self, String> {
        // JsEngineWorker::new already validates and binds this host before
        // creating the runtime state. Keep that same owner for worker requests
        // and callbacks instead of authenticating the package a second time.
        let packageBinding = Arc::new(OnceLock::new());
        if let (Some(host), Some(context)) = (&executionHost, &toolPkgContext) {
            packageBinding.set(JsPackageBinding {
                executionHost: host.clone(), context: context.clone(),
            }).map_err(|_| "ToolPkg execution context is already bound".to_string())?;
        }
        let (asyncCallbackSender, asyncCallbackReceiver) = mpsc::channel();
        let mut state = Self {
            lifetimeInterrupt,
            routeLifetime: tokio::sync::watch::channel(()).0,
            runtime,
            asyncCallbackSender,
            asyncCallbackReceiver,
            backgroundWake,
            executionHost,
            toolPkgContext,
            composeDslTextResources: None,
            detachedCallContexts: BTreeMap::new(),
            pendingScriptExecutions: BTreeMap::new(),
            registeredScriptSources: BTreeMap::new(),
            jsEnvironmentInitialized: false,
            packageBinding,
        };
        state.registerHostBindings()?;
        // Trusted SDK initialization belongs to runtime creation, not a caller's
        // script deadline. An interrupted partial bootstrap cannot be retried safely.
        state.initJavaScriptEnvironment()?;
        Ok(state)
    }

    /// Registers immutable source once per realm, using exact native content equality for reuse.
    #[allow(non_snake_case)]
    fn registerScriptSource(&mut self, script: &str) -> Result<String, String> {
        if let Some(sourceId) = self.registeredScriptSources.get(script) {
            return Ok(sourceId.clone());
        }
        let sourceId = format!("source_{}", self.registeredScriptSources.len());
        self.runtime
            .callHostJavaScriptFunction(
                "__operitRegisterScriptSource",
                &[Value::String(sourceId.clone()), Value::String(script.to_string())],
            )
            .map_err(|error| error.to_string())?;
        self.registeredScriptSources.insert(script.to_string(), sourceId.clone());
        Ok(sourceId)
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
        let _logBatch = AppLogger::batch_file_writes();
        if let JsScriptRequestStep::Start(request) = &step {
            if let Some(timing) = &request.ipcTiming {
                timing.mark("engineQueueMs").map_err(javaScriptExecutionErrorFromHost)?;
            }
        }
        self.installActiveCallContexts();
        match step {
            JsScriptRequestStep::Start(request) => {
                if request.cancelled.load(Ordering::Acquire) {
                    return Err(JsExecutionError::worker_unavailable(
                        "JavaScript execution was cancelled",
                    ));
                }
                let pending = self.startScriptFunctionForRequest(request)?;
                if let Some(timing) = &pending.context.ipcTiming {
                    timing.mark("startSyncMs").map_err(javaScriptExecutionErrorFromHost)?;
                }
                self.pendingScriptExecutions
                    .insert(pending.callId.clone(), pending);
                self.advanceJavaScriptExecutions(interrupt)
            }
            JsScriptRequestStep::Cancel(callId) => {
                if let Some(pending) = self.pendingScriptExecutions.remove(&callId) {
                    installThreadLocalCallContext(&pending.context);
                    self.cancelJavaScriptExecution(&callId);
                }
                self.advanceJavaScriptExecutions(interrupt)
            }
        }
    }

    /// Loads the module and starts its function, without retaining the state while awaiting it.
    #[allow(non_snake_case)]
    fn startScriptFunctionForRequest(
        &mut self,
        request: JsScriptRequest,
    ) -> JsExecutionResult<JsPendingScriptExecution> {
        let startedAtMs = currentTimeMillisU128();
        let JsScriptRequest {
            submittedAtMs,
            executionHost,
            callId,
            script,
            functionName,
            params,
            envOverrides,
            onIntermediateResult,
            timeout,
            interrupt,
            executionListener,
            textResources,
            useComposeDslTextResources,
            completion,
            cancelled: _,
            deadlineMillis,
            ipcTiming,
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
            interrupt,
            executionHost,
            intermediateCallback: onIntermediateResult,
            executionListener,
            envOverrides: Arc::new(Mutex::new(envOverrides)),
            environmentOwner: environmentOwner(self.toolPkgContext.as_ref(), &params, false)?,
            textResourceSource,
            ipcTiming,
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
            bindEnvironmentOwner(&mut effectiveParams, context.environmentOwner.as_ref());
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
            let invokeAtMs = currentTimeMillisU128();
            let scriptBytes = script.len();
            let sourceCount = self.registeredScriptSources.len();
            let sourceId = self.registerScriptSource(&script).map_err(JsExecutionError::runtime)?;
            let sourceRegistered = self.registeredScriptSources.len() != sourceCount;
            let toolPkgContext = toolPkgCallContext(self.toolPkgContext.as_ref(), &effectiveParams)
                .map_err(JsExecutionError::invalid_request)?;
            let invocation = self.runtime.callHostJavaScriptFunction(
                "__operitExecuteScriptFunction",
                &[
                    Value::String(callId.clone()),
                    Value::Object(effectiveParams.into_iter().collect()),
                    Value::String(sourceId),
                    Value::String(functionName.clone()),
                    Value::Bool(useComposeDslTextResources),
                    toolPkgContext,
                ],
            );
            let completedAtMs = currentTimeMillisU128();
            AppLogger::trace("JsExecutionTiming", &format!(
                "kind=js_start callId={} function={} scriptBytes={} sourceRegistered={} queueMs={} initializeMs={} invokeMs={}",
                callId, functionName, scriptBytes, sourceRegistered,
                startedAtMs.saturating_sub(submittedAtMs),
                invokeAtMs.saturating_sub(startedAtMs),
                completedAtMs.saturating_sub(invokeAtMs),
            ));
            if let Err(error) = invocation {
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

        let environmentOwner = environmentOwner(self.toolPkgContext.as_ref(), params, false)?;
        let mut effectiveParams = params.clone();
        bindEnvironmentOwner(&mut effectiveParams, environmentOwner.as_ref());
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
                    interrupt: HostJavaScriptExecutionInterrupt::unlimited(),
                    executionHost: self.executionHost.clone(),
                    intermediateCallback: CURRENT_INTERMEDIATE_CALLBACK
                        .with(|value| value.borrow().clone()),
                    executionListener: CURRENT_EXECUTION_LISTENER
                        .with(|value| value.borrow().clone()),
                    envOverrides: Arc::new(Mutex::new(envOverrides.clone())),
                    environmentOwner,
                    textResourceSource,
                    ipcTiming: None,
                },
            );
        });
        clearNativeExecutionSession(&callId);
        let output = match self.invokeExecutionFunction(
            &callId,
            effectiveParams,
            script,
            functionName,
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
            let environmentOwner = environmentOwner(self.toolPkgContext.as_ref(), params, true)?;
            if let Some(declared) = params.get("__operit_registration_environment_names") {
                let declared: Vec<String> = serde_json::from_value(declared.clone()).map_err(|error| {
                    JsExecutionError::invalid_request(format!("Invalid environment declarations: {error}"))
                })?;
                if !declared.is_empty() {
                    let owner = environmentOwner.as_ref().ok_or_else(|| {
                        JsExecutionError::invalid_request("Environment preparation requires an owning package")
                    })?;
                    self.executionHost
                        .as_ref()
                        .ok_or_else(|| {
                            JsExecutionError::invalid_request("Environment preparation requires an execution host")
                        })?
                        .prepare_environment(owner, &declared)
                        .map_err(JsExecutionError::runtime)?;
                }
            }
            let mut registrationParams = params.clone();
            bindEnvironmentOwner(&mut registrationParams, environmentOwner.as_ref());
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
                    interrupt: HostJavaScriptExecutionInterrupt::unlimited(),
                        executionHost: self.executionHost.clone(),
                        intermediateCallback: None,
                        executionListener: None,
                        envOverrides: Arc::new(Mutex::new(BTreeMap::new())),
                        environmentOwner,
                        textResourceSource,
                        ipcTiming: None,
                    },
                );
            });
            clearNativeExecutionSession(&callId);
            if let Err(error) =
                self.invokeExecutionFunction(&callId, registrationParams, script, functionName)
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
            ensureRegistrationExecutionSucceeded(&output?).map_err(JsExecutionError::runtime)?;

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
        // Rejection resumes finally blocks under a fresh deadline, independent of the cancelled request.
        let cleanup = HostJavaScriptExecutionInterrupt::new(100)
            .expect("cancellation cleanup deadline must fit the host clock");
        let scope = ExecutionScope::enter(self, Some(cleanup.clone()));
        CURRENT_CLEANUP_OWNER.with(|owner| *owner.borrow_mut() = Some((callId.to_owned(), cleanup)));
        // Retire the session before rejecting host waits: Web can run their cleanup jobs immediately.
        if let Err(error) = self.runtime.callHostJavaScriptFunction(
            "__operitCancelCallSession",
            &[Value::String(callId.to_string())],
        ) {
            AppLogger::e(
                TAG,
                &format!("cancel JavaScript execution failed callId={callId}: {error}"),
            );
        }
        activateCancelledScriptCall(callId.to_owned());
        if let Err(error) = self.runtime.cancelHostJavaScriptPromises(callId) {
            AppLogger::e(
                TAG,
                &format!("cancel structured JavaScript promises failed: {error}"),
            );
        }
        if let Err(error) = self.runJavaScriptJobs() {
            AppLogger::e(TAG, &format!("cancel JavaScript cleanup failed callId={callId}: {error}"));
        }
        drop(scope);
    }

    /// Advances owned callbacks without imposing a shared runtime deadline.
    #[allow(non_snake_case)]
    fn advanceJavaScriptExecutions(
        &mut self,
        _hostInterrupt: HostJavaScriptExecutionInterrupt,
    ) -> JsExecutionResult<()> {
        let _logBatch = AppLogger::batch_file_writes();
        let scope = ExecutionScope::enter(self, None);
        let jobs = self.advanceDetachedJavaScriptExecution();
        let interruptedCall = CURRENT_SCRIPT_OWNER.with(|current| {
            current.borrow().as_ref().and_then(|(owner, token)| {
                token.shouldInterrupt().then(|| (owner.clone(), token.didTimeOut()))
            })
        });
        drop(scope);
        let advanced = if let Some((_, timedOut)) = interruptedCall.as_ref() {
            Err(if *timedOut {
                JsExecutionError::timeout("Before hook execution timed out")
            } else {
                JsExecutionError::worker_unavailable("JavaScript execution was cancelled")
            })
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
                Err(error) if interruptedCall.as_ref().is_none_or(|(owner, _)| owner == &callId) => Some(Err(error.clone())),
                Err(_) => readNativeExecutionSession(&callId).map(|output| output.map(Some)),
                Ok(()) => {
                    let pending = &self.pendingScriptExecutions[&callId];
                    if pending.deadlineMillis.is_some_and(|deadline| currentTimeMillisU128() >= deadline) {
                        Some(Err(JsExecutionError::timeout(format!(
                            "Script execution timed out after {} milliseconds",
                            pending.timeout.expect("Timed before hook must own a duration").as_millis(),
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
                if let Some(timing) = &pending.context.ipcTiming {
                    if let Err(error) = timing.mark("activeMs") {
                        result = Err(javaScriptExecutionErrorFromHost(error));
                    }
                }
                // Retain detached context before publishing the primary result.
                completeScriptExecution(&pending.completion, result);
            }
        }
        clearThreadLocalCallState();
        if interruptedCall.is_some() { Ok(()) } else { advanced }
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

    /// Retains the callback context while a call owns pending Promise reactions or host work.
    fn rememberDetachedCallContext(
        &mut self,
        pending: &JsPendingScriptExecution,
    ) -> Result<(), String> {
        self.runtime.callHostJavaScriptFunction(
            "__operitFinalizeDetachedCall",
            &[Value::String(pending.callId.clone())],
        ).map_err(|error| error.to_string())?;
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
                    .try_into()
                    .expect("JavaScript deadline must fit in milliseconds"),
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
            JsAsyncCallback::Promise { requestId, result, reject } => {
                self.runtime.settleHostJavaScriptPromise(requestId, result, reject)
            }
            JsAsyncCallback::ProfiledToolResult { requestId, result, timing, successful } => {
                timing.mark("callbackQueueMs").map_err(|error| error.to_string())?;
                let settled = self.runtime.settleHostJavaScriptPromise(requestId, result, false);
                timing.mark("callbackSettleMs").map_err(|error| error.to_string())?;
                timing.log(successful && settled.is_ok());
                settled
            }
            JsAsyncCallback::CancelScope { scope } => self.runtime.cancelHostJavaScriptPromises(&scope),
        }.map_err(|error| error.to_string())
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
    ) -> Result<(), String> {
        let sourceId = self.registerScriptSource(script)?;
        let toolPkgContext = toolPkgCallContext(self.toolPkgContext.as_ref(), &params)?;
        self.runtime
            .callHostJavaScriptFunction(
                "__operitExecuteScriptFunction",
                &[
                    Value::String(callId.to_string()),
                    Value::Object(params.into_iter().collect()),
                    Value::String(sourceId),
                    Value::String(functionName.to_string()),
                    Value::Bool(false),
                    toolPkgContext,
                ],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Installs the single structured host transport for synchronous and Promise-backed operations.
    #[allow(non_snake_case)]
    fn registerHostBindings(&mut self) -> Result<(), String> {
        self.runtime
            .registerHostJavaScriptFunctionConstructor(
                "__operitNativeCompileModuleFactory",
                &["module", "exports", "require", "__operit_call_runtime"],
            )
            .map_err(|error| error.to_string())?;
        let syncNames = [
            "__operitNativePrepareComposeModule",
            "__operitNativeReadBootstrapLibrary",
            "__operitNativeHashText",
            "__operitNativeGetPluginLocalDataDir",
            "__operitNativeGetPluginSpaceDataDir",
            "__operitNativeSetCallStructuredResult",
            "__operitNativeSendStructuredIntermediate",
            "__operitNativeReadToolPkgTextResource",
            "__operitNativeGetEnvForCall",
            "__operitNativeSetEnv",
            "__operitNativeSetEnvs",
            "__operitNativeGetScopedPluginConfigDir",
            "__operitNativeGetToolCatalog",
            "__operitNativeComposeWebViewControllerCommand",
            "__operitNativeLog",
            "__operitNativeLogVerbose",
            "__operitNativeRegisterMedia",
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
            "__operitNativeActivateCall",
            "__operitNativeSetCallResult",
            "__operitNativeSetCallError",
            "__operitNativeLogJsExecutionTrace",
            "__operitNativeCloseCoreRouteWatch",
            "__operitNativeShouldRouteToolPkg",
        ];
        for name in syncNames {
            self.runtime
                .registerHostJavaScriptFunction(
                    name,
                    Arc::new(move |arguments| executeHostOperation(name, arguments)),
                )
                .map_err(|error| error.to_string())?;
        }

        self.runtime.registerHostJavaScriptBinaryFunction(
            "__operitNativeLoadToolPkgWasm",
            Arc::new(|arguments| {
                let [package, module] = exactHostJavaScriptJsonArguments("__operitNativeLoadToolPkgWasm", arguments)?;
                let package = hostJavaScriptStringArgument(package)?;
                let module = hostJavaScriptStringArgument(module)?;
                let loaded = currentExecutionHost().and_then(|host| host.load_toolpkg_wasm(&package, &module))
                    .map_err(HostError::new)?;
                Ok((serde_json::json!({"exports": loaded.exports}), loaded.bytes))
            }),
        ).map_err(|error| error.to_string())?;

        let detachedWake = self.backgroundWake.clone();
        self.runtime.registerHostJavaScriptFunction(
            "__operitNativeNotifyDetachedCall",
            Arc::new(move |arguments| {
                let [callId] = exactHostJavaScriptJsonArguments(
                    "__operitNativeNotifyDetachedCall",
                    arguments.into_iter().map(operit_host_api::HostJavaScriptValue::into_json).collect::<HostResult<Vec<_>>>()?,
                )?;
                hostJavaScriptStringArgument(callId)?;
                let wake = detachedWake.lock().expect("background wake mutex poisoned").clone();
                if let Some(wake) = wake { wake(); }
                Ok(Value::Null)
            }),
        ).map_err(|error| error.to_string())?;

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
        let toolSink = asyncCallbackSink.clone();
        let timerSink = asyncCallbackSink.clone();
        let ipcBinding = self.packageBinding.clone();
        let ipcSink = asyncCallbackSink.clone();
        let dependencyBinding = self.packageBinding.clone();
        let dependencySink = asyncCallbackSink.clone();
        let callRouteSink = asyncCallbackSink.clone();
        let openRouteWatchSink = asyncCallbackSink.clone();
        let nextRouteWatchSink = asyncCallbackSink.clone();
        let storageRecordSink = asyncCallbackSink.clone();
        let storageRecordLifetime = self.routeLifetime.subscribe();
        let callRouteLifetime = self.routeLifetime.subscribe();
        let openRouteWatchLifetime = self.routeLifetime.subscribe();
        let nextRouteWatchLifetime = self.routeLifetime.subscribe();
        let openRoutePushLifetime = self.routeLifetime.subscribe();
        let sendRoutePushLifetime = self.routeLifetime.subscribe();
        let closeRoutePushLifetime = self.routeLifetime.subscribe();
        let openRoutePushSink = asyncCallbackSink.clone();
        let sendRoutePushSink = asyncCallbackSink.clone();
        let closeRoutePushSink = asyncCallbackSink.clone();
        let asyncFunctions: Vec<(&str, HostJavaScriptAsyncJsonCallback)> = vec![
            (
                "__operitNativeCallToolStructured",
                Arc::new(move |requestId, arguments| {
                    dispatchStructuredToolCall(
                        CURRENT_EXECUTION_HOST.with(|host| host.borrow().clone()),
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
                    let binding = ipcBinding.get().ok_or_else(|| {
                        HostError::new("ToolPkg IPC requires a bound package context")
                    })?;
                    let context = &binding.context;
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
                    dispatchToolPkgIpcAsync(Some(binding.executionHost.clone()), ipcSink.clone(), requestId, request);
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
                    let binding = dependencyBinding.get().ok_or_else(|| {
                        HostError::new("Dependency calls require a bound ToolPkg context")
                    })?;
                    let context = &binding.context;
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
                        Some(binding.executionHost.clone()),
                        dependencySink.clone(),
                        requestId,
                        request,
                    );
                    Ok(())
                }),
            ),
            (
                "__operitNativeCallCoreRoute",
                Arc::new(move |requestId, arguments| {
                    let [target, method, args] = exactHostJavaScriptJsonArguments(
                        "__operitNativeCallCoreRoute",
                        arguments,
                    )?;
                    let host = CURRENT_EXECUTION_HOST.with(|value| value.borrow().clone());
                    let Some(host) = host else {
                        return Err(HostError::new("JavaScript execution host is unavailable"));
                    };
                    let target = hostJavaScriptStringArgument(target)?;
                    let method = hostJavaScriptStringArgument(method)?;
                    dispatchCoreRouteAsync(
                        Some(host.clone()),
                        callRouteSink.clone(),
                        callRouteLifetime.clone(),
                        requestId,
                        Box::new(move || host.call_core_route(target, method, args)),
                    );
                    Ok(())
                }),
            ),
            (
                "__operitNativeOpenCoreRouteWatch",
                Arc::new(move |requestId, arguments| {
                    let [target, property, args] = exactHostJavaScriptJsonArguments(
                        "__operitNativeOpenCoreRouteWatch",
                        arguments,
                    )?;
                    let host = CURRENT_EXECUTION_HOST.with(|value| value.borrow().clone());
                    let Some(host) = host else {
                        return Err(HostError::new("JavaScript execution host is unavailable"));
                    };
                    let target = hostJavaScriptStringArgument(target)?;
                    let property = hostJavaScriptStringArgument(property)?;
                    dispatchCoreRouteAsync(
                        Some(host.clone()),
                        openRouteWatchSink.clone(),
                        openRouteWatchLifetime.clone(),
                        requestId,
                        Box::new(move || host.open_core_route_watch(target, property, args)),
                    );
                    Ok(())
                }),
            ),
            (
                "__operitNativeNextCoreRouteWatch",
                Arc::new(move |requestId, arguments| {
                    let [subscription] = exactHostJavaScriptJsonArguments(
                        "__operitNativeNextCoreRouteWatch",
                        arguments,
                    )?;
                    let host = CURRENT_EXECUTION_HOST.with(|value| value.borrow().clone());
                    let Some(host) = host else {
                        return Err(HostError::new("JavaScript execution host is unavailable"));
                    };
                    let subscription = hostJavaScriptStringArgument(subscription)?;
                    dispatchCoreRouteAsync(
                        Some(host.clone()),
                        nextRouteWatchSink.clone(),
                        nextRouteWatchLifetime.clone(),
                        requestId,
                        Box::new(move || host.next_core_route_watch(subscription)),
                    );
                    Ok(())
                }),
            ),
            (
                "__operitNativeOpenToolPkgRoutePush",
                Arc::new(move |requestId, arguments| {
                    let [method, args] = exactHostJavaScriptJsonArguments("__operitNativeOpenToolPkgRoutePush", arguments)?;
                    let host = currentExecutionHost().map_err(HostError::new)?;
                    let method = hostJavaScriptStringArgument(method)?;
                    dispatchCoreRouteAsync(Some(host.clone()), openRoutePushSink.clone(), openRoutePushLifetime.clone(), requestId,
                        Box::new(move || host.open_toolpkg_route_push(method, args)));
                    Ok(())
                }),
            ),
            (
                "__operitNativeSendToolPkgRoutePush",
                Arc::new(move |requestId, arguments| {
                    let [id, value] = exactHostJavaScriptJsonArguments("__operitNativeSendToolPkgRoutePush", arguments)?;
                    let host = currentExecutionHost().map_err(HostError::new)?;
                    let id = hostJavaScriptStringArgument(id)?;
                    dispatchCoreRouteAsync(Some(host.clone()), sendRoutePushSink.clone(), sendRoutePushLifetime.clone(), requestId,
                        Box::new(move || host.send_toolpkg_route_push(id, value)));
                    Ok(())
                }),
            ),
            (
                "__operitNativeCloseToolPkgRoutePush",
                Arc::new(move |requestId, arguments| {
                    let [id, abort] = exactHostJavaScriptJsonArguments("__operitNativeCloseToolPkgRoutePush", arguments)?;
                    let host = currentExecutionHost().map_err(HostError::new)?;
                    let id = hostJavaScriptStringArgument(id)?;
                    let abort = abort.as_bool().ok_or_else(|| HostError::new("Push abort flag must be a boolean"))?;
                    dispatchCoreRouteAsync(Some(host.clone()), closeRoutePushSink.clone(), closeRoutePushLifetime.clone(), requestId,
                        Box::new(move || host.close_toolpkg_route_push(id, abort)));
                    Ok(())
                }),
            ),
            (
                "__operitNativeWaitPluginStorageRecord",
                Arc::new(move |requestId, arguments| {
                    let [handle, collection, key, after] = exactHostJavaScriptJsonArguments(
                        "__operitNativeWaitPluginStorageRecord", arguments,
                    )?;
                    let host = currentExecutionHost().map_err(HostError::new)?;
                    let handle = hostJavaScriptStringArgument(handle)?;
                    let collection = hostJavaScriptStringArgument(collection)?;
                    let key = hostJavaScriptStringArgument(key)?;
                    let after = serde_json::from_value(after.clone())
                        .map_err(|error| HostError::new(format!("Invalid record watch state: {error}")))?;
                    dispatchCoreRouteAsync(Some(host.clone()), storageRecordSink.clone(),
                        storageRecordLifetime.clone(), requestId,
                        Box::new(move || host.wait_plugin_storage_record(handle, collection, key, after)));
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
            "__operitNativeComposeFileSaveCommand",
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
            .registerHostJavaScriptFunction(
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
        self.runtime.evaluateHostJavaScriptBootstrap(bootstrap)
            .map_err(|error| error.to_string())?;
        self.jsEnvironmentInitialized = true;
        Ok(())
    }
}

/// Validates the exact argument count of a structured host binding.
#[allow(non_snake_case)]
fn exactHostJavaScriptJsonArguments<const N: usize>(
    name: &str,
    arguments: Vec<impl Into<operit_host_api::HostJavaScriptValue>>,
) -> HostResult<[Value; N]> {
    let arguments = arguments.into_iter().map(|value| value.into().into_json()).collect::<HostResult<Vec<_>>>()?;
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
    let mut arguments = arguments;
    let device = if arguments.len() == 5 { arguments.pop() } else { None };
    let [toolType, toolName, params, callId]: [Value; 4] = match arguments.try_into() {
        Ok(arguments) => arguments,
        Err(_) => {
            fail("Structured tool call requires type, name, params, owning call ID and optional device metadata".to_string());
            return;
        }
    };
    let (Some(toolType), Some(toolName)) = (toolType.as_str(), toolName.as_str()) else {
        fail("Tool type and name must be strings".to_string());
        return;
    };
    let Some(callId) = callId.as_str() else {
        fail("Tool owning call ID must be a string".to_string());
        return;
    };
    let context = CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow().get(callId).cloned());
    let Some(context) = context else {
        fail("Tool call requires an active owning call".to_string());
        return;
    };
    let timing = match context.ipcTiming.as_ref().map(|parent| {
        JsIpcTiming::new(parent.traceId.clone(), callId.to_string(), "tool",
            format!("{requestId}:{toolType}:{toolName}"))
    }).transpose() {
        Ok(timing) => timing,
        Err(error) => { fail(error.to_string()); return; }
    };
    let mut request = match JsHostOperations::parseToolCallValue(toolType, toolName, params) {
        Ok(request) => request,
        Err(error) => {
            fail(error);
            return;
        }
    };
    if let Some(device) = device {
        let metadata = serde_json::from_value::<operit_plugin_sdk::javascript::JsDeviceToolCall>(device)
            .map_err(|error| format!("Invalid device metadata: {error}"))
            .and_then(|metadata| { metadata.validate()?; Ok(metadata) });
        match metadata {
            Ok(metadata) => request.device = Some(metadata),
            Err(error) => { fail(error); return; }
        }
    }
    let Some(executionHost) = executionHost else {
        fail("JavaScript execution host is unavailable".to_string());
        return;
    };
    // Reserve device invocations before scheduling so an immediate cancel cannot overtake
    // the original call. Execution itself remains asynchronous on the host scheduler.
    let completion = if request.device.is_some() {
        executionHost.execute_tool_call(request)
    } else {
        Box::pin(async move { executionHost.execute_tool_call(request).await })
    };
    let sink = callbackSink.clone();
    // Tool executors may perform blocking host calls (terminal sessions, device
    // commands, package I/O), so they run on the blocking pool. Coordination
    // deadlines and promise pumps must stay responsive behind a busy tool.
    let scheduled = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeBlockingAsyncTask(
        "operit-js-tool-call",
        Box::new(move || {
            Box::pin(async move {
                let result = async {
                    if let Some(timing) = &timing { timing.mark("schedulerWaitMs")?; }
                    let result = completion.await;
                    let successful = result.success;
                    if let Some(timing) = &timing { timing.mark("hostExecuteMs")?; }
                    let value = JsHostOperations::toolExecutionResultValue(result);
                    if let Some(timing) = &timing { timing.mark("resultEncodeMs")?; }
                    Ok::<_, HostError>((value, successful))
                }.await;
                let (value, successful) = match result {
                    Ok(result) => result,
                    Err(error) => (serde_json::json!({"success": false, "message": error.to_string()}), false),
                };
                let callback = match timing {
                    Some(timing) => JsAsyncCallback::ProfiledToolResult {
                        requestId, result: value, timing, successful,
                    },
                    None => JsAsyncCallback::Promise { requestId, result: value, reject: false },
                };
                sink(callback);
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

/// Schedules one Core route Promise and settles it on the owning JavaScript runtime.
#[allow(non_snake_case)]
fn dispatchCoreRouteAsync(
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    callbackSink: JsAsyncCallbackSink,
    mut lifetime: tokio::sync::watch::Receiver<()>,
    requestId: u64,
    completion: Box<dyn FnOnce() -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> + Send>,
) {
    let Some(_executionHost) = executionHost else {
        callbackSink(JsAsyncCallback::Promise {
            requestId,
            result: Value::String("JavaScript execution host is unavailable".to_string()),
            reject: true,
        });
        return;
    };
    let sink = callbackSink.clone();
    let scheduled = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
        "operit-core-route",
        Box::new(move || {
            Box::pin(async move {
                let result = tokio::select! {
                    biased;
                    _ = lifetime.changed() => return,
                    result = completion() => result,
                };
                match result {
                    Ok(value) => sink(JsAsyncCallback::Promise {
                        requestId,
                        result: value,
                        reject: false,
                    }),
                    Err(error) => sink(JsAsyncCallback::Promise {
                        requestId,
                        result: Value::String(error),
                        reject: true,
                    }),
                }
            })
        }),
    );
    if let Err(error) = scheduled {
        callbackSink(JsAsyncCallback::Promise {
            requestId,
            result: Value::String(error.to_string()),
            reject: true,
        });
    }
}

/// Normalizes execution identity once at the native entry, before any plugin code runs.
/// The engine owns package/context identity; a shared main engine can execute sandbox calls.
#[allow(non_snake_case)]
fn toolPkgCallContext(
    bound: Option<&ToolPkgExecutionContext>,
    params: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    let text = |key: &str| {
        params
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    };
    let packageTarget = bound
        .map(|context| context.container_package_name.as_str())
        .or_else(|| text("__operit_ui_package_name"))
        .or_else(|| text("toolPkgId"))
        .unwrap_or_default();
    let contextKey = bound
        .map(|context| context.context_key.clone())
        .or_else(|| text("__operit_execution_context_key").map(str::to_owned))
        .unwrap_or_else(|| {
            if packageTarget.is_empty() {
                String::new()
            } else {
                format!("toolpkg_main:{packageTarget}")
            }
        });
    let runtime = match text("__operit_toolpkg_runtime_kind") {
        Some(value) => value.to_ascii_lowercase(),
        None => {
            let key = contextKey.to_ascii_lowercase();
            if key.starts_with("toolpkg_provider:") {
                "provider"
            } else if key.starts_with("toolpkg_compose:")
                || key.starts_with("toolpkg_compose_dsl:")
                || key.starts_with("toolpkg_xml_render:")
            {
                "ui"
            } else if text("__operit_toolpkg_subpackage_id").is_some() {
                "sandbox"
            } else {
                "main"
            }
            .to_string()
        }
    };
    if !matches!(runtime.as_str(), "main" | "ui" | "sandbox" | "provider") {
        return Err(format!("ToolPkg execution runtime is invalid: {runtime}"));
    }
    Ok(serde_json::json!({
        "packageTarget": packageTarget,
        "contextKey": contextKey,
        "runtime": runtime,
    }))
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
    selectScriptExecution(None);
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

/// Installs one engine-owned call context for the next Host JavaScript execution step.
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
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        JsEngine::execute_script_function(
            self,
            script,
            functionName,
            params,
            envOverrides,
            on_intermediate_result,
            dispatchIntermediateOnMain,
            None,
        )
    }

    /// Moves the selected script and argument maps directly onto the owning JavaScript worker.
    fn execute_script_function_owned(
        &self, script: String, function_name: String, params: BTreeMap<String, Value>,
        env_overrides: BTreeMap<String, String>,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>> {
        textScriptCompletion(self.worker.execute_script_function_async(
            script, function_name, params, env_overrides, None, true, None, None, None, false,
        ))
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
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
        JsEngine::execute_script_function_async(
            self,
            script,
            functionName,
            params,
            envOverrides,
            on_intermediate_result,
            dispatchIntermediateOnMain,
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

    /// Streams render responses directly from the retained commit's owning JavaScript call.
    fn execute_compose_dsl_script_stream_async(
        &self, script: String, runtime_options: BTreeMap<String, Value>,
        env_overrides: BTreeMap<String, String>, text_resources: Arc<ToolPkgTextResources>,
        on_response: Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
        JsEngine::execute_compose_dsl_script_stream_async(self, script, runtime_options, env_overrides, text_resources, on_response)
    }

    /// Dispatches one Compose DSL action through the asynchronous engine contract.
    #[allow(non_snake_case)]
    fn dispatch_compose_dsl_action_stream_async(
        &self,
        actionId: String,
        payload: Option<Value>,
        runtimeOptions: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_response: Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>> {
        JsEngine::dispatch_compose_dsl_action_stream_async(
            self,
            actionId,
            payload,
            runtimeOptions,
            envOverrides,
            on_response,
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
        callback(Value::String(result).into());
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
fn textIntermediateCallback(callback: Option<Arc<dyn Fn(String) + Send + Sync>>) -> Option<Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>> {
    callback.map(|callback| Arc::new(move |value| {
        let operit_host_api::HostJavaScriptValue::Json(Value::String(text)) = value else { panic!("Text intermediate callback received a non-text value"); };
        callback(text);
    }) as Arc<dyn Fn(operit_host_api::HostJavaScriptValue) + Send + Sync>)
}

/// Delivers DSL values to their active or detached execution owner without JSON text.
fn sendStructuredIntermediate(callId: &str, value: operit_host_api::HostJavaScriptValue) {
    let active = CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow().get(callId).and_then(|context| context.intermediateCallback.clone()));
    let detached = CURRENT_DETACHED_INTERMEDIATE_CALLBACKS.with(|callbacks| callbacks.borrow().get(callId).cloned());
    match (active, detached) {
        (Some(callback), _) => callback(value),
        (None, Some(callback)) => callback(value),
        (None, None) => {},
    }
}

#[allow(non_snake_case)]
fn executeHostOperation(name: &str, arguments: Vec<impl Into<operit_host_api::HostJavaScriptValue>>) -> HostResult<Value> {
    use operit_host_api::HostJavaScriptValue;
    let arguments: Vec<HostJavaScriptValue> = arguments.into_iter().map(Into::into).collect();
    if name == "__operitNativeSendStructuredIntermediate" {
        let [call_id, value]: [_; 2] = arguments.try_into().map_err(|_| HostError::new("Compose response requires its owner and one binary frame"))?;
        let call_id = hostJavaScriptStringArgument(call_id.into_json()?)?;
        sendStructuredIntermediate(&call_id, value);
        return Ok(Value::Null);
    }
    let arguments = arguments.into_iter().map(HostJavaScriptValue::into_json).collect::<HostResult<Vec<_>>>()?;
    if name == "__operitNativeActivateCall" {
        let [callId, cancelled] = exactHostJavaScriptJsonArguments(name, arguments)?;
        let callId = hostJavaScriptStringArgument(callId)?;
        let cancelled = hostJavaScriptBoolArgument(cancelled)?;
        if cancelled {
            activateCancelledScriptCall(callId);
        } else {
            let owner = CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| {
                contexts.borrow().get(&callId).map(|context| (callId.clone(), context.interrupt.clone()))
            });
            selectScriptExecution(owner);
        }
        return Ok(Value::Null);
    }
    match name {
        "__operitNativeSetCallStructuredResult" => {
            let [callId, value] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let callId = hostJavaScriptStringArgument(callId)?;
            if !CURRENT_ACTIVE_CALL_CONTEXTS
                .with(|contexts| contexts.borrow().contains_key(&callId))
            {
                return Err(HostError::new("Completion requires an active owning call"));
            }
            CURRENT_CALL_RESULTS.with(|results| {
                results.borrow_mut().insert(callId, Ok(value));
            });
            Ok(Value::Null)
        }
        "__operitNativeGetPluginLocalDataDir" | "__operitNativeGetPluginSpaceDataDir" => {
            let [owner] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let owner = hostJavaScriptStringArgument(owner)?;
            let host = currentExecutionHost().map_err(HostError::new)?;
            let local = name == "__operitNativeGetPluginLocalDataDir";
            let path =
                CURRENT_REGISTRATION_CONFIG_PARAMS
                    .with(|current| match current.borrow().as_ref() {
                        Some(params) => {
                            let registeredOwner =
                                params.get("toolPkgId").and_then(Value::as_str).ok_or_else(
                                    || "ToolPkg registration data owner is missing".to_string(),
                                )?;
                            if owner != registeredOwner {
                                return Err("ToolPkg registration data owner mismatch".to_string());
                            }
                            if local {
                                host.registration_plugin_local_data_dir(registeredOwner)
                            } else {
                                host.registration_plugin_space_data_dir(registeredOwner)
                            }
                        }
                        None => {
                            if local {
                                host.plugin_local_data_dir(&owner)
                            } else {
                                host.plugin_space_data_dir(&owner)
                            }
                        }
                    })
                    .map_err(HostError::new)?;
            if !path.starts_with('/') {
                return Err(HostError::new(
                    "Plugin data directory must be an absolute VFS path",
                ));
            }
            Ok(Value::String(path))
        }
        "__operitNativePrepareComposeModule" => {
            let [source, identity] = exactHostJavaScriptJsonArguments(name, arguments)?;
            crate::javascript::JsComposeCompiler::prepareComposeModule(
                hostJavaScriptStringArgument(source)?, hostJavaScriptStringArgument(identity)?,
            ).map(Value::String)
        }
        "__operitNativeReadBootstrapLibrary" => {
            let [library] = exactHostJavaScriptJsonArguments(name, arguments)?;
            loadBootstrapLibrary(&hostJavaScriptStringArgument(library)?)
                .map(Value::String).map_err(HostError::new)
        }
        "__operitNativeHashText" => {
            let [source] = exactHostJavaScriptJsonArguments(name, arguments)?;
            Ok(Value::String(javaScriptSourceFingerprint(
                &hostJavaScriptStringArgument(source)?,
            )))
        }
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
        "__operitNativeComposeFileSaveCommand" => {
            let [payload] = exactHostJavaScriptJsonArguments(name, arguments)?;
            requireHostObject(&payload)?;
            currentExecutionHost()
                .and_then(|host| host.save_compose_file(&payload))
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
                .read_environment_variable(context.environmentOwner.as_ref().ok_or_else(||
                    HostError::new("Environment access requires an owning package installation"))?, &key)
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
        "__operitNativeRegisterMedia" => {
            let [payload] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let payload = hostJavaScriptStringArgument(payload)?;
            currentExecutionHost()
                .and_then(|host| host.register_media(&payload))
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
        "__operitNativeLogVerbose" => {
            let [level, callId, message] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let level = level
                .as_i64()
                .filter(|level| (1..=6).contains(level))
                .ok_or_else(|| HostError::new("Verbose level must be an integer from 1 to 6"))?
                as i32;
            let callId = hostJavaScriptStringArgument(callId)?;
            let message = hostJavaScriptStringArgument(message)?;
            let message = if callId.is_empty() {
                message
            } else {
                format!("[{callId}] {message}")
            };
            AppLogger::v_with_level("ToolPkg", &message, level);
            Ok(Value::Null)
        }
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
                "debug" => AppLogger::d("ToolPkg", &message),
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
        "__operitNativeLogJsExecutionTrace" => {
            let [callId, message] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let callId = hostJavaScriptStringArgument(callId)?;
            let message = hostJavaScriptStringArgument(message)?;
            AppLogger::trace_detail(TAG, &format!("[{callId}] {message}"));
            Ok(Value::Null)
        }
        "__operitNativeShouldRouteToolPkg" => {
            let [binding, streaming] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let binding = hostJavaScriptStringArgument(binding)?;
            let streaming = streaming.as_bool().ok_or_else(|| HostError::new("Route stream flag must be a boolean"))?;
            currentExecutionHost().map_err(HostError::new)?.should_route_toolpkg(binding, streaming).map(Value::Bool).map_err(HostError::new)
        }
        "__operitNativeCloseCoreRouteWatch" => {
            let [subscription] = exactHostJavaScriptJsonArguments(name, arguments)?;
            let subscription = hostJavaScriptStringArgument(subscription)?;
            currentExecutionHost()
                .map_err(HostError::new)?
                .close_core_route_watch(subscription)
                .map_err(HostError::new)?;
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

/// Publishes native ownership metadata for versioned SDK adapters without accepting caller aliases.
#[allow(non_snake_case)]
fn bindEnvironmentOwner(params: &mut BTreeMap<String, Value>, owner: Option<&JsEnvironmentOwner>) {
    params.remove("__operit_environment_owner");
    if let Some(owner) = owner {
        params.insert(
            "__operit_environment_owner".into(),
            Value::String(owner.package_name().into()),
        );
    }
}

/// Captures the environment owner before JavaScript runs or schedules asynchronous callbacks.
#[allow(non_snake_case)]
fn environmentOwner(
    context: Option<&ToolPkgExecutionContext>,
    params: &BTreeMap<String, Value>,
    registration: bool,
) -> JsExecutionResult<Option<JsEnvironmentOwner>> {
    if registration {
        return match params.get("toolPkgId") {
            None => Ok(None),
            Some(owner) => {
                let id = owner
                    .as_str()
                    .filter(|id| !id.trim().is_empty())
                    .ok_or_else(|| {
                        JsExecutionError::invalid_request(
                            "Registration environment owner is invalid",
                        )
                    })?;
                let scope = params
                    .get("__operit_registration_config_scope")
                    .ok_or_else(|| {
                        JsExecutionError::invalid_request(
                            "ToolPkg registration configuration scope is missing",
                        )
                    })?;
                let scope = serde_json::from_value(scope.clone()).map_err(|error| {
                    JsExecutionError::invalid_request(format!("Invalid environment scope: {error}"))
                })?;
                Ok(Some(JsEnvironmentOwner::Registration {
                    id: id.to_string(),
                    scope,
                }))
            }
        };
    }
    match context {
        Some(context) => Ok(Some(JsEnvironmentOwner::Package(
            context.container_package_name.clone(),
        ))),
        None => match params.get("__operit_package_name") {
            None => Ok(None),
            Some(name) => name
                .as_str()
                .filter(|name| !name.trim().is_empty())
                .map(|name| Some(JsEnvironmentOwner::Package(name.to_string())))
                .ok_or_else(|| {
                    JsExecutionError::invalid_request("Package environment owner is invalid")
                }),
        },
    }
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
        .write_environment_variable(context.environmentOwner.as_ref().ok_or_else(||
            HostError::new("Environment access requires an owning package installation"))?, key, value)
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

/// Validates registration after adapting its explicit ordinary-script text result contract.
#[allow(non_snake_case)]
fn ensureRegistrationExecutionSucceeded(value: &Value) -> Result<(), String> {
    let decoded;
    let value = match value {
        Value::String(text) => {
            let text = text.trim();
            if text.is_empty() || text == "undefined" {
                return Ok(());
            }
            decoded = serde_json::from_str::<Value>(text).map_err(|error| error.to_string())?;
            &decoded
        }
        value => value,
    };
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

