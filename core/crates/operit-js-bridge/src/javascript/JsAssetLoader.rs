#[allow(non_snake_case)]
pub fn loadUINodeJs() -> String {
    include_str!("UINode.script.js").to_string()
}

#[allow(non_snake_case)]
pub fn loadAndroidUtilsJs() -> String {
    include_str!("AndroidUtils.script.js").to_string()
}

#[allow(non_snake_case)]
pub fn loadOkHttp3Js() -> String {
    include_str!("OkHttp3.script.js").to_string()
}

#[allow(non_snake_case)]
pub fn loadPluginConfigJs() -> String {
    include_str!("PluginConfig.script.js").to_string()
}

#[allow(non_snake_case)]
pub fn loadRuntimeContextJs() -> String {
    include_str!("RuntimeContext.script.js").to_string()
}
