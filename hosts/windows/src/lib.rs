pub mod bridge;
#[path = "../../common/chromium_browser.rs"]
pub mod chromium_browser;
#[path = "host_runtime_event.rs"]
pub mod host_runtime_event;
pub mod registry;
pub mod tools;

pub use host_runtime_event::WindowsHostRuntimeEventHost;
pub use operit_host_native_common::NativeHostRuntimeEventSchedulerHost as WindowsHostRuntimeEventSchedulerHost;
pub use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost as WindowsHostRuntimeTaskSchedulerHost;
pub use tools::audio::WindowsAudioPlaybackHost;
pub use tools::bluetooth::WindowsBluetoothHost;
pub use tools::browser::{WindowsBrowserAutomationHost, WindowsWebVisitHost};
pub use tools::fs::WindowsFileSystemHost;
pub use tools::http::WindowsHttpHost;
pub use tools::runtime::WindowsManagedRuntimeHost;
pub use tools::storage::WindowsRuntimeStorageHost;
pub use tools::system::WindowsSystemOperationHost;
pub use tools::terminal::WindowsTerminalHost;
pub use tools::tts::{WindowsTtsPlaybackHost, WindowsTtsSynthesisHost};

/// Creates the Windows-owned runtime host manager for explicit storage roots.
pub fn createRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
) -> HostManager {
    let runtimeStorageHost = Arc::new(WindowsRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(WindowsFileSystemHost::new()),
        webVisitHost,
        Arc::new(WindowsHttpHost::new()),
        Arc::new(WindowsSystemOperationHost::new()),
        Arc::new(WindowsManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore)
    .withAudioPlaybackHost(Arc::new(WindowsAudioPlaybackHost::new()))
    .withBluetoothHost(Arc::new(WindowsBluetoothHost::new()))
    .withTtsSynthesisHost(Arc::new(WindowsTtsSynthesisHost::new()))
    .withTtsPlaybackHost(Arc::new(WindowsTtsPlaybackHost::new()))
    .withHostRuntimeEventHost(Arc::new(WindowsHostRuntimeEventHost::new()))
    .withHostRuntimeEventSchedulerHost(Arc::new(WindowsHostRuntimeEventSchedulerHost::new()))
    .withHostRuntimeTaskSchedulerHost(Arc::new(WindowsHostRuntimeTaskSchedulerHost::new()))
}
use std::path::PathBuf;
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
