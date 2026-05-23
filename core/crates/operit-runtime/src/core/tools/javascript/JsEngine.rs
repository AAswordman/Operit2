use std::cell::RefCell;
use std::collections::BTreeMap;

use boa_engine::native_function::NativeFunction;
use boa_engine::{js_string, Context, JsResult, JsValue, Source};
use serde_json::Value;
use uuid::Uuid;

use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::core::tools::javascript::JsExecutionResultProtocol::buildJsExecutionErrorPayload;
use crate::core::tools::javascript::JsNativeInterfaceDelegates;
use crate::core::tools::javascript::JsTools::getJsToolsDefinition;

thread_local! {
    static CURRENT_TOOL_HANDLER: RefCell<Option<AIToolHandler>> = RefCell::new(None);
}

#[derive(Clone)]
pub struct JsEngine {
    toolHandler: AIToolHandler,
}

impl JsEngine {
    pub fn new(toolHandler: AIToolHandler) -> Self {
        Self { toolHandler }
    }

    #[allow(non_snake_case)]
    pub fn executeScriptFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
    ) -> Option<String> {
        let engine = self.clone();
        let script = script.to_string();
        let functionName = functionName.to_string();
        let params = params.clone();
        match std::thread::Builder::new()
            .name("OperitBoaJsEngine".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                engine.executeScriptFunctionOnCurrentThread(&script, &functionName, &params)
            }) {
            Ok(handle) => handle
                .join()
                .unwrap_or_else(|_| Some(buildJsExecutionErrorPayload("JavaScript engine thread panicked"))),
            Err(error) => Some(buildJsExecutionErrorPayload(&error.to_string())),
        }
    }

    #[allow(non_snake_case)]
    fn executeScriptFunctionOnCurrentThread(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
    ) -> Option<String> {
        let mut context = Context::default();
        CURRENT_TOOL_HANDLER.with(|handler| {
            *handler.borrow_mut() = Some(self.toolHandler.clone());
        });
        let registerResult = context.register_global_callable(
            js_string!("__operitNativeCallTool"),
            3,
            NativeFunction::from_copy_closure(nativeCallTool),
        );
        if let Err(error) = registerResult {
            CURRENT_TOOL_HANDLER.with(|handler| {
                *handler.borrow_mut() = None;
            });
            return Some(buildJsExecutionErrorPayload(&error.to_string()));
        }

        let paramsJson = match serde_json::to_string(params) {
            Ok(value) => value,
            Err(error) => return Some(buildJsExecutionErrorPayload(&error.to_string())),
        };
        let normalizedScript = normalizeLegacyAsyncToolScript(script);
        let scriptJson = serde_json::to_string(&normalizedScript).unwrap_or_else(|_| "\"\"".to_string());
        let functionNameJson =
            serde_json::to_string(functionName).unwrap_or_else(|_| "\"\"".to_string());
        let callId = format!(
            "operit_call_{}",
            Uuid::new_v4().to_string().replace('-', "")
        );
        let callIdJson =
            serde_json::to_string(&callId).unwrap_or_else(|_| "\"operit_call\"".to_string());

        let bootstrap = buildRuntimeBootstrapScript();
        if let Err(error) = context.eval(Source::from_bytes(bootstrap.as_bytes())) {
            CURRENT_TOOL_HANDLER.with(|handler| {
                *handler.borrow_mut() = None;
            });
            return Some(buildJsExecutionErrorPayload(&error.to_string()));
        }

        let executionScript =
            buildExecutionScript(&scriptJson, &functionNameJson, &paramsJson, &callIdJson);
        let output = match context.eval(Source::from_bytes(executionScript.as_bytes())) {
            Ok(value) => match value.to_string(&mut context) {
                Ok(value) => {
                    let value = value.to_std_string_escaped();
                    if value == format!("__operit_pending:{callId}") {
                        context.run_jobs();
                        Some(readPendingExecutionSession(&mut context, &callId))
                    } else {
                        Some(value)
                    }
                }
                Err(error) => Some(buildJsExecutionErrorPayload(&error.to_string())),
            },
            Err(error) => Some(buildJsExecutionErrorPayload(&error.to_string())),
        };
        CURRENT_TOOL_HANDLER.with(|handler| {
            *handler.borrow_mut() = None;
        });
        output
    }

    pub fn destroy(&self) {}
}

