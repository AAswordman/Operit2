use operit_proxy_scan::{scan_core_proxy, CoreProxyScanConfig};

/// Peer persistence synchronization must never silently leave the proxy surface.
/// An unsupported argument or return type is rejected by the scanner after this
/// crate's build script compiles, so the generated dispatch simply loses the
/// method's arm and every later call fails with METHOD_NOT_FOUND at runtime
/// instead of failing the build (see the 2026-11 blob-presence outage).
#[test]
fn peer_synchronization_application_methods_remain_callable() {
    // Build scripts receive this from Cargo; tests must supply the same input.
    std::env::set_var("CARGO_CFG_TARGET_ARCH", std::env::consts::ARCH);
    let output = scan_core_proxy(CoreProxyScanConfig::from_proxy_manifest_dir(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    ));
    let application = output
        .objects
        .iter()
        .find(|object| object.object_id == "core/application")
        .expect("core application surface");
    let methods = [
        "syncClock",
        "syncClockDigest",
        "syncOperationsSince",
        "syncApplyOperations",
        "syncApplyImmediateBindingOperation",
        "syncConsistency",
        "syncReadSnapshot",
        "syncReadCoverage",
        "syncRepairSnapshot",
        "syncBlobExists",
        "syncReadBlobChunk",
    ];
    for name in methods {
        let method = application
            .methods
            .iter()
            .find(|method| method.name == name)
            .unwrap_or_else(|| panic!("application surface is missing peer sync method {name}"));
        assert!(
            method.call_protocol().is_some(),
            "peer sync method {name} lost its call protocol: {:?}",
            method.protocol
        );
    }
}
