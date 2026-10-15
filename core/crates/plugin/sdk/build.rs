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
    println!("cargo:rerun-if-changed=../../foundation/host-api/src/PluginStorage.rs");
    println!("cargo:rerun-if-changed=src/js_sdk/runtime_bindings.rs");
    println!("cargo:rerun-if-changed=../codegen/src/runtime_bindings.rs");
    // Rust and Node tooling consume the same trusted source manifest.
    let compose_dir = manifest_dir.join("src/toolpkg/compose");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(compose_dir.join("sources.json"))?)?;
    let asset = |key: &str| -> Result<String, Box<dyn std::error::Error>> {
        let name = manifest[key]
            .as_str()
            .ok_or("Compose asset name must be a string")?;
        Ok(std::fs::read_to_string(compose_dir.join(name))?)
    };
    // Keep trusted parser bytes outside the eagerly compiled bootstrap.
    std::fs::write(build_output.join("compose_dsl_parser.js"), asset("parser")?)?;
    std::fs::write(
        build_output.join("compose_dsl_parser_source.js"),
        asset("parserSource")?,
    )?;
    let assemble = |key: &str| -> Result<String, Box<dyn std::error::Error>> {
        let names: Vec<String> = serde_json::from_value(manifest[key].clone())?;
        Ok(names
            .iter()
            .map(|name| std::fs::read_to_string(compose_dir.join(name)))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n"))
    };
    std::fs::write(build_output.join("compose_dsl_compiler.js"), format!("{}\n{}", asset("artifact")?, asset("compiler")?))?;
    let context = assemble("context")?;
    std::fs::write(build_output.join("compose_dsl_context.js"), context)?;
    let execution = assemble("execution")?;
    std::fs::write(
        build_output.join("compose_dsl_execution.js"),
        format!("(function() {{\n{execution}\n}})();"),
    )?;
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
