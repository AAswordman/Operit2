#![allow(non_snake_case)]

use operit_host_api::HostEnvironmentDescriptor;

pub use operit_host_native_common::NativeHttpHost as OhosHttpHost;
pub use operit_host_native_common::NativePtyTerminalHost as OhosTerminalHost;
pub use operit_host_native_common::NativeRuntimeStorageHost as OhosRuntimeStorageHost;

pub type OhosFileSystemHost = operit_host_native_common::PosixFileSystemHost;

/// Creates the OpenHarmony POSIX file host.
pub fn newOhosFileSystemHost() -> OhosFileSystemHost {
    OhosFileSystemHost::newForEnvironment("ohos", HostEnvironmentDescriptor::ohos())
}
