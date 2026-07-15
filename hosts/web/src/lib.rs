#![allow(non_snake_case)]

pub(crate) mod common;
pub mod runtime_event;
pub mod runtime_event_scheduler;
pub mod tools;

pub use tools::audio::WebAudioPlaybackHost;
pub use tools::bluetooth::WebBluetoothHost;
pub use tools::browser::WebWebVisitHost;
pub use tools::fs::WebFileSystemHost;
pub use tools::http::WebHttpHost;
pub use tools::local_inference::WebLocalInferenceHost;
pub use tools::runtime::WebManagedRuntimeHost;
pub use tools::storage::WebRuntimeStorageHost;
pub use tools::system::WebSystemOperationHost;
pub use tools::tts::WebTtsPlaybackHost;
pub use runtime_event_scheduler::WebHostRuntimeEventSchedulerHost;
pub use runtime_event::WebHostRuntimeEventHost;
