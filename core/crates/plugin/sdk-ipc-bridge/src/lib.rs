//! Application wiring for the external Plugin SDK IPC server.

use std::sync::Arc;

use operit_host_api::{HostRuntimeTaskSchedulerHost, PluginSdkIpcEndpoint, PluginSdkIpcHost};
use operit_plugin_sdk_ipc::{PluginSdkIpcServer, PluginSdkLinkTarget, PluginSdkSurface};

/// Builds the SDK IPC server around an existing Core Link target.
pub struct OperitPluginSdkIpcBridge {
    server: PluginSdkIpcServer,
}

impl OperitPluginSdkIpcBridge {
    /// Creates the platform SDK bridge without introducing a tool-specific handler.
    pub fn new(
        host: Arc<dyn PluginSdkIpcHost>,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
        endpoint: PluginSdkIpcEndpoint,
        target: Arc<dyn PluginSdkLinkTarget>,
        surface: PluginSdkSurface,
    ) -> Self {
        Self {
            server: PluginSdkIpcServer::new(host, scheduler, endpoint, target, surface),
        }
    }

    /// Starts the SDK IPC listener owned by the running Core application.
    pub fn start(&self) -> Result<(), operit_link::CoreLinkError> {
        self.server.start()
    }

    /// Stops the SDK IPC listener owned by the running Core application.
    pub fn stop(&self) -> Result<(), operit_link::CoreLinkError> {
        self.server.stop()
    }
}
