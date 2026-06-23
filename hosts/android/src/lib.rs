#![allow(non_snake_case)]

mod audio_playback;
mod filesystem;
mod http;
mod managed_runtime;
mod runtime_common;
mod runtime_storage;
mod system_operation;
mod terminal;
mod tts_playback;
mod tts_synthesis;
mod web_visit;

pub use audio_playback::AndroidAudioPlaybackHost;
pub use filesystem::AndroidFileSystemHost;
pub use http::AndroidHttpHost;
pub use managed_runtime::AndroidManagedRuntimeHost;
pub use runtime_storage::AndroidRuntimeStorageHost;
pub use system_operation::AndroidSystemOperationHost;
pub use terminal::AndroidTerminalHost;
pub use tts_playback::{AndroidTtsPlaybackCommand, AndroidTtsPlaybackHost};
pub use tts_synthesis::AndroidTtsSynthesisHost;
pub use web_visit::AndroidWebVisitHost;
