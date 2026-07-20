#![allow(non_snake_case)]

pub mod runtime;
pub mod terminal;

pub use operit_host_apple_native::{
    AppleAudioPlaybackHost as IosAudioPlaybackHost, AppleBluetoothHost as IosBluetoothHost,
    AppleFileSystemHost as IosFileSystemHost,
    AppleHostRuntimeEventSchedulerHost as IosHostRuntimeEventSchedulerHost,
    AppleHostRuntimeTaskSchedulerHost as IosHostRuntimeTaskSchedulerHost,
    AppleHttpHost as IosHttpHost, AppleLocalInferenceCommand as IosLocalInferenceCommand,
    AppleLocalInferenceHost as IosLocalInferenceHost, AppleMusicCommand as IosMusicCommand,
    AppleRuntimeStorageHost as IosRuntimeStorageHost,
    AppleSystemOperationHost as IosSystemOperationHost,
    AppleTtsPlaybackCommand as IosTtsPlaybackCommand, AppleTtsPlaybackHost as IosTtsPlaybackHost,
    AppleTtsSynthesisHost as IosTtsSynthesisHost,
};
pub use runtime::IosManagedRuntimeHost;
pub use terminal::IosTerminalHost;
