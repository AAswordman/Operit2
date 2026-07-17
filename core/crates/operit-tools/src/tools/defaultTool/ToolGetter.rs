use operit_host_api::HostManager::HostManager;
use std::sync::Arc;

use crate::runtime_support::ToolRuntimeSupport;
use operit_tools::tools::defaultTool::standard::StandardBluetoothTools::StandardBluetoothTools;
use operit_tools::tools::defaultTool::standard::StandardBrowserAutomationTools::StandardBrowserAutomationTools;
use operit_tools::tools::defaultTool::standard::StandardFileSystemTools::StandardFileSystemTools;
use operit_tools::tools::defaultTool::standard::StandardHttpTools::StandardHttpTools;
use operit_tools::tools::defaultTool::standard::StandardMusicTools::StandardMusicTools;
use operit_tools::tools::defaultTool::standard::StandardSystemOperationTools::StandardSystemOperationTools;
use operit_tools::tools::defaultTool::standard::StandardTerminalTools::StandardTerminalTools;
use operit_tools::tools::defaultTool::standard::StandardWebVisitTool::StandardWebVisitTool;

/// Builds standard tool groups from the host capabilities present in an application context.
pub struct ToolGetter;

impl ToolGetter {
    /// Creates file-system tools with workspace and download support from host storage paths.
    #[allow(non_snake_case)]
    pub fn getFileSystemTools(
        context: &HostManager,
        runtimeSupport: Arc<dyn ToolRuntimeSupport>,
    ) -> Option<StandardFileSystemTools> {
        context.fileSystemHost.clone().and_then(|fileSystemHost| {
            let runtimeStorageHost = context.runtimeStorageHost.as_ref()?;
            let runtimeStoreRoot = runtimeStorageHost.runtimeRootDir()?;
            let workspaceCollectionRoot = runtimeStorageHost.workspaceRootDir()?;
            Some(StandardFileSystemTools::new(
                fileSystemHost,
                context
                    .httpHost
                    .clone()
                    .expect("HTTP host must be configured before registering file download tool"),
                context.systemOperationHost.clone(),
                runtimeStoreRoot,
                workspaceCollectionRoot,
                runtimeSupport,
            ))
        })
    }

    /// Creates HTTP tools bound to the configured HTTP host.
    #[allow(non_snake_case)]
    pub fn getHttpTools(context: &HostManager) -> StandardHttpTools {
        StandardHttpTools::new(
            context
                .httpHost
                .clone()
                .expect("HTTP host must be configured before registering HTTP tools"),
            context
                .fileSystemHost
                .clone()
                .expect("FileSystemHost must be configured before registering HTTP tools"),
        )
    }

    /// Creates the web-visit tool wrapper around the optional web host.
    #[allow(non_snake_case)]
    pub fn getWebVisitTool(context: &HostManager) -> StandardWebVisitTool {
        StandardWebVisitTool::new(
            context.webVisitHost.clone(),
            context
                .fileSystemHost
                .clone()
                .expect("FileSystemHost must be configured before registering web-visit tools"),
        )
    }

    /// Creates browser automation tools from the browser host capability.
    #[allow(non_snake_case)]
    pub fn getBrowserAutomationTools(
        context: &HostManager,
    ) -> Option<StandardBrowserAutomationTools> {
        context
            .browserAutomationHost
            .clone()
            .map(StandardBrowserAutomationTools::new)
    }

    /// Creates system-operation tools from the optional platform operation host.
    #[allow(non_snake_case)]
    pub fn getSystemOperationTools(context: &HostManager) -> StandardSystemOperationTools {
        StandardSystemOperationTools::new(context.systemOperationHost.clone())
    }

    /// Creates terminal tools from the optional terminal host.
    #[allow(non_snake_case)]
    pub fn getTerminalTools(context: &HostManager) -> StandardTerminalTools {
        StandardTerminalTools::new(context.terminalHost.clone())
    }

    /// Creates music playback tools from the optional audio playback host.
    #[allow(non_snake_case)]
    pub fn getMusicTools(context: &HostManager) -> StandardMusicTools {
        StandardMusicTools::new(context.audioPlaybackHost.clone())
    }

    /// Creates Bluetooth tools from the optional Bluetooth host.
    #[allow(non_snake_case)]
    pub fn getBluetoothTools(context: &HostManager) -> StandardBluetoothTools {
        StandardBluetoothTools::new(context.bluetoothHost.clone())
    }
}
