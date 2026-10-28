#![allow(non_snake_case)]

#[cfg(target_os = "macos")]
#[path = "../../common/chromium_browser.rs"]
pub mod chromium_browser;
pub mod tools;
#[cfg(target_vendor = "apple")]
mod service_discovery;
#[cfg(target_vendor = "apple")]
pub use service_discovery::AppleServiceDiscoveryHost;
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod logging;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use logging::installAppleLogSink;

pub use operit_host_native_common::NativeHostJavaScriptRuntimeHost as AppleHostJavaScriptRuntimeHost;
pub use operit_host_native_common::NativeHostRuntimeEventSchedulerHost as AppleHostRuntimeEventSchedulerHost;
pub use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost as AppleHostRuntimeTaskSchedulerHost;
pub use tools::audio::{AppleAudioPlaybackHost, AppleMusicCommand};
pub use tools::bluetooth::AppleBluetoothHost;
#[cfg(target_os = "macos")]
pub use tools::browser::{AppleBrowserAutomationHost, AppleWebVisitHost};
pub use tools::fs::AppleFileSystemHost;
pub use tools::host_runtime_event::AppleHostRuntimeEventHost;
pub use tools::http::AppleHttpHost;
pub use tools::local_inference::{AppleLocalInferenceCommand, AppleLocalInferenceHost};
#[cfg(target_os = "macos")]
pub use tools::runtime::AppleManagedRuntimeHost;
pub use tools::storage::AppleRuntimeStorageHost;
pub use tools::system::AppleSystemOperationHost;
pub use tools::terminal::AppleTerminalHost;
pub use tools::tts::{AppleTtsPlaybackCommand, AppleTtsPlaybackHost, AppleTtsSynthesisHost};

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use plugin_sdk_ipc::UnixPluginSdkIpcHost as ApplePluginSdkIpcHost;
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod plugin_sdk_ipc;
