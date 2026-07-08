pub fn buildRuntimeBootstrapScript() -> String {
    include_str!("JsInitRuntime.script.js").to_string()
}
