use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{mpsc, Arc};

use boa_engine::native_function::NativeFunction;
use boa_engine::{js_string, Context, JsResult, JsValue, Source};
use serde_json::Value;
use uuid::Uuid;

use crate::core::tools::javascript::JsExecutionScriptBuilder;
use crate::core::tools::javascript::JsExecutionResultProtocol::buildJsExecutionErrorPayload;
use crate::core::tools::javascript::JsInitRuntimeScriptBuilder;
use crate::core::tools::javascript::JsNativeInterfaceDelegates;
use crate::core::tools::javascript::JsToolPkgRegistration::{
    buildToolPkgRegistrationBridgeScript, ToolPkgMainRegistrationCapture,
};
use crate::core::tools::javascript::JsTools::getJsToolsDefinition;
use crate::core::tools::AIToolHandler::AIToolHandler;

thread_local! {
    static CURRENT_TOOL_HANDLER: RefCell<Option<AIToolHandler>> = RefCell::new(None);
    static CURRENT_INTERMEDIATE_CALLBACK: RefCell<Option<Arc<dyn Fn(String) + Send + Sync>>> = RefCell::new(None);
}

#[derive(Clone)]
pub struct JsEngine {
    worker: Arc<JsEngineWorker>,
}

struct JsEngineWorker {
    sender: mpsc::Sender<JsEngineRequest>,
}

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
    context: Context,
    toolHandler: Option<AIToolHandler>,
    jsEnvironmentInitialized: bool,
}

impl JsEngine {
    pub fn new(toolHandler: AIToolHandler) -> Self {
        Self {
            worker: Arc::new(JsEngineWorker::new(Some(toolHandler))),
        }
    }

    #[allow(non_snake_case)]
    pub fn newToolPkgRegistrationEngine() -> Self {
        Self {
            worker: Arc::new(JsEngineWorker::new(None)),
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
        let (response, receiver) = mpsc::channel();
        let request = JsEngineRequest::ExecuteScript {
            script: script.to_string(),
            functionName: functionName.to_string(),
            params: params.clone(),
            onIntermediateResult,
            response,
        };
        if let Err(error) = self.worker.sender.send(request) {
            return Some(buildJsExecutionErrorPayload(&error.to_string()));
        }
        receiver
            .recv()
            .unwrap_or_else(|error| Some(buildJsExecutionErrorPayload(&error.to_string())))
    }

    #[allow(non_snake_case)]
    pub fn executeToolPkgMainRegistrationFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
    ) -> Result<ToolPkgMainRegistrationCapture, String> {
        let (response, receiver) = mpsc::channel();
        let request = JsEngineRequest::ExecuteToolPkgMainRegistration {
            script: script.to_string(),
            functionName: functionName.to_string(),
            params: params.clone(),
            response,
        };
        if let Err(error) = self.worker.sender.send(request) {
            return Err(error.to_string());
        }
        receiver.recv().map_err(|error| error.to_string())?
    }
}

impl JsEngineWorker {
    fn new(toolHandler: Option<AIToolHandler>) -> Self {
        let (sender, receiver) = mpsc::channel::<JsEngineRequest>();
        std::thread::Builder::new()
            .name("OperitBoaJsEngine".to_string())
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
                            let _ = response.send(output);
                        }
                        JsEngineRequest::ExecuteToolPkgMainRegistration {
                            script,
                            functionName,
                            params,
                            response,
                        } => {
                            let output = state.executeToolPkgMainRegistrationFunctionOnCurrentThread(
                                &script,
                                &functionName,
                                &params,
                            );
                            let _ = response.send(output);
                        }
                    }
                }
            })
            .expect("OperitBoaJsEngine worker thread must start");
        Self { sender }
    }
}

