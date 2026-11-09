#![allow(non_snake_case)]

#[cfg(target_os = "android")]
use std::path::PathBuf;
#[cfg(target_os = "android")]
use std::sync::Arc;

#[cfg(target_os = "android")]
use operit_host_api::HostManager::HostManager;
mod audio_playback;
mod bluetooth;
mod document_filesystem;
mod filesystem;
mod http;
#[cfg(target_os = "android")]
mod jni_thread;
#[cfg(target_os = "android")]
mod local_inference;
#[cfg(target_os = "android")]
mod logging;
mod managed_runtime;
mod runtime_common;
#[cfg(target_os = "android")]
mod runtime_event_scheduler;
mod runtime_storage;
#[cfg(target_os = "android")]
mod secret_store;
#[cfg(target_os = "android")]
mod service_discovery;
mod system_operation;
mod terminal;
#[path = "../../common/terminal_command.rs"]
mod terminal_command;
mod terminal_privilege;
#[cfg(target_os = "android")]
pub use terminal::androidPrivilegedPtyOperation;
mod tts_playback;
mod tts_synthesis;
mod web_visit;

pub use audio_playback::{AndroidAudioPlaybackHost, AndroidMusicCommand};
pub use bluetooth::AndroidBluetoothHost;
pub use filesystem::AndroidFileSystemHost;
pub use http::AndroidHttpHost;
#[cfg(target_os = "android")]
pub use jni_thread::attachCurrentThread;
#[cfg(target_os = "android")]
pub use local_inference::AndroidLocalInferenceHost;
#[cfg(target_os = "android")]
pub use logging::installAndroidLogSink;
pub use managed_runtime::AndroidManagedRuntimeHost;
pub use operit_host_native_common::NativeHostJavaScriptRuntimeHost as AndroidHostJavaScriptRuntimeHost;
pub use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost as AndroidHostRuntimeTaskSchedulerHost;
#[cfg(target_os = "android")]
pub use plugin_sdk_ipc::{
    acceptAndroidPluginSdkPipes, setAndroidPluginSdkBridge, AndroidPluginSdkIpcHost,
};
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
#[cfg(target_os = "android")]
mod plugin_sdk_ipc;

/// Creates the Android-owned runtime host manager for explicit storage roots.
#[cfg(target_os = "android")]
pub fn createRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    archiveRuntimeRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    systemOperationHost: Arc<dyn operit_host_api::SystemOperationHost>,
) -> HostManager {
    let secrets = Arc::new(AndroidRuntimeStorageHost::new(
        runtimeRoot.clone(),
        workspaceRoot.clone(),
    ));
    createBaseRuntimeHostManager(
        runtimeRoot,
        workspaceRoot,
        archiveRuntimeRoot,
        webVisitHost,
        systemOperationHost,
    )
    .withHostSecretStore(secrets)
    .withLocalInferenceHost(Arc::new(AndroidLocalInferenceHost::new()))
    .withHostRuntimeEventSchedulerHost(Arc::new(AndroidHostRuntimeEventSchedulerHost::new()))
    .withServiceDiscoveryHost(Arc::new(
        operit_host_native_common::ServiceDiscovery::ServiceDiscoveryProvider::withNetworkLease(
            service_discovery::acquireMulticastLease,
        ),
    ))
    .withPluginSdkIpcHost(Arc::new(AndroidPluginSdkIpcHost::new()))
}

/// Creates the privileged Android Host composition without application-owned JNI registrations.
#[cfg(target_os = "android")]
pub fn createPrivilegedRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    secretRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    systemOperationHost: Arc<dyn operit_host_api::SystemOperationHost>,
) -> HostManager {
    createBaseRuntimeHostManager(
        runtimeRoot.clone(),
        workspaceRoot,
        runtimeRoot,
        webVisitHost,
        systemOperationHost,
    )
    .withHostSecretStore(Arc::new(operit_host_native_common::NativeFileSecretStore(
        secretRoot,
    )))
    .withHostRuntimeEventSchedulerHost(Arc::new(
        operit_host_native_common::NativeHostRuntimeEventSchedulerHost::new(),
    ))
}

/// Assembles Android capabilities shared by application and standalone module owners.
#[cfg(target_os = "android")]
fn createBaseRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    archiveRuntimeRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    systemOperationHost: Arc<dyn operit_host_api::SystemOperationHost>,
) -> HostManager {
    installAndroidLogSink();
    let runtimeStorageWriteHost =
        Arc::new(operit_host_native_common::NativeRuntimeStorageHost::new(
            runtimeRoot.clone(),
            workspaceRoot.clone(),
        ));
    let runtimeStorageHost = Arc::new(AndroidRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    // Android shared storage rejects flock with ENOSYS. Archive payloads and their
    // cross-process leases belong to the owner's private storage, even when user
    // data lives in a selected shared directory. Module runtime roots are private.
    let archiveStagingHost = Arc::new(operit_host_native_common::NativeArchiveStagingHost::new(
        archiveRuntimeRoot,
    ));
    HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(AndroidFileSystemHost::new()),
        webVisitHost,
        Arc::new(AndroidHttpHost::new()),
        systemOperationHost,
        Arc::new(AndroidManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withWebSocketHost(Arc::new(AndroidHttpHost::new()))
    .withHttpServerHost(Arc::new(operit_host_native_common::NativeHttpServerHost))
    .withTcpHost(Arc::new(operit_host_native_common::NativeTcpHost))
    .withSerialPortHost(Arc::new(operit_host_native_common::NativeSerialPortHost))
    .withArchiveStagingHost(archiveStagingHost)
    .withRuntimeStorageWriteHost(runtimeStorageWriteHost)
    .withHostJavaScriptRuntimeHost(Arc::new(AndroidHostJavaScriptRuntimeHost::new()))
    .withHostRuntimeTaskSchedulerHost(Arc::new(AndroidHostRuntimeTaskSchedulerHost::new()))
}
