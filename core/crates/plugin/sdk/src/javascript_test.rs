//! Explicit native Host setup for JavaScript integration tests.
use operit_host_api::{HostJavaScriptRuntime, HostJavaScriptRuntimeHost};
use operit_host_native_scheduler::NativeHostJavaScriptRuntimeHost;
use std::sync::Arc;

/// Installs the native JavaScript Host required by SDK publication tests.
pub(crate) fn install_host() {
    operit_host_api::HostManager::setDefaultHostJavaScriptRuntimeHost(Arc::new(
        NativeHostJavaScriptRuntimeHost::new(),
    ));
}

/// Creates an isolated runtime through the production Host contract.
pub(crate) fn runtime() -> Box<dyn HostJavaScriptRuntime> {
    NativeHostJavaScriptRuntimeHost::new()
        .createHostJavaScriptRuntime()
        .expect("Host JavaScript runtime should start")
}
