#![allow(non_snake_case)]

pub mod DartPort;

#[cfg(all(feature = "serial", not(any(target_os = "ios", target_os = "espidf", target_arch = "wasm32"))))]
mod SerialPort;
#[cfg(all(feature = "serial", not(any(target_os = "ios", target_os = "espidf", target_arch = "wasm32"))))]
pub use SerialPort::NativeSerialPortHost;

#[cfg(feature = "fs")]
pub use operit_host_native_filesystem::PosixFileSystemHost;
#[cfg(feature = "http")]
pub use operit_host_native_http::NativeHttpHost;
#[cfg(all(feature = "scheduler", not(target_arch = "wasm32")))]
pub use operit_host_native_scheduler::NativeHostJavaScriptRuntimeHost;
#[cfg(all(feature = "scheduler", not(target_arch = "wasm32")))]
pub use operit_host_native_scheduler::NativeHostRuntimeEventSchedulerHost;
#[cfg(all(feature = "scheduler", not(target_arch = "wasm32")))]
pub use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
#[cfg(feature = "storage")]
pub use operit_host_native_storage::NativeArchiveStagingHost;
#[cfg(feature = "storage")]
pub use operit_host_native_storage::NativeRuntimeStorageHost;
#[cfg(feature = "terminal")]
pub use operit_host_native_terminal::{NativePtyShellCommand, NativePtyTerminalHost};
#[cfg(feature = "terminal")]
mod ManagedRuntimePty;
#[cfg(feature = "terminal")]
pub use ManagedRuntimePty::{TerminalManagedRuntimeLaunch, TerminalManagedRuntimeProcess};

#[cfg(not(any(target_os = "espidf", target_arch = "wasm32")))]
pub mod ServiceDiscovery;
