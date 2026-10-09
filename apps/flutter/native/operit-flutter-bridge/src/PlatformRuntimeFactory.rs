//! Common Flutter bridge startup pipeline.
//!
//! This file intentionally contains no target selection. The selected platform
//! is implemented behind `platform_runtime::selected`, while ABI execution is
//! implemented in `PlatformRuntimeExecution.rs`.

use std::path::PathBuf;
use std::sync::Arc;

use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;

use crate::FlutterHostAdapters::{
    FlutterBrowserAutomationBridge, FlutterBrowserSessionBridge, FlutterComposeDslWebViewBridge,
};
use crate::{current_time_millis_u64, OperitFlutterBridge};

#[path = "PlatformRuntimeExecution.rs"]
mod execution;
#[path = "platform_runtime/mod.rs"]
mod platform;

pub(crate) use execution::PlatformBridgeState;

/// Carries the startup information required by one selected platform module.
pub(crate) enum StartupMetadata {
    HostProvided,
    AndroidDevice { model: String },
    OpenHarmonyLanguage { code: String },
}

/// Carries immutable storage roots and owner-provided startup information.
pub(crate) struct BridgeStartup {
    pub(crate) runtimeRoot: PathBuf,
    pub(crate) workspaceRoot: PathBuf,
    pub(crate) metadata: StartupMetadata,
}

/// Creates one Core tree after the selected platform module has assembled hosts.
pub(crate) fn startBridge(startup: BridgeStartup) -> Result<OperitFlutterBridge, String> {
    if startup.runtimeRoot.as_os_str().is_empty() || startup.workspaceRoot.as_os_str().is_empty() {
        return Err("Runtime and workspace storage roots must be explicit".to_string());
    }
    let mut context = platform::create_host_context(&startup)?;
    context = context
        .withBrowserAutomationHostFactory(Arc::new(|files| {
            Arc::new(FlutterBrowserAutomationBridge::new(files))
        }))
        .withBrowserSessionHost(Arc::new(FlutterBrowserSessionBridge::new()))
        .withComposeDslWebViewHost(Arc::new(FlutterComposeDslWebViewBridge::new()))
        .withToastHost(Arc::new(
            crate::FlutterOwnerCapabilities::presentFlutterToast,
        ));
    let terminal = context
        .terminalHost
        .clone()
        .ok_or_else(|| "Flutter runtime requires a terminal host".to_string())?;
    context = context.withTerminalHost(Arc::new(
        crate::FlutterOwnerCapabilities::RuntimeSessionPublishingTerminalHost::new(terminal),
    ));
    let deviceInfo = platform::startup_device_info(&context, &startup.metadata)?;
    OperitFlutterBridge::start(
        LocalCoreProxy::new(OperitApplication::newWithContext(context)),
        deviceInfo,
    )
}

/// Resolves storage roots through the selected platform host module.
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    platform::default_native_storage_roots()
}

/// Releases registrations owned by the selected platform host module.
pub(crate) fn release_host() {
    platform::release_host();
}

/// Dispatches one external runtime event into the generated application target.
impl OperitFlutterBridge {
    /// Dispatches one encoded owner event into the generated application target.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn emitRuntimeEvent(&self, encoded: &str) -> String {
        let result = (|| -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {
            let event: serde_json::Value = serde_json::from_str(encoded).map_err(|error| {
                operit_link::CoreLinkError::internal(format!(
                    "Runtime event is invalid JSON: {error}"
                ))
            })?;
            let target =
                LocalCoreProxy::generatedTargetForSchema("application").ok_or_else(|| {
                    operit_link::CoreLinkError::internal("Generated application target is missing")
                })?;
            let args = operit_link::toCoreValue(serde_json::json!({ "event": event }))
                .map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;
            self.call(operit_link::CoreCallRequest::new(
                format!("runtime-event-{}", current_time_millis_u64()),
                target,
                "ingestRuntimeEvent",
                args,
            ))
            .result
        })();
        match result {
            Ok(value) => serde_json::json!({ "ok": true, "result": value }).to_string(),
            Err(error) => serde_json::json!({ "ok": false, "error": error.message }).to_string(),
        }
    }
}
