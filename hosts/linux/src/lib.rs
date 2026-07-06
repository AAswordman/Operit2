#![allow(non_snake_case)]

pub mod bridge;
pub mod registry;
pub mod tools;
#[path = "../../common/chromium_browser.rs"]
pub mod chromium_browser;
#[cfg(target_os = "linux")]
#[path = "host_runtime_event.rs"]
pub mod host_runtime_event;

#[cfg(target_os = "linux")]
pub use host_runtime_event::LinuxHostRuntimeEventHost;
pub use tools::audio::LinuxAudioPlaybackHost;
pub use tools::bluetooth::LinuxBluetoothHost;
pub use tools::tts::{LinuxTtsPlaybackHost, LinuxTtsSynthesisHost};
pub use tools::browser::{LinuxBrowserAutomationHost, LinuxWebVisitHost};
pub use tools::fs::LinuxFileSystemHost;
pub use tools::http::LinuxHttpHost;
pub use tools::runtime::LinuxManagedRuntimeHost;
pub use tools::storage::LinuxRuntimeStorageHost;
pub use tools::system::LinuxSystemOperationHost;
pub use tools::terminal::LinuxTerminalHost;
