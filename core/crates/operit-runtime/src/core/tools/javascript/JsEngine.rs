use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
#[cfg(target_arch = "wasm32")]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;
use std::sync::Arc;

use operit_store::RuntimeStorePaths::default_data_dir;
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

use crate::core::tools::javascript::JsExecutionResultProtocol::buildJsExecutionErrorPayload;
use crate::core::tools::javascript::JsExecutionScriptBuilder;
use crate::core::tools::javascript::JsInitRuntimeScriptBuilder;
use crate::core::tools::javascript::JsNativeInterfaceDelegates;
use crate::core::tools::javascript::JsToolPkgRegistration::{
    buildToolPkgRegistrationBridgeScript, ToolPkgMainRegistrationCapture,
};
use crate::core::tools::javascript::JsTools::getJsToolsDefinition;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::data::preferences::EnvPreferences::EnvPreferences;
use crate::util::AppLogger::AppLogger;

const TAG: &str = "OperitQuickJsEngine";

thread_local! {
    static CURRENT_TOOL_HANDLER: RefCell<Option<AIToolHandler>> = RefCell::new(None);
    static CURRENT_INTERMEDIATE_CALLBACK: RefCell<Option<Arc<dyn Fn(String) + Send + Sync>>> = RefCell::new(None);
    static CURRENT_CALL_RESULTS: RefCell<BTreeMap<String, String>> = RefCell::new(BTreeMap::new());
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
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        response: mpsc::Sender<Option<String>>,
    },
    ExecuteToolPkgMainRegistration {
        script: String,
        functionName: String,
        params: BTreeMap<String, Value>,
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
    pub fn executeScriptFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Option<String> {
        #[cfg(target_arch = "wasm32")]
        {
            return self.worker.executeScriptFunction(
                script,
                functionName,
                params,
                onIntermediateResult,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (response, receiver) = mpsc::channel();
            let request = JsEngineRequest::ExecuteScript {
                script: script.to_string(),
                functionName: functionName.to_string(),
                params: params.clone(),
                onIntermediateResult,
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
                return Some(buildJsExecutionErrorPayload(&error.to_string()));
            }
            match receiver.recv() {
                Ok(value) => value,
                Err(error) => {
                    AppLogger::e(
                        TAG,
                        &format!(
                            "response-recv-error function={} scriptLen={} params={} error={}",
                            functionName,
                            script.len(),
                            summarizeParams(params),
                            error
                        ),
                    );
                    Some(buildJsExecutionErrorPayload(&error.to_string()))
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
        #[cfg(target_arch = "wasm32")]
        {
            return self.worker.executeToolPkgMainRegistrationFunction(
                script,
                functionName,
                params,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (response, receiver) = mpsc::channel();
            let request = JsEngineRequest::ExecuteToolPkgMainRegistration {
                script: script.to_string(),
                functionName: functionName.to_string(),
                params: params.clone(),
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
                            onIntermediateResult,
                            response,
                        } => {
                            let output = state.executeScriptFunctionOnCurrentThread(
                                &script,
                                &functionName,
                                &params,
                                onIntermediateResult,
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
                            response,
                        } => {
                            let output = state
                                .executeToolPkgMainRegistrationFunctionOnCurrentThread(
                                    &script,
                                    &functionName,
                                    &params,
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
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
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
                    onIntermediateResult,
                )
        })
    }

    #[allow(non_snake_case)]
    fn executeToolPkgMainRegistrationFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
    ) -> Result<ToolPkgMainRegistrationCapture, String> {
        WASM_JS_ENGINE_STATES.with(|states| {
            states
                .borrow_mut()
                .get_mut(&self.stateId)
                .expect("wasm JsEngine state must exist")
                .executeToolPkgMainRegistrationFunctionOnCurrentThread(script, functionName, params)
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
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
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

        let paramsJson = match serde_json::to_string(params) {
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

        clearNativeExecutionSession(&callId);
        let executionScript = format!(
            "__operitExecuteScriptFunction({callIdJson}, {paramsJson}, {scriptJson}, {functionNameJson}, 60, 10000);"
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
    ) -> Result<ToolPkgMainRegistrationCapture, String> {
        self.initJavaScriptEnvironment()?;
        let bridge = buildToolPkgRegistrationBridgeScript();
        self.evalJavaScriptVoid(&bridge)?;

        let mut registrationParams = params.clone();
        registrationParams.insert("__operit_registration_mode".to_string(), Value::Bool(true));
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
        self.evalJavaScriptVoid(&executionScript)?;
        self.runJavaScriptJobs();
        let output = readNativeExecutionSession(&callId)
            .ok_or_else(|| "ToolPkg registration JavaScript did not complete".to_string())?;
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

                let sendIntermediateResult = QuickJsFunction::new(ctx.clone(), |result: String| {
                    nativeSendIntermediateResultString(result)
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

                let logJsExecutionTrace =
                    QuickJsFunction::new(ctx.clone(), |callId: String, message: String| {
                        nativeLogJsExecutionTraceStrings(callId, message)
                    })
                    .map_err(|error| error.to_string())?;
                globals
                    .set("__operitNativeLogJsExecutionTrace", logJsExecutionTrace)
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
                    nativeSendIntermediateResultString(wasmQuickJsArgString(args, 0));
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
fn clearThreadLocalCallState() {
    CURRENT_TOOL_HANDLER.with(|handler| {
        *handler.borrow_mut() = None;
    });
    CURRENT_INTERMEDIATE_CALLBACK.with(|callback| {
        *callback.borrow_mut() = None;
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
mod tests {
    use super::JsEngineState;
    use crate::data::preferences::EnvPreferences::EnvPreferences;
    use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
    use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
    use operit_store::RuntimeStorePaths::setDefaultRuntimeStoreRoot;
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::path::{Component, Path, PathBuf};
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    struct TestRuntimeStorageHost {
        root: PathBuf,
    }

    impl TestRuntimeStorageHost {
        fn new(root: PathBuf) -> Self {
            Self { root }
        }

        fn resolve(&self, path: &str) -> HostResult<PathBuf> {
            let path = Path::new(path);
            if path.is_absolute() {
                return Err(HostError::new(format!(
                    "Runtime storage path must be relative: {}",
                    path.display()
                )));
            }
            let mut resolved = self.root.clone();
            for component in path.components() {
                match component {
                    Component::Normal(segment) => resolved.push(segment),
                    Component::CurDir => {}
                    _ => {
                        return Err(HostError::new(format!(
                            "Invalid runtime storage path: {}",
                            path.display()
                        )))
                    }
                }
            }
            Ok(resolved)
        }
    }

    impl RuntimeStorageHost for TestRuntimeStorageHost {
        fn rootDir(&self) -> Option<PathBuf> {
            Some(self.root.clone())
        }

        fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            Ok(std::fs::read(self.resolve(path)?)?)
        }

        fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            let path = self.resolve(path)?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)?;
            Ok(())
        }

        fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
            let path = self.resolve(path)?;
            if !path.exists() {
                return Ok(());
            }
            if path.is_dir() {
                if recursive {
                    std::fs::remove_dir_all(path)?;
                } else {
                    std::fs::remove_dir(path)?;
                }
            } else {
                std::fs::remove_file(path)?;
            }
            Ok(())
        }

        fn exists(&self, path: &str) -> HostResult<bool> {
            Ok(self.resolve(path)?.exists())
        }

        fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
            let directory = self.resolve(prefix)?;
            let mut entries = Vec::new();
            if !directory.exists() {
                return Ok(entries);
            }
            for entry in std::fs::read_dir(directory)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                let path = entry
                    .path()
                    .strip_prefix(&self.root)
                    .map_err(|error| HostError::new(error.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                entries.push(RuntimeStorageEntry {
                    path,
                    isDirectory: metadata.is_dir(),
                    size: metadata.len() as i64,
                });
            }
            Ok(entries)
        }
    }

    fn ensure_test_runtime_root() {
        let root = std::env::temp_dir().join("operit-runtime-js-engine-tests");
        std::fs::create_dir_all(&root).expect("test runtime root");
        let host = Arc::new(TestRuntimeStorageHost::new(root.clone()));
        setDefaultRuntimeStoreRoot(root);
        setDefaultRuntimeStorageHost(host);
    }

    #[test]
    fn execute_promise_script_repeatedly_on_same_engine() {
        let mut state = JsEngineState::new(None);
        let script = r#"
            globalThis.__operit_cached_async_echo = globalThis.__operit_cached_async_echo || function(params) {
                return Promise.resolve("ASYNC_ECHO:" + params.text);
            };
            exports.async_echo = globalThis.__operit_cached_async_echo;
        "#;

        for index in 0..16 {
            let mut params = BTreeMap::new();
            params.insert(
                "text".to_string(),
                Value::String(format!("same-engine-{index}")),
            );
            let output =
                state.executeScriptFunctionOnCurrentThread(script, "async_echo", &params, None);
            assert_eq!(
                output.as_deref(),
                Some(format!("\"ASYNC_ECHO:same-engine-{index}\"").as_str())
            );
        }
    }

    #[test]
    fn execute_complete_finishes_call_before_return_value() {
        let mut state = JsEngineState::new(None);
        let script = r#"
            exports.complete_first = function(_params) {
                complete("first");
                return "second";
            };
        "#;
        let params = BTreeMap::new();

        let output =
            state.executeScriptFunctionOnCurrentThread(script, "complete_first", &params, None);

        assert_eq!(output.as_deref(), Some("\"first\""));
    }

    #[test]
    fn execute_function_with_active_module_context() {
        let mut state = JsEngineState::new(None);
        let script = r#"
            exports.marker = "root-marker";
            exports.inspect_context = function(_params) {
                return String(globalThis.__operitActiveModuleExports === exports) +
                    ":" +
                    String(globalThis.__operitActiveModule && globalThis.__operitActiveModule.exports === exports) +
                    ":" +
                    globalThis.__operitActiveModuleExports.marker;
            };
        "#;
        let params = BTreeMap::new();

        let output =
            state.executeScriptFunctionOnCurrentThread(script, "inspect_context", &params, None);

        assert_eq!(output.as_deref(), Some("\"true:true:root-marker\""));
    }

    #[test]
    fn execute_inline_hook_function_source() {
        let mut state = JsEngineState::new(None);
        let script = r#"
            exports.marker = "inline-root";
        "#;
        let mut params = BTreeMap::new();
        params.insert(
            "__operit_inline_function_name".to_string(),
            Value::String("__operit_inline_test".to_string()),
        );
        params.insert(
            "__operit_inline_function_source".to_string(),
            Value::String(
                r#"function(_params) { return globalThis.__operitActiveModuleExports.marker; }"#
                    .to_string(),
            ),
        );

        let output = state.executeScriptFunctionOnCurrentThread(
            script,
            "__operit_inline_test",
            &params,
            None,
        );

        assert_eq!(output.as_deref(), Some("\"inline-root\""));
    }

    #[test]
    fn execute_function_from_module_exports() {
        let mut state = JsEngineState::new(None);
        let script = r#"
            module.exports = {
                module_only: function(params) {
                    return "module:" + params.text;
                }
            };
        "#;
        let mut params = BTreeMap::new();
        params.insert("text".to_string(), Value::String("exports".to_string()));

        let output =
            state.executeScriptFunctionOnCurrentThread(script, "module_only", &params, None);

        assert_eq!(output.as_deref(), Some("\"module:exports\""));
    }

    #[test]
    fn register_thinking_guidance_toolpkg_main() {
        let engine = super::JsEngine::newToolPkgRegistrationEngine();
        let repoRoot = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repo root");
        let scriptPath = repoRoot.join("plugins/buildin/thinking_guidance/dist/main.js");
        let script = std::fs::read_to_string(&scriptPath).expect("thinking_guidance main.js");
        let mut params = BTreeMap::new();
        params.insert(
            "toolPkgId".to_string(),
            Value::String("thinking_guidance".to_string()),
        );

        let capture = engine
            .executeToolPkgMainRegistrationFunction(&script, "registerToolPkg", &params)
            .expect("thinking_guidance registration");

        assert_eq!(capture.inputMenuTogglePlugins.len(), 1);
        assert_eq!(capture.systemPromptComposeHooks.len(), 1);
        let menu = serde_json::from_str::<Value>(&capture.inputMenuTogglePlugins[0]).unwrap();
        assert_eq!(menu["function"], "onInputMenuToggle");
        let prompt = serde_json::from_str::<Value>(&capture.systemPromptComposeHooks[0]).unwrap();
        assert_eq!(prompt["function"], "onSystemPromptCompose");
    }

    #[test]
    fn execute_script_can_require_axios_and_uuid() {
        let mut state = JsEngineState::new(None);
        let script = r#"
            exports.inspect_require = function(_params) {
                var axios = require('axios');
                var uuid = require('uuid');
                return typeof axios.get + ":" + typeof axios.post + ":" + uuid.v4().length;
            };
        "#;
        let params = BTreeMap::new();

        let output =
            state.executeScriptFunctionOnCurrentThread(script, "inspect_require", &params, None);

        assert_eq!(output.as_deref(), Some("\"function:function:36\""));
    }

    #[test]
    fn registration_mode_uses_ui_module_placeholder() {
        let engine = super::JsEngine::newToolPkgRegistrationEngine();
        let script = r#"
            var Screen = require('./screens/main.ui.js');
            exports.registerToolPkg = function(_params) {
                ToolPkg.registerUiRoute({
                    id: "main",
                    path: "/main",
                    screen: Screen
                });
                return true;
            };
        "#;
        let mut params = BTreeMap::new();
        params.insert("toolPkgId".to_string(), Value::String("ui_pkg".to_string()));

        let capture = engine
            .executeToolPkgMainRegistrationFunction(script, "registerToolPkg", &params)
            .expect("ui registration");

        assert_eq!(capture.uiRoutes.len(), 1);
        let route = serde_json::from_str::<Value>(&capture.uiRoutes[0]).unwrap();
        assert_eq!(route["screen"], "screens/main.ui.js");
    }

    #[test]
    fn native_interface_reads_env_for_call() {
        ensure_test_runtime_root();
        let key = "OPERIT_JS_NATIVE_ENV_TEST";
        std::env::set_var(key, "enabled");
        EnvPreferences::getInstance()
            .setEnv(key, "enabled")
            .expect("set env");
        let mut state = JsEngineState::new(None);
        let script = r#"
            exports.read_env = function(_params) {
                return getEnv("OPERIT_JS_NATIVE_ENV_TEST");
            };
        "#;
        let params = BTreeMap::new();

        let output = state.executeScriptFunctionOnCurrentThread(script, "read_env", &params, None);

        assert_eq!(output.as_deref(), Some("\"enabled\""));
        EnvPreferences::getInstance()
            .removeEnv(key)
            .expect("remove env");
        std::env::remove_var(key);
    }

    #[test]
    fn native_interface_resolves_plugin_config_dir() {
        ensure_test_runtime_root();
        let mut state = JsEngineState::new(None);
        let script = r#"
            exports.config_dir = function(_params) {
                return getPluginConfigDir('plugin:name');
            };
        "#;
        let params = BTreeMap::new();

        let output =
            state.executeScriptFunctionOnCurrentThread(script, "config_dir", &params, None);
        let path = serde_json::from_str::<String>(&output.expect("config dir"))
            .expect("serialized config dir");

        let normalized = path.replace('\\', "/");
        assert!(normalized.contains("/plugins/plugin_name-"));
        assert!(std::path::Path::new(&path).is_dir());
    }

    #[test]
    fn probe_async_function_declaration_inside_iife() {
        let mut state = JsEngineState::new(None);
        let script = r#"
            const SystemTools = (function () {
                async function get_device_info(_params) {
                    const result = Tools.System.getDeviceInfo();
                    return { success: true, data: result };
                }
                async function wrapToolExecution(func, params) {
                    const result = await func(params);
                    complete(result);
                }
                return {
                    get_device_info: (params) => wrapToolExecution(get_device_info, params),
                };
            })();
            exports.get_device_info = SystemTools.get_device_info;
        "#;
        let params = BTreeMap::new();

        let output =
            state.executeScriptFunctionOnCurrentThread(script, "get_device_info", &params, None);

        assert!(output.is_some());
    }
}

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
fn nativeSendIntermediateResultString(result: String) {
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
            .readToolPkgTextResource(&packageNameOrSubpackageId, &resourcePath)
            .unwrap_or_default()
    })
}

#[allow(non_snake_case)]
fn nativeSetCallResultStrings(callId: String, result: String) {
    CURRENT_CALL_RESULTS.with(|results| {
        results.borrow_mut().insert(callId, result);
    });
}

#[allow(non_snake_case)]
fn nativeSetCallErrorStrings(callId: String, error: String) {
    CURRENT_CALL_RESULTS.with(|results| {
        results.borrow_mut().insert(callId, error);
    });
}

#[allow(non_snake_case)]
fn nativeGetEnvForCallStrings(key: String) -> String {
    let value = EnvPreferences::getInstance()
        .getEnv(&key)
        .ok()
        .flatten()
        .unwrap_or_default();
    value
}

#[allow(non_snake_case)]
fn nativeGetPluginConfigDirString(pluginId: String) -> String {
    let path = pluginConfigDirPath(&pluginId);
    let _ = std::fs::create_dir_all(&path);
    path.to_string_lossy().to_string()
}

#[allow(non_snake_case)]
fn nativeLogJsExecutionTraceStrings(callId: String, message: String) {
    let _ = callId;
    let _ = message;
}

#[cfg(target_arch = "wasm32")]
#[allow(non_snake_case)]
fn wasmQuickJsArgString(args: &[WasmQuickJsValueRef], index: usize) -> String {
    args.get(index)
        .map(|value| value.to_string())
        .unwrap_or_default()
}

#[allow(non_snake_case)]
fn pluginConfigDirPath(pluginId: &str) -> std::path::PathBuf {
    let trimmed = pluginId.trim();
    let safeBaseName = sanitizePluginConfigDirName(trimmed);
    let safeName = if safeBaseName == trimmed {
        safeBaseName
    } else {
        format!("{safeBaseName}-{:x}", javaStringHashCode(trimmed))
    };
    default_data_dir().join("plugins").join(safeName)
}

#[allow(non_snake_case)]
fn sanitizePluginConfigDirName(pluginId: &str) -> String {
    let replaced = pluginId
        .chars()
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch <= '\u{1f}'
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>();
    let trimmed = replaced
        .trim_matches(|ch| ch == '.' || ch == ' ')
        .to_string();
    if trimmed.is_empty() {
        "plugin".to_string()
    } else {
        trimmed
    }
}

#[allow(non_snake_case)]
fn javaStringHashCode(value: &str) -> i32 {
    value.encode_utf16().fold(0_i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(unit as i32)
    })
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

#[allow(non_snake_case)]
fn buildExecutionPreludeSource() -> String {
    r#"
        function __operitGetActiveCallRuntime() {
            var root = typeof globalThis !== 'undefined'
                ? globalThis
                : (typeof window !== 'undefined' ? window : this);
            var runtime =
                root &&
                root.__operit_call_runtime_ref &&
                typeof root.__operit_call_runtime_ref === 'object'
                    ? root.__operit_call_runtime_ref
                    : __operit_call_runtime;
            return runtime && typeof runtime === 'object' ? runtime : __operit_call_runtime;
        }
        function __operitInvokeCallRuntime(methodName, argsLike) {
            var runtime = __operitGetActiveCallRuntime();
            var method = runtime ? runtime[methodName] : undefined;
            if (typeof method !== 'function') {
                return undefined;
            }
            return method.apply(runtime, Array.prototype.slice.call(argsLike || []));
        }
        function __operitInvokeCallRuntimeConsole(methodName, argsLike) {
            var runtime = __operitGetActiveCallRuntime();
            var runtimeConsole = runtime && runtime.console ? runtime.console : null;
            var method = runtimeConsole ? runtimeConsole[methodName] : undefined;
            if (typeof method !== 'function') {
                return undefined;
            }
            return method.apply(runtimeConsole, Array.prototype.slice.call(argsLike || []));
        }
        var sendIntermediateResult = function() { return __operitInvokeCallRuntime('sendIntermediateResult', arguments); };
        var emit = function() { return __operitInvokeCallRuntime('emit', arguments); };
        var delta = function() { return __operitInvokeCallRuntime('delta', arguments); };
        var log = function() { return __operitInvokeCallRuntime('log', arguments); };
        var update = function() { return __operitInvokeCallRuntime('update', arguments); };
        var done = function() { return __operitInvokeCallRuntime('done', arguments); };
        var complete = function() { return __operitInvokeCallRuntime('complete', arguments); };
        var getEnv = function() { return __operitInvokeCallRuntime('getEnv', arguments); };
        var getPluginConfigDir = function() { return __operitInvokeCallRuntime('getPluginConfigDir', arguments); };
        var getState = function() { return __operitInvokeCallRuntime('getState', arguments); };
        var getLang = function() { return __operitInvokeCallRuntime('getLang', arguments); };
        var getCallerName = function() { return __operitInvokeCallRuntime('getCallerName', arguments); };
        var getChatId = function() { return __operitInvokeCallRuntime('getChatId', arguments); };
        var getCallerCardId = function() { return __operitInvokeCallRuntime('getCallerCardId', arguments); };
        var __handleAsync = function() { return __operitInvokeCallRuntime('handleAsync', arguments); };
        var console = {
            log: function() { return __operitInvokeCallRuntimeConsole('log', arguments); },
            info: function() { return __operitInvokeCallRuntimeConsole('info', arguments); },
            warn: function() { return __operitInvokeCallRuntimeConsole('warn', arguments); },
            error: function() { return __operitInvokeCallRuntimeConsole('error', arguments); }
        };
        var reportDetailedError = function() { return __operitInvokeCallRuntime('reportDetailedError', arguments); };
        var ToolPkg = globalThis.ToolPkg;
        var Tools = globalThis.Tools;
        var Java = globalThis.Java;
        var Android = globalThis.Android;
        var Intent = globalThis.Intent;
        var PackageManager = globalThis.PackageManager;
        var ContentProvider = globalThis.ContentProvider;
        var SystemManager = globalThis.SystemManager;
        var DeviceController = globalThis.DeviceController;
        var OperitComposeDslRuntime = globalThis.OperitComposeDslRuntime;
        var CryptoJS = globalThis.CryptoJS;
        var Jimp = globalThis.Jimp;
        var UINode = globalThis.UINode;
        var OkHttpClientBuilder = globalThis.OkHttpClientBuilder;
        var OkHttpClient = globalThis.OkHttpClient;
        var RequestBuilder = globalThis.RequestBuilder;
        var OkHttp = globalThis.OkHttp;
        var pako = globalThis.pako;
        var _ = globalThis._;
        var dataUtils = globalThis.dataUtils;
        var toolCall = globalThis.toolCall;
    "#
    .to_string()
}

#[allow(non_snake_case)]
fn buildRuntimeBootstrapScript() -> String {
    let executionPreludeJson = serde_json::to_string(&buildExecutionPreludeSource())
        .unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"
        {}
        var globalThis = this;
        var window = globalThis;
        var __operitRuntimePrelude = {};
        var console = {{
            log: function() {{ NativeInterface.logInfoForCall('', Array.prototype.slice.call(arguments).join(' ')); }},
            info: function() {{ NativeInterface.logInfoForCall('', Array.prototype.slice.call(arguments).join(' ')); }},
            warn: function() {{ NativeInterface.logInfoForCall('', Array.prototype.slice.call(arguments).join(' ')); }},
            error: function() {{ NativeInterface.logErrorForCall('', Array.prototype.slice.call(arguments).join(' ')); }}
        }};
        var NativeInterface = {{
            callTool: function(toolType, toolName, paramsJson) {{
                return __operitNativeCallTool(String(toolType || 'default'), String(toolName || ''), String(paramsJson || '{{}}'));
            }},
            callToolAsync: function(callbackId, toolType, toolName, paramsJson) {{
                var raw = __operitNativeCallTool(String(toolType || 'default'), String(toolName || ''), String(paramsJson || '{{}}'));
                var parsed;
                try {{
                    parsed = JSON.parse(raw);
                }} catch (_error) {{
                    parsed = {{ success: false, message: String(raw || '') }};
                }}
                if (typeof window[callbackId] === 'function') {{
                    window[callbackId](parsed, !parsed.success);
                }}
            }},
            callToolAsyncStreaming: function(callbackId, intermediateCallbackId, toolType, toolName, paramsJson) {{
                this.callToolAsync(callbackId, toolType, toolName, paramsJson);
            }},
            logInfoForCall: function() {{}},
            logErrorForCall: function() {{}},
            reportErrorForCall: function() {{}},
            sendCallIntermediateResult: function(_callId, result) {{
                __operitSendIntermediateResult(String(result == null ? '' : result));
            }},
            readToolPkgTextResource: function(packageNameOrSubpackageId, resourcePath) {{
                return __operitNativeReadToolPkgTextResource(
                    String(packageNameOrSubpackageId || ''),
                    String(resourcePath || '')
                );
            }},
            getEnvForCall: function(callId, key) {{
                return __operitNativeGetEnvForCall(String(callId || ''), String(key || ''));
            }},
            getPluginConfigDir: function(pluginId) {{
                return __operitNativeGetPluginConfigDir(String(pluginId || ''));
            }},
            logJsExecutionTrace: function(callId, message) {{
                __operitNativeLogJsExecutionTrace(String(callId || ''), String(message || ''));
            }},
            setCallResult: function(callId, result) {{
                __operitNativeSetCallResult(String(callId || ''), String(result == null ? '' : result));
            }},
            setCallError: function(callId, error) {{
                __operitNativeSetCallError(String(callId || ''), String(error == null ? '' : error));
            }}
        }};

        function __operitParseToolResult(result, isError) {{
            if (isError) {{
                if (result && typeof result === 'object' && result.success === false) {{
                    var err = new Error(String(result.message || 'Tool call failed'));
                    err.data = result.data;
                    throw err;
                }}
                throw new Error(typeof result === 'string' ? result : JSON.stringify(result));
            }}
            if (result && typeof result === 'object' && Object.prototype.hasOwnProperty.call(result, 'success')) {{
                if (result.success) {{
                    return result.data;
                }}
                var error = new Error(String(result.message || 'Tool call failed'));
                error.data = result.data;
                throw error;
            }}
            if (typeof result === 'string' && result.length > 1) {{
                var first = result.charAt(0);
                if (first === '{{' || first === '[') {{
                    try {{
                        return __operitParseToolResult(JSON.parse(result), false);
                    }} catch (_error) {{
                        return result;
                    }}
                }}
            }}
            return result;
        }}

        function toolCall() {{
            var type = 'default';
            var name = '';
            var params = {{}};
            if (arguments.length === 1 && typeof arguments[0] === 'object') {{
                type = String(arguments[0].type || 'default');
                name = String(arguments[0].name || '');
                params = arguments[0].params || {{}};
            }} else if (arguments.length === 1) {{
                name = String(arguments[0] || '');
            }} else if (arguments.length === 2) {{
                name = String(arguments[0] || '');
                params = arguments[1] || {{}};
            }} else {{
                type = String(arguments[0] || 'default');
                name = String(arguments[1] || '');
                params = arguments[2] || {{}};
            }}
            var raw = NativeInterface.callTool(type, name, JSON.stringify(params));
            var parsed;
            try {{
                parsed = JSON.parse(raw);
            }} catch (_parseError) {{
                parsed = raw;
            }}
            return __operitParseToolResult(parsed, false);
        }}

        globalThis.__operitCompleteCalled = false;
        globalThis.__operitCompleteValue = undefined;
        function complete(value) {{
            globalThis.__operitCompleteCalled = true;
            globalThis.__operitCompleteValue = value;
        }}

        function sendIntermediateResult(value) {{
            __operitSendIntermediateResult(__operitFinishExecutionResult(value));
        }}
        var emit = sendIntermediateResult;
        var delta = sendIntermediateResult;
        var log = sendIntermediateResult;
        var update = sendIntermediateResult;

        function __operitFinishExecutionResult(result) {{
            if (result && result.__operit_error) {{
                return JSON.stringify({{
                    success: false,
                    message: String(result.message || ''),
                    data: result.data
                }});
            }}
            if (result !== null && typeof result === 'object') {{
                return JSON.stringify(result);
            }}
            return result === undefined ? "undefined" : String(result);
        }}

        function __operitHasUsableJavaInstanceMarker(value) {{
            if (!value || typeof value !== 'object') {{
                return false;
            }}
            try {{
                return (
                    Object.prototype.hasOwnProperty.call(value, '__javaHandle') &&
                    Object.prototype.hasOwnProperty.call(value, '__javaClass') &&
                    typeof value.__javaHandle === 'string' &&
                    typeof value.__javaClass === 'string' &&
                    __operitText(value.__javaHandle).trim().length > 0 &&
                    __operitText(value.__javaClass).trim().length > 0
                );
            }} catch (_javaMarkerError) {{
                return false;
            }}
        }}

        function __operitNormalizeSerializableValue(value, seen) {{
            if (value == null || typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {{
                return value;
            }}
            if (typeof value === 'bigint' || typeof value === 'function') {{
                return String(value);
            }}
            if (typeof value !== 'object') {{
                return String(value);
            }}
            seen = seen || [];
            if (seen.indexOf(value) >= 0) {{
                return '[Circular]';
            }}
            seen.push(value);
            try {{
                if (typeof value.toJSON === 'function') {{
                    return __operitNormalizeSerializableValue(value.toJSON(), seen);
                }}
                if (Array.isArray(value)) {{
                    return value.map(function(item) {{
                        return __operitNormalizeSerializableValue(item, seen);
                    }});
                }}
                if (__operitHasUsableJavaInstanceMarker(value)) {{
                    return {{
                        __javaHandle: __operitText(value.__javaHandle),
                        __javaClass: __operitText(value.__javaClass)
                    }};
                }}
                var out = {{}};
                Object.keys(value).forEach(function(key) {{
                    out[key] = __operitNormalizeSerializableValue(value[key], seen);
                }});
                return out;
            }} finally {{
                seen.pop();
            }}
        }}

        function __operitSerializeOrThrow(value) {{
            return JSON.stringify(__operitNormalizeSerializableValue(value, []));
        }}

        function __operitSafeSerialize(value) {{
            try {{
                return __operitSerializeOrThrow(value);
            }} catch (error) {{
                return JSON.stringify({{
                    error: 'Failed to serialize value',
                    message: __operitText(error && error.message ? error.message : error),
                    value: __operitText(value).slice(0, 1000)
                }});
            }}
        }}

        function __operitNormalizeComposeResult(value) {{
            if (!value || typeof value !== 'object' || !value.composeDsl || typeof value.composeDsl !== 'object') {{
                return value;
            }}
            if (!Object.prototype.hasOwnProperty.call(value.composeDsl, 'screen')) {{
                return value;
            }}
            var screenRef = value.composeDsl.screen;
            var resolved = '';
            if (typeof screenRef === 'function') {{
                resolved = __operitText(screenRef.__operit_toolpkg_module_path).trim();
            }} else if (
                screenRef &&
                typeof screenRef === 'object' &&
                typeof screenRef.default === 'function'
            ) {{
                resolved = __operitText(screenRef.default.__operit_toolpkg_module_path).trim();
            }} else if (typeof screenRef === 'string') {{
                throw new Error('composeDsl.screen must be a compose_dsl screen function, not a string path');
            }}
            if (!resolved) {{
                throw new Error('composeDsl.screen is missing a toolpkg module path marker');
            }}
            value.composeDsl.screen = resolved.replace(/\\/g, '/');
            return value;
        }}

        function __operitGetFactoryCache() {{
            if (!globalThis.__operitFactoryCache || typeof globalThis.__operitFactoryCache !== 'object') {{
                globalThis.__operitFactoryCache = {{}};
            }}
            return globalThis.__operitFactoryCache;
        }}

        function __operitGetModuleInstanceCache() {{
            if (!globalThis.__operitModuleInstanceCache || typeof globalThis.__operitModuleInstanceCache !== 'object') {{
                globalThis.__operitModuleInstanceCache = {{}};
            }}
            return globalThis.__operitModuleInstanceCache;
        }}

        function __operitNormalizePath(pathValue) {{
            var parts = String(pathValue == null ? '' : pathValue).replace(/\\/g, '/').split('/');
            var stack = [];
            for (var i = 0; i < parts.length; i += 1) {{
                var part = parts[i];
                if (!part || part === '.') {{
                    continue;
                }}
                if (part === '..') {{
                    if (stack.length > 0) {{
                        stack.pop();
                    }}
                    continue;
                }}
                stack.push(part);
            }}
            return stack.join('/');
        }}

        function __operitDirname(pathValue) {{
            var normalized = __operitNormalizePath(pathValue);
            var index = normalized.lastIndexOf('/');
            return index < 0 ? '' : normalized.slice(0, index);
        }}

        function __operitResolveModulePath(request, fromPath) {{
            var normalized = String(request == null ? '' : request).replace(/\\/g, '/').trim();
            if (!normalized) {{
                return '';
            }}
            if (!(normalized.startsWith('.') || normalized.startsWith('/'))) {{
                return normalized;
            }}
            if (normalized.startsWith('/')) {{
                return __operitNormalizePath(normalized);
            }}
            var base = __operitDirname(fromPath);
            return __operitNormalizePath(base ? base + '/' + normalized : normalized);
        }}

        function __operitBuildCandidatePaths(modulePath) {{
            var normalized = __operitNormalizePath(modulePath);
            if (!normalized) {{
                return [];
            }}
            if (/\.[a-z0-9]+$/i.test(normalized)) {{
                return [normalized];
            }}
            return [
                normalized,
                normalized + '.js',
                normalized + '.json',
                normalized + '/index.js',
                normalized + '/index.json'
            ];
        }}

        function __operitHashText(value) {{
            var textValue = String(value == null ? '' : value);
            var hash = 0;
            for (var i = 0; i < textValue.length; i += 1) {{
                hash = (((hash << 5) - hash) + textValue.charCodeAt(i)) | 0;
            }}
            return (hash >>> 0).toString(16);
        }}

        function __operitBuildFactoryKey(kind, identity, source) {{
            return [String(kind || ''), String(identity || ''), String(source || '').length, __operitHashText(source)].join(':');
        }}

        function __operitGetFactory(kind, identity, source) {{
            var key = __operitBuildFactoryKey(kind, identity, source);
            var cache = __operitGetFactoryCache();
            if (typeof cache[key] === 'function') {{
                return cache[key];
            }}
            var factory = new Function(
                'module',
                'exports',
                'require',
                '__operit_call_runtime',
                __operitRuntimePrelude + '\n' + source
            );
            cache[key] = factory;
            return factory;
        }}

        function __operitTagModuleExports(modulePath, exportsRef) {{
            if (typeof exportsRef === 'function') {{
                exportsRef.__operit_toolpkg_module_path = modulePath;
                return;
            }}
            if (!exportsRef || typeof exportsRef !== 'object') {{
                return;
            }}
            exportsRef.__operit_toolpkg_module_path = modulePath;
            Object.keys(exportsRef).forEach(function(key) {{
                if (typeof exportsRef[key] === 'function') {{
                    exportsRef[key].__operit_toolpkg_module_path = modulePath;
                    exportsRef[key].__operit_toolpkg_export_name = key;
                }}
            }});
        }}

        function __operitText(value) {{
            return value == null ? '' : String(value);
        }}

        function __operitToBoolean(value) {{
            if (typeof value === 'boolean') {{
                return value;
            }}
            var normalized = __operitText(value).trim().toLowerCase();
            return normalized === 'true' || normalized === '1' || normalized === 'yes' || normalized === 'on';
        }}

        function __operitCreateRegistrationScreenPlaceholder(modulePath) {{
            function ScreenPlaceholder() {{
                return null;
            }}
            ScreenPlaceholder.__operit_toolpkg_module_path = modulePath;
            return ScreenPlaceholder;
        }}

        function __operitIsLocalUiModulePath(modulePath) {{
            var normalized = __operitNormalizePath(modulePath);
            return /\.ui\.js$/i.test(normalized);
        }}

        function __operitFindTargetFunction(exportsRef, moduleRef, functionName) {{
            if (exportsRef && typeof exportsRef[functionName] === 'function') {{
                return exportsRef[functionName];
            }}
            if (moduleRef && moduleRef.exports && typeof moduleRef.exports[functionName] === 'function') {{
                return moduleRef.exports[functionName];
            }}
            if (typeof globalThis[functionName] === 'function') {{
                return globalThis[functionName];
            }}
            return null;
        }}

        function __operitBuildAvailableFunctions(exportsRef, moduleRef) {{
            var names = [];
            function collect(target) {{
                if (!target || typeof target !== 'object') {{
                    return;
                }}
                Object.keys(target).forEach(function(key) {{
                    if (typeof target[key] === 'function' && names.indexOf(key) < 0) {{
                        names.push(key);
                    }}
                }});
            }}
            collect(exportsRef);
            collect(moduleRef && moduleRef.exports ? moduleRef.exports : null);
            return names;
        }}

        function __operitExecuteScriptFunction(callId, params, scriptText, targetFunctionName, timeoutSec, preTimeoutMs) {{
            var previousCallRuntime = globalThis.__operit_call_runtime_ref;
            var previousCallId = globalThis.__operitCurrentCallId;
            var registerCallSession = globalThis.__operitRegisterCallSession;
            if (typeof registerCallSession !== 'function') {{
                NativeInterface.setCallError(callId, JSON.stringify({{
                    success: false,
                    message: 'JS execution runtime bridge is unavailable'
                }}));
                return;
            }}
            var callState = registerCallSession(callId, params);
            globalThis.__operitCurrentCallId = callId;
            function getCallState() {{
                return typeof globalThis.__operitGetCallState === 'function'
                    ? globalThis.__operitGetCallState(callId)
                    : null;
            }}
            function finalizeCall() {{
                if (globalThis.__operitCurrentCallId === callId) {{
                    globalThis.__operitCurrentCallId =
                        typeof previousCallId === 'string' ? previousCallId : '';
                }}
                if (globalThis.__operit_call_runtime_ref === callRuntime) {{
                    if (previousCallRuntime && typeof previousCallRuntime === 'object') {{
                        globalThis.__operit_call_runtime_ref = previousCallRuntime;
                    }} else {{
                        delete globalThis.__operit_call_runtime_ref;
                    }}
                }}
                if (typeof globalThis.__operitCleanupCallSession === 'function') {{
                    globalThis.__operitCleanupCallSession(callId);
                }}
            }}
            function isActive() {{
                var state = getCallState();
                return !!(state && !state.completed);
            }}
            function readCallValue(key, fallbackValue) {{
                var state = getCallState();
                var currentParams = state && state.params && typeof state.params === 'object'
                    ? state.params
                    : null;
                var value = currentParams ? currentParams[key] : undefined;
                return value == null || value === '' ? fallbackValue : __operitText(value);
            }}
            function markStage(stage) {{
                callState.lastExecStage = __operitText(stage);
                NativeInterface.logJsExecutionTrace(
                    callId,
                    'stage=' + callState.lastExecStage +
                        ' function=' + __operitText(callState.lastExecFunction) +
                        ' module=' + __operitText(callState.lastModulePath) +
                        ' require=' + __operitText(callState.lastRequireRequest) +
                        ' from=' + __operitText(callState.lastRequireFrom) +
                        ' resolved=' + __operitText(callState.lastRequireResolved)
                );
            }}
            function markFunction(name) {{
                callState.lastExecFunction = __operitText(name);
                NativeInterface.logJsExecutionTrace(
                    callId,
                    'function=' + callState.lastExecFunction +
                        ' package=' + __operitText(params && (params.__operit_ui_package_name || params.toolPkgId || params.__operit_package_name)) +
                        ' screen=' + __operitText(params && params.__operit_script_screen) +
                        ' context=' + __operitText(params && params.__operit_execution_context_key)
                );
            }}
            function markRequire(request, fromPath, resolvedPath) {{
                callState.lastRequireRequest = __operitText(request);
                callState.lastRequireFrom = __operitText(fromPath);
                callState.lastRequireResolved = __operitText(resolvedPath);
                NativeInterface.logJsExecutionTrace(
                    callId,
                    'require=' + callState.lastRequireRequest +
                        ' from=' + callState.lastRequireFrom +
                        ' resolved=' + callState.lastRequireResolved
                );
            }}
            function markModule(modulePath) {{
                callState.lastModulePath = __operitText(modulePath);
                NativeInterface.logJsExecutionTrace(callId, 'module=' + callState.lastModulePath);
            }}
            function completeCall(resultText) {{
                var state = getCallState();
                if (!state || state.completed) {{
                    return;
                }}
                state.completed = true;
                NativeInterface.logJsExecutionTrace(callId, 'complete ' + __operitText(resultText).slice(0, 240));
                NativeInterface.setCallResult(callId, resultText);
                finalizeCall();
            }}
            function emitError(message) {{
                var state = getCallState();
                if (!state || state.completed) {{
                    return;
                }}
                state.completed = true;
                NativeInterface.logJsExecutionTrace(callId, 'error ' + __operitText(message).slice(0, 240));
                NativeInterface.setCallError(callId, JSON.stringify({{
                    success: false,
                    message: __operitText(message)
                }}));
                finalizeCall();
            }}
            function callRuntimeReport(error, context) {{
                if (typeof globalThis.__operitReportDetailedErrorForCall === 'function') {{
                    return globalThis.__operitReportDetailedErrorForCall(callId, error, context);
                }}
                return {{
                    formatted: __operitText(context) + ': ' + __operitText(error),
                    details: {{
                        message: __operitText(error && error.message ? error.message : error),
                        stack: __operitText(error && error.stack ? error.stack : error),
                        lineNumber: 0
                    }}
                }};
            }}
            function emitIntermediate(value) {{
                if (isActive()) {{
                    NativeInterface.sendCallIntermediateResult(callId, __operitSafeSerialize(value));
                }}
            }}
            function complete(value) {{
                try {{
                    completeCall(__operitSerializeOrThrow(__operitNormalizeComposeResult(value)));
                }} catch (error) {{
                    var report = callRuntimeReport(error, 'Result Serialization Failure');
                    var serializationMessage =
                        report &&
                        report.details &&
                        typeof report.details.message === 'string' &&
                        report.details.message
                            ? report.details.message
                            : __operitText(error && error.message ? error.message : error);
                    emitError('Result serialization failed: ' + serializationMessage);
                }}
            }}
            var callRuntime = {{
                callId: callId,
                emit: emitIntermediate,
                delta: emitIntermediate,
                log: emitIntermediate,
                update: emitIntermediate,
                sendIntermediateResult: emitIntermediate,
                done: complete,
                complete: complete,
                getState: function() {{ return readCallValue('__operit_package_state', undefined); }},
                getLang: function() {{ return readCallValue('__operit_package_lang', 'en'); }},
                getCallerName: function() {{ return readCallValue('__operit_package_caller_name', undefined); }},
                getChatId: function() {{ return readCallValue('__operit_package_chat_id', undefined); }},
                getCallerCardId: function() {{ return readCallValue('__operit_package_caller_card_id', undefined); }},
                getEnv: function(key) {{
                    var value = NativeInterface.getEnvForCall(callId, __operitText(key).trim());
                    return value == null || value === '' ? undefined : __operitText(value);
                }},
                getPluginConfigDir: function(pluginId) {{
                    var explicitId = pluginId == null ? '' : __operitText(pluginId).trim();
                    var resolvedId =
                        explicitId ||
                        readCallValue('__operit_ui_package_name', '') ||
                        readCallValue('toolPkgId', '') ||
                        readCallValue('containerPackageName', '') ||
                        readCallValue('__operit_package_name', '');
                    if (
                        !resolvedId ||
                        typeof NativeInterface === 'undefined' ||
                        !NativeInterface ||
                        typeof NativeInterface.getPluginConfigDir !== 'function'
                    ) {{
                        return '';
                    }}
                    var path = NativeInterface.getPluginConfigDir(resolvedId);
                    return typeof path === 'string' ? path : '';
                }},
                reportDetailedError: callRuntimeReport,
                handleAsync: function(value) {{
                    if (!value || typeof value.then !== 'function') {{
                        return false;
                    }}
                    Promise.resolve(value).then(
                        function(result) {{
                            if (isActive()) {{
                                complete(result);
                            }}
                        }},
                        function(error) {{
                            if (isActive()) {{
                                var report = callRuntimeReport(error, 'Async Promise Rejection');
                                var rejectionMessage =
                                    report &&
                                    report.details &&
                                    typeof report.details.message === 'string' &&
                                    report.details.message
                                        ? report.details.message
                                        : __operitText(error && error.message ? error.message : error);
                                emitError(rejectionMessage || 'Promise rejection');
                            }}
                        }}
                    );
                    return true;
                }},
                console: console
            }};
            globalThis.__operit_call_runtime_ref = callRuntime;
            try {{
                var registrationMode = __operitToBoolean(readCallValue('__operit_registration_mode', false));
                var packageTarget =
                    readCallValue('__operit_ui_package_name', '') ||
                    readCallValue('toolPkgId', '');
                var screenPath = __operitNormalizePath(readCallValue(
                    '__operit_script_screen',
                    params && params.moduleSpec && params.moduleSpec.screen
                        ? __operitText(params.moduleSpec.screen)
                        : ''
                ));
                var moduleCache = registrationMode ? {{}} : __operitGetModuleInstanceCache();
                var mainModuleKey = ['instance', 'main', packageTarget + ':' + screenPath, String(scriptText || '').length, __operitHashText(scriptText)].join(':');
                var module = moduleCache[mainModuleKey];
                var exports = module && module.exports ? module.exports : null;
                function readToolPkgModule(modulePath) {{
                    if (!packageTarget || !NativeInterface || typeof NativeInterface.readToolPkgTextResource !== 'function') {{
                        return null;
                    }}
                    var candidates = __operitBuildCandidatePaths(modulePath);
                    for (var i = 0; i < candidates.length; i += 1) {{
                        var candidate = candidates[i];
                        var textResult = NativeInterface.readToolPkgTextResource(packageTarget, candidate);
                        if (typeof textResult === 'string' && textResult.length > 0) {{
                            return {{ path: candidate, text: textResult }};
                        }}
                    }}
                    return null;
                }}
                function executeModule(modulePath, moduleText, requireInternal) {{
                    var moduleKey = ['instance', 'module', packageTarget + ':' + modulePath, String(moduleText || '').length, __operitHashText(moduleText)].join(':');
                    if (moduleCache[moduleKey]) {{
                        return moduleCache[moduleKey].exports;
                    }}
                    var requiredModule = {{ exports: {{}} }};
                    moduleCache[moduleKey] = requiredModule;
                    if (/\.json$/i.test(modulePath)) {{
                        try {{
                            requiredModule.exports = JSON.parse(moduleText);
                            return requiredModule.exports;
                        }} catch (error) {{
                            delete moduleCache[moduleKey];
                            throw error;
                        }}
                    }}
                    var localRequire = function(nextName) {{
                        return requireInternal(nextName, modulePath);
                    }};
                    var factory = __operitGetFactory('module', packageTarget + ':' + modulePath, moduleText);
                    var previousActiveModule = globalThis.__operitActiveModule;
                    var previousActiveExports = globalThis.__operitActiveModuleExports;
                    var previousModule = callState.currentModule;
                    var previousExports = callState.currentModuleExports;
                    globalThis.__operitActiveModule = requiredModule;
                    globalThis.__operitActiveModuleExports = requiredModule.exports;
                    callState.currentModule = requiredModule;
                    callState.currentModuleExports = requiredModule.exports;
                    try {{
                        factory(requiredModule, requiredModule.exports, localRequire, callRuntime);
                    }} catch (error) {{
                        delete moduleCache[moduleKey];
                        throw error;
                    }} finally {{
                        callState.currentModule = previousModule;
                        callState.currentModuleExports = previousExports;
                        globalThis.__operitActiveModule = previousActiveModule;
                        globalThis.__operitActiveModuleExports = previousActiveExports;
                    }}
                    __operitTagModuleExports(modulePath, requiredModule.exports);
                    return requiredModule.exports;
                }}
                function requireInternal(moduleName, fromPath) {{
                    var request = String(moduleName == null ? '' : moduleName).trim();
                    if (request === 'lodash') {{
                        return globalThis._;
                    }}
                    if (request === 'uuid') {{
                        return {{
                            v4: function() {{
                                return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(char) {{
                                    var random = Math.random() * 16 | 0;
                                    var value = char === 'x' ? random : ((random & 0x3) | 0x8);
                                    return value.toString(16);
                                }});
                            }}
                        }};
                    }}
                    if (request === 'axios') {{
                        return {{
                            get: function(url, config) {{
                                return toolCall('http_request', config ? Object.assign({{ url: url }}, config) : {{ url: url }});
                            }},
                            post: function(url, data, config) {{
                                return toolCall('http_request', config ? Object.assign({{ url: url, data: data }}, config) : {{ url: url, data: data }});
                            }}
                        }};
                    }}
                    if (!(request.startsWith('.') || request.startsWith('/'))) {{
                        return {{}};
                    }}
                    var resolvedPath = __operitResolveModulePath(request, fromPath || screenPath);
                    markStage('require_module');
                    markRequire(request, fromPath || screenPath || '<root>', resolvedPath);
                    markModule(resolvedPath);
                    if (registrationMode && __operitIsLocalUiModulePath(resolvedPath)) {{
                        return __operitCreateRegistrationScreenPlaceholder(resolvedPath);
                    }}
                    var loaded = readToolPkgModule(resolvedPath);
                    if (!loaded) {{
                        throw new Error('Cannot resolve module "' + request + '" from "' + (fromPath || screenPath || '<root>') + '"');
                    }}
                    return executeModule(loaded.path, loaded.text, requireInternal);
                }}
                var require = function(moduleName) {{
                    markStage('require_request');
                    markRequire(moduleName, screenPath || '<root>', '');
                    return requireInternal(moduleName, screenPath);
                }};
                markFunction(targetFunctionName);
                if (!module) {{
                    module = {{ exports: {{}} }};
                    moduleCache[mainModuleKey] = module;
                    exports = module.exports;
                    markStage('compile_main_script');
                    var mainFactory = __operitGetFactory('main', packageTarget + ':' + screenPath, scriptText);
                    markStage('execute_main_script');
                    var previousActiveModule = globalThis.__operitActiveModule;
                    var previousActiveExports = globalThis.__operitActiveModuleExports;
                    var previousModule = callState.currentModule;
                    var previousExports = callState.currentModuleExports;
                    globalThis.__operitActiveModule = module;
                    globalThis.__operitActiveModuleExports = exports;
                    callState.currentModule = module;
                    callState.currentModuleExports = exports;
                    try {{
                        mainFactory(module, exports, require, callRuntime);
                    }} catch (error) {{
                        delete moduleCache[mainModuleKey];
                        throw error;
                    }} finally {{
                        callState.currentModule = previousModule;
                        callState.currentModuleExports = previousExports;
                        globalThis.__operitActiveModule = previousActiveModule;
                        globalThis.__operitActiveModuleExports = previousActiveExports;
                    }}
                }} else {{
                    if (exports == null) {{
                        exports = {{}};
                        module.exports = exports;
                    }}
                    markStage('reuse_main_script');
                }}
                var rootExports = module.exports || exports || {{}};
                __operitTagModuleExports(screenPath || '<root>', rootExports);

                var inlineFunctionName = readCallValue('__operit_inline_function_name', '');
                var inlineFunctionSource = readCallValue('__operit_inline_function_source', '');
                if (inlineFunctionName && inlineFunctionSource) {{
                    markStage('evaluate_inline_hook_function');
                    var inlineFunction = eval('(' + inlineFunctionSource + ')');
                    if (typeof inlineFunction !== 'function') {{
                        throw new Error('inline hook source did not evaluate to function');
                    }}
                    rootExports[inlineFunctionName] = inlineFunction;
                    module.exports[inlineFunctionName] = inlineFunction;
                }}

                var targetFunction = __operitFindTargetFunction(rootExports, module, targetFunctionName);
                if (typeof targetFunction !== 'function') {{
                    emitError(
                        "Function '" +
                            targetFunctionName +
                            "' not found in script. Available functions: " +
                            __operitBuildAvailableFunctions(rootExports, module).join(', ')
                    );
                    return;
                }}
                markStage('invoke_target_function');
                var invokePreviousActiveModule = globalThis.__operitActiveModule;
                var invokePreviousActiveExports = globalThis.__operitActiveModuleExports;
                var previousModule = callState.currentModule;
                var previousExports = callState.currentModuleExports;
                globalThis.__operitActiveModule = module;
                globalThis.__operitActiveModuleExports = rootExports;
                callState.currentModule = module;
                callState.currentModuleExports = rootExports;
                var functionResult;
                try {{
                    functionResult = targetFunction(params);
                }} finally {{
                    callState.currentModule = previousModule;
                    callState.currentModuleExports = previousExports;
                    globalThis.__operitActiveModule = invokePreviousActiveModule;
                    globalThis.__operitActiveModuleExports = invokePreviousActiveExports;
                }}
                markStage('handle_function_result');
                if (!callRuntime.handleAsync(functionResult)) {{
                    complete(functionResult);
                }}
            }} catch (error) {{
                var runtimeContext = typeof globalThis.__operitBuildRuntimeContext === 'function'
                    ? __operitText(globalThis.__operitBuildRuntimeContext(callId))
                    : '';
                emitError(
                    'Script error: ' +
                        __operitText(error && error.message ? error.message : error) +
                        (runtimeContext ? '\nRuntime Context: ' + runtimeContext : '') +
                        (error && error.stack ? '\nStack: ' + __operitText(error.stack) : '')
                );
            }}
        }}

        {}
        {}
        "#,
        JsInitRuntimeScriptBuilder::buildRuntimeBootstrapScript(),
        executionPreludeJson,
        getJsToolsDefinition(),
        JsExecutionScriptBuilder::buildExecutionRuntimeBridgeScript()
    )
}
