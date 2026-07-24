#![allow(non_snake_case)]

#[cfg(target_os = "ios")]
use std::path::PathBuf;
#[cfg(target_os = "ios")]
use std::sync::Arc;

#[cfg(target_os = "ios")]
use operit_host_api::HostManager::HostManager;

pub mod runtime;
pub mod terminal;

pub use operit_host_apple_native::{
    AppleAudioPlaybackHost as IosAudioPlaybackHost, AppleBluetoothHost as IosBluetoothHost,
    AppleFileSystemHost as IosFileSystemHost,
    AppleHostRuntimeEventSchedulerHost as IosHostRuntimeEventSchedulerHost,
    AppleHostRuntimeTaskSchedulerHost as IosHostRuntimeTaskSchedulerHost,
    AppleHttpHost as IosHttpHost, AppleLocalInferenceCommand as IosLocalInferenceCommand,
    AppleLocalInferenceHost as IosLocalInferenceHost, AppleMusicCommand as IosMusicCommand,
    AppleRuntimeStorageHost as IosRuntimeStorageHost,
    AppleSystemOperationHost as IosSystemOperationHost,
    AppleTtsPlaybackCommand as IosTtsPlaybackCommand, AppleTtsPlaybackHost as IosTtsPlaybackHost,
    AppleTtsSynthesisHost as IosTtsSynthesisHost,
};
pub use runtime::IosManagedRuntimeHost;
pub use terminal::IosTerminalHost;

/// Creates the iOS-owned runtime host manager for explicit storage roots.
#[cfg(target_os = "ios")]
pub fn createRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
) -> HostManager {
    let runtimeStorageHost = Arc::new(IosRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(IosFileSystemHost::new()),
        webVisitHost,
        Arc::new(IosHttpHost::new()),
        Arc::new(IosSystemOperationHost::new()),
        Arc::new(IosManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore)
    .withHostRuntimeEventSchedulerHost(Arc::new(IosHostRuntimeEventSchedulerHost::new()))
    .withHostRuntimeTaskSchedulerHost(Arc::new(IosHostRuntimeTaskSchedulerHost::new()))
}
