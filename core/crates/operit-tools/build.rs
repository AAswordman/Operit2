use std::env;
use std::path::PathBuf;

use operit_plugin_sdk_codegen::{
    check_tool_registration_contract, generate_js_tools_host_implementation,
};

/// Enforces the canonical Rust SDK tool-registration contract during every build.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("missing manifest directory")?);
    let tool_types = manifest_dir.join("../operit-plugin-sdk/src/js_sdk/tool_types.rs");
    let sdk_src = manifest_dir.join("../operit-plugin-sdk/src");
    let registration = manifest_dir.join("src/tools/ToolRegistration.rs");
    let bindings = manifest_dir.join("../operit-plugin-sdk/src/js_sdk/runtime_bindings.rs");
    let output = PathBuf::from(env::var_os("OUT_DIR").ok_or("missing build output directory")?)
        .join("js_tools_host_impl.rs");
    println!("cargo:rerun-if-changed={}", tool_types.display());
    println!("cargo:rerun-if-changed={}", registration.display());
    println!("cargo:rerun-if-changed={}", bindings.display());
    println!(
        "cargo:rerun-if-changed={}",
        sdk_src.join("js_sdk").display()
    );
    check_tool_registration_contract(&tool_types, &registration)?;
    generate_js_tools_host_implementation(&sdk_src, &bindings, &output)
}