#[allow(non_snake_case)]
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

#[allow(non_snake_case)]
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
fn buildRuntimeBootstrapScript() -> String {
    format!(
        r#"
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
            reportErrorForCall: function() {{}}
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
        "#,
        getJsToolsDefinition()
    )
}

#[allow(non_snake_case)]
fn buildExecutionScript(
    scriptJson: &str,
    functionNameJson: &str,
    paramsJson: &str,
    callIdJson: &str,
) -> String {
    format!(
        r#"
        (function() {{
            var callId = {callIdJson};
            var params = {paramsJson};
            var scriptText = {scriptJson};
            var targetFunctionName = {functionNameJson};
            var module = {{ exports: {{}} }};
            var exports = module.exports;
            try {{
                var factory = new Function(
                    'module',
                    'exports',
                    'require',
                    '__operit_call_runtime',
                    scriptText
                );
                factory(module, exports, function(_name) {{ return {{}}; }}, {{ callId: callId }});
                var target =
                    module.exports && typeof module.exports[targetFunctionName] === 'function'
                        ? module.exports[targetFunctionName]
                        : (typeof globalThis[targetFunctionName] === 'function'
                            ? globalThis[targetFunctionName]
                            : null);
                if (typeof target !== 'function') {{
                    return JSON.stringify({{
                        success: false,
                        message: "Function '" + targetFunctionName + "' not found in script"
                    }});
                }}
                globalThis.__operitExecutionSessions = globalThis.__operitExecutionSessions || {{}};
                globalThis.__operitCompleteCalled = false;
                globalThis.__operitCompleteValue = undefined;
                var result = target(params);
                if (globalThis.__operitCompleteCalled) {{
                    return __operitFinishExecutionResult(globalThis.__operitCompleteValue);
                }}
                if (result && typeof result.then === 'function') {{
                    globalThis.__operitExecutionSessions[callId] = {{
                        completed: false,
                        output: null
                    }};
                    result.then(
                        function(value) {{
                            globalThis.__operitExecutionSessions[callId] = {{
                                completed: true,
                                output: __operitFinishExecutionResult(globalThis.__operitCompleteCalled ? globalThis.__operitCompleteValue : value)
                            }};
                        }},
                        function(error) {{
                            globalThis.__operitExecutionSessions[callId] = {{
                                completed: true,
                                output: JSON.stringify({{
                                    success: false,
                                    message: String(error && error.message ? error.message : error),
                                    data: error && error.data !== undefined ? error.data : null
                                }})
                            }};
                        }}
                    );
                    return "__operit_pending:" + callId;
                }}
                return __operitFinishExecutionResult(result);
            }} catch (error) {{
                return JSON.stringify({{
                    success: false,
                    message: "Script error: " + String(error && error.message ? error.message : error),
                    data: error && error.data !== undefined ? error.data : ""
                }});
            }}
        }})()
        "#
    )
}

