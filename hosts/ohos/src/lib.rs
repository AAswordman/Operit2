#![allow(non_snake_case)]

use std::path::PathBuf;
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;

mod audio_playback;
mod bluetooth;
mod filesystem;
mod http;
mod local_inference;
mod managed_runtime;
mod storage;
mod system_operation;
mod terminal;
mod tts_playback;

pub use audio_playback::{OhosAudioPlaybackHost, OhosMusicCommand};
pub use bluetooth::OhosBluetoothHost;
pub use filesystem::{OhosFileOpener, OhosFileSharer, OhosFileSystemHost};
pub use http::OhosHttpHost;
pub use local_inference::OhosLocalInferenceHost;
pub use managed_runtime::OhosManagedRuntimeHost;
pub use operit_host_native_common::NativeHostRuntimeEventSchedulerHost as OhosHostRuntimeEventSchedulerHost;
pub use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost as OhosHostRuntimeTaskSchedulerHost;
pub use storage::OhosRuntimeStorageHost;
pub use system_operation::{
    OhosLanguageReader, OhosScreenshotCapturer, OhosSystemController, OhosSystemOperationHost,
    OhosTextRecognizer,
};
pub use terminal::OhosTerminalHost;
pub use tts_playback::{OhosTtsPlaybackCommand, OhosTtsPlaybackHost};

/// Creates the OpenHarmony file host.
pub fn newOhosFileSystemHost() -> OhosFileSystemHost {
    OhosFileSystemHost::new()
}

/// Creates the OpenHarmony-owned runtime host manager for explicit storage roots.
pub fn createRuntimeHostManager(
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    fileOpener: OhosFileOpener,
    fileSharer: OhosFileSharer,
    languageReader: OhosLanguageReader,
    screenshotCapturer: OhosScreenshotCapturer,
    textRecognizer: OhosTextRecognizer,
    systemController: OhosSystemController,
) -> HostManager {
    let managedRuntimeHost = Arc::new(OhosManagedRuntimeHost::new(workspaceRoot.clone()));
    let runtimeStorageHost = Arc::new(OhosRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let systemOperationHost = Arc::new(OhosSystemOperationHost::fromOwnerCallbacks(
        languageReader,
        screenshotCapturer,
        textRecognizer,
        systemController,
    ));
    let mut context = HostManager::withFileSystemWebVisitAndSystemOperationHosts(
        Arc::new(OhosFileSystemHost::fromPlatformActions(
            fileOpener, fileSharer,
        )),
        webVisitHost,
        systemOperationHost,
    );
    context.httpHost = Some(Arc::new(OhosHttpHost::new()));
    context.managedRuntimeHost = Some(managedRuntimeHost);
    context.runtimeStorageHost = Some(runtimeStorageHost);
    context.runtimeSqliteHost = Some(runtimeSqliteHost);
    context = context.withLocalInferenceHost(Arc::new(OhosLocalInferenceHost::new()));
    context = context
        .withHostRuntimeEventSchedulerHost(Arc::new(OhosHostRuntimeEventSchedulerHost::new()));
    context =
        context.withHostRuntimeTaskSchedulerHost(Arc::new(OhosHostRuntimeTaskSchedulerHost::new()));
    context
}
