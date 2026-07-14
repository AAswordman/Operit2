use std::env;
use std::path::PathBuf;

use operit_plugin_sdk_codegen::check_tool_registration_contract;

/// Enforces the canonical Rust SDK tool-registration contract during every build.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("missing manifest directory")?);
    let tool_types = manifest_dir.join("../operit-plugin-sdk/src/js_sdk/tool_types.rs");
    let registration = manifest_dir.join("src/tools/ToolRegistration.rs");
    println!("cargo:rerun-if-changed={}", tool_types.display());
    println!("cargo:rerun-if-changed={}", registration.display());
    check_tool_registration_contract(&tool_types, &registration)
}
