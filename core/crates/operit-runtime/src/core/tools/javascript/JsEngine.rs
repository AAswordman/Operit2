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

use operit_context::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::javascript::JsComposeDslRuntimeScript::buildComposeDslRuntimeWrappedScript;
use crate::core::tools::javascript::JsExecutionResultProtocol::{
    buildJsExecutionErrorPayload, decodeJsExecutionResultValue, extractJsExecutionErrorMessage,
};
use crate::core::tools::javascript::JsJavaBridgeDelegates::{
    nativeJavaCallInstanceStrings, nativeJavaCallStaticString, nativeJavaClassExistsString,
    nativeJavaGetApplicationContextString, nativeJavaNewInstanceString,
};
use crate::core::tools::javascript::JsLibraries::buildRuntimeBootstrapScript;
use crate::core::tools::javascript::JsNativeInterfaceDelegates;
use crate::core::tools::javascript::JsToolPkgRegistration::{
    buildToolPkgRegistrationBridgeScript, ToolPkgMainRegistrationCapture,
};
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::data::preferences::EnvPreferences::EnvPreferences;
use operit_util::stream::Stream::Stream;
use operit_util::AppLogger::AppLogger;
use crate::util::LocaleUtils::LocaleUtils;
use operit_util::OperitPaths;

const TAG: &str = "OperitQuickJsEngine";
const TOOLPKG_SCRIPT_TIMEOUT_SECONDS: u64 = 60;
type ToolPkgTextResources = BTreeMap<String, String>;

#[allow(non_snake_case)]
pub trait JsExecutionListener {
    fn onIntermediateResult(&self, callId: &str, result: &str);
    fn onFailed(&self, callId: &str, reason: &str);
}

type JsExecutionListenerRef = Arc<dyn JsExecutionListener + Send + Sync>;

thread_local! {
    static CURRENT_TOOL_HANDLER: RefCell<Option<AIToolHandler>> = RefCell::new(None);
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
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        executionListener: Option<JsExecutionListenerRef>,
        timeoutSec: u64,
        response: mpsc::Sender<Option<String>>,
    },
    ExecuteToolPkgMainRegistration {
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
        response: mpsc::Sender<Result<ToolPkgMainRegistrationCapture, String>>,
    },
}

struct JsEngineState {
    #[cfg(not(target_arch = "wasm32"))]
    runtime: QuickJsRuntime,
    #[cfg(not(target_arch = "wasm32"))]
    context: QuickJsContext,
    #[cfg(target_arch = "wasm32")]
    context: WasmQuickJsContext,
    toolHandler: Option<AIToolHandler>,
    jsEnvironmentInitialized: bool,
}

impl JsEngine {
    pub fn new(toolHandler: AIToolHandler) -> Self {
        Self {
            worker: JsEngineWorker::new(Some(toolHandler)),
        }
    }

    #[allow(non_snake_case)]
    pub fn newToolPkgRegistrationEngine() -> Self {
        Self {
            worker: JsEngineWorker::new(None),
        }
    }

    #[allow(non_snake_case)]
    pub fn newToolPkgRegistrationEngineWithContext(context: OperitApplicationContext) -> Self {
        Self {
            worker: JsEngineWorker::new(Some(AIToolHandler::getInstance(context))),
        }
    }

