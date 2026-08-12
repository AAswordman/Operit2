use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::mpsc;
use std::sync::Arc;
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
    JsExecutionEngine, JsExecutionFuture, JsExecutionHost, JsToolNameResolutionRequest,
    JsToolPkgIpcRequest, JsToolPkgResourceRequest, JsToolPkgWasmArg, JsToolPkgWasmRequest,
    ToolPkgExecutionContext, ToolPkgMainRegistrationCapture, ToolPkgTextResourceHost,
};
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
    static CURRENT_EXECUTION_HOST: RefCell<Option<Arc<dyn JsExecutionHost>>> = RefCell::new(None);
    static CURRENT_INTERMEDIATE_CALLBACK: RefCell<Option<Arc<dyn Fn(String) + Send + Sync>>> = RefCell::new(None);
    static CURRENT_EXECUTION_LISTENER: RefCell<Option<JsExecutionListenerRef>> = RefCell::new(None);
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
}

struct JsAsyncCallback {
    callbackId: String,
    result: String,
    isError: bool,
}

type JsAsyncCallbackSink = Arc<dyn Fn(JsAsyncCallback) + Send + Sync>;

#[derive(Clone)]
struct JsCallContext {
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    intermediateCallback: Option<Arc<dyn Fn(String) + Send + Sync>>,
    executionListener: Option<JsExecutionListenerRef>,
    envOverrides: BTreeMap<String, String>,
    textResources: Option<Arc<ToolPkgTextResources>>,
    textResourceHost: Option<Arc<dyn ToolPkgTextResourceHost>>,
}

struct JsPendingScriptExecution {
    callId: String,
    context: JsCallContext,
    deadlineMillis: u128,
    timeout: Duration,
}

impl Drop for JsPendingScriptExecution {
    /// Clears the native call session whenever cooperative execution leaves scope.
    fn drop(&mut self) {
        clearNativeExecutionSession(&self.callId);
    }
}

enum JsScriptExecutionPoll {
    Pending,
    Complete(Option<String>),
}

