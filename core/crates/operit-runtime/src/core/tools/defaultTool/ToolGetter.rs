use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::defaultTool::standard::StandardFileSystemTools::StandardFileSystemTools;

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
}
