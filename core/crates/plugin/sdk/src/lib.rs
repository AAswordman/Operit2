//! Public contracts for building, validating, and loading Operit plugins.

pub mod execution_result;
pub mod javascript;
pub mod js_sdk;
pub mod package;
pub mod toolpkg;

#[path = "JsExecutionScriptBuilder.rs"]
pub mod JsExecutionScriptBuilder;

#[path = "PackageManager.rs"]
pub mod PackageManager;

#[path = "JsPackageLoader.rs"]
pub mod JsPackageLoader;

#[path = "JsTools.rs"]
pub mod JsTools;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod javascript_test;
