use std::path::PathBuf;

use operit_proxy_dart_codegen::write_dart_proxy_artifacts;
use operit_proxy_rust_codegen::write_rust_proxy_artifacts;
use operit_proxy_scan::{scan_core_proxy, CoreProxyScanConfig};

/// Runs the reusable core-app proxy code generator for this crate.
fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let output = scan_core_proxy(CoreProxyScanConfig::from_proxy_manifest_dir(manifest_dir));
    write_rust_proxy_artifacts(
        &out_dir,
        &output.proxy_manifest_dir,
        &output.objects,
        &output.serializable_type_definitions,
        &output.error_type_definitions,
    );
    write_dart_proxy_artifacts(
        &output.proxy_manifest_dir,
        &output.objects,
        &output.serializable_type_definitions,
    );
}