impl JsEngineState {
    fn new(toolHandler: Option<AIToolHandler>) -> Self {
        let mut state = Self {
            context: Context::default(),
            toolHandler,
            jsEnvironmentInitialized: false,
        };
        state
            .registerNativeInterface()
            .expect("NativeInterface bridge must register");
        state
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
                self.clearThreadLocalCallState();
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

        let executionScript =
            JsExecutionScriptBuilder::buildExecutionScript(&scriptJson, &functionNameJson, &paramsJson, &callIdJson);
        let output = match self
            .context
            .eval(Source::from_bytes(executionScript.as_bytes()))
        {
            Ok(value) => match value.to_string(&mut self.context) {
                Ok(value) => {
                    let value = value.to_std_string_escaped();
                    if value == format!("__operit_pending:{callId}") {
                        self.context.run_jobs();
                        Some(readPendingExecutionSession(&mut self.context, &callId))
                    } else {
                        Some(value)
                    }
                }
                Err(error) => Some(buildJsExecutionErrorPayload(&error.to_string())),
            },
            Err(error) => Some(buildJsExecutionErrorPayload(&error.to_string())),
        };
        self.clearThreadLocalCallState();
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
        self.context
            .eval(Source::from_bytes(bridge.as_bytes()))
            .map_err(|error| error.to_string())?;

        let paramsJson = serde_json::to_string(params).map_err(|error| error.to_string())?;
        let scriptJson = serde_json::to_string(script).map_err(|error| error.to_string())?;
        let functionNameJson =
            serde_json::to_string(functionName).map_err(|error| error.to_string())?;
        let callId = format!(
            "operit_registration_{}",
            Uuid::new_v4().to_string().replace('-', "")
        );
        let callIdJson = serde_json::to_string(&callId).map_err(|error| error.to_string())?;
        let executionScript =
            JsExecutionScriptBuilder::buildExecutionScript(&scriptJson, &functionNameJson, &paramsJson, &callIdJson);
        let output = self
            .context
            .eval(Source::from_bytes(executionScript.as_bytes()))
            .map_err(|error| error.to_string())?
            .to_string(&mut self.context)
            .map_err(|error| error.to_string())?
            .to_std_string_escaped();
        if output == format!("__operit_pending:{callId}") {
            self.context.run_jobs();
            let pending_output = readPendingExecutionSession(&mut self.context, &callId);
            ensureRegistrationExecutionSucceeded(&pending_output)?;
        } else {
            ensureRegistrationExecutionSucceeded(&output)?;
        }

        let captureScript = r#"
        (function() {
            return JSON.stringify(globalThis.__operitToolPkgRegistrationCapture);
        })()
        "#;
        let captureJson = self
            .context
            .eval(Source::from_bytes(captureScript.as_bytes()))
            .map_err(|error| error.to_string())?
            .to_string(&mut self.context)
            .map_err(|error| error.to_string())?
            .to_std_string_escaped();
        serde_json::from_str::<ToolPkgMainRegistrationCapture>(&captureJson)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn registerNativeInterface(&mut self) -> Result<(), String> {
        self.context
            .register_global_callable(
                js_string!("__operitNativeCallTool"),
                3,
                NativeFunction::from_copy_closure(nativeCallTool),
            )
            .map_err(|error| error.to_string())?;
        self.context
            .register_global_callable(
                js_string!("__operitSendIntermediateResult"),
                1,
                NativeFunction::from_copy_closure(nativeSendIntermediateResult),
            )
            .map_err(|error| error.to_string())?;
        self.context
            .register_global_callable(
                js_string!("__operitNativeReadToolPkgTextResource"),
                2,
                NativeFunction::from_copy_closure(nativeReadToolPkgTextResource),
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
        self.context
            .eval(Source::from_bytes(bootstrap.as_bytes()))
            .map_err(|error| error.to_string())?;
        self.jsEnvironmentInitialized = true;
        Ok(())
    }

    #[allow(non_snake_case)]
    fn clearThreadLocalCallState(&self) {
        CURRENT_TOOL_HANDLER.with(|handler| {
            *handler.borrow_mut() = None;
        });
        CURRENT_INTERMEDIATE_CALLBACK.with(|callback| {
            *callback.borrow_mut() = None;
        });
    }
}

impl JsEngine {
    pub fn destroy(&self) {}
}
fn nativeCallTool(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let toolType = jsValueToString(args.get(0), context);
    let toolName = jsValueToString(args.get(1), context);
    let paramsJson = jsValueToString(args.get(2), context);
    let result = CURRENT_TOOL_HANDLER.with(|handler| {
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
    });
    Ok(JsValue::new(js_string!(result)))
}
fn nativeSendIntermediateResult(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let result = jsValueToString(args.get(0), context);
    CURRENT_INTERMEDIATE_CALLBACK.with(|callback| {
        if let Some(callback) = callback.borrow().as_ref() {
            callback(result);
        }
    });
    Ok(JsValue::undefined())
}

#[allow(non_snake_case)]
fn nativeReadToolPkgTextResource(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let packageNameOrSubpackageId = jsValueToString(args.get(0), context);
    let resourcePath = jsValueToString(args.get(1), context);
    let text = CURRENT_TOOL_HANDLER.with(|handler| {
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
    });
    Ok(JsValue::new(js_string!(text)))
}
fn jsValueToString(value: Option<&JsValue>, context: &mut Context) -> String {
    value
        .cloned()
        .unwrap_or_default()
        .to_string(context)
        .map(|value| value.to_std_string_escaped())
        .unwrap_or_default()
}

#[allow(non_snake_case)]
fn readPendingExecutionSession(context: &mut Context, callId: &str) -> String {
    let callIdJson = serde_json::to_string(callId).unwrap_or_else(|_| "\"\"".to_string());
    let readScript = format!(
        r#"
        (function() {{
            var session = globalThis.__operitExecutionSessions && globalThis.__operitExecutionSessions[{callIdJson}];
            if (!session || !session.completed) {{
                return JSON.stringify({{
                    success: false,
                    message: "Asynchronous JavaScript result did not complete"
                }});
            }}
            return session.output;
        }})()
        "#
    );
    match context.eval(Source::from_bytes(readScript.as_bytes())) {
        Ok(value) => value
            .to_string(context)
            .map(|value| value.to_std_string_escaped())
            .unwrap_or_else(|error| buildJsExecutionErrorPayload(&error.to_string())),
        Err(error) => buildJsExecutionErrorPayload(&error.to_string()),
    }
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
fn buildRuntimeBootstrapScript() -> String {
    format!(
        r#"
        {}
        var globalThis = this;
        var window = globalThis;
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

        {}
        {}
        "#,
        JsInitRuntimeScriptBuilder::buildRuntimeBootstrapScript(),
        getJsToolsDefinition(),
        JsExecutionScriptBuilder::buildExecutionRuntimeBridgeScript()
    )
}
