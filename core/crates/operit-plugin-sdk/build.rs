use std::env;
use std::path::PathBuf;

use operit_plugin_sdk_codegen::generate_declaration_tree;

/// Generates TypeScript declarations from the canonical Rust SDK source.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("missing manifest directory")?);
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("missing build output directory")?)
        .join("types");
    println!("cargo:rerun-if-changed=src");
    generate_declaration_tree(&manifest_dir.join("src"), &output_dir)?;
    Ok(())
}
