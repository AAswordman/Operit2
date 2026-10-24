#![allow(non_snake_case)]
mod data_rescue;
pub use data_rescue::executeDataRescue;

use std::sync::Arc;

use operit_host_api::HostManager::HostManager;

pub(crate) mod common;
pub mod javascript_runtime;
pub mod runtime_event;
pub mod runtime_event_scheduler;
pub mod runtime_task_scheduler;
pub mod tools;

pub use javascript_runtime::WebHostJavaScriptRuntimeHost;
pub use runtime_event::WebHostRuntimeEventHost;
pub use runtime_event_scheduler::WebHostRuntimeEventSchedulerHost;
pub use runtime_task_scheduler::WebHostRuntimeTaskSchedulerHost;
pub use tools::archive::WebArchiveStagingHost;
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
pub use tools::websocket::WebWebSocketHost;

/// Creates the browser-owned host bundle without constructing a second Core tree.
pub fn createRuntimeHostManager() -> HostManager {
    let runtimeStorageHost = Arc::new(WebRuntimeStorageHost::new());
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    let runtimeStorageWriteHost = runtimeStorageHost.clone();
    let archiveStagingHost = Arc::new(WebArchiveStagingHost::new());
    let systemOperationHost = Arc::new(WebSystemOperationHost::new());
    let mut context = HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(WebFileSystemHost::new()),
        Arc::new(WebWebVisitHost::new()),
        Arc::new(WebHttpHost::new()),
        systemOperationHost.clone(),
        Arc::new(WebManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withToastHost(systemOperationHost)
    .withHostSecretStore(hostSecretStore)
    .withRuntimeStorageWriteHost(runtimeStorageWriteHost)
    .withArchiveStagingHost(archiveStagingHost);
    context = context.withBrowserSessionHost(Arc::new(WebBrowserSessionHost::new()));
    context = context.withWebSocketHost(Arc::new(WebWebSocketHost::new()));
    context = context.withTerminalHost(Arc::new(WebTerminalHost::new()));
    context = context.withAudioPlaybackHost(Arc::new(WebAudioPlaybackHost::new()));
    context = context.withBluetoothHost(Arc::new(WebBluetoothHost::new()));
    context = context.withLocalInferenceHost(Arc::new(WebLocalInferenceHost::new()));
    context = context.withTtsPlaybackHost(Arc::new(WebTtsPlaybackHost::new()));
    context = context
        .withHostRuntimeEventSchedulerHost(Arc::new(WebHostRuntimeEventSchedulerHost::new()));
    context =
        context.withHostRuntimeTaskSchedulerHost(Arc::new(WebHostRuntimeTaskSchedulerHost::new()));
    context = context.withHostJavaScriptRuntimeHost(Arc::new(WebHostJavaScriptRuntimeHost::new()));
    context = context.withHostRuntimeEventHost(Arc::new(WebHostRuntimeEventHost::new()));
    context
}
