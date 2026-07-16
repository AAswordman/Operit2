#![allow(non_snake_case)]

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
pub use filesystem::OhosFileSystemHost;
pub use http::OhosHttpHost;
pub use local_inference::OhosLocalInferenceHost;
pub use managed_runtime::OhosManagedRuntimeHost;
pub use operit_host_native_common::NativeHostRuntimeEventSchedulerHost as OhosHostRuntimeEventSchedulerHost;
pub use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost as OhosHostRuntimeTaskSchedulerHost;
pub use storage::OhosRuntimeStorageHost;
pub use system_operation::OhosSystemOperationHost;
pub use terminal::OhosTerminalHost;
pub use tts_playback::{OhosTtsPlaybackCommand, OhosTtsPlaybackHost};

/// Creates the OpenHarmony file host.
pub fn newOhosFileSystemHost() -> OhosFileSystemHost {
    OhosFileSystemHost::new()
}
