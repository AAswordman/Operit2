#![allow(non_snake_case)]

#[cfg(feature = "fs")]
pub use operit_host_native_filesystem::PosixFileSystemHost;
#[cfg(feature = "http")]
pub use operit_host_native_http::NativeHttpHost;
#[cfg(all(feature = "scheduler", not(target_arch = "wasm32")))]
pub use operit_host_native_scheduler::NativeHostRuntimeEventSchedulerHost;
#[cfg(all(feature = "scheduler", not(target_arch = "wasm32")))]
pub use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
#[cfg(feature = "storage")]
pub use operit_host_native_storage::NativeRuntimeStorageHost;
#[cfg(feature = "terminal")]
pub use operit_host_native_terminal::NativePtyTerminalHost;
