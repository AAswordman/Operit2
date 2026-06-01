use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::defaultTool::standard::StandardBrowserAutomationTools::StandardBrowserAutomationTools;
use crate::core::tools::defaultTool::standard::StandardFileSystemTools::StandardFileSystemTools;
use crate::core::tools::defaultTool::standard::StandardHttpTools::StandardHttpTools;
use crate::core::tools::defaultTool::standard::StandardSystemOperationTools::StandardSystemOperationTools;
use crate::core::tools::defaultTool::standard::StandardTerminalTools::StandardTerminalTools;
use crate::core::tools::defaultTool::standard::StandardWebVisitTool::StandardWebVisitTool;

pub struct ToolGetter;

impl ToolGetter {
    #[allow(non_snake_case)]
    pub fn getFileSystemTools(
        context: &OperitApplicationContext,
    ) -> Option<StandardFileSystemTools> {
        context.fileSystemHost.clone().map(|fileSystemHost| {
            StandardFileSystemTools::new(
                fileSystemHost,
                context
                    .httpHost
                    .clone()
                    .expect("HTTP host must be configured before registering file download tool"),
            )
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
