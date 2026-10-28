//! Checks the portable Bonjour adapter logic without requiring an Apple SDK.

#[path = "../../../apple/src/service_discovery.rs"]
mod apple_service_discovery;

/// Rejects accidental OS-handle use in the portable callback and encoding tests.
#[cfg(not(target_vendor = "apple"))]
#[no_mangle]
unsafe extern "C" fn DNSServiceRefDeallocate(_: *mut std::ffi::c_void) {
    panic!("portable Bonjour contract tests must not allocate OS handles");
}
