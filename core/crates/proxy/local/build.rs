use std::path::PathBuf;

use operit_proxy_dart_codegen::write_dart_proxy_artifacts;
use operit_proxy_rust_codegen::write_rust_proxy_artifacts;
use operit_plugin_sdk_codegen::{generate_plugin_sdk_client_bindings, generate_plugin_sdk_surface};
use operit_proxy_scan::{scan_core_proxy, CoreProxyScanConfig};

/// Runs the reusable core-app proxy code generator for this crate.
fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("../../runtime/application/src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("src").display()
    );
    // Theme and package DTOs also contribute to the generated client contracts.
    println!("cargo:rerun-if-changed={}", manifest_dir.join("../../plugin/theme/src").display());
    println!("cargo:rerun-if-changed={}", manifest_dir.join("../../tool/services/src").display());
    println!("cargo:rerun-if-changed={}", manifest_dir.join("../../plugin/sdk/src/toolpkg").display());
    // Node facade DTOs/methods are scanned too, not just application objects.
    println!("cargo:rerun-if-changed={}", manifest_dir.join("../../node/runtime/src").display());
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
    let repository_root = output
        .proxy_manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("proxy/local must live under core/crates/proxy");
    generate_plugin_sdk_client_bindings(
        &output.objects,
        &output.serializable_type_definitions,
        &repository_root.join("plugins/sdk/clients"),
    )
    .expect("write generated Plugin SDK clients");
    generate_plugin_sdk_surface(&output.objects, &out_dir.join("plugin_sdk_surface.rs"))
        .expect("write generated Plugin SDK route surface");
}
