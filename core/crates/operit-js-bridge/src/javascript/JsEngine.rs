use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
#[cfg(target_arch = "wasm32")]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use quickjs_wasm_rs::{
    JSContextRef as WasmQuickJsContext, JSValue as WasmQuickJsValue,
    JSValueRef as WasmQuickJsValueRef,
};
#[cfg(not(target_arch = "wasm32"))]
use rquickjs::{
    CatchResultExt, Context as QuickJsContext, Function as QuickJsFunction,
    Runtime as QuickJsRuntime,
};
use serde_json::Value;
use uuid::Uuid;

use crate::javascript::JsJavaBridgeDelegates::{
    nativeJavaCallInstanceStrings, nativeJavaCallStaticString, nativeJavaClassExistsString,
    nativeJavaGetApplicationContextString, nativeJavaNewInstanceString,
};
use crate::javascript::JsLibraries::buildRuntimeBootstrapScript;
use crate::javascript::JsNativeInterfaceDelegates;
use operit_plugin_sdk::execution_result::{
    build_js_execution_error_payload as buildJsExecutionErrorPayload,
    extract_js_execution_error_message as extractJsExecutionErrorMessage, JsExecutionError,
    JsExecutionResult,
};
use operit_plugin_sdk::javascript::{
    JsExecutionEngine, JsExecutionHost, JsToolNameResolutionRequest, JsToolPkgIpcRequest,
    JsToolPkgResourceRequest, ToolPkgMainRegistrationCapture,
};
use operit_plugin_sdk::toolpkg::ToolPkgComposeDslRuntimeScript::buildComposeDslRuntimeWrappedScript;
use operit_plugin_sdk::toolpkg::ToolPkgRegistrationBridge::buildToolPkgRegistrationBridgeScript;
use operit_util::stream::Stream::Stream;
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
    #[cfg(target_arch = "wasm32")]
    static WASM_JS_ENGINE_STATES: RefCell<BTreeMap<usize, JsEngineState>> = RefCell::new(BTreeMap::new());
}

#[cfg(target_arch = "wasm32")]
static NEXT_WASM_JS_ENGINE_STATE_ID: AtomicUsize = AtomicUsize::new(1);

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
#[cfg(not(target_arch = "wasm32"))]
struct JsEngineWorker {
    sender: mpsc::Sender<JsEngineRequest>,
}

#[derive(Clone)]
#[cfg(target_arch = "wasm32")]
struct JsEngineWorker {
    stateId: usize,
}

#[cfg(not(target_arch = "wasm32"))]
enum JsEngineRequest {
    ExecuteScript {
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        envOverrides: BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        executionListener: Option<JsExecutionListenerRef>,
        timeoutSec: u64,
        response: mpsc::Sender<JsExecutionResult<Option<String>>>,
    },
    ExecuteToolPkgMainRegistration {
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        response: mpsc::Sender<JsExecutionResult<ToolPkgMainRegistrationCapture>>,
    },
}

struct JsEngineState {
    #[cfg(not(target_arch = "wasm32"))]
    runtime: QuickJsRuntime,
    #[cfg(not(target_arch = "wasm32"))]
    context: QuickJsContext,
    #[cfg(target_arch = "wasm32")]
    context: WasmQuickJsContext,
    executionHost: Option<Arc<dyn JsExecutionHost>>,
    jsEnvironmentInitialized: bool,
}

impl JsEngine {
    /// Creates a JavaScript execution engine backed by a caller-supplied execution host.
    pub fn new(executionHost: Arc<dyn JsExecutionHost>) -> Self {
        Self {
            worker: JsEngineWorker::new(Some(executionHost)),
        }
    }

