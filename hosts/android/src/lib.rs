#![allow(non_snake_case)]

#[cfg(target_os = "android")]
use std::path::PathBuf;
#[cfg(target_os = "android")]
use std::sync::Arc;

#[cfg(target_os = "android")]
use operit_host_api::HostManager::HostManager;

mod audio_playback;
mod bluetooth;
mod filesystem;
mod http;
#[cfg(target_os = "android")]
mod local_inference;
mod managed_runtime;
mod runtime_common;
#[cfg(target_os = "android")]
mod runtime_event_scheduler;
mod runtime_storage;
#[cfg(target_os = "android")]
mod secret_store;
mod system_operation;
mod terminal;
mod tts_playback;
mod tts_synthesis;
mod web_visit;

pub use audio_playback::{AndroidAudioPlaybackHost, AndroidMusicCommand};
pub use bluetooth::AndroidBluetoothHost;
pub use filesystem::AndroidFileSystemHost;
pub use http::AndroidHttpHost;
#[cfg(target_os = "android")]
pub use local_inference::AndroidLocalInferenceHost;
pub use managed_runtime::AndroidManagedRuntimeHost;
pub use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost as AndroidHostRuntimeTaskSchedulerHost;
#[cfg(target_os = "android")]
pub use runtime_event_scheduler::{
    emitAndroidHostRuntimeEventSchedule, AndroidHostRuntimeEventSchedulerHost,
};
pub use runtime_storage::AndroidRuntimeStorageHost;
#[cfg(target_os = "android")]
pub use secret_store::{clearAndroidHostSecretStoreBridge, setAndroidHostSecretStoreBridge};
pub use system_operation::AndroidSystemOperationHost;
pub use terminal::AndroidTerminalHost;
pub use tts_playback::{AndroidTtsPlaybackCommand, AndroidTtsPlaybackHost};
pub use tts_synthesis::AndroidTtsSynthesisHost;
pub use web_visit::AndroidWebVisitHost;

/// Creates the Android-owned runtime host manager for explicit storage roots.
#[cfg(target_os = "android")]
pub fn createRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
) -> HostManager {
    let runtimeStorageHost = Arc::new(AndroidRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(AndroidFileSystemHost::new()),
        webVisitHost,
        Arc::new(AndroidHttpHost::new()),
        Arc::new(AndroidSystemOperationHost::new()),
        Arc::new(AndroidManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore)
    .withLocalInferenceHost(Arc::new(AndroidLocalInferenceHost::new()))
    .withHostRuntimeEventSchedulerHost(Arc::new(AndroidHostRuntimeEventSchedulerHost::new()))
    .withHostRuntimeTaskSchedulerHost(Arc::new(AndroidHostRuntimeTaskSchedulerHost::new()))
}
