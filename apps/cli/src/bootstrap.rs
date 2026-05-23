use std::sync::Arc;

#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxFileSystemHost as NativeFileSystemHost,
    LinuxManagedRuntimeHost as NativeManagedRuntimeHost,
    LinuxSystemOperationHost as NativeSystemOperationHost,
    LinuxWebVisitHost as NativeWebVisitHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
    WindowsFileSystemHost as NativeFileSystemHost,
    WindowsManagedRuntimeHost as NativeManagedRuntimeHost,
    WindowsSystemOperationHost as NativeSystemOperationHost,
    WindowsWebVisitHost as NativeWebVisitHost,
};
use operit_link::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("operit2 CLI host is implemented for Windows and Linux.");

pub(crate) fn create_cli_application() -> OperitApplication {
    OperitApplication::newWithContext(
        OperitApplicationContext::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
            Arc::new(NativeFileSystemHost::new()),
            Arc::new(NativeWebVisitHost::new()),
            Arc::new(NativeSystemOperationHost::new()),
            Arc::new(NativeManagedRuntimeHost::new()),
        ),
    )
}

pub(crate) fn create_local_core() -> LocalCoreProxy {
    LocalCoreProxy::new(create_cli_application())
}