    #[allow(non_snake_case)]
    pub fn executeScriptFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> Option<String> {
        let safeTimeoutSec = timeoutSec.max(1);
        #[cfg(target_arch = "wasm32")]
        {
            return self.worker.executeScriptFunction(
                script,
                functionName,
                params,
                envOverrides,
                onIntermediateResult,
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
                onIntermediateResult,
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
                    listener.onFailed("", &error.to_string());
                }
                return Some(buildJsExecutionErrorPayload(&error.to_string()));
            }
            match receiver.recv_timeout(Duration::from_secs(safeTimeoutSec)) {
                Ok(value) => value,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let reason =
                        format!("Script execution timed out after {safeTimeoutSec} seconds");
                    if let Some(listener) = executionListener.as_ref() {
                        listener.onFailed("", &reason);
                    }
                    Some(buildJsExecutionErrorPayload(&reason))
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
                        listener.onFailed("", "JS execution worker disconnected");
                    }
                    Some(buildJsExecutionErrorPayload(
                        "JS execution worker disconnected",
                    ))
                }
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn executeToolPkgMainRegistrationFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
    ) -> Result<ToolPkgMainRegistrationCapture, String> {
        self.executeToolPkgMainRegistrationFunctionWithTextResources(
            script,
            functionName,
            params,
            None,
        )
    }

    #[allow(non_snake_case)]
    pub(crate) fn executeToolPkgMainRegistrationFunctionWithTextResources(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> Result<ToolPkgMainRegistrationCapture, String> {
        #[cfg(target_arch = "wasm32")]
        {
            return self.worker.executeToolPkgMainRegistrationFunction(
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
                return Err(error.to_string());
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
                    Err(error.to_string())
                }
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn executeComposeDslScript(
        &self,
        script: &str,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
    ) -> Option<String> {
        self.executeScriptFunction(
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
    pub fn executeComposeDslAction(
        &self,
        actionId: &str,
        payload: Option<Value>,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Option<String> {
        let normalizedActionId = actionId.trim();
        if normalizedActionId.is_empty() {
            return Some(buildJsExecutionErrorPayload(
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
        self.executeScriptFunction(
            "",
            "__operit_dispatch_compose_dsl_action",
            &params,
            envOverrides,
            onIntermediateResult,
            true,
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
            None,
        )
    }

    #[allow(non_snake_case)]
    pub fn rerenderComposeDslTree(
        &self,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
    ) -> Option<String> {
        self.executeScriptFunction(
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
    pub fn dispatchComposeDslActionAsync(
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
    let result = engine.executeComposeDslAction(
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
    if let Some(error) = extractJsExecutionErrorMessage(result.as_deref()) {
        emit(composeDslActionEvent("error", Some(&error), None));
    } else if let Some(result) = result {
        emit(composeDslActionEvent("final", None, Some(&result)));
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
    fn new(toolHandler: Option<AIToolHandler>) -> Self {
        let (sender, receiver) = mpsc::channel::<JsEngineRequest>();
        std::thread::Builder::new()
            .name("OperitQuickJsEngine".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut state = JsEngineState::new(toolHandler);
                for request in receiver {
                    match request {
                        JsEngineRequest::ExecuteScript {
                            script,
                            functionName,
                            params,
                            envOverrides,
                            onIntermediateResult,
                            dispatchIntermediateOnMain,
                            executionListener,
                            timeoutSec,
                            response,
                        } => {
                            let output = state.executeScriptFunctionOnCurrentThread(
                                &script,
                                &functionName,
                                &params,
                                &envOverrides,
                                onIntermediateResult,
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
                                .executeToolPkgMainRegistrationFunctionOnCurrentThread(
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
    fn new(toolHandler: Option<AIToolHandler>) -> Self {
        let stateId = NEXT_WASM_JS_ENGINE_STATE_ID.fetch_add(1, Ordering::Relaxed);
        WASM_JS_ENGINE_STATES.with(|states| {
            states
                .borrow_mut()
                .insert(stateId, JsEngineState::new(toolHandler));
        });
        Self { stateId }
    }

    #[allow(non_snake_case)]
    fn executeScriptFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> Option<String> {
        WASM_JS_ENGINE_STATES.with(|states| {
            states
                .borrow_mut()
                .get_mut(&self.stateId)
                .expect("wasm JsEngine state must exist")
                .executeScriptFunctionOnCurrentThread(
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

    #[allow(non_snake_case)]
    fn executeToolPkgMainRegistrationFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> Result<ToolPkgMainRegistrationCapture, String> {
        WASM_JS_ENGINE_STATES.with(|states| {
            states
                .borrow_mut()
                .get_mut(&self.stateId)
                .expect("wasm JsEngine state must exist")
                .executeToolPkgMainRegistrationFunctionOnCurrentThread(
                    script,
                    functionName,
                    params,
                    textResources,
                )
        })
    }
}

impl JsEngineState {
    fn new(toolHandler: Option<AIToolHandler>) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let runtime = QuickJsRuntime::new().expect("OperitQuickJsEngine runtime must start");
            let context =
                QuickJsContext::full(&runtime).expect("OperitQuickJsEngine context must start");
            let mut state = Self {
                runtime,
                context,
                toolHandler,
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
                toolHandler,
                jsEnvironmentInitialized: false,
            };
            state
                .registerNativeInterface()
                .expect("NativeInterface bridge must register");
            state
        }
    }

    #[allow(non_snake_case)]
    fn executeScriptFunctionOnCurrentThread(
        &mut self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        _dispatchIntermediateOnMain: bool,
        timeoutSec: u64,
        executionListener: Option<JsExecutionListenerRef>,
    ) -> Option<String> {
        if let Err(error) = self.initJavaScriptEnvironment() {
            return Some(buildJsExecutionErrorPayload(&error));
        }
        CURRENT_TOOL_HANDLER.with(|handler| {
            *handler.borrow_mut() = self.toolHandler.clone();
        });
        CURRENT_INTERMEDIATE_CALLBACK.with(|callback| {
            *callback.borrow_mut() = onIntermediateResult;
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
                    return Some(buildJsExecutionErrorPayload(&error));
                }
            };
            effectiveParams.insert("__operit_package_lang".to_string(), Value::String(language));
        }

        let paramsJson = match serde_json::to_string(&effectiveParams) {
            Ok(value) => value,
            Err(error) => {
                clearThreadLocalCallState();
                return Some(buildJsExecutionErrorPayload(&error.to_string()));
            }
        };
        let scriptJson = serde_json::to_string(script).unwrap_or_else(|_| "\"\"".to_string());
        let functionNameJson =
            serde_json::to_string(functionName).unwrap_or_else(|_| "\"\"".to_string());
        let callId = format!(
            "operit_call_{}",
            Uuid::new_v4().to_string().replace('-', "")
        );
        let callIdJson =
            serde_json::to_string(&callId).unwrap_or_else(|_| "\"operit_call\"".to_string());
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
                Some(buildJsExecutionErrorPayload(&error.to_string()))
            }
        };
        clearNativeExecutionSession(&callId);
        clearThreadLocalCallState();
        output
    }

    #[allow(non_snake_case)]
    fn executeToolPkgMainRegistrationFunctionOnCurrentThread(
        &mut self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<ToolPkgTextResources>>,
    ) -> Result<ToolPkgMainRegistrationCapture, String> {
        self.initJavaScriptEnvironment()?;
        let bridge = buildToolPkgRegistrationBridgeScript();
        self.evalJavaScriptVoid(&bridge)?;
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
            let language = self.resolveCurrentPackageLanguage()?;
            registrationParams.insert("__operit_package_lang".to_string(), Value::String(language));
        }
        let paramsJson =
            serde_json::to_string(&registrationParams).map_err(|error| error.to_string())?;
        let scriptJson = serde_json::to_string(script).map_err(|error| error.to_string())?;
        let functionNameJson =
            serde_json::to_string(functionName).map_err(|error| error.to_string())?;
        let callId = format!(
            "operit_registration_{}",
            Uuid::new_v4().to_string().replace('-', "")
        );
        let callIdJson = serde_json::to_string(&callId).map_err(|error| error.to_string())?;
        clearNativeExecutionSession(&callId);
        let executionScript = format!(
            "__operitExecuteScriptFunction({callIdJson}, {paramsJson}, {scriptJson}, {functionNameJson}, 60, 10000);"
        );
        if let Err(error) = self.evalJavaScriptVoid(&executionScript) {
            CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
                *resources.borrow_mut() = None;
            });
            return Err(error);
        }
        self.runJavaScriptJobs();
        let output = readNativeExecutionSession(&callId)
            .ok_or_else(|| "ToolPkg registration JavaScript did not complete".to_string());
        CURRENT_TOOLPKG_TEXT_RESOURCES.with(|resources| {
            *resources.borrow_mut() = None;
        });
        let output = output?;
        clearNativeExecutionSession(&callId);
        ensureRegistrationExecutionSucceeded(&output)?;

        let captureScript = r#"
        (function() {
            return JSON.stringify(globalThis.__operitToolPkgRegistrationCapture);
        })()
        "#;
        let captureJson = self.evalJavaScriptString(captureScript)?;
        serde_json::from_str::<ToolPkgMainRegistrationCapture>(&captureJson)
            .map_err(|error| error.to_string())
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
        let toolHandler = self
            .toolHandler
            .as_ref()
            .ok_or_else(|| "AIToolHandler is required to resolve package language".to_string())?;
        let language = LocaleUtils::getCurrentLanguage(&toolHandler.getContext())?;
        let trimmed = language.trim();
        if trimmed.is_empty() {
            Ok("en".to_string())
        } else {
            Ok(trimmed.to_string())
        }
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
fn inferToolPkgIpcRuntimeFromContextKey(contextKey: &str) -> String {
    let normalized = contextKey.trim().to_ascii_lowercase();
    if normalized.starts_with("toolpkg_main:") {
        return "main".to_string();
    }
    if normalized.starts_with("toolpkg_provider:") {
        return "provider".to_string();
    }
    if normalized.starts_with("toolpkg_compose:")
        || normalized.starts_with("toolpkg_compose_dsl:")
        || normalized.starts_with("toolpkg_xml_render:")
    {
        return "ui".to_string();
    }
    String::new()
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

    let resolved = CURRENT_TOOL_HANDLER.with(|handler| -> Result<(JsEngine, String, String, String, String), String> {
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return Err("NativeInterface tool handler is unavailable".to_string());
        };
        let managerSnapshot = {
            let packageManager = toolHandler.getOrCreatePackageManager();
            let guard = packageManager
                .lock()
                .expect("package manager mutex poisoned");
            guard.clone()
        };
        let Some(containerRuntime) = managerSnapshot.getToolPkgContainerRuntime(&normalizedTarget) else {
            return Err(format!("ToolPkg container not found: {normalizedTarget}"));
        };
        let explicitTargetContextKey = targetContextKey.trim().to_string();
        let resolvedTargetContextKey = if !explicitTargetContextKey.is_empty() {
            explicitTargetContextKey.clone()
        } else if requestedRuntime.is_empty() || requestedRuntime == "main" {
            format!("toolpkg_main:{normalizedTarget}")
        } else {
            return Err(format!(
                "ToolPkg.ipc targetContextKey is required for targetRuntime={requestedRuntime}"
            ));
        };
        let inferredRuntime = inferToolPkgIpcRuntimeFromContextKey(&resolvedTargetContextKey);
        if !requestedRuntime.is_empty()
            && !inferredRuntime.is_empty()
            && requestedRuntime != inferredRuntime
        {
            return Err(format!(
                "ToolPkg.ipc targetRuntime does not match targetContextKey: {requestedRuntime} != {inferredRuntime}"
            ));
        }
        let resolvedTargetRuntime = if !requestedRuntime.is_empty() {
            requestedRuntime.clone()
        } else if !inferredRuntime.is_empty() {
            inferredRuntime
        } else {
            return Err(format!(
                "ToolPkg.ipc targetRuntime is required for targetContextKey={resolvedTargetContextKey}"
            ));
        };
        let isMainTarget = resolvedTargetRuntime == "main";
        if isMainTarget
            && resolvedTargetContextKey.to_ascii_lowercase()
                != format!("toolpkg_main:{normalizedTarget}").to_ascii_lowercase()
        {
            return Err(format!(
                "ToolPkg.ipc main targetContextKey is invalid: {resolvedTargetContextKey}"
            ));
        }
        if !isMainTarget && explicitTargetContextKey.is_empty() {
            return Err(format!(
                "ToolPkg.ipc targetContextKey is required for targetRuntime={resolvedTargetRuntime}"
            ));
        }
        let engine = if isMainTarget {
            managerSnapshot.getToolPkgExecutionEngine(&resolvedTargetContextKey)
        } else {
            let Some(engine) = managerSnapshot.findToolPkgExecutionEngine(&resolvedTargetContextKey) else {
                return Err(format!(
                    "ToolPkg.ipc target runtime is not active: {resolvedTargetContextKey}"
                ));
            };
            engine
        };
        let (scriptPath, script) = if isMainTarget {
            let mainEntry = containerRuntime.mainEntry.trim().to_string();
            if mainEntry.is_empty() {
                return Err(format!("ToolPkg main entry is unavailable: {normalizedTarget}"));
            }
            let Some(mainScript) = managerSnapshot.getToolPkgMainScriptInternal(&normalizedTarget) else {
                return Err(format!("ToolPkg main script is unavailable: {normalizedTarget}"));
            };
            (mainEntry, mainScript)
        } else {
            (String::new(), String::new())
        };
        Ok((
            engine,
            scriptPath,
            script,
            resolvedTargetContextKey,
            resolvedTargetRuntime,
        ))
    });

    let (engine, scriptPath, script, resolvedTargetContextKey, resolvedTargetRuntime) =
        match resolved {
            Ok(value) => value,
            Err(error) => return buildToolPkgIpcFailure(&error),
        };

    let dispatchFunctionName = "__operit_toolpkg_runtime_dispatch__";
    let dispatchFunctionSource = r#"
        async function(params) {
            var dispatch = globalThis.__operitInvokeToolPkgIpcLocal;
            if (typeof dispatch !== 'function') {
                throw new Error('ToolPkg.ipc runtime is unavailable in target context');
            }
            var payloadJson =
                params && typeof params.__operit_toolpkg_ipc_payload_json === 'string'
                    ? params.__operit_toolpkg_ipc_payload_json
                    : 'null';
            var payload;
            try {
                payload = JSON.parse(payloadJson);
            } catch (error) {
                throw new Error(
                    'ToolPkg.ipc payload JSON is invalid: ' +
                    String(error && error.message ? error.message : error)
                );
            }
            var channel =
                params && typeof params.__operit_toolpkg_ipc_channel === 'string'
                    ? params.__operit_toolpkg_ipc_channel.trim()
                    : '';
            if (!channel) {
                throw new Error('ToolPkg.ipc channel is required');
            }
            var callerContextKey =
                params && typeof params.__operit_toolpkg_ipc_caller_context_key === 'string'
                    ? params.__operit_toolpkg_ipc_caller_context_key
                    : '';
            var currentContextKey =
                params && typeof params.__operit_execution_context_key === 'string'
                    ? params.__operit_execution_context_key
                    : '';
            var packageTarget =
                params && typeof params.__operit_ui_package_name === 'string'
                    ? params.__operit_ui_package_name
                    : '';
            var currentRuntime =
                params && typeof params.__operit_toolpkg_runtime_kind === 'string'
                    ? params.__operit_toolpkg_runtime_kind.trim()
                    : '';
            return await dispatch(channel, payload, {
                channel: channel,
                callerContextKey: callerContextKey,
                currentContextKey: currentContextKey,
                currentRuntime: currentRuntime,
                packageTarget: packageTarget
            });
        }
    "#
    .trim();

    let mut params = BTreeMap::new();
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String(normalizedTarget.clone()),
    );
    params.insert(
        "toolPkgId".to_string(),
        Value::String(normalizedTarget.clone()),
    );
    params.insert(
        "containerPackageName".to_string(),
        Value::String(normalizedTarget.clone()),
    );
    params.insert(
        "__operit_execution_context_key".to_string(),
        Value::String(resolvedTargetContextKey.clone()),
    );
    params.insert(
        "__operit_toolpkg_runtime_kind".to_string(),
        Value::String(resolvedTargetRuntime.clone()),
    );
    params.insert(
        "__operit_script_screen".to_string(),
        Value::String(scriptPath),
    );
    params.insert(
        "__operit_inline_function_name".to_string(),
        Value::String(dispatchFunctionName.to_string()),
    );
    params.insert(
        "__operit_inline_function_source".to_string(),
        Value::String(dispatchFunctionSource.to_string()),
    );
    params.insert(
        "__operit_toolpkg_ipc_channel".to_string(),
        Value::String(normalizedChannel),
    );
    let normalizedPayloadJson = if payloadJson.trim().is_empty() {
        "null".to_string()
    } else {
        payloadJson.trim().to_string()
    };
    params.insert(
        "__operit_toolpkg_ipc_payload_json".to_string(),
        Value::String(normalizedPayloadJson),
    );
    params.insert(
        "__operit_toolpkg_ipc_caller_context_key".to_string(),
        Value::String(callerContextKey.trim().to_string()),
    );

    let result = engine.executeScriptFunction(
        &script,
        dispatchFunctionName,
        &params,
        &BTreeMap::new(),
        None,
        true,
        TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
        None,
    );
    if let Some(errorMessage) = extractJsExecutionErrorMessage(result.as_deref()) {
        return buildToolPkgIpcFailure(&errorMessage);
    }
    serde_json::json!({
        "success": true,
        "value": decodeJsExecutionResultValue(result.as_deref())
    })
    .to_string()
}

#[allow(non_snake_case)]
fn clearThreadLocalCallState() {
    CURRENT_TOOL_HANDLER.with(|handler| {
        *handler.borrow_mut() = None;
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
    pub fn destroy(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            WASM_JS_ENGINE_STATES.with(|states| {
                states.borrow_mut().remove(&self.worker.stateId);
            });
        }
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
    CURRENT_TOOL_HANDLER.with(|handler| {
        handler
            .borrow()
            .as_ref()
            .map(|toolHandler| {
                JsNativeInterfaceDelegates::callToolSync(
                    toolHandler,
                    &toolType,
                    &toolName,
                    &paramsJson,
                )
            })
            .unwrap_or_else(|| {
                serde_json::json!({
                    "success": false,
                    "message": "NativeInterface tool handler is unavailable"
                })
                .to_string()
            })
    })
}

#[allow(non_snake_case)]
fn nativeSendIntermediateResultString(callId: String, result: String) {
    CURRENT_EXECUTION_LISTENER.with(|listener| {
        if let Some(listener) = listener.borrow().as_ref() {
            listener.onIntermediateResult(&callId, &result);
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
    CURRENT_TOOL_HANDLER.with(|handler| {
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return String::new();
        };
        let packageManager = toolHandler.getOrCreatePackageManager();
        let guard = packageManager
            .lock()
            .expect("package manager mutex poisoned");
        guard
            .readToolPkgTextResource(&packageNameOrSubpackageId, &resourcePath, true)
            .unwrap_or_default()
    })
}

#[allow(non_snake_case)]
fn nativeReadToolPkgResourceStrings(
    packageNameOrSubpackageId: String,
    resourceKey: String,
    outputFileName: String,
    internal: String,
) -> String {
    CURRENT_TOOL_HANDLER.with(|handler| {
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return String::new();
        };
        let target = packageNameOrSubpackageId.trim().to_string();
        let key = resourceKey.trim().to_string();
        if target.is_empty() || key.is_empty() {
            return String::new();
        }
        let packageManager = toolHandler.getOrCreatePackageManager();
        let guard = packageManager
            .lock()
            .expect("package manager mutex poisoned");
        let trimmedOutputFileName = outputFileName.trim();
        let fileName = if trimmedOutputFileName.is_empty() {
            guard
                .getToolPkgResourceOutputFileName(&target, &key, true)
                .unwrap_or_else(|| format!("{key}.bin"))
        } else {
            trimmedOutputFileName.to_string()
        };
        let safeName = toolPkgResourceOutputFileName(&key, &fileName);
        let outputDir = toolPkgResourceOutputDir(parseBooleanFlag(&internal));
        if std::fs::create_dir_all(&outputDir).is_err() {
            return String::new();
        }
        let outputFile = outputDir.join(safeName);
        let copied = guard.copyToolPkgResourceToFile(&target, &key, &outputFile)
            || guard.copyToolPkgResourceToFileBySubpackageId(&target, &key, &outputFile, true);
        if copied {
            outputFile.to_string_lossy().to_string()
        } else {
            String::new()
        }
    })
}

#[allow(non_snake_case)]
fn nativeComposeWebViewControllerCommandString(payloadJson: String) -> String {
    CURRENT_TOOL_HANDLER.with(|handler| {
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return buildJsExecutionErrorPayload("NativeInterface tool handler is unavailable");
        };
        let context = toolHandler.getContext();
        let Some(host) = context.composeDslWebViewHost.as_ref() else {
            return buildJsExecutionErrorPayload("ComposeDslWebViewHost is not registered");
        };
        host.handleControllerCommand(&payloadJson)
            .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error.to_string()))
    })
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
            listener.onFailed(&callId, &error);
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
    let value = EnvPreferences::getInstance()
        .getEnv(&key)
        .ok()
        .flatten()
        .unwrap_or_default();
    value
}

#[allow(non_snake_case)]
fn nativeGetPluginConfigDirString(pluginId: String) -> String {
    OperitPaths::pluginConfigDir(&pluginId)
        .expect("plugin config dir must be available")
        .to_string_lossy()
        .to_string()
}

#[allow(non_snake_case)]
fn nativeIsPackageImportedString(packageName: String) -> String {
    CURRENT_TOOL_HANDLER.with(|handler| {
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return "false".to_string();
        };
        let Some(normalizedPackageName) = normalizeNonBlankString(&packageName) else {
            return "false".to_string();
        };
        let packageManager = toolHandler.getOrCreatePackageManager();
        let guard = packageManager
            .lock()
            .expect("package manager mutex poisoned");
        guard.isPackageEnabled(&normalizedPackageName).to_string()
    })
}

#[allow(non_snake_case)]
fn nativeImportPackageString(packageName: String) -> String {
    CURRENT_TOOL_HANDLER.with(|handler| {
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return "package import failed".to_string();
        };
        let Some(normalizedPackageName) = normalizeNonBlankString(&packageName) else {
            return "Package name is required".to_string();
        };
        let packageManager = toolHandler.getOrCreatePackageManager();
        let mut guard = packageManager
            .lock()
            .expect("package manager mutex poisoned");
        guard.enablePackage(&normalizedPackageName)
    })
}

#[allow(non_snake_case)]
fn nativeRemovePackageString(packageName: String) -> String {
    CURRENT_TOOL_HANDLER.with(|handler| {
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return "package removal failed".to_string();
        };
        let Some(normalizedPackageName) = normalizeNonBlankString(&packageName) else {
            return "Package name is required".to_string();
        };
        let packageManager = toolHandler.getOrCreatePackageManager();
        let mut guard = packageManager
            .lock()
            .expect("package manager mutex poisoned");
        guard.disablePackage(&normalizedPackageName)
    })
}

#[allow(non_snake_case)]
fn nativeUsePackageString(packageName: String) -> String {
    CURRENT_TOOL_HANDLER.with(|handler| {
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return "package activation failed".to_string();
        };
        let Some(normalizedPackageName) = normalizeNonBlankString(&packageName) else {
            return "Package name is required".to_string();
        };
        let packageManager = toolHandler.getOrCreatePackageManager();
        let mut guard = packageManager
            .lock()
            .expect("package manager mutex poisoned");
        guard.usePackage(&normalizedPackageName)
    })
}

#[allow(non_snake_case)]
fn nativeListImportedPackagesJsonString() -> String {
    CURRENT_TOOL_HANDLER.with(|handler| {
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return "[]".to_string();
        };
        let packageManager = toolHandler.getOrCreatePackageManager();
        let guard = packageManager
            .lock()
            .expect("package manager mutex poisoned");
        serde_json::to_string(&guard.getEnabledPackageNames()).unwrap_or_else(|_| "[]".to_string())
    })
}

#[allow(non_snake_case)]
fn nativeResolveToolNameString(
    packageName: String,
    subpackageId: String,
    toolName: String,
    preferImported: String,
) -> String {
    CURRENT_TOOL_HANDLER.with(|handler| {
        let normalizedTool = match normalizeNonBlankString(&toolName) {
            Some(value) => value,
            None => return String::new(),
        };
        if normalizedTool.contains(':') {
            return normalizedTool;
        }
        let borrowed = handler.borrow();
        let Some(toolHandler) = borrowed.as_ref() else {
            return toolName.trim().to_string();
        };
        let preferEnabled = !preferImported.eq_ignore_ascii_case("false");
        let packageManager = toolHandler.getOrCreatePackageManager();
        let guard = packageManager
            .lock()
            .expect("package manager mutex poisoned");
        let resolvedPackageName = if let Some(candidate) = normalizeNonBlankString(&packageName) {
            guard
                .findPreferredPackageNameForSubpackageId(&candidate, preferEnabled)
                .unwrap_or(candidate)
        } else if let Some(candidate) = normalizeNonBlankString(&subpackageId) {
            guard
                .findPreferredPackageNameForSubpackageId(&candidate, preferEnabled)
                .unwrap_or(candidate)
        } else {
            String::new()
        };
        if resolvedPackageName.trim().is_empty() {
            normalizedTool
        } else {
            format!("{resolvedPackageName}:{normalizedTool}")
        }
    })
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
fn toolPkgResourceOutputFileName(resourceKey: &str, outputFileName: &str) -> String {
    let safeName = outputFileName
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    if safeName.is_empty() {
        format!("{resourceKey}.bin")
    } else {
        safeName.to_string()
    }
}

#[allow(non_snake_case)]
fn toolPkgResourceOutputDir(internal: bool) -> std::path::PathBuf {
    OperitPaths::toolPkgResourceExportsDir(internal)
        .expect("toolpkg resource export path must be available")
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
