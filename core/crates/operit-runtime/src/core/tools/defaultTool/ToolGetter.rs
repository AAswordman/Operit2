use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::defaultTool::standard::StandardFileSystemTools::StandardFileSystemTools;
use crate::core::tools::defaultTool::standard::StandardHttpTools::StandardHttpTools;
use crate::core::tools::defaultTool::standard::StandardSystemOperationTools::StandardSystemOperationTools;
use crate::core::tools::defaultTool::standard::StandardWebVisitTool::StandardWebVisitTool;

pub struct ToolGetter;

impl ToolGetter {
    #[allow(non_snake_case)]
    pub fn getFileSystemTools(
        context: &OperitApplicationContext,
    ) -> Option<StandardFileSystemTools> {
        context
            .fileSystemHost
            .clone()
            .map(StandardFileSystemTools::new)
    }

    #[allow(non_snake_case)]
    pub fn getHttpTools(_context: &OperitApplicationContext) -> StandardHttpTools {
        StandardHttpTools::new()
    }

    #[allow(non_snake_case)]
    pub fn getWebVisitTool(context: &OperitApplicationContext) -> StandardWebVisitTool {
        StandardWebVisitTool::new(context.webVisitHost.clone())
    }

    #[allow(non_snake_case)]
    pub fn getSystemOperationTools(
        context: &OperitApplicationContext,
    ) -> StandardSystemOperationTools {
        StandardSystemOperationTools::new(context.systemOperationHost.clone())
    }
}
