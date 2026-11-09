//! Runs the real Host engine and ToolPkg WASM API against scalar, reference and USearch modules.
use operit_host_api::{HostError, HostJavaScriptRuntimeHost};
use operit_host_native_scheduler::NativeHostJavaScriptRuntimeHost;
use serde_json::{json, Value};
use std::{
    ffi::{c_char, CStr},
    sync::Arc,
};
mod wasm_engine_modules;
use wasm_engine_modules::{reference_module, scalar_module};

fn run(script: &str) -> Result<(), String> {
    // No code generation is enabled; exposing GC only exercises reference barriers.
    v8::V8::set_flags_from_string("--expose-gc");
    for round in 0..2 {
        let host = NativeHostJavaScriptRuntimeHost::new();
        let mut runtime = host
            .createHostJavaScriptRuntime()
            .map_err(|e| e.to_string())?;
        runtime
            .registerHostJavaScriptBinaryFunction(
                "__operitWasmFixture",
                Arc::new(|arguments| {
                    let bytes = match arguments.first().and_then(Value::as_str) {
                        Some("scalar") => scalar_module(),
                        Some("references") => reference_module(),
                        _ => return Err(HostError::new("Unknown WASM fixture")),
                    };
                    Ok((json!({}), bytes))
                }),
            )
            .map_err(|e| e.to_string())?;
        runtime
            .registerHostJavaScriptBinaryFunction(
                "__operitNativeLoadToolPkgWasm",
                Arc::new(|arguments| {
                    if arguments != vec![json!("character_cards"), json!("usearch")] {
                        return Err(HostError::new("Unexpected package or module"));
                    }
                    let manifest: Value = serde_json::from_str(include_str!(concat!(
                        env!("OPERIT_REPO_ROOT"),
                        "/plugins/packages/buildin/character_cards/manifest.json"
                    )))
                    .map_err(|e| HostError::new(e.to_string()))?;
                    let bytes = include_bytes!(concat!(
                        env!("OPERIT_REPO_ROOT"),
                        "/plugins/packages/buildin/character_cards/resources/usearch.wasm"
                    ));
                    Ok((
                        json!({"exports": manifest["wasm_modules"][0]["exports"]}),
                        bytes.to_vec(),
                    ))
                }),
            )
            .map_err(|e| e.to_string())?;
        runtime
            .evaluateHostJavaScriptBootstrap(include_str!(concat!(
                env!("OPERIT_REPO_ROOT"),
                "/core/crates/plugin/sdk/src/toolpkg/ToolPkgWasmApi.js"
            )))
            .map_err(|e| e.to_string())?;
        runtime
            .evaluateHostJavaScriptVoid("ios-wasm-engine-smoke.js", script)
            .map_err(|e| e.to_string())?;
        runtime
            .executePendingHostJavaScriptJobs()
            .map_err(|e| e.to_string())?;
        let result = runtime
            .evaluateHostJavaScriptString(
                "smoke-result",
                "JSON.stringify(globalThis.__wasmSmokeResult)",
            )
            .map_err(|e| e.to_string())?;
        let value: Value = serde_json::from_str(&result).map_err(|e| e.to_string())?;
        if value["status"] != "passed" {
            return Err(result);
        }
        eprintln!("WASM engine realm {round}: {result}");
    }
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn operit_ios_wasm_engine_smoke(path: *const c_char) -> i32 {
    let result = std::panic::catch_unwind(|| {
        let path = unsafe { CStr::from_ptr(path) }
            .to_str()
            .map_err(|e| e.to_string())?;
        let script = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        run(&script)
    });
    match result {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            eprintln!("WASM engine smoke failed: {error}");
            1
        }
        Err(_) => {
            eprintln!("WASM engine smoke panicked");
            2
        }
    }
}
