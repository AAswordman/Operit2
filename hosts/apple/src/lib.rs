#![allow(non_snake_case)]

pub mod tools;

pub use tools::audio::{AppleAudioPlaybackHost, AppleMusicCommand};
pub use tools::bluetooth::AppleBluetoothHost;
pub use tools::fs::AppleFileSystemHost;
pub use tools::host_runtime_event::AppleHostRuntimeEventHost;
pub use tools::http::AppleHttpHost;
pub use tools::runtime::AppleManagedRuntimeHost;
pub use tools::storage::AppleRuntimeStorageHost;
pub use tools::system::AppleSystemOperationHost;
pub use tools::terminal::AppleTerminalHost;
pub use tools::tts::{AppleTtsPlaybackCommand, AppleTtsPlaybackHost, AppleTtsSynthesisHost};
