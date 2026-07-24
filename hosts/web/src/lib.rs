#![allow(non_snake_case)]

use std::path::PathBuf;
use std::sync::Arc;

use operit_core_proxy::LocalCoreProxy;
use operit_host_api::HostManager::HostManager;
use operit_runtime::core::application::OperitApplication::OperitApplication;

pub(crate) mod common;
pub mod runtime_event;
pub mod runtime_event_scheduler;
pub mod runtime_task_scheduler;
pub mod tools;

pub use runtime_event::WebHostRuntimeEventHost;
pub use runtime_event_scheduler::WebHostRuntimeEventSchedulerHost;
pub use runtime_task_scheduler::WebHostRuntimeTaskSchedulerHost;
pub use tools::audio::WebAudioPlaybackHost;
pub use tools::bluetooth::WebBluetoothHost;
pub use tools::browser::WebWebVisitHost;
pub use tools::browser_session::WebBrowserSessionHost;
pub use tools::fs::WebFileSystemHost;
pub use tools::http::WebHttpHost;
pub use tools::local_inference::WebLocalInferenceHost;
pub use tools::runtime::WebManagedRuntimeHost;
pub use tools::storage::WebRuntimeStorageHost;
pub use tools::system::WebSystemOperationHost;
pub use tools::terminal::WebTerminalHost;
pub use tools::tts::WebTtsPlaybackHost;

/// Creates the Web runtime with the Web host capability bundle.
pub fn createLocalCore(
    _runtimeRoot: PathBuf,
    _workspaceRoot: PathBuf,
    _webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    _browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    _browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    _composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
) -> Result<LocalCoreProxy, String> {
    let runtimeStorageHost = Arc::new(WebRuntimeStorageHost::new());
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    let mut context = HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(WebFileSystemHost::new()),
        Arc::new(WebWebVisitHost::new()),
        Arc::new(WebHttpHost::new()),
        Arc::new(WebSystemOperationHost::new()),
        Arc::new(WebManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore);
    context = context.withBrowserSessionHost(Arc::new(WebBrowserSessionHost::new()));
    context = context.withTerminalHost(Arc::new(WebTerminalHost::new()));
    context = context.withAudioPlaybackHost(Arc::new(WebAudioPlaybackHost::new()));
    context = context.withBluetoothHost(Arc::new(WebBluetoothHost::new()));
    context = context.withLocalInferenceHost(Arc::new(WebLocalInferenceHost::new()));
    context = context.withTtsPlaybackHost(Arc::new(WebTtsPlaybackHost::new()));
    context = context
        .withHostRuntimeEventSchedulerHost(Arc::new(WebHostRuntimeEventSchedulerHost::new()));
    context =
        context.withHostRuntimeTaskSchedulerHost(Arc::new(WebHostRuntimeTaskSchedulerHost::new()));
    context = context.withHostRuntimeEventHost(Arc::new(WebHostRuntimeEventHost::new()));
    Ok(LocalCoreProxy::new(OperitApplication::newWithContext(
        context,
    )))
}
