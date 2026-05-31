use std::sync::Arc;

use operit_core_proxy::LocalCoreProxy;
#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxFileSystemHost as NativeFileSystemHost, LinuxHttpHost as NativeHttpHost,
    LinuxManagedRuntimeHost as NativeManagedRuntimeHost,
    LinuxRuntimeStorageHost as NativeRuntimeStorageHost,
    LinuxSystemOperationHost as NativeSystemOperationHost, LinuxTerminalHost as NativeTerminalHost,
    LinuxWebVisitHost as NativeWebVisitHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
    WindowsFileSystemHost as NativeFileSystemHost, WindowsHttpHost as NativeHttpHost,
    WindowsManagedRuntimeHost as NativeManagedRuntimeHost,
    WindowsRuntimeStorageHost as NativeRuntimeStorageHost,
    WindowsSystemOperationHost as NativeSystemOperationHost,
    WindowsTerminalHost as NativeTerminalHost, WindowsWebVisitHost as NativeWebVisitHost,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("operit2 CLI host is implemented for Windows and Linux.");

pub(crate) fn create_cli_application() -> OperitApplication {
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(
        NativeRuntimeStorageHost::defaultRoot(),
    ));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let mut context =
        OperitApplicationContext::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
            Arc::new(NativeFileSystemHost::new()),
            Arc::new(NativeWebVisitHost::new()),
            Arc::new(NativeHttpHost::new()),
            Arc::new(NativeSystemOperationHost::new()),
            Arc::new(NativeManagedRuntimeHost::new()),
            runtimeStorageHost,
            runtimeSqliteHost,
        );
    #[cfg(any(target_os = "linux", windows))]
    {
        context = context.withTerminalHost(Arc::new(NativeTerminalHost::new()));
    }
    let commandContext = context.clone();
    OperitApplication::newWithContext(context.withCoreCommandExecutor(Arc::new(move |args| {
        let output =
            operit_command_core::run_core_command_with_context(commandContext.clone(), &args)?;
        Ok(output.stdout)
    })))
}

pub(crate) fn create_local_core() -> LocalCoreProxy {
    LocalCoreProxy::new(create_cli_application())
}
