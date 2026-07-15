/// Returns the generated JavaScript `Tools` namespace backed by canonical Rust bindings.
#[allow(non_snake_case)]
pub fn getJsToolsDefinition() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/js_tools.js"))
}
