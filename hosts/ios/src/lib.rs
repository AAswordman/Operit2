#![allow(non_snake_case)]

#[cfg(target_os = "ios")]
use std::path::PathBuf;
#[cfg(target_os = "ios")]
use std::sync::Arc;

#[cfg(target_os = "ios")]
use operit_host_api::HostManager::HostManager;
#[cfg(target_os = "ios")]
use operit_host_api::RuntimeStorageHost;

pub mod bridge;
mod managed_runtime;
pub mod terminal;

pub use managed_runtime::IosManagedRuntimeHost;
pub use operit_host_apple_native::{
    AppleAudioPlaybackHost as IosAudioPlaybackHost, AppleBluetoothHost as IosBluetoothHost,
    AppleFileSystemHost as IosFileSystemHost,
    AppleHostRuntimeEventSchedulerHost as IosHostRuntimeEventSchedulerHost,
    AppleHostJavaScriptRuntimeHost as IosHostJavaScriptRuntimeHost,
    AppleHostRuntimeTaskSchedulerHost as IosHostRuntimeTaskSchedulerHost,
    AppleHttpHost as IosHttpHost, AppleLocalInferenceCommand as IosLocalInferenceCommand,
    AppleLocalInferenceHost as IosLocalInferenceHost, AppleMusicCommand as IosMusicCommand,
    AppleRuntimeStorageHost as IosRuntimeStorageHost,
    AppleSystemOperationHost as IosSystemOperationHost,
    AppleTtsPlaybackCommand as IosTtsPlaybackCommand, AppleTtsPlaybackHost as IosTtsPlaybackHost,
    AppleTtsSynthesisHost as IosTtsSynthesisHost,
};
pub use terminal::IosTerminalHost;

/// Creates the iOS-owned runtime host manager for explicit storage roots.
#[cfg(target_os = "ios")]
pub fn createRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    managedRuntimeHost: Arc<dyn operit_host_api::ManagedRuntimeHost>,
) -> HostManager {
    let runtimeStorageWriteHost =
        Arc::new(operit_host_native_common::NativeRuntimeStorageHost::new(
            runtimeRoot.clone(),
            workspaceRoot.clone(),
        ));
    let runtimeStorageHost = Arc::new(IosRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    let archiveStagingHost = Arc::new(operit_host_native_common::NativeArchiveStagingHost::new(
        runtimeStorageHost
            .runtimeRootDir()
            .expect("iOS runtime storage root must be configured"),
    ));
    let mut hostManager = HostManager::withFileSystemWebVisitAndSystemOperationHosts(
        Arc::new(IosFileSystemHost::new()),
        webVisitHost,
        Arc::new(IosSystemOperationHost::new()),
    );
    hostManager.httpHost = Some(Arc::new(IosHttpHost::new()));
    hostManager.webSocketHost = Some(Arc::new(IosHttpHost::new()));
    hostManager.managedRuntimeHost = Some(managedRuntimeHost);
    hostManager.runtimeStorageHost = Some(runtimeStorageHost);
    hostManager.runtimeSqliteHost = Some(runtimeSqliteHost);
    hostManager = hostManager.withHostSecretStore(hostSecretStore);
    hostManager = hostManager.withArchiveStagingHost(archiveStagingHost);
    hostManager = hostManager.withRuntimeStorageWriteHost(runtimeStorageWriteHost);
    hostManager = hostManager
        .withHostRuntimeEventSchedulerHost(Arc::new(IosHostRuntimeEventSchedulerHost::new()));
    hostManager = hostManager
        .withHostJavaScriptRuntimeHost(Arc::new(IosHostJavaScriptRuntimeHost::new()));
    hostManager.withHostRuntimeTaskSchedulerHost(Arc::new(IosHostRuntimeTaskSchedulerHost::new()))
}
