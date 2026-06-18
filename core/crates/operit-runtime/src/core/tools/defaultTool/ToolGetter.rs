use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::defaultTool::standard::StandardBrowserAutomationTools::StandardBrowserAutomationTools;
use crate::core::tools::defaultTool::standard::StandardFileSystemTools::StandardFileSystemTools;
use crate::core::tools::defaultTool::standard::StandardHttpTools::StandardHttpTools;
use crate::core::tools::defaultTool::standard::StandardSystemOperationTools::StandardSystemOperationTools;
use crate::core::tools::defaultTool::standard::StandardTerminalTools::StandardTerminalTools;
use crate::core::tools::defaultTool::standard::StandardWebVisitTool::StandardWebVisitTool;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

pub struct ToolGetter;

impl ToolGetter {
    #[allow(non_snake_case)]
    pub fn getFileSystemTools(
        context: &OperitApplicationContext,
    ) -> Option<StandardFileSystemTools> {
        context.fileSystemHost.clone().and_then(|fileSystemHost| {
            let runtimeStoreRoot = context.runtimeStorageHost.as_ref()?.rootDir()?;
            let runtimeStorePaths = RuntimeStorePaths::new(runtimeStoreRoot.clone());
            Some(StandardFileSystemTools::new(
                fileSystemHost,
                context
                    .httpHost
                    .clone()
                    .expect("HTTP host must be configured before registering file download tool"),
                context.systemOperationHost.clone(),
                runtimeStoreRoot,
                context.appFilesRoot.clone(),
                runtimeStorePaths.workspace_dir(),
            ))
        })
    }

    #[allow(non_snake_case)]
    pub fn getHttpTools(context: &OperitApplicationContext) -> StandardHttpTools {
        StandardHttpTools::new(
            context
                .httpHost
                .clone()
                .expect("HTTP host must be configured before registering HTTP tools"),
            context.fileSystemHost.clone(),
        )
    }

    #[allow(non_snake_case)]
    pub fn getWebVisitTool(context: &OperitApplicationContext) -> StandardWebVisitTool {
        StandardWebVisitTool::new(context.webVisitHost.clone())
    }

    #[allow(non_snake_case)]
    pub fn getBrowserAutomationTools(
        context: &OperitApplicationContext,
    ) -> Option<StandardBrowserAutomationTools> {
        context
            .browserAutomationHost
            .clone()
            .map(StandardBrowserAutomationTools::new)
    }

    #[allow(non_snake_case)]
    pub fn getSystemOperationTools(
        context: &OperitApplicationContext,
    ) -> StandardSystemOperationTools {
        StandardSystemOperationTools::new(context.systemOperationHost.clone())
    }

    #[allow(non_snake_case)]
    pub fn getTerminalTools(context: &OperitApplicationContext) -> StandardTerminalTools {
        StandardTerminalTools::new(context.terminalHost.clone())
    }
}
