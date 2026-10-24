//! Windows host assembly for the Flutter bridge.

use std::path::PathBuf;
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_api::SystemOperationHost;
use operit_host_windows_native::{
    createRuntimeHostManager, WindowsRuntimeStorageHost, WindowsTerminalHost,
};
use operit_link::LinkDeviceInfo;

use super::{BridgeStartup, StartupMetadata};
use crate::FlutterHostAdapters::FlutterWebVisitBridge;
use crate::FlutterOwnerCapabilities::{
    ownerLocationForeground, FlutterSystemBindings, FlutterSystemOperationBridge,
};

/// Creates the Windows host bundle with the native terminal implementation.
pub(crate) fn create_host_context(startup: &BridgeStartup) -> Result<HostManager, String> {
    let mut context = createRuntimeHostManager(
        startup.runtimeRoot.clone(),
        startup.workspaceRoot.clone(),
        Arc::new(FlutterWebVisitBridge::new()),
    )
    .withTerminalHost(Arc::new(WindowsTerminalHost::new()));
    let system = context
        .systemOperationHost
        .clone()
        .ok_or_else(|| "Flutter runtime requires its Windows system host".to_string())?;
    let notifications = system.clone();
    let device_info = system.clone();
    let screenshot = system.clone();
    let recognition = system.clone();
    let notification_sender = system.clone();
    context.systemOperationHost = Some(Arc::new(FlutterSystemOperationBridge::new(
        system,
        FlutterSystemBindings {
            notificationSender: Arc::new(move |request| {
                notification_sender.sendNotification(request)
            }),
            notifications: Arc::new(move |limit, ongoing| {
                notifications.getNotifications(limit, ongoing)
            }),
            location: Arc::new(ownerLocationForeground),
            deviceInfo: Arc::new(move || device_info.getDeviceInfo()),
            screenshot: Arc::new(move || screenshot.captureScreenshot()),
            recognition: Arc::new(move |path, language, quality| {
                recognition.recognizeText(path, language, quality)
            }),
        },
    )));
    Ok(context)
}

/// Reads Windows identity through its system host.
pub(crate) fn startup_device_info(
    context: &HostManager,
    _metadata: &StartupMetadata,
) -> Result<LinkDeviceInfo, String> {
    let system = context
        .systemOperationHost
        .as_ref()
        .ok_or_else(|| "Runtime identity requires its Windows system host".to_string())?;
    Ok(LinkDeviceInfo {
        platform: context.hostEnvironment.id.clone(),
        model: system
            .getDeviceInfo()
            .map_err(|error| error.to_string())?
            .model,
    pairingIncarnation: None,
})
}

/// Resolves Windows storage roots from the native storage host.
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Ok((
        WindowsRuntimeStorageHost::defaultRuntimeRoot(),
        WindowsRuntimeStorageHost::defaultWorkspaceRoot(),
    ))
}

/// Windows has no process-global bridge registration to release here.
pub(crate) fn release_host() {}

impl crate::OperitFlutterBridge {
    /// Starts Windows using its native storage roots.
    pub(crate) fn new() -> Result<Self, String> {
        let (runtimeRoot, workspaceRoot) = default_native_storage_roots()?;
        Self::new_with_storage_roots(runtimeRoot, workspaceRoot)
    }

    /// Starts Windows with explicit storage roots.
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

/// Creates only the platform secret-store adapter without starting Core or owner services.
pub(crate) fn rescue_secret_store(default_runtime: &std::path::Path) -> Result<Option<std::sync::Arc<dyn operit_host_api::HostSecretStore>>, String> {
    Ok(Some(std::sync::Arc::new(operit_host_windows_native::WindowsRuntimeStorageHost::new(
        default_runtime.to_path_buf(), default_runtime.with_file_name("workspaces"),
    ))))
}
