#![allow(non_snake_case)]

#[cfg(target_os = "macos")]
pub use operit_host_apple_native::AppleServiceDiscoveryHost as MacosServiceDiscoveryHost;

#[cfg(all(feature = "daemon", target_os = "macos"))]
#[path = "../../unix/daemon.rs"]
pub mod daemon;

#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::Arc;

#[cfg(target_os = "macos")]
use operit_host_api::HostManager::HostManager;

pub use operit_host_apple_native::{
    AppleAudioPlaybackHost as MacosAudioPlaybackHost, AppleBluetoothHost as MacosBluetoothHost,
    AppleFileSystemHost as MacosFileSystemHost,
    AppleHostJavaScriptRuntimeHost as MacosHostJavaScriptRuntimeHost,
    AppleHostRuntimeEventHost as MacosHostRuntimeEventHost,
    AppleHostRuntimeEventSchedulerHost as MacosHostRuntimeEventSchedulerHost,
    AppleHostRuntimeTaskSchedulerHost as MacosHostRuntimeTaskSchedulerHost,
    AppleHttpHost as MacosHttpHost, AppleLocalInferenceCommand as MacosLocalInferenceCommand,
    AppleLocalInferenceHost as MacosLocalInferenceHost, AppleMusicCommand as MacosMusicCommand,
    AppleRuntimeStorageHost as MacosRuntimeStorageHost,
    AppleSystemOperationHost as MacosSystemOperationHost, AppleTerminalHost as MacosTerminalHost,
    AppleTtsPlaybackCommand as MacosTtsPlaybackCommand,
    AppleTtsPlaybackHost as MacosTtsPlaybackHost, AppleTtsSynthesisHost as MacosTtsSynthesisHost,
};

pub use operit_host_apple_native::ApplePluginSdkIpcHost as MacosPluginSdkIpcHost;

#[cfg(target_os = "macos")]
pub use operit_host_apple_native::{
    AppleBrowserAutomationHost as MacosBrowserAutomationHost,
    AppleManagedRuntimeHost as MacosManagedRuntimeHost, AppleWebVisitHost as MacosWebVisitHost,
};

/// Creates the macOS-owned runtime host manager for explicit storage roots.
#[cfg(target_os = "macos")]
pub fn createRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
) -> HostManager {
    let archiveStagingHost = Arc::new(operit_host_native_common::NativeArchiveStagingHost::new(
        runtimeRoot.clone(),
    ));
    let runtimeStorageHost = Arc::new(MacosRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
    let runtimeStorageWriteHost = runtimeStorageHost.clone();
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(MacosFileSystemHost::new()),
        webVisitHost,
        Arc::new(MacosHttpHost::new()),
        Arc::new(MacosSystemOperationHost::new()),
        Arc::new(MacosManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore)
    .withWebSocketHost(Arc::new(MacosHttpHost::new()))
    .withHttpServerHost(Arc::new(operit_host_native_common::NativeHttpServerHost))
    .withTcpHost(Arc::new(operit_host_native_common::NativeTcpHost))
    .withSerialPortHost(Arc::new(operit_host_native_common::NativeSerialPortHost))
    .withArchiveStagingHost(archiveStagingHost)
    .withRuntimeStorageWriteHost(runtimeStorageWriteHost)
    .withHostRuntimeEventHost(Arc::new(MacosHostRuntimeEventHost::new()))
    .withHostRuntimeEventSchedulerHost(Arc::new(MacosHostRuntimeEventSchedulerHost::new()))
    .withHostJavaScriptRuntimeHost(Arc::new(MacosHostJavaScriptRuntimeHost::new()))
    .withServiceDiscoveryHost(Arc::new(operit_host_apple_native::AppleServiceDiscoveryHost::default()))
    .withHostRuntimeTaskSchedulerHost(Arc::new(MacosHostRuntimeTaskSchedulerHost::new()))
    .withPluginSdkIpcHost(Arc::new(MacosPluginSdkIpcHost::new()))
}
