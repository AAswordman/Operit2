#![allow(non_snake_case)]

#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "http")]
pub mod http;
#[cfg(not(target_arch = "wasm32"))]
pub mod runtime_event_scheduler;
#[cfg(feature = "storage")]
pub mod storage;
#[cfg(feature = "terminal")]
pub mod terminal;

#[cfg(feature = "fs")]
pub use fs::PosixFileSystemHost;
#[cfg(feature = "http")]
pub use http::NativeHttpHost;
#[cfg(not(target_arch = "wasm32"))]
pub use runtime_event_scheduler::NativeHostRuntimeEventSchedulerHost;
#[cfg(feature = "storage")]
pub use storage::NativeRuntimeStorageHost;
#[cfg(feature = "terminal")]
pub use terminal::NativePtyTerminalHost;
