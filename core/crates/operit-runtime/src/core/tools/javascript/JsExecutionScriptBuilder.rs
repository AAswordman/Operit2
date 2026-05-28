#[allow(non_snake_case)]
pub fn buildExecutionRuntimeBridgeScript() -> String {
    let script = include_str!("JsExecutionRuntimeBridge.script.js");
    script.to_string()
}

#[allow(non_snake_case)]
pub fn buildExecutionScript(
    scriptJson: &str,
    functionNameJson: &str,
    paramsJson: &str,
    callIdJson: &str,
) -> String {
    let script = include_str!("JsExecutionScript.template.js");
    script
        .replace("__CALL_ID_JSON__", callIdJson)
        .replace("__PARAMS_JSON__", paramsJson)
        .replace("__SCRIPT_JSON__", scriptJson)
        .replace("__FUNCTION_NAME_JSON__", functionNameJson)
}
