//! Platform selection lives here so the factory file reads as one common pipeline.

use std::path::PathBuf;
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_link::LinkDeviceInfo;

use super::{BridgeStartup, StartupMetadata};

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_env = "ohos")]
mod ohos;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "android")]
use android as selected;
#[cfg(target_os = "ios")]
use ios as selected;
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
use linux as selected;
#[cfg(target_os = "macos")]
use macos as selected;
#[cfg(target_env = "ohos")]
use ohos as selected;
#[cfg(target_arch = "wasm32")]
use web as selected;
#[cfg(target_os = "windows")]
use windows as selected;

/// Creates the host bundle for the one target selected by the compiler.
pub(crate) fn create_host_context(startup: &BridgeStartup) -> Result<HostManager, String> {
    selected::create_host_context(startup)
}

/// Reads startup identity from the selected host or owner boundary.
pub(crate) fn startup_device_info(
    context: &HostManager,
    metadata: &StartupMetadata,
) -> Result<LinkDeviceInfo, String> {
    selected::startup_device_info(context, metadata)
}

/// Resolves storage roots through the selected host contract.
pub(crate) fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    selected::default_native_storage_roots()
}

/// Installs owner media capabilities shared by mobile owner integrations.
pub(crate) fn install_owner_media(context: HostManager, system_speech: bool) -> HostManager {
    use crate::FlutterOwnerCapabilities::{FlutterAudioPlaybackHost, FlutterTtsPlaybackHost};

    context
        .withAudioPlaybackHost(Arc::new(FlutterAudioPlaybackHost))
        .withTtsPlaybackHost(Arc::new(FlutterTtsPlaybackHost {
            systemSpeech: system_speech,
        }))
}

/// Releases registrations that belong to the selected host boundary.
pub(crate) fn release_host() {
    selected::release_host();
}

/// Creates a minimal secret-store adapter through the selected Host distribution.
pub(crate) fn rescue_secret_store(root: &std::path::Path) -> Result<Option<std::sync::Arc<dyn operit_host_api::HostSecretStore>>, String> {
    selected::rescue_secret_store(root)
}
