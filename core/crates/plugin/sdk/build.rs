use std::env;
use std::path::PathBuf;

use operit_plugin_sdk_codegen::{
    generate_builtin_tool_names, generate_declaration_tree, generate_js_tools_runtime,
};

/// Generates TypeScript declarations from the canonical Rust SDK source.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("missing manifest directory")?);
    let build_output =
        PathBuf::from(env::var_os("OUT_DIR").ok_or("missing build output directory")?);
    let output_dir = build_output.join("types");
    println!("cargo:rerun-if-changed=src");
    generate_js_tools_runtime(
        &manifest_dir.join("src"),
        &manifest_dir.join("src/js_sdk/runtime_bindings.rs"),
        &build_output.join("js_tools.js"),
    )?;
    generate_declaration_tree(&manifest_dir.join("src"), &output_dir)?;
    generate_builtin_tool_names(
        &manifest_dir.join("src/js_sdk/tool_types.rs"),
        &build_output.join("builtin_tool_names.rs"),
    )?;
    Ok(())
}