struct JsEngineState {
    runtime: Box<dyn HostJavaScriptRuntime>,
    asyncCallbackSender: mpsc::Sender<JsAsyncCallback>,
    asyncCallbackReceiver: mpsc::Receiver<JsAsyncCallback>,
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    toolPkgContext: Option<ToolPkgExecutionContext>,
    composeDslTextResources: Option<Arc<ToolPkgTextResources>>,
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
        let stateHandle = runtimeHost
            .createHostJavaScriptRuntimeState(
                "OperitQuickJsEngine",
                Box::new(move || {
                    let runtime = runtimeHostForState.createHostJavaScriptRuntime()?;
                    let state =
                        JsEngineState::newWithRuntime(runtime, executionHost, toolPkgContext)
                            .map_err(HostError::new)?;
                    Ok(Box::new(state))
                }),
            )
            .expect("JavaScript runtime state must be created by the Host");
        Self {
            runtimeHost,
            stateHandle,
        }
    }

    /// Executes one JavaScript request through the host-owned state executor.
    #[allow(non_snake_case)]
    fn execute_script_function(
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
    ) -> JsExecutionResult<Option<String>> {
        let script = script.to_string();
        let functionName = functionName.to_string();
        let params = params.clone();
        let envOverrides = envOverrides.clone();
        let composeResourceCount = textResources
            .as_ref()
            .map_or(0, |resources| resources.len());
        let result = self
            .runtimeHost
            .executeHostJavaScriptRuntimeStateTask(
                self.stateHandle,
                u64::try_from(timeout.as_millis())
                    .expect("JavaScript execution timeout must fit in milliseconds"),
                Box::new(move |state, interrupt| {
                    let state = state.downcast_mut::<JsEngineState>().ok_or_else(|| {
                        HostError::new("JavaScript runtime state type does not match")
                    })?;
                    let executionStartedMillis = currentTimeMillisU128();
                    if useComposeDslTextResources {
                        AppLogger::d(
                            TAG,
                            &format!(
                                "compose-request-start function={} resourceEntries={}",
                                functionName, composeResourceCount
                            ),
                        );
                    }
                    let output = executeWithInterrupt(
                        state,
                        interrupt,
                        timeoutSec,
                        "Script execution",
                        |state| {
                            state.executeScriptFunctionForRequest(
                                &script,
                                &functionName,
                                &params,
                                &envOverrides,
                                on_intermediate_result,
                                dispatchIntermediateOnMain,
                                timeoutSec,
                                executionListener,
                                textResources,
                                useComposeDslTextResources,
                            )
                        },
                    );
                    if useComposeDslTextResources {
                        AppLogger::d(
                            TAG,
                            &format!(
                                "compose-request-finish function={} resourceEntries={} elapsedMs={} success={}",
                                functionName,
                                composeResourceCount,
                                currentTimeMillisU128().saturating_sub(executionStartedMillis),
                                output.is_ok()
                            ),
                        );
                    }
                    Ok(Box::new(output))
                }),
            );
        match result {
            Ok(output) => downcastJavaScriptStateOutput(output),
            Err(error) => Err(javaScriptExecutionErrorFromHost(error)),
        }
    }

    /// Executes one JavaScript request asynchronously through the host-owned state executor.
    #[allow(non_snake_case)]
    fn execute_script_function_async(
        &self,
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeout: Duration,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        useComposeDslTextResources: bool,
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>> {
        let stateHandle = self.stateHandle;
        let runtimeHost = self.runtimeHost.clone();
        Box::pin(async move {
            let output = runtimeHost
                .executeHostJavaScriptRuntimeStateAsyncTask(
                    stateHandle,
                    u64::try_from(timeout.as_millis())
                        .expect("JavaScript execution timeout must fit in milliseconds"),
                    Box::new(move |state, interrupt| {
                        Box::pin(async move {
                            let state = state.downcast_mut::<JsEngineState>().ok_or_else(|| {
                                HostError::new("JavaScript runtime state type does not match")
                            })?;
                            let interruptForHandler = interrupt.clone();
                            let handler: HostJavaScriptInterruptHandler =
                                Arc::new(move || interruptForHandler.shouldInterrupt());
                            state
                                .runtime
                                .setHostJavaScriptInterruptHandler(Some(handler))?;
                            let output = state
                                .executeScriptFunctionForRequestAsync(
                                    script,
                                    functionName,
                                    params,
                                    envOverrides,
                                    on_intermediate_result,
                                    dispatchIntermediateOnMain,
                                    timeout,
                                    timeoutSec,
                                    executionListener,
                                    textResources,
                                    useComposeDslTextResources,
                                )
                                .await;
                            let clearResult = state.runtime.setHostJavaScriptInterruptHandler(None);
                            let output = match clearResult {
                                Err(error) => Err(JsExecutionError::runtime(error.to_string())),
                                Ok(()) if interrupt.didTimeOut() => {
                                    Err(JsExecutionError::timeout(format!(
                                        "Script execution timed out after {timeoutSec} seconds"
                                    )))
                                }
                                Ok(()) => output,
                            };
                            Ok(Box::new(output) as HostJavaScriptRuntimeStateOutput)
                        })
                    }),
                )
                .await
                .map_err(javaScriptExecutionErrorFromHost)?;
            downcastJavaScriptStateOutput(output)
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
        self.runtimeHost
            .destroyHostJavaScriptRuntimeState(self.stateHandle)
            .expect("JavaScript runtime state must be destroyed by the Host");
    }
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
    ) -> JsExecutionResult<Option<String>> {
        if timeoutSec == 0 {
            let reason = "Script execution timed out after 0 seconds";
            if let Some(listener) = executionListener.as_ref() {
                listener.on_failed("", reason);
            }
            return Err(JsExecutionError::timeout(reason));
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
    ) -> JsExecutionResult<Option<String>> {
        if timeoutMillis == 0 {
            let reason = "Script execution timed out after 0 milliseconds";
            if let Some(listener) = executionListener.as_ref() {
                listener.on_failed("", reason);
            }
            return Err(JsExecutionError::timeout(reason));
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
    ) -> JsExecutionResult<Option<String>> {
        self.worker.execute_script_function(
            script,
            functionName,
            params,
            envOverrides,
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
    ) -> JsExecutionResult<Option<String>> {
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
    ) -> JsExecutionResult<Option<String>> {
        let normalizedActionId = actionId.trim();
        if normalizedActionId.is_empty() {
            return Err(JsExecutionError::invalid_request(
                "compose action id is required",
            ));
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
    ) -> JsExecutionResult<Option<String>> {
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
    ) -> JsExecutionResult<Option<String>> {
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
    ) -> Result<Self, String> {
        let (asyncCallbackSender, asyncCallbackReceiver) = mpsc::channel();
        let mut state = Self {
            runtime,
            asyncCallbackSender,
            asyncCallbackReceiver,
            executionHost,
            toolPkgContext,
            composeDslTextResources: None,
            jsEnvironmentInitialized: false,
        };
        state.registerNativeInterface()?;
        Ok(state)
    }

    /// Runs one request with the engine-bound ToolPkg host or page-owned Compose DSL resources.
    #[allow(non_snake_case)]
    fn executeScriptFunctionForRequest(
        &mut self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        useComposeDslTextResources: bool,
    ) -> JsExecutionResult<Option<String>> {
        if !useComposeDslTextResources {
            return self.execute_script_function_on_current_thread(
                script,
                functionName,
                params,
                envOverrides,
                onIntermediateResult,
                dispatchIntermediateOnMain,
                timeoutSec,
                executionListener,
            );
        }
        if let Some(textResources) = textResources {
            self.composeDslTextResources = Some(textResources);
        }
        let textResources = self.composeDslTextResources.clone().ok_or_else(|| {
            JsExecutionError::invalid_request(
                "Compose DSL action requires a rendered page resource snapshot",
            )
        })?;
        executeWithToolPkgTextResources(textResources, || {
            self.execute_script_function_on_current_thread(
                script,
                functionName,
                params,
                envOverrides,
                onIntermediateResult,
                dispatchIntermediateOnMain,
                timeoutSec,
                executionListener,
            )
        })
    }

    /// Executes one request cooperatively across host runtime turns.
    #[allow(non_snake_case)]
    async fn executeScriptFunctionForRequestAsync(
        &mut self,
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        _dispatchIntermediateOnMain: bool,
        timeout: Duration,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        useComposeDslTextResources: bool,
    ) -> JsExecutionResult<Option<String>> {
        let activeTextResources = if useComposeDslTextResources {
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
        self.executeScriptFunctionCooperatively(
            script,
            functionName,
            params,
            envOverrides,
            onIntermediateResult,
            timeout,
            timeoutSec,
            executionListener,
            activeTextResources,
            textResourceHost,
        )
        .await
    }

    /// Starts and advances one JavaScript call while yielding through the execution host.
    #[allow(non_snake_case)]
    async fn executeScriptFunctionCooperatively(
        &mut self,
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        timeout: Duration,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        textResourceHost: Option<Arc<dyn ToolPkgTextResourceHost>>,
    ) -> JsExecutionResult<Option<String>> {
        let context = JsCallContext {
            executionHost: self.executionHost.clone(),
            intermediateCallback: onIntermediateResult,
            executionListener,
            envOverrides,
            textResources,
            textResourceHost,
        };
        installThreadLocalCallContext(&context);
        let started = (|| {
            self.initJavaScriptEnvironment()
                .map_err(JsExecutionError::initialization)?;
            let mut effectiveParams = params;
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
            let callId = format!(
                "operit_call_{}",
                Uuid::new_v4().to_string().replace('-', "")
            );
            let callIdJson = serde_json::to_string(&callId)
                .map_err(|error| JsExecutionError::serialization(error.to_string()))?;
            clearNativeExecutionSession(&callId);
            let executionScript = format!(
                "__operitExecuteScriptFunction({callIdJson}, {paramsJson}, {scriptJson}, {functionNameJson}, {timeoutSec}, 10000);"
            );
            self.evalJavaScriptVoid(&executionScript)
                .map_err(JsExecutionError::runtime)?;
            Ok(JsPendingScriptExecution {
                callId,
                context: context.clone(),
                deadlineMillis: currentTimeMillisU128()
                    .checked_add(timeout.as_millis())
                    .ok_or_else(|| {
                        JsExecutionError::timeout(
                            "Script execution deadline exceeds host clock range",
                        )
                    })?,
                timeout,
            })
        })();
        clearThreadLocalCallState();
        let pending = started?;
        loop {
            installThreadLocalCallContext(&pending.context);
            let polled = self.pollCooperativeScriptExecution(&pending);
            clearThreadLocalCallState();
            match polled? {
                JsScriptExecutionPoll::Complete(output) => {
                    if let Some(message) = extractJsExecutionErrorMessage(output.as_deref()) {
                        return Err(JsExecutionError::runtime(message));
                    }
                    return Ok(output);
                }
                JsScriptExecutionPoll::Pending => {}
            }
            let executionHost = pending.context.executionHost.clone().ok_or_else(|| {
                JsExecutionError::worker_unavailable(
                    "JavaScript execution host is unavailable for asynchronous execution",
                )
            })?;
            if let Err(error) = executionHost.wait_for_javascript_runtime_turn().await {
                return Err(JsExecutionError::worker_unavailable(error.to_string()));
            }
        }
    }

    /// Advances pending QuickJS jobs and queued host callbacks for one cooperative call.
    #[allow(non_snake_case)]
    fn pollCooperativeScriptExecution(
        &mut self,
        pending: &JsPendingScriptExecution,
    ) -> JsExecutionResult<JsScriptExecutionPoll> {
        self.runJavaScriptJobs()
            .map_err(JsExecutionError::runtime)?;
        loop {
            match self.asyncCallbackReceiver.try_recv() {
                Ok(callback) => {
                    self.deliverAsyncCallback(callback)
                        .map_err(JsExecutionError::runtime)?;
                    self.runJavaScriptJobs()
                        .map_err(JsExecutionError::runtime)?;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(JsExecutionError::worker_unavailable(
                        "JavaScript asynchronous callback queue disconnected",
                    ));
                }
            }
        }
        if let Some(output) = readNativeExecutionSession(&pending.callId) {
            return Ok(JsScriptExecutionPoll::Complete(Some(output)));
        }
        if currentTimeMillisU128() >= pending.deadlineMillis {
            return Err(JsExecutionError::timeout(format!(
                "Script execution timed out after {} milliseconds",
                pending.timeout.as_millis()
            )));
        }
        Ok(JsScriptExecutionPoll::Pending)
    }

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
        let bridge = buildToolPkgRegistrationBridgeScript(true);
        self.evalJavaScriptVoid(&bridge)
            .map_err(JsExecutionError::runtime)?;
        CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
            *resources.borrow_mut() = textResources;
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
            self.evalJavaScriptVoid(&executionScript)
                .map_err(JsExecutionError::runtime)?;
            self.runJavaScriptJobs()
                .map_err(JsExecutionError::runtime)?;
            let output = readNativeExecutionSession(&callId).ok_or_else(|| {
                JsExecutionError::runtime("ToolPkg registration JavaScript did not complete")
            })?;
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

    #[allow(non_snake_case)]
    fn runJavaScriptJobs(&mut self) -> Result<(), String> {
        self.runtime
            .executePendingHostJavaScriptJobs()
            .map_err(|error| error.to_string())
    }

    /// Waits for one JavaScript call to complete while delivering queued host callbacks.
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
            self.runJavaScriptJobs()
                .map_err(JsExecutionError::runtime)?;
            if let Some(output) = readNativeExecutionSession(callId) {
                return Ok(Some(output));
            }
            let nowMillis = currentTimeMillisU128();
            if nowMillis >= deadlineMillis {
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
                Ok(callback) => self
                    .deliverAsyncCallback(callback)
                    .map_err(JsExecutionError::runtime)?,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
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
        let callbackIdJson =
            serde_json::to_string(&callback.callbackId).map_err(|error| error.to_string())?;
        let resultJson =
            serde_json::to_string(&callback.result).map_err(|error| error.to_string())?;
        let callbackScript = format!(
            "(function() {{ var callback = globalThis[{callbackIdJson}]; if (typeof callback === 'function') {{ callback({resultJson}, {}); }} }})();",
            callback.isError
        );
        self.evalJavaScriptVoid(&callbackScript)
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
                "__operitNativeCallTool",
                Arc::new(|arguments| {
                    let [toolType, toolName, paramsJson] =
                        exactHostJavaScriptArguments("__operitNativeCallTool", arguments)?;
                    Ok(nativeCallToolStrings(toolType, toolName, paramsJson))
                }),
            ),
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
                    let [_callId, key] =
                        exactHostJavaScriptArguments("__operitNativeGetEnvForCall", arguments)?;
                    Ok(nativeGetEnvForCallStrings(key))
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
        let asyncCallbackSink: JsAsyncCallbackSink = Arc::new(move |callback| {
            let _ = asyncCallbackSender.send(callback);
        });
        let toolExecutionHost = self.executionHost.clone();
        let toolAsyncCallbackSink = asyncCallbackSink.clone();
        let timerCallbackSink = asyncCallbackSink.clone();
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
                Arc::new(|arguments| {
                    let [callId, result] =
                        exactHostJavaScriptArguments("__operitNativeSetCallResult", arguments)?;
                    nativeSetCallResultStrings(callId, result);
                    Ok(())
                }),
            ),
            (
                "__operitNativeSetCallError",
                Arc::new(|arguments| {
                    let [callId, error] =
                        exactHostJavaScriptArguments("__operitNativeSetCallError", arguments)?;
                    nativeSetCallErrorStrings(callId, error);
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
                    dispatchToolPkgIpcAsync(
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
        callbackSink(JsAsyncCallback {
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
    let scheduleResult = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeTask(
        "operit-js-tool-call",
        Box::new(move || {
            let (result, isError) = JsNativeInterfaceDelegates::callToolSerialized(
                executionHost.as_ref(),
                &toolType,
                &toolName,
                &paramsJson,
            );
            callbackSinkForTask(JsAsyncCallback {
                callbackId: callbackIdForTask,
                result,
                isError,
            });
        }),
    );
    if let Err(error) = scheduleResult {
        callbackSink(JsAsyncCallback {
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
                callbackSink(JsAsyncCallback {
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
    let request = match buildToolPkgIpcRequest(
        packageTarget,
        callerContextKey,
        targetContextKey,
        targetRuntime,
        channel,
        payloadJson,
    ) {
        Ok(request) => request,
        Err(error) => {
            callbackSink(JsAsyncCallback {
                callbackId: normalizedCallbackId,
                result: error,
                isError: false,
            });
            return;
        }
    };
    let Some(executionHost) = executionHost else {
        callbackSink(JsAsyncCallback {
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
            callbackSinkForCompletion(JsAsyncCallback {
                callbackId: callbackIdForCompletion,
                result,
                isError,
            });
        }),
    );
    if let Err(error) = submitResult {
        callbackSink(JsAsyncCallback {
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
        *overrides.borrow_mut() = context.envOverrides.clone();
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
    ) -> JsExecutionResult<Option<String>> {
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
    ) -> JsExecutionResult<Option<String>> {
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
    ) -> JsExecutionResult<Option<String>> {
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
    ) -> JsExecutionResult<Option<String>> {
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
fn nativeCallToolStrings(toolType: String, toolName: String, paramsJson: String) -> String {
    match currentExecutionHost() {
        Ok(host) => JsNativeInterfaceDelegates::callToolSync(
            host.as_ref(),
            &toolType,
            &toolName,
            &paramsJson,
        ),
        Err(error) => serde_json::json!({"success": false, "message": error}).to_string(),
    }
}

#[allow(non_snake_case)]
fn nativeSendIntermediateResultString(callId: String, result: String) {
    CURRENT_EXECUTION_LISTENER.with(|listener| {
        if let Some(listener) = listener.borrow().as_ref() {
            listener.on_intermediate_result(&callId, &result);
        }
    });
    CURRENT_INTERMEDIATE_CALLBACK.with(|callback| {
        if let Some(callback) = callback.borrow().as_ref() {
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
    CURRENT_EXECUTION_LISTENER.with(|listener| {
        if let Some(listener) = listener.borrow().as_ref() {
            listener.on_failed(&callId, &error);
        }
    });
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
