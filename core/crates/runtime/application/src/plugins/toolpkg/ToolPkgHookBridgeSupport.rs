use operit_host_api::HostManager::HostManager;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::AIToolHandler::AIToolHandler;

/// Holds the ToolPkg execution dependencies owned by one runtime instance.
#[derive(Clone)]
pub struct ToolPkgBridgeRuntime {
    tool_handler: AIToolHandler,
    host_manager: HostManager,
}

impl ToolPkgBridgeRuntime {
    /// Creates bridge runtime state for one application runtime.
    pub fn new(tool_handler: AIToolHandler, host_manager: HostManager) -> Self {
        Self {
            tool_handler,
            host_manager,
        }
    }

    /// Returns a snapshot of this runtime's package manager.
    pub fn package_manager(&self) -> RuntimePackageManager {
        self.tool_handler
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .clone()
    }

    /// Returns this runtime's tool handler.
    pub fn tool_handler(&self) -> AIToolHandler {
        self.tool_handler.clone()
    }

    /// Returns the host capabilities attached to this ToolPkg runtime.
    pub fn host_manager(&self) -> HostManager {
        self.host_manager.clone()
    }
}