#[allow(non_snake_case)]
fn normalizeLegacyAsyncToolScript(script: &str) -> String {
    let chars = script.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(script.len());
    let mut index = 0usize;
    let mut state = ScriptLexState::Normal;
    while index < chars.len() {
        match state {
            ScriptLexState::Normal => {
                if startsWithKeyword(&chars, index, "async") && isAsyncMarker(&chars, index + 5) {
                    index = skipWhitespace(&chars, index + 5);
                    continue;
                }
                if startsWithKeyword(&chars, index, "await") {
                    index = skipWhitespace(&chars, index + 5);
                    continue;
                }
                if startsWithKeyword(&chars, index, "const") {
                    out.push_str("var");
                    index += 5;
                    continue;
                }
                if startsWithKeyword(&chars, index, "let") {
                    out.push_str("var");
                    index += 3;
                    continue;
                }
                let ch = chars[index];
                if ch == '"' {
                    state = ScriptLexState::DoubleString;
                } else if ch == '\'' {
                    state = ScriptLexState::SingleString;
                } else if ch == '`' {
                    state = ScriptLexState::TemplateString;
                } else if ch == '/' && index + 1 < chars.len() && chars[index + 1] == '/' {
                    state = ScriptLexState::LineComment;
                } else if ch == '/' && index + 1 < chars.len() && chars[index + 1] == '*' {
                    state = ScriptLexState::BlockComment;
                }
                out.push(ch);
                index += 1;
            }
            ScriptLexState::DoubleString => {
                let ch = chars[index];
                out.push(ch);
                if ch == '"' && !isEscaped(&chars, index) {
                    state = ScriptLexState::Normal;
                }
                index += 1;
            }
            ScriptLexState::SingleString => {
                let ch = chars[index];
                out.push(ch);
                if ch == '\'' && !isEscaped(&chars, index) {
                    state = ScriptLexState::Normal;
                }
                index += 1;
            }
            ScriptLexState::TemplateString => {
                let ch = chars[index];
                out.push(ch);
                if ch == '`' && !isEscaped(&chars, index) {
                    state = ScriptLexState::Normal;
                }
                index += 1;
            }
            ScriptLexState::LineComment => {
                let ch = chars[index];
                out.push(ch);
                if ch == '\n' {
                    state = ScriptLexState::Normal;
                }
                index += 1;
            }
            ScriptLexState::BlockComment => {
                let ch = chars[index];
                out.push(ch);
                if ch == '*' && index + 1 < chars.len() && chars[index + 1] == '/' {
                    out.push('/');
                    index += 2;
                    state = ScriptLexState::Normal;
                } else {
                    index += 1;
                }
            }
        }
    }
    out
}

#[derive(Clone, Copy)]
enum ScriptLexState {
    Normal,
    DoubleString,
    SingleString,
    TemplateString,
    LineComment,
    BlockComment,
}

#[allow(non_snake_case)]
fn startsWithKeyword(chars: &[char], index: usize, keyword: &str) -> bool {
    let keywordChars = keyword.chars().collect::<Vec<_>>();
    if index + keywordChars.len() > chars.len() {
        return false;
    }
    if index > 0 && isIdentifierPart(chars[index - 1]) {
        return false;
    }
    for (offset, expected) in keywordChars.iter().enumerate() {
        if chars[index + offset] != *expected {
            return false;
        }
    }
    let end = index + keywordChars.len();
    end >= chars.len() || !isIdentifierPart(chars[end])
}

#[allow(non_snake_case)]
fn isAsyncMarker(chars: &[char], index: usize) -> bool {
    let next = skipWhitespace(chars, index);
    startsWithKeyword(chars, next, "function")
        || chars.get(next).is_some_and(|ch| *ch == '(' || isIdentifierStart(*ch))
}

#[allow(non_snake_case)]
fn skipWhitespace(chars: &[char], mut index: usize) -> usize {
    while index < chars.len() && chars[index].is_whitespace() {
        index += 1;
    }
    index
}

#[allow(non_snake_case)]
fn isEscaped(chars: &[char], index: usize) -> bool {
    let mut slashCount = 0usize;
    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        if chars[cursor] == '\\' {
            slashCount += 1;
        } else {
            break;
        }
    }
    slashCount % 2 == 1
}

#[allow(non_snake_case)]
fn isIdentifierStart(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

#[allow(non_snake_case)]
fn isIdentifierPart(ch: char) -> bool {
    isIdentifierStart(ch) || ch.is_ascii_digit()
}
