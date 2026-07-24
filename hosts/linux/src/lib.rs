#![allow(non_snake_case)]

#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::sync::Arc;

#[cfg(target_os = "linux")]
use operit_host_api::HostManager::HostManager;

pub mod bridge;
#[path = "../../common/chromium_browser.rs"]
pub mod chromium_browser;
#[cfg(target_os = "linux")]
#[path = "host_runtime_event.rs"]
pub mod host_runtime_event;
pub mod registry;
pub mod tools;

#[cfg(target_os = "linux")]
pub use host_runtime_event::LinuxHostRuntimeEventHost;
#[cfg(target_os = "linux")]
pub use operit_host_native_common::NativeHostRuntimeEventSchedulerHost as LinuxHostRuntimeEventSchedulerHost;
#[cfg(target_os = "linux")]
pub use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost as LinuxHostRuntimeTaskSchedulerHost;
pub use tools::audio::LinuxAudioPlaybackHost;
pub use tools::bluetooth::LinuxBluetoothHost;
pub use tools::browser::{LinuxBrowserAutomationHost, LinuxWebVisitHost};
pub use tools::fs::LinuxFileSystemHost;
pub use tools::http::LinuxHttpHost;
pub use tools::runtime::LinuxManagedRuntimeHost;
pub use tools::storage::LinuxRuntimeStorageHost;
pub use tools::system::LinuxSystemOperationHost;
pub use tools::terminal::LinuxTerminalHost;
pub use tools::tts::{LinuxTtsPlaybackHost, LinuxTtsSynthesisHost};

/// Creates the Linux-owned runtime host manager for explicit storage roots.
#[cfg(target_os = "linux")]
pub fn createRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
) -> HostManager {
    let runtimeStorageHost = Arc::new(LinuxRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(LinuxFileSystemHost::new()),
        webVisitHost,
        Arc::new(LinuxHttpHost::new()),
        Arc::new(LinuxSystemOperationHost::new()),
        Arc::new(LinuxManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore)
    .withAudioPlaybackHost(Arc::new(LinuxAudioPlaybackHost::new()))
    .withBluetoothHost(Arc::new(LinuxBluetoothHost::new()))
    .withTtsSynthesisHost(Arc::new(LinuxTtsSynthesisHost::new()))
    .withTtsPlaybackHost(Arc::new(LinuxTtsPlaybackHost::new()))
    .withHostRuntimeEventHost(Arc::new(LinuxHostRuntimeEventHost::new()))
    .withHostRuntimeEventSchedulerHost(Arc::new(LinuxHostRuntimeEventSchedulerHost::new()))
    .withHostRuntimeTaskSchedulerHost(Arc::new(LinuxHostRuntimeTaskSchedulerHost::new()))
}
