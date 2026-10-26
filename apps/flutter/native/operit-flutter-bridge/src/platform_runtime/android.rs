//! Android host assembly for the Flutter bridge.

use std::path::PathBuf;
use std::sync::Arc;

use operit_host_android_native::{
    createRuntimeHostManager, AndroidBluetoothHost, AndroidSystemOperationHost, AndroidTerminalHost,
};
use operit_host_api::HostManager::HostManager;
use operit_host_api::SystemOperationHost;
use operit_link::LinkDeviceInfo;

use super::{install_owner_media, BridgeStartup, StartupMetadata};
use crate::FlutterHostAdapters::FlutterWebVisitBridge;
use crate::FlutterOwnerCapabilities::{
    ownerBluetooth, ownerDeviceInfo, ownerGetSystemSetting, ownerLocation, ownerModifySystemSetting,
    ownerNotifications, ownerRecognizeText, ownerScreenshot, ownerSendNotification,
    FlutterSystemBindings, FlutterSystemOperationBridge, FlutterTtsSynthesisHost,
};

/// Creates Android hosts and binds operations owned by the Flutter application.
pub(crate) fn create_host_context(startup: &BridgeStartup) -> Result<HostManager, String> {
    let mut context = createRuntimeHostManager(
        startup.runtimeRoot.clone(),
        startup.workspaceRoot.clone(),
        Arc::new(FlutterWebVisitBridge::new()),
        Arc::new(AndroidSystemOperationHost::new(
            Arc::new(ownerGetSystemSetting),
            Arc::new(ownerModifySystemSetting),
        )),
    )
    .withTerminalHost(Arc::new(AndroidTerminalHost::new()));
    let system = context
        .systemOperationHost
        .clone()
        .ok_or_else(|| "Flutter runtime requires its Android system host".to_string())?;
    context.systemOperationHost = Some(Arc::new(FlutterSystemOperationBridge::new(
        system,
        FlutterSystemBindings {
            notificationSender: Arc::new(ownerSendNotification),
            notifications: Arc::new(ownerNotifications),
            location: Arc::new(ownerLocation),
            deviceInfo: Arc::new(ownerDeviceInfo),
            screenshot: Arc::new(ownerScreenshot),
            recognition: Arc::new(ownerRecognizeText),
        },
    )));
    Ok(install_owner_media(context, true)
        .withBluetoothHost(Arc::new(AndroidBluetoothHost::fromController(Arc::new(
            ownerBluetooth,
        ))))
        .withTtsSynthesisHost(Arc::new(FlutterTtsSynthesisHost)))
}

/// Uses the Android owner identity supplied before Flutter subscriptions exist.
pub(crate) fn startup_device_info(
    context: &HostManager,
    metadata: &StartupMetadata,
) -> Result<LinkDeviceInfo, String> {
    let StartupMetadata::AndroidDevice { model } = metadata else {
        return Err("Android startup requires owner-supplied device identity".to_string());
    };
    if model.trim().is_empty() {
        return Err("Android startup device model must not be empty".to_string());
    }
    Ok(LinkDeviceInfo {
        platform: context.hostEnvironment.id.clone(),
        model: model.clone(),
    pairingIncarnation: None,
})
}

/// Requires Android to receive storage roots from its application owner.
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Err("The Android owner must supply its application storage roots".to_string())
}

/// Releases Android registrations owned by the host boundary.
pub(crate) fn release_host() {
    crate::PlatformRuntimeAbi::AndroidJni::release_work_presentation();
    operit_host_android_native::clearAndroidHostSecretStoreBridge();
}

impl crate::OperitFlutterBridge {
    /// Starts Android using storage roots and identity supplied by its owner.
    pub(crate) fn new_with_storage_roots(
        runtimeRoot: PathBuf,
        workspaceRoot: PathBuf,
        model: String,
    ) -> Result<Self, String> {
        crate::PlatformRuntimeFactory::startBridge(BridgeStartup {
            runtimeRoot,
            workspaceRoot,
            metadata: StartupMetadata::AndroidDevice { model },
        })
    }
}

/// Creates only the platform secret-store adapter without starting Core or owner services.
pub(crate) fn rescue_secret_store(default_runtime: &std::path::Path) -> Result<Option<std::sync::Arc<dyn operit_host_api::HostSecretStore>>, String> {
    Ok(Some(std::sync::Arc::new(operit_host_android_native::AndroidRuntimeStorageHost::new(
        default_runtime.to_path_buf(), default_runtime.with_file_name("workspaces"),
    ))))
}
