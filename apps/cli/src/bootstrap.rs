use std::sync::Arc;

use operit_host_windows_native::WindowsFileSystemHost;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::core::tools::ToolPermissionSystem::PermissionRequestResult;

pub(crate) fn create_cli_application() -> OperitApplication {
    let application = OperitApplication::newWithContext(OperitApplicationContext::withFileSystemHost(Arc::new(
        WindowsFileSystemHost::new(),
    )));
    let handler = AIToolHandler::getInstance(application.applicationContext.clone());
    handler
        .getToolPermissionSystem()
        .setPermissionRequester(|_tool, _description| PermissionRequestResult::ALLOW);
    application
}
