use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use operit_rslink_codegen::*;

mod build_rust_codegen;
mod build_rust_codegen_utils;
mod build_rust_dispatch_codegen;
mod build_rust_proxy_codegen;
mod build_rust_schema_codegen;

use build_rust_codegen::{render_generated, render_schema};

/// Writes generated Rust proxy dispatch and schema artifacts from one proxy scan.
pub fn write_rust_proxy_artifacts(
    out_dir: &Path,
    proxy_manifest_dir: &Path,
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let schema_json = render_schema(objects, serializable_types);
    let generated = render_generated(objects, &schema_json, error_types);
    fs::write(out_dir.join("generated_core_dispatch.rs"), generated)
        .expect("write generated_core_dispatch.rs");
    write_proxy_schema_artifact(proxy_manifest_dir, &schema_json);
    schema_json
}

/// Writes the generated proxy schema JSON artifact.
fn write_proxy_schema_artifact(proxy_manifest_dir: &Path, schema_json: &str) {
    let repo_root = proxy_manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("proxy/local must live under core/crates/proxy");
    let schema_dir = repo_root.join("core/generated");
    fs::create_dir_all(&schema_dir).expect("create generated schema directory");
    write_generated_file(&schema_dir.join("core_proxy_schema.json"), schema_json);
}

/// Writes generated content only when bytes changed.
fn write_generated_file(path: &Path, contents: &str) {
    if fs::read(path).is_ok_and(|current| current == contents.as_bytes()) {
        return;
    }

    fs::write(path, contents)
        .unwrap_or_else(|error| panic!("write generated file {}: {error}", path.display()));
}
