use std::sync::Arc;

#[cfg(target_os = "linux")]
use operit_host_linux_native::LinuxFileSystemHost as NativeFileSystemHost;
#[cfg(windows)]
use operit_host_windows_native::WindowsFileSystemHost as NativeFileSystemHost;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::core::tools::ToolPermissionSystem::PermissionRequestResult;

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("operit2 CLI host is implemented for Windows and Linux.");

pub(crate) fn create_cli_application() -> OperitApplication {
    let application = OperitApplication::newWithContext(OperitApplicationContext::withFileSystemHost(Arc::new(
        NativeFileSystemHost::new(),
    )));
    let handler = AIToolHandler::getInstance(application.applicationContext.clone());
    handler
        .getToolPermissionSystem()
        .setPermissionRequester(|_tool, _description| PermissionRequestResult::ALLOW);
    application
}
