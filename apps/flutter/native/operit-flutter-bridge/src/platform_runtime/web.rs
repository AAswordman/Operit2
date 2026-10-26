//! Browser host assembly for the Flutter bridge.

use std::path::PathBuf;

use operit_host_api::HostManager::HostManager;
use operit_host_api::SystemOperationHost;
use operit_host_web::WebRuntimeStorageHost;
use operit_link::LinkDeviceInfo;

use super::{BridgeStartup, StartupMetadata};

/// Creates the browser host bundle supplied by the Web host crate.
pub(crate) fn create_host_context(_startup: &BridgeStartup) -> Result<HostManager, String> {
    let mut context = operit_host_web::createRuntimeHostManager();
    context.webVisitHost = Some(std::sync::Arc::new(
        crate::FlutterHostAdapters::FlutterWebVisitBridge::new(),
    ));
    Ok(context)
}

/// Reads browser identity from the Web system host.
pub(crate) fn startup_device_info(
    context: &HostManager,
    _metadata: &StartupMetadata,
) -> Result<LinkDeviceInfo, String> {
    let system = context
        .systemOperationHost
        .as_ref()
        .ok_or_else(|| "Runtime identity requires its browser system host".to_string())?;
    Ok(LinkDeviceInfo {
        platform: context.hostEnvironment.id.clone(),
        model: system
            .getDeviceInfo()
            .map_err(|error| error.to_string())?
            .model,
    pairingIncarnation: None,
})
}

/// Resolves browser storage roots from the Web runtime storage host.
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Ok((
        WebRuntimeStorageHost::defaultRuntimeRoot(),
        WebRuntimeStorageHost::defaultWorkspaceRoot(),
    ))
}

/// The browser host owns its resources through the Web runtime.
pub(crate) fn release_host() {}

impl crate::OperitFlutterBridge {
    /// Starts the browser runtime using its Web storage host.
    pub(crate) fn new() -> Result<Self, String> {
        let (runtimeRoot, workspaceRoot) = default_native_storage_roots()?;
        Self::new_with_storage_roots(runtimeRoot, workspaceRoot)
    }

    /// Starts the browser runtime with explicit storage roots.
    pub(crate) fn new_with_storage_roots(
        runtimeRoot: PathBuf,
        workspaceRoot: PathBuf,
    ) -> Result<Self, String> {
        crate::PlatformRuntimeFactory::startBridge(BridgeStartup {
            runtimeRoot,
            workspaceRoot,
            metadata: StartupMetadata::HostProvided,
        })
    }
}

/// Browser secrets are included by its independent browser rescue Host.
pub(crate) fn rescue_secret_store(_root: &std::path::Path) -> Result<Option<std::sync::Arc<dyn operit_host_api::HostSecretStore>>, String> {
    Ok(None)
}
