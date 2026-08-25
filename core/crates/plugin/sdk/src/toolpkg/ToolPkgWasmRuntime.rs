#![allow(non_snake_case)]

use crate::javascript::{JsToolPkgWasmArg, JsToolPkgWasmResult};
use serde_json::{Number, Value};

#[cfg(not(target_arch = "wasm32"))]
use wasmi::{Engine, Linker, Module, Store, Val, ValType};

/// Calls one scalar ToolPkg WASM export from validated module bytes.
pub fn callWasmExport(
    moduleBytes: &[u8],
    exportName: &str,
    args: &[JsToolPkgWasmArg],
) -> Result<JsToolPkgWasmResult, String> {
    let exportName = exportName.trim();
    if exportName.is_empty() {
        return Err("ToolPkg WASM export name is required".to_string());
    }
    if moduleBytes.is_empty() {
        return Err("ToolPkg WASM module bytes are empty".to_string());
    }
    callWasmExportForTarget(moduleBytes, exportName, args)
}

/// Calls one scalar ToolPkg WASM export on native Rust targets.
#[cfg(not(target_arch = "wasm32"))]
fn callWasmExportForTarget(
    moduleBytes: &[u8],
    exportName: &str,
    args: &[JsToolPkgWasmArg],
) -> Result<JsToolPkgWasmResult, String> {
    let engine = Engine::default();
    let module = Module::new(&engine, moduleBytes).map_err(|error| error.to_string())?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::<()>::new(&engine);
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|error| error.to_string())?;
    let func = instance
        .get_func(&store, exportName)
        .ok_or_else(|| format!("ToolPkg WASM export not found: {exportName}"))?;
    let params = args
        .iter()
        .enumerate()
        .map(|(index, arg)| parseWasmArg(index, arg))
        .collect::<Result<Vec<_>, _>>()?;
    let funcType = func.ty(&store);
    if funcType.results().len() > 1 {
        return Err(format!(
            "ToolPkg WASM export must return at most one value: {exportName}"
        ));
    }
    let mut results = funcType
        .results()
        .iter()
        .copied()
        .map(Val::default_for_ty)
        .collect::<Vec<_>>();
    func.call(&mut store, &params, &mut results)
        .map_err(|error| error.to_string())?;
    encodeWasmResult(results.first())
}

/// Reports unsupported host execution when the SDK itself is compiled to WASM.
#[cfg(target_arch = "wasm32")]
fn callWasmExportForTarget(
    _moduleBytes: &[u8],
    _exportName: &str,
    _args: &[JsToolPkgWasmArg],
) -> Result<JsToolPkgWasmResult, String> {
    Err("ToolPkg WASM host execution is unavailable on wasm32".to_string())
}

/// Converts one JavaScript ABI argument into a wasmi value.
#[cfg(not(target_arch = "wasm32"))]
fn parseWasmArg(index: usize, arg: &JsToolPkgWasmArg) -> Result<Val, String> {
    let valueType = arg.value_type.trim().to_ascii_lowercase();
    match valueType.as_str() {
        "i32" => Ok(Val::I32(parseI32Value(index, &arg.value)?)),
        "i64" => Ok(Val::I64(parseI64Value(index, &arg.value)?)),
        "f32" => Ok(Val::F32(parseF32Value(index, &arg.value)?.into())),
        "f64" => Ok(Val::F64(parseF64Value(index, &arg.value)?.into())),
        _ => Err(format!(
            "ToolPkg WASM arg {index} has unsupported type: {valueType}"
        )),
    }
}

/// Parses one i32 ABI value from JSON.
#[cfg(not(target_arch = "wasm32"))]
fn parseI32Value(index: usize, value: &Value) -> Result<i32, String> {
    let raw = value
        .as_i64()
        .ok_or_else(|| format!("ToolPkg WASM arg {index} i32 value must be a number"))?;
    i32::try_from(raw).map_err(|_| format!("ToolPkg WASM arg {index} exceeds i32 range"))
}

/// Parses one i64 ABI value from JSON.
#[cfg(not(target_arch = "wasm32"))]
fn parseI64Value(index: usize, value: &Value) -> Result<i64, String> {
    if let Some(raw) = value.as_i64() {
        return Ok(raw);
    }
    let raw = value
        .as_str()
        .ok_or_else(|| format!("ToolPkg WASM arg {index} i64 value must be a number or string"))?;
    raw.trim()
        .parse::<i64>()
        .map_err(|error| format!("ToolPkg WASM arg {index} i64 parse failed: {error}"))
}

/// Parses one f32 ABI value from JSON.
#[cfg(not(target_arch = "wasm32"))]
fn parseF32Value(index: usize, value: &Value) -> Result<f32, String> {
    let raw = parseF64Value(index, value)?;
    if raw < f32::MIN as f64 || raw > f32::MAX as f64 {
        return Err(format!("ToolPkg WASM arg {index} exceeds f32 range"));
    }
    Ok(raw as f32)
}

/// Parses one f64 ABI value from JSON.
#[cfg(not(target_arch = "wasm32"))]
fn parseF64Value(index: usize, value: &Value) -> Result<f64, String> {
    let raw = if let Some(raw) = value.as_f64() {
        raw
    } else {
        let text = value.as_str().ok_or_else(|| {
            format!("ToolPkg WASM arg {index} float value must be a number or string")
        })?;
        text.trim()
            .parse::<f64>()
            .map_err(|error| format!("ToolPkg WASM arg {index} float parse failed: {error}"))?
    };
    if !raw.is_finite() {
        return Err(format!(
            "ToolPkg WASM arg {index} float value must be finite"
        ));
    }
    Ok(raw)
}

