/// Adds session and action entry points without interpolating JavaScript braces.
#[allow(non_snake_case)]
pub fn buildComposeDslRuntimeWrappedScript(script: &str) -> String {
    format!(
        "{script}\n;{}",
        include_str!(concat!(env!("OUT_DIR"), "/compose_dsl_execution.js"))
    )
}
