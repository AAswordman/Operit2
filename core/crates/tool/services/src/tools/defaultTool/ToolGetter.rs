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
        context.vfs().ok().and_then(|filesystem| {
            let runtimeStorageHost = context.runtimeStorageHost.as_ref()?;
            Some(StandardFileSystemTools::new(
                filesystem,
                context
                    .httpHost
                    .clone()
                    .expect("HTTP host must be configured before registering file download tool"),
                context.systemOperationHost.clone(),
                runtimeStorageHost.clone(),
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
            Self::getVirtualFileSystem(context),
            context.runtimeStorageHost.clone()
                .expect("Runtime storage must be configured before registering HTTP tools"),
        )
    }

    /// Creates the web-visit tool wrapper around the optional web host.
    #[allow(non_snake_case)]
    pub fn getWebVisitTool(context: &HostManager) -> StandardWebVisitTool {
        StandardWebVisitTool::new(
            context.webVisitHost.clone(),
            Self::getVirtualFileSystem(context),
        )
    }

    /// Binds file consumers to the owning runtime's VFS and registered mounts.
    #[allow(non_snake_case)]
    fn getVirtualFileSystem(context: &HostManager) -> operit_vfs::VisualFileSystem::VisualFileSystem {
        (context).vfs()
            .expect("Runtime VFS must be configured before registering file consumers")
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
    pub fn getSystemOperationTools(
        context: &HostManager,
        runtimeSupport: Arc<dyn ToolRuntimeSupport>,
    ) -> StandardSystemOperationTools {
        StandardSystemOperationTools::new(
            context.systemOperationHost.clone(),
            context.toastHost.clone(),
            context.clipboardHost.clone(),
            Self::getVirtualFileSystem(context),
            crate::files::FileToolRouting::FileToolRouting::new(
                Self::getVirtualFileSystem(context),
                context.runtimeStorageHost.clone()
                    .expect("Runtime storage must be configured before registering system tools"),
                crate::files::RemoteFileGateway::toolRuntimeFileGateway(runtimeSupport),
            ),
        )
    }

    /// Creates terminal tools from the optional terminal host.
    #[allow(non_snake_case)]
    pub fn getTerminalTools(context: &HostManager) -> StandardTerminalTools {
        let tools = StandardTerminalTools::new(context.terminalHost.clone());
        let Some(storage) = context.runtimeStorageHost.as_ref() else { return tools; };
        let tools = tools.withStorage(storage.clone());
        tools.withVfs(context.vfs().expect("Runtime VFS must be configured for terminal tools"))
    }

    /// Creates music playback tools from the optional audio playback host.
    #[allow(non_snake_case)]
    pub fn getMusicTools(context: &HostManager) -> StandardMusicTools {
        StandardMusicTools::new(context.audioPlaybackHost.clone(), context.vfs().expect("Music tools require VFS"), context.runtimeStorageHost.clone().expect("Music tools require storage"))
    }

    /// Creates Bluetooth tools from the optional Bluetooth host.
    #[allow(non_snake_case)]
    pub fn getBluetoothTools(context: &HostManager) -> StandardBluetoothTools {
        StandardBluetoothTools::new(context.bluetoothHost.clone())
    }
}