/// Converts one wasmi value into the JavaScript result envelope.
#[cfg(not(target_arch = "wasm32"))]
fn encodeWasmResult(value: Option<&Val>) -> Result<JsToolPkgWasmResult, String> {
    match value {
        Some(Val::I32(value)) => Ok(JsToolPkgWasmResult {
            value_type: Some("i32".to_string()),
            value: Value::Number(Number::from(*value)),
        }),
        Some(Val::I64(value)) => Ok(JsToolPkgWasmResult {
            value_type: Some("i64".to_string()),
            value: Value::String(value.to_string()),
        }),
        Some(Val::F32(value)) => numberResult("f32", value.to_float() as f64),
        Some(Val::F64(value)) => numberResult("f64", value.to_float()),
        Some(value) => Err(format!(
            "ToolPkg WASM result type is unsupported: {}",
            wasmTypeName(value.ty())
        )),
        None => Ok(JsToolPkgWasmResult {
            value_type: None,
            value: Value::Null,
        }),
    }
}

/// Builds one numeric JavaScript result.
#[cfg(not(target_arch = "wasm32"))]
fn numberResult(valueType: &str, value: f64) -> Result<JsToolPkgWasmResult, String> {
    let number = Number::from_f64(value)
        .ok_or_else(|| format!("ToolPkg WASM {valueType} result is not finite"))?;
    Ok(JsToolPkgWasmResult {
        value_type: Some(valueType.to_string()),
        value: Value::Number(number),
    })
}

/// Returns the JavaScript-facing name for a wasmi value type.
#[cfg(not(target_arch = "wasm32"))]
fn wasmTypeName(valueType: ValType) -> &'static str {
    match valueType {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
        ValType::V128 => "v128",
        ValType::FuncRef => "funcref",
        ValType::ExternRef => "externref",
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    const PRIME_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x06, 0x01, 0x60, 0x01, 0x7f, 0x01,
        0x7f, 0x03, 0x03, 0x02, 0x00, 0x00, 0x05, 0x03, 0x01, 0x00, 0x00, 0x07, 0x1f, 0x03, 0x07,
        0x69, 0x73, 0x50, 0x72, 0x69, 0x6d, 0x65, 0x00, 0x00, 0x08, 0x6e, 0x74, 0x68, 0x50, 0x72,
        0x69, 0x6d, 0x65, 0x00, 0x01, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, 0x0a,
        0x8a, 0x01, 0x02, 0x4f, 0x01, 0x01, 0x7f, 0x20, 0x00, 0x41, 0x02, 0x48, 0x04, 0x40, 0x41,
        0x00, 0x0f, 0x0b, 0x20, 0x00, 0x41, 0x02, 0x46, 0x04, 0x40, 0x41, 0x01, 0x0f, 0x0b, 0x20,
        0x00, 0x41, 0x01, 0x71, 0x45, 0x04, 0x40, 0x41, 0x00, 0x0f, 0x0b, 0x41, 0x03, 0x21, 0x01,
        0x03, 0x40, 0x20, 0x01, 0x20, 0x00, 0x20, 0x01, 0x6d, 0x4c, 0x04, 0x40, 0x20, 0x00, 0x20,
        0x01, 0x6f, 0x45, 0x04, 0x40, 0x41, 0x00, 0x0f, 0x0b, 0x20, 0x01, 0x41, 0x02, 0x6a, 0x21,
        0x01, 0x0c, 0x01, 0x0b, 0x0b, 0x41, 0x01, 0x0b, 0x38, 0x01, 0x02, 0x7f, 0x20, 0x00, 0x41,
        0x00, 0x4c, 0x04, 0x40, 0x41, 0x00, 0x0f, 0x0b, 0x41, 0x01, 0x21, 0x01, 0x03, 0x40, 0x20,
        0x00, 0x20, 0x02, 0x4a, 0x04, 0x40, 0x20, 0x02, 0x41, 0x01, 0x6a, 0x20, 0x02, 0x20, 0x01,
        0x41, 0x01, 0x6a, 0x22, 0x01, 0x10, 0x00, 0x41, 0x01, 0x46, 0x1b, 0x21, 0x02, 0x0c, 0x01,
        0x0b, 0x0b, 0x20, 0x01, 0x0b,
    ];

    /// Builds one i32 ToolPkg WASM argument.
    fn i32Arg(value: i32) -> JsToolPkgWasmArg {
        JsToolPkgWasmArg {
            value_type: "i32".to_string(),
            value: Value::Number(Number::from(value)),
        }
    }

    /// Calls one i32 export from the embedded prime demo module.
    fn callPrimeExport(exportName: &str, value: i32) -> i64 {
        let result =
            callWasmExport(PRIME_WASM, exportName, &[i32Arg(value)]).expect("wasm call should run");
        result
            .value
            .as_i64()
            .expect("prime demo export should return i32")
    }

    /// Verifies the demo prime WASM module through the Rust runtime.
    #[test]
    fn callsPrimeDemoExports() {
        assert_eq!(callPrimeExport("isPrime", 1), 0);
        assert_eq!(callPrimeExport("isPrime", 2), 1);
        assert_eq!(callPrimeExport("isPrime", 21), 0);
        assert_eq!(callPrimeExport("isPrime", 29), 1);
        assert_eq!(callPrimeExport("nthPrime", 10), 29);
    }
}