    /// Creates a JavaScript engine used only for ToolPkg registration.
    #[allow(non_snake_case)]
    pub fn new_toolpkg_registration_engine() -> Self {
        Self {
            worker: JsEngineWorker::new(None),
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
        let safeTimeoutSec = timeoutSec.max(1);
        #[cfg(target_arch = "wasm32")]
        {
            return self.worker.execute_script_function(
                script,
                functionName,
                params,
                envOverrides,
                on_intermediate_result,
                dispatchIntermediateOnMain,
                safeTimeoutSec,
                executionListener,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (response, receiver) = mpsc::channel();
            let request = JsEngineRequest::ExecuteScript {
                script: script.to_string(),
                functionName: functionName.to_string(),
                params: params.clone(),
                envOverrides: envOverrides.clone(),
                on_intermediate_result,
                dispatchIntermediateOnMain,
                executionListener: executionListener.clone(),
                timeoutSec: safeTimeoutSec,
                response,
            };
            if let Err(error) = self.worker.sender.send(request) {
                AppLogger::e(
                    TAG,
                    &format!(
                        "request-send-error function={} scriptLen={} params={} error={}",
                        functionName,
                        script.len(),
                        summarizeParams(params),
                        error
                    ),
                );
                if let Some(listener) = executionListener.as_ref() {
                    listener.on_failed("", &error.to_string());
                }
                return Err(JsExecutionError::worker_unavailable(error.to_string()));
            }
            match receiver.recv_timeout(Duration::from_secs(safeTimeoutSec)) {
                Ok(value) => value,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let reason =
                        format!("Script execution timed out after {safeTimeoutSec} seconds");
                    if let Some(listener) = executionListener.as_ref() {
                        listener.on_failed("", &reason);
                    }
                    Err(JsExecutionError::timeout(reason))
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    AppLogger::e(
                        TAG,
                        &format!(
                            "response-recv-error function={} scriptLen={} params={} error=disconnected",
                            functionName,
                            script.len(),
                            summarizeParams(params),
                        ),
                    );
                    if let Some(listener) = executionListener.as_ref() {
                        listener.on_failed("", "JS execution worker disconnected");
                    }
                    Err(JsExecutionError::worker_unavailable(
                        "JS execution worker disconnected",
                    ))
                }
            }
        }
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

    #[allow(non_snake_case)]
    pub(crate) fn execute_toolpkg_main_registration_function_with_text_resources(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        #[cfg(target_arch = "wasm32")]
        {
            return self.worker.execute_toolpkg_main_registration_function(
                script,
                functionName,
                params,
                textResources,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (response, receiver) = mpsc::channel();
            let request = JsEngineRequest::ExecuteToolPkgMainRegistration {
                script: script.to_string(),
                functionName: functionName.to_string(),
                params: params.clone(),
                textResources,
                response,
            };
            if let Err(error) = self.worker.sender.send(request) {
                AppLogger::e(
                    TAG,
                    &format!(
                        "registration-send-error function={} scriptLen={} params={} error={}",
                        functionName,
                        script.len(),
                        summarizeParams(params),
                        error
                    ),
                );
                return Err(JsExecutionError::worker_unavailable(error.to_string()));
            }
            match receiver.recv() {
                Ok(value) => value,
                Err(error) => {
                    AppLogger::e(
                        TAG,
                        &format!(
                            "registration-recv-error function={} scriptLen={} params={} error={}",
                            functionName,
                            script.len(),
                            summarizeParams(params),
                            error
                        ),
                    );
                    Err(JsExecutionError::worker_unavailable(error.to_string()))
                }
            }
        }
    }

    /// Executes a Compose DSL script and returns its rendered event stream.
    #[allow(non_snake_case)]
    pub fn execute_compose_dsl_script(
        &self,
        script: &str,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
    ) -> JsExecutionResult<Option<String>> {
        self.execute_script_function(
            &buildComposeDslRuntimeWrappedScript(script),
            "__operit_render_compose_dsl",
            runtimeOptions,
            envOverrides,
            None,
            true,
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
            None,
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
        self.execute_script_function(
            "",
            "__operit_dispatch_compose_dsl_action",
            &params,
            envOverrides,
            on_intermediate_result,
            true,
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
            None,
        )
    }

    #[allow(non_snake_case)]
    pub fn rerender_compose_dsl_tree(
        &self,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
    ) -> JsExecutionResult<Option<String>> {
        self.execute_script_function(
            "",
            "__operit_rerender_compose_dsl",
            runtimeOptions,
            envOverrides,
            None,
            true,
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
            None,
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

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let engine = self.engine.clone();
            let actionId = self.actionId.clone();
            let payload = self.payload.clone();
            let runtimeOptions = self.runtimeOptions.clone();
            let envOverrides = self.envOverrides.clone();
            let (sender, receiver) = mpsc::channel::<String>();
            std::thread::spawn(move || {
                let intermediateSender = sender.clone();
                runComposeDslActionDispatch(
                    engine,
                    actionId,
                    payload,
                    runtimeOptions,
                    envOverrides,
                    Arc::new(move |event| {
                        let _ = intermediateSender.send(event);
                    }),
                    move |event| {
                        let _ = sender.send(event);
                    },
                );
            });
            for event in receiver {
                collector(event);
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let engine = self.engine.clone();
            let actionId = self.actionId.clone();
            let payload = self.payload.clone();
            let runtimeOptions = self.runtimeOptions.clone();
            let envOverrides = self.envOverrides.clone();
            let intermediateEvents = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
            let intermediateEventsForCallback = intermediateEvents.clone();
            let flushedIntermediateEvents = Arc::new(std::sync::Mutex::new(false));
            let flushedIntermediateEventsForEmit = flushedIntermediateEvents.clone();
            runComposeDslActionDispatch(
                engine,
                actionId,
                payload,
                runtimeOptions,
                envOverrides,
                Arc::new(move |event| {
                    if let Ok(mut values) = intermediateEventsForCallback.lock() {
                        values.push(event);
                    }
                }),
                |event| {
                    if let Ok(mut flushed) = flushedIntermediateEventsForEmit.lock() {
                        if !*flushed {
                            if let Ok(values) = intermediateEvents.lock() {
                                for intermediate in values.iter() {
                                    collector(intermediate.clone());
                                }
                            }
                            *flushed = true;
                        }
                    }
                    collector(event);
                },
            );
        }
    }
}

#[allow(non_snake_case)]
fn runComposeDslActionDispatch(
    engine: JsEngine,
    actionId: String,
    payload: Option<Value>,
    runtimeOptions: BTreeMap<String, Value>,
    envOverrides: BTreeMap<String, String>,
    emitIntermediate: Arc<dyn Fn(String) + Send + Sync>,
    mut emit: impl FnMut(String),
) {
    let normalizedActionId = actionId.trim().to_string();
    if normalizedActionId.is_empty() {
        emit(composeDslActionEvent(
            "error",
            Some("compose action id is required"),
            None,
        ));
        emit(composeDslActionEvent("complete", None, None));
        return;
    }
    let result = engine.execute_compose_dsl_action(
        &normalizedActionId,
        payload,
        &runtimeOptions,
        &envOverrides,
        Some(Arc::new(move |intermediate| {
            emitIntermediate(composeDslActionEvent(
                "intermediate",
                None,
                Some(&intermediate),
            ));
        })),
    );
    match result {
        Ok(Some(result)) => emit(composeDslActionEvent("final", None, Some(&result))),
        Ok(None) => {}
        Err(error) => emit(composeDslActionEvent("error", Some(&error.message), None)),
    }
    emit(composeDslActionEvent("complete", None, None));
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

#[cfg(not(target_arch = "wasm32"))]
impl JsEngineWorker {
    /// Starts a worker containing one isolated JavaScript runtime state.
    fn new(executionHost: Option<Arc<dyn JsExecutionHost>>) -> Self {
        let (sender, receiver) = mpsc::channel::<JsEngineRequest>();
        std::thread::Builder::new()
            .name("OperitQuickJsEngine".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut state = JsEngineState::new(executionHost);
                for request in receiver {
                    match request {
                        JsEngineRequest::ExecuteScript {
                            script,
                            functionName,
                            params,
                            envOverrides,
                            on_intermediate_result,
                            dispatchIntermediateOnMain,
                            executionListener,
                            timeoutSec,
                            response,
                        } => {
                            let output = state.execute_script_function_on_current_thread(
                                &script,
                                &functionName,
                                &params,
                                &envOverrides,
                                on_intermediate_result,
                                dispatchIntermediateOnMain,
                                timeoutSec,
                                executionListener,
                            );
                            if let Err(error) = response.send(output) {
                                AppLogger::e(
                                    TAG,
                                    &format!(
                                        "worker-send-error kind=execute function={} error={}",
                                        functionName, error
                                    ),
                                );
                            }
                        }
                        JsEngineRequest::ExecuteToolPkgMainRegistration {
                            script,
                            functionName,
                            params,
                            textResources,
                            response,
                        } => {
                            let output = state
                                .execute_toolpkg_main_registration_function_on_current_thread(
                                    &script,
                                    &functionName,
                                    &params,
                                    textResources,
                                );
                            if let Err(error) = response.send(output) {
                                AppLogger::e(
                                    TAG,
                                    &format!(
                                        "worker-send-error kind=registration function={} error={}",
                                        functionName, error
                                    ),
                                );
                            }
                        }
                    }
                }
            })
            .expect("OperitQuickJsEngine worker thread must start");
        Self { sender }
    }
}

#[cfg(target_arch = "wasm32")]
impl JsEngineWorker {
    /// Creates one WebAssembly JavaScript runtime state.
    fn new(executionHost: Option<Arc<dyn JsExecutionHost>>) -> Self {
        let stateId = NEXT_WASM_JS_ENGINE_STATE_ID.fetch_add(1, Ordering::Relaxed);
        WASM_JS_ENGINE_STATES.with(|states| {
            states
                .borrow_mut()
                .insert(stateId, JsEngineState::new(executionHost));
        });
        Self { stateId }
    }

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
        executionListener: Option<JsExecutionListenerRef>,
    ) -> JsExecutionResult<Option<String>> {
        WASM_JS_ENGINE_STATES.with(|states| {
            states
                .borrow_mut()
                .get_mut(&self.stateId)
                .expect("wasm JsEngine state must exist")
                .execute_script_function_on_current_thread(
                    script,
                    functionName,
                    params,
                    envOverrides,
                    on_intermediate_result,
                    dispatchIntermediateOnMain,
                    timeoutSec,
                    executionListener,
                )
        })
    }

    #[allow(non_snake_case)]
    fn execute_toolpkg_main_registration_function(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
        WASM_JS_ENGINE_STATES.with(|states| {
            states
                .borrow_mut()
                .get_mut(&self.stateId)
                .expect("wasm JsEngine state must exist")
                .execute_toolpkg_main_registration_function_on_current_thread(
                    script,
                    functionName,
                    params,
                    textResources,
                )
        })
    }
}

impl JsEngineState {
    /// Creates a JavaScript runtime state from an optional execution host.
    fn new(executionHost: Option<Arc<dyn JsExecutionHost>>) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let runtime = QuickJsRuntime::new().expect("OperitQuickJsEngine runtime must start");
            let context =
                QuickJsContext::full(&runtime).expect("OperitQuickJsEngine context must start");
            let mut state = Self {
                runtime,
                context,
                executionHost,
                jsEnvironmentInitialized: false,
            };
            state
                .registerNativeInterface()
                .expect("NativeInterface bridge must register");
            state
        }
        #[cfg(target_arch = "wasm32")]
        {
            let context = WasmQuickJsContext::default();
            let mut state = Self {
                context,
                executionHost,
                jsEnvironmentInitialized: false,
            };
            state
                .registerNativeInterface()
                .expect("NativeInterface bridge must register");
            state
        }
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
        let safeTimeoutSec = timeoutSec.max(1);

        clearNativeExecutionSession(&callId);
        let executionScript = format!(
            "__operitExecuteScriptFunction({callIdJson}, {paramsJson}, {scriptJson}, {functionNameJson}, {safeTimeoutSec}, 10000);"
        );
        let output = match self.evalJavaScriptVoid(&executionScript) {
            Ok(_) => {
                self.runJavaScriptJobs();
                readNativeExecutionSession(&callId)
            }
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
        let bridge = buildToolPkgRegistrationBridgeScript();
        self.evalJavaScriptVoid(&bridge)
            .map_err(JsExecutionError::runtime)?;
        CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
            *resources.borrow_mut() = textResources;
        });

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
            registrationParams.insert("__operit_package_lang".to_string(), Value::String(language));
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
            CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
                *resources.borrow_mut() = None;
            });
            return Err(JsExecutionError::runtime(error));
        }
        self.runJavaScriptJobs();
        let output = readNativeExecutionSession(&callId).ok_or_else(|| {
            JsExecutionError::runtime("ToolPkg registration JavaScript did not complete")
        });
        CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
            *resources.borrow_mut() = None;
        });
        let output = output?;
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
    }

    #[allow(non_snake_case)]
    fn evalJavaScriptVoid(&mut self, script: &str) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.context.with(|ctx| {
                ctx.eval::<(), _>(script)
                    .catch(&ctx)
                    .map_err(|error| error.to_string())
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.context
                .eval_global("operit.js", script)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
    }

    #[allow(non_snake_case)]
    fn evalJavaScriptString(&mut self, script: &str) -> Result<String, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.context.with(|ctx| {
                ctx.eval::<String, _>(script)
                    .catch(&ctx)
                    .map_err(|error| error.to_string())
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.context
                .eval_global("operit.js", script)
                .map(|value| value.to_string())
                .map_err(|error| error.to_string())
        }
    }

    #[allow(non_snake_case)]
    fn runJavaScriptJobs(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            while self.context.with(|ctx| ctx.execute_pending_job()) {}
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.context
                .execute_pending()
                .expect("OperitQuickJsEngine pending jobs must execute");
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.context.with(|ctx| {
                let globals = ctx.globals();
                let nativeCallTool = QuickJsFunction::new(
                    ctx.clone(),
                    |toolType: String, toolName: String, paramsJson: String| {
                        nativeCallToolStrings(toolType, toolName, paramsJson)
                    },
                )
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeCallTool", nativeCallTool)
                    .map_err(|error| error.to_string())?;

                let sendIntermediateResult =
                    QuickJsFunction::new(ctx.clone(), |callId: String, result: String| {
                        nativeSendIntermediateResultString(callId, result)
                    })
                    .map_err(|error| error.to_string())?;
                globals
                    .set("__operitSendIntermediateResult", sendIntermediateResult)
                    .map_err(|error| error.to_string())?;

                let readToolPkgTextResource = QuickJsFunction::new(
                    ctx.clone(),
                    |packageNameOrSubpackageId: String, resourcePath: String| {
                        nativeReadToolPkgTextResourceStrings(
                            packageNameOrSubpackageId,
                            resourcePath,
                        )
                    },
                )
                .map_err(|error| error.to_string())?;
                globals
                    .set(
                        "__operitNativeReadToolPkgTextResource",
                        readToolPkgTextResource,
                    )
                    .map_err(|error| error.to_string())?;

                let readToolPkgResource = QuickJsFunction::new(
                    ctx.clone(),
                    |packageNameOrSubpackageId: String,
                     resourceKey: String,
                     outputFileName: String,
                     internal: String| {
                        nativeReadToolPkgResourceStrings(
                            packageNameOrSubpackageId,
                            resourceKey,
                            outputFileName,
                            internal,
                        )
                    },
                )
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeReadToolPkgResource", readToolPkgResource)
                    .map_err(|error| error.to_string())?;

                let composeWebViewControllerCommand =
                    QuickJsFunction::new(ctx.clone(), |payloadJson: String| {
                        nativeComposeWebViewControllerCommandString(payloadJson)
                    })
                    .map_err(|error| error.to_string())?;
                globals
                    .set(
                        "__operitNativeComposeWebViewControllerCommand",
                        composeWebViewControllerCommand,
                    )
                    .map_err(|error| error.to_string())?;

                let setCallResult =
                    QuickJsFunction::new(ctx.clone(), |callId: String, result: String| {
                        nativeSetCallResultStrings(callId, result)
                    })
                    .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeSetCallResult", setCallResult)
                    .map_err(|error| error.to_string())?;

                let setCallError =
                    QuickJsFunction::new(ctx.clone(), |callId: String, error: String| {
                        nativeSetCallErrorStrings(callId, error)
                    })
                    .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeSetCallError", setCallError)
                    .map_err(|error| error.to_string())?;

                let getEnvForCall =
                    QuickJsFunction::new(ctx.clone(), |_callId: String, key: String| {
                        nativeGetEnvForCallStrings(key)
                    })
                    .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeGetEnvForCall", getEnvForCall)
                    .map_err(|error| error.to_string())?;

                let getPluginConfigDir = QuickJsFunction::new(ctx.clone(), |pluginId: String| {
                    nativeGetPluginConfigDirString(pluginId)
                })
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeGetPluginConfigDir", getPluginConfigDir)
                    .map_err(|error| error.to_string())?;

                let isPackageImported = QuickJsFunction::new(ctx.clone(), |packageName: String| {
                    nativeIsPackageImportedString(packageName)
                })
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeIsPackageImported", isPackageImported)
                    .map_err(|error| error.to_string())?;

                let importPackage = QuickJsFunction::new(ctx.clone(), |packageName: String| {
                    nativeImportPackageString(packageName)
                })
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeImportPackage", importPackage)
                    .map_err(|error| error.to_string())?;

                let removePackage = QuickJsFunction::new(ctx.clone(), |packageName: String| {
                    nativeRemovePackageString(packageName)
                })
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeRemovePackage", removePackage)
                    .map_err(|error| error.to_string())?;

                let usePackage = QuickJsFunction::new(ctx.clone(), |packageName: String| {
                    nativeUsePackageString(packageName)
                })
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeUsePackage", usePackage)
                    .map_err(|error| error.to_string())?;

                let listImportedPackagesJson =
                    QuickJsFunction::new(ctx.clone(), || nativeListImportedPackagesJsonString())
                        .map_err(|error| error.to_string())?;
                globals
                    .set(
                        "__operitNativeListImportedPackagesJson",
                        listImportedPackagesJson,
                    )
                    .map_err(|error| error.to_string())?;

                let resolveToolName = QuickJsFunction::new(
                    ctx.clone(),
                    |packageName: String,
                     subpackageId: String,
                     toolName: String,
                     preferImported: String| {
                        nativeResolveToolNameString(
                            packageName,
                            subpackageId,
                            toolName,
                            preferImported,
                        )
                    },
                )
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeResolveToolName", resolveToolName)
                    .map_err(|error| error.to_string())?;

                let invokeToolPkgIpc = QuickJsFunction::new(
                    ctx.clone(),
                    |packageTarget: String,
                     callerContextKey: String,
                     targetContextKey: String,
                     targetRuntime: String,
                     channel: String,
                     payloadJson: String| {
                        nativeInvokeToolPkgIpcStrings(
                            packageTarget,
                            callerContextKey,
                            targetContextKey,
                            targetRuntime,
                            channel,
                            payloadJson,
                        )
                    },
                )
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeInvokeToolPkgIpc", invokeToolPkgIpc)
                    .map_err(|error| error.to_string())?;

                let logJsExecutionTrace =
                    QuickJsFunction::new(ctx.clone(), |callId: String, message: String| {
                        nativeLogJsExecutionTraceStrings(callId, message)
                    })
                    .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeLogJsExecutionTrace", logJsExecutionTrace)
                    .map_err(|error| error.to_string())?;

                let decompress =
                    QuickJsFunction::new(ctx.clone(), |data: String, algorithm: String| {
                        nativeDecompressStrings(data, algorithm)
                    })
                    .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeDecompress", decompress)
                    .map_err(|error| error.to_string())?;

                let crypto = QuickJsFunction::new(
                    ctx.clone(),
                    |algorithm: String, operation: String, argsJson: String| {
                        nativeCryptoStrings(algorithm, operation, argsJson)
                    },
                )
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeCrypto", crypto)
                    .map_err(|error| error.to_string())?;

                let imageProcessing = QuickJsFunction::new(
                    ctx.clone(),
                    |callbackId: String, operation: String, argsJson: String| {
                        nativeImageProcessingStrings(callbackId, operation, argsJson)
                    },
                )
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeImageProcessing", imageProcessing)
                    .map_err(|error| error.to_string())?;

                let javaClassExists = QuickJsFunction::new(ctx.clone(), |className: String| {
                    nativeJavaClassExistsString(className)
                })
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeJavaClassExists", javaClassExists)
                    .map_err(|error| error.to_string())?;

                let javaGetApplicationContext =
                    QuickJsFunction::new(ctx.clone(), || nativeJavaGetApplicationContextString())
                        .map_err(|error| error.to_string())?;
                globals
                    .set(
                        "__operitNativeJavaGetApplicationContext",
                        javaGetApplicationContext,
                    )
                    .map_err(|error| error.to_string())?;

                let javaCallInstance = QuickJsFunction::new(
                    ctx.clone(),
                    |instanceHandle: String, methodName: String, argsJson: String| {
                        nativeJavaCallInstanceStrings(instanceHandle, methodName, argsJson)
                    },
                )
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeJavaCallInstance", javaCallInstance)
                    .map_err(|error| error.to_string())?;

                let javaNewInstance =
                    QuickJsFunction::new(ctx.clone(), |className: String, _argsJson: String| {
                        nativeJavaNewInstanceString(className)
                    })
                    .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeJavaNewInstance", javaNewInstance)
                    .map_err(|error| error.to_string())?;

                let javaCallStatic = QuickJsFunction::new(
                    ctx.clone(),
                    |className: String, methodName: String, _argsJson: String| {
                        nativeJavaCallStaticString(className, methodName)
                    },
                )
                .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeJavaCallStatic", javaCallStatic)
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let globals = self
                .context
                .global_object()
                .map_err(|error| error.to_string())?;

            let nativeCallTool = self
                .context
                .wrap_callback(|_, _, args| {
                    let toolType = wasmQuickJsArgString(args, 0);
                    let toolName = wasmQuickJsArgString(args, 1);
                    let paramsJson = wasmQuickJsArgString(args, 2);
                    Ok(WasmQuickJsValue::String(nativeCallToolStrings(
                        toolType, toolName, paramsJson,
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeCallTool", nativeCallTool)
                .map_err(|error| error.to_string())?;

            let sendIntermediateResult = self
                .context
                .wrap_callback(|_, _, args| {
                    nativeSendIntermediateResultString(
                        wasmQuickJsArgString(args, 0),
                        wasmQuickJsArgString(args, 1),
                    );
                    Ok(WasmQuickJsValue::Undefined)
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitSendIntermediateResult", sendIntermediateResult)
                .map_err(|error| error.to_string())?;

            let readToolPkgTextResource = self
                .context
                .wrap_callback(|_, _, args| {
                    let packageNameOrSubpackageId = wasmQuickJsArgString(args, 0);
                    let resourcePath = wasmQuickJsArgString(args, 1);
                    Ok(WasmQuickJsValue::String(
                        nativeReadToolPkgTextResourceStrings(
                            packageNameOrSubpackageId,
                            resourcePath,
                        ),
                    ))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property(
                    "__operitNativeReadToolPkgTextResource",
                    readToolPkgTextResource,
                )
                .map_err(|error| error.to_string())?;

            let readToolPkgResource = self
                .context
                .wrap_callback(|_, _, args| {
                    let packageNameOrSubpackageId = wasmQuickJsArgString(args, 0);
                    let resourceKey = wasmQuickJsArgString(args, 1);
                    let outputFileName = wasmQuickJsArgString(args, 2);
                    let internal = wasmQuickJsArgString(args, 3);
                    Ok(WasmQuickJsValue::String(nativeReadToolPkgResourceStrings(
                        packageNameOrSubpackageId,
                        resourceKey,
                        outputFileName,
                        internal,
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeReadToolPkgResource", readToolPkgResource)
                .map_err(|error| error.to_string())?;

            let composeWebViewControllerCommand = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(
                        nativeComposeWebViewControllerCommandString(wasmQuickJsArgString(args, 0)),
                    ))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property(
                    "__operitNativeComposeWebViewControllerCommand",
                    composeWebViewControllerCommand,
                )
                .map_err(|error| error.to_string())?;

            let setCallResult = self
                .context
                .wrap_callback(|_, _, args| {
                    nativeSetCallResultStrings(
                        wasmQuickJsArgString(args, 0),
                        wasmQuickJsArgString(args, 1),
                    );
                    Ok(WasmQuickJsValue::Undefined)
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeSetCallResult", setCallResult)
                .map_err(|error| error.to_string())?;

            let setCallError = self
                .context
                .wrap_callback(|_, _, args| {
                    nativeSetCallErrorStrings(
                        wasmQuickJsArgString(args, 0),
                        wasmQuickJsArgString(args, 1),
                    );
                    Ok(WasmQuickJsValue::Undefined)
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeSetCallError", setCallError)
                .map_err(|error| error.to_string())?;

            let getEnvForCall = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeGetEnvForCallStrings(
                        wasmQuickJsArgString(args, 1),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeGetEnvForCall", getEnvForCall)
                .map_err(|error| error.to_string())?;

            let getPluginConfigDir = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeGetPluginConfigDirString(
                        wasmQuickJsArgString(args, 0),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeGetPluginConfigDir", getPluginConfigDir)
                .map_err(|error| error.to_string())?;

            let isPackageImported = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeIsPackageImportedString(
                        wasmQuickJsArgString(args, 0),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeIsPackageImported", isPackageImported)
                .map_err(|error| error.to_string())?;

            let importPackage = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeImportPackageString(
                        wasmQuickJsArgString(args, 0),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeImportPackage", importPackage)
                .map_err(|error| error.to_string())?;

            let removePackage = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeRemovePackageString(
                        wasmQuickJsArgString(args, 0),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeRemovePackage", removePackage)
                .map_err(|error| error.to_string())?;

            let usePackage = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeUsePackageString(
                        wasmQuickJsArgString(args, 0),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeUsePackage", usePackage)
                .map_err(|error| error.to_string())?;

            let listImportedPackagesJson = self
                .context
                .wrap_callback(|_, _, _args| {
                    Ok(WasmQuickJsValue::String(
                        nativeListImportedPackagesJsonString(),
                    ))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property(
                    "__operitNativeListImportedPackagesJson",
                    listImportedPackagesJson,
                )
                .map_err(|error| error.to_string())?;

            let resolveToolName = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeResolveToolNameString(
                        wasmQuickJsArgString(args, 0),
                        wasmQuickJsArgString(args, 1),
                        wasmQuickJsArgString(args, 2),
                        wasmQuickJsArgString(args, 3),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeResolveToolName", resolveToolName)
                .map_err(|error| error.to_string())?;

            let invokeToolPkgIpc = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeInvokeToolPkgIpcStrings(
                        wasmQuickJsArgString(args, 0),
                        wasmQuickJsArgString(args, 1),
                        wasmQuickJsArgString(args, 2),
                        wasmQuickJsArgString(args, 3),
                        wasmQuickJsArgString(args, 4),
                        wasmQuickJsArgString(args, 5),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeInvokeToolPkgIpc", invokeToolPkgIpc)
                .map_err(|error| error.to_string())?;

            let logJsExecutionTrace = self
                .context
                .wrap_callback(|_, _, args| {
                    nativeLogJsExecutionTraceStrings(
                        wasmQuickJsArgString(args, 0),
                        wasmQuickJsArgString(args, 1),
                    );
                    Ok(WasmQuickJsValue::Undefined)
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeLogJsExecutionTrace", logJsExecutionTrace)
                .map_err(|error| error.to_string())?;

            let decompress = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeDecompressStrings(
                        wasmQuickJsArgString(args, 0),
                        wasmQuickJsArgString(args, 1),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeDecompress", decompress)
                .map_err(|error| error.to_string())?;

            let crypto = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeCryptoStrings(
                        wasmQuickJsArgString(args, 0),
                        wasmQuickJsArgString(args, 1),
                        wasmQuickJsArgString(args, 2),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeCrypto", crypto)
                .map_err(|error| error.to_string())?;

            let imageProcessing = self
                .context
                .wrap_callback(|_, _, args| {
                    Ok(WasmQuickJsValue::String(nativeImageProcessingStrings(
                        wasmQuickJsArgString(args, 0),
                        wasmQuickJsArgString(args, 1),
                        wasmQuickJsArgString(args, 2),
                    )))
                })
                .map_err(|error| error.to_string())?;
            globals
                .set_property("__operitNativeImageProcessing", imageProcessing)
                .map_err(|error| error.to_string())?;
            Ok(())
        }
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

#[allow(non_snake_case)]
fn buildToolPkgIpcFailure(message: &str) -> String {
    serde_json::json!({
        "success": false,
        "message": message.trim()
    })
    .to_string()
}

#[allow(non_snake_case)]
fn nativeInvokeToolPkgIpcStrings(
    packageTarget: String,
    callerContextKey: String,
    targetContextKey: String,
    targetRuntime: String,
    channel: String,
    payloadJson: String,
) -> String {
    let normalizedTarget = packageTarget.trim().to_string();
    if normalizedTarget.is_empty() {
        return buildToolPkgIpcFailure("ToolPkg.ipc package target is empty");
    }
    let normalizedChannel = channel.trim().to_string();
    if normalizedChannel.is_empty() {
        return buildToolPkgIpcFailure("ToolPkg.ipc channel is required");
    }
    let requestedRuntime = targetRuntime.trim().to_ascii_lowercase();
    if !requestedRuntime.is_empty()
        && requestedRuntime != "main"
        && requestedRuntime != "ui"
        && requestedRuntime != "sandbox"
        && requestedRuntime != "provider"
    {
        return buildToolPkgIpcFailure(&format!(
            "ToolPkg.ipc targetRuntime is invalid: {requestedRuntime}"
        ));
    }
    let payload = if payloadJson.trim().is_empty() {
        Value::Null
    } else {
        match serde_json::from_str::<Value>(payloadJson.trim()) {
            Ok(value) => value,
            Err(error) => {
                return buildToolPkgIpcFailure(&format!(
                    "ToolPkg.ipc payload JSON is invalid: {error}"
                ))
            }
        }
    };
    let request = JsToolPkgIpcRequest {
        package_target: normalizedTarget,
        caller_context_key: callerContextKey.trim().to_string(),
        target_context_key: normalizeOptionalString(&targetContextKey),
        target_runtime: normalizeOptionalString(&requestedRuntime),
        channel: normalizedChannel,
        payload,
    };
    match currentExecutionHost().and_then(|host| host.invoke_toolpkg_ipc(request)) {
        Ok(value) => serde_json::json!({
            "success": true,
            "value": value
        })
        .to_string(),
        Err(error) => buildToolPkgIpcFailure(&error),
    }
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
        #[cfg(target_arch = "wasm32")]
        {
            WASM_JS_ENGINE_STATES.with(|states| {
                states.borrow_mut().remove(&self.worker.stateId);
            });
        }
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
    ) -> JsExecutionResult<Option<String>> {
        JsEngine::execute_compose_dsl_script(self, script, runtimeOptions, envOverrides)
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
    currentExecutionHost()
        .and_then(|host| host.read_toolpkg_text_resource(&packageNameOrSubpackageId, &resourcePath))
        .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error))
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
fn nativeComposeWebViewControllerCommandString(payloadJson: String) -> String {
    currentExecutionHost()
        .and_then(|host| host.handle_compose_webview_controller_command(&payloadJson))
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

#[cfg(target_arch = "wasm32")]
#[allow(non_snake_case)]
fn wasmQuickJsArgString(args: &[WasmQuickJsValueRef], index: usize) -> String {
    args.get(index)
        .map(|value| value.to_string())
        .unwrap_or_default()
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
