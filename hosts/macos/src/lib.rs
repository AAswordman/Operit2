#![allow(non_snake_case)]

pub use operit_host_apple_native::{
    AppleAudioPlaybackHost as MacosAudioPlaybackHost,
    AppleBluetoothHost as MacosBluetoothHost,
    AppleBrowserAutomationHost as MacosBrowserAutomationHost,
    AppleFileSystemHost as MacosFileSystemHost,
    AppleHostRuntimeEventHost as MacosHostRuntimeEventHost,
    AppleHostRuntimeEventSchedulerHost as MacosHostRuntimeEventSchedulerHost,
    AppleHostRuntimeTaskSchedulerHost as MacosHostRuntimeTaskSchedulerHost,
    AppleHttpHost as MacosHttpHost, AppleLocalInferenceCommand as MacosLocalInferenceCommand,
    AppleLocalInferenceHost as MacosLocalInferenceHost,
    AppleManagedRuntimeHost as MacosManagedRuntimeHost, AppleMusicCommand as MacosMusicCommand,
    AppleRuntimeStorageHost as MacosRuntimeStorageHost,
    AppleSystemOperationHost as MacosSystemOperationHost, AppleTerminalHost as MacosTerminalHost,
    AppleTtsPlaybackCommand as MacosTtsPlaybackCommand,
    AppleTtsPlaybackHost as MacosTtsPlaybackHost,
    AppleTtsSynthesisHost as MacosTtsSynthesisHost, AppleWebVisitHost as MacosWebVisitHost,
};
