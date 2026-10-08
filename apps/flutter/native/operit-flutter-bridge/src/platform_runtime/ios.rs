//! iOS host assembly for the Flutter bridge.

use std::path::PathBuf;
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_api::SystemOperationHost;
use operit_host_ios_native::{
    createRuntimeHostManager, IosBluetoothHost, IosFileSystemHost, IosManagedRuntimeHost,
    IosRuntimeStorageHost, IosTerminalHost,
};
use operit_link::LinkDeviceInfo;

use super::{install_owner_media, BridgeStartup, StartupMetadata};
use crate::FlutterHostAdapters::FlutterWebVisitBridge;
use crate::FlutterOwnerCapabilities::{
    ownerBluetooth, ownerFileOpen, ownerLocation, ownerRecognizeText, ownerScreenshot,
    ownerSendNotification, FlutterSystemBindings, FlutterSystemOperationBridge,
};

/// Creates iOS hosts around one retained terminal and managed runtime instance.
pub(crate) fn create_host_context(startup: &BridgeStartup) -> Result<HostManager, String> {
    let terminal = Arc::new(IosTerminalHost::new());
    let managed = Arc::new(IosManagedRuntimeHost::new(terminal.clone()));
    let mut context = createRuntimeHostManager(
        startup.runtimeRoot.clone(),
        startup.workspaceRoot.clone(),
        Arc::new(FlutterWebVisitBridge::new()),
        managed,
    )
    .withTerminalHost(terminal);
    context.fileSystemHost = Some(Arc::new(IosFileSystemHost::fromFileOpener(Arc::new(
        ownerFileOpen,
    ))));
    let system = context
        .systemOperationHost
        .clone()
        .ok_or_else(|| "Flutter runtime requires its iOS system host".to_string())?;
    let notifications = system.clone();
    let device_info = system.clone();
    context.systemOperationHost = Some(Arc::new(FlutterSystemOperationBridge::new(
        system,
        FlutterSystemBindings {
            notificationSender: Arc::new(ownerSendNotification),
            notifications: Arc::new(move |limit, ongoing| {
                notifications.getNotifications(limit, ongoing)
            }),
            location: Arc::new(ownerLocation),
            deviceInfo: Arc::new(move || device_info.getDeviceInfo()),
            screenshot: Arc::new(ownerScreenshot),
            recognition: Arc::new(ownerRecognizeText),
        },
    )));
    Ok(
        install_owner_media(context, true).withBluetoothHost(Arc::new(
            IosBluetoothHost::fromController(Arc::new(ownerBluetooth)),
        )),
    )
}

/// Reads iOS identity through the selected system host.
pub(crate) fn startup_device_info(
    context: &HostManager,
    _metadata: &StartupMetadata,
) -> Result<LinkDeviceInfo, String> {
    let system = context
        .systemOperationHost
        .as_ref()
        .ok_or_else(|| "Runtime identity requires its iOS system host".to_string())?;
    Ok(LinkDeviceInfo {
        platform: context.hostEnvironment.id.clone(),
        model: system
            .getDeviceInfo()
            .map_err(|error| error.to_string())?
            .model,
    })
}

/// Resolves iOS storage roots from the native storage host.
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Ok((
        IosRuntimeStorageHost::defaultRuntimeRoot(),
        IosRuntimeStorageHost::defaultWorkspaceRoot(),
    ))
}

/// iOS has no process-global bridge registration to release here.
pub(crate) fn release_host() {}

impl crate::OperitFlutterBridge {
    /// Starts iOS using its native storage roots.
    pub(crate) fn new() -> Result<Self, String> {
        let (runtimeRoot, workspaceRoot) = default_native_storage_roots()?;
        Self::new_with_storage_roots(runtimeRoot, workspaceRoot)
    }

    /// Starts iOS with explicit storage roots.
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
