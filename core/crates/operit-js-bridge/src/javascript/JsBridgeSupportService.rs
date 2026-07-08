use std::sync::{Arc, Mutex};

use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::AITool;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::javascript::{
    setJsBridgeSupport, JsBridgeSupport, JsExecutionEngine,
};
use operit_tools::tools::packTool::PackageManager::PackageManager;

use crate::javascript::JsEngine::JsEngine;
use crate::javascript::JsToolManager::JsToolManager;

/// Installs the JavaScript bridge implementation used by the tools crate.
pub struct JsBridgeSupportService;

impl JsBridgeSupportService {
    /// Installs JavaScript bridge support for this process.
    pub fn install() -> Result<(), String> {
        setJsBridgeSupport(Arc::new(RuntimeJsBridgeSupport))
    }
}

/// Bridges tool-owned JavaScript interfaces to the JS engine implementation.
struct RuntimeJsBridgeSupport;

impl JsBridgeSupport for RuntimeJsBridgeSupport {
    /// Creates a general package execution engine.
    #[allow(non_snake_case)]
    fn newPackageExecutionEngine(&self, toolHandler: AIToolHandler) -> Arc<dyn JsExecutionEngine> {
        Arc::new(JsEngine::new(toolHandler))
    }

    /// Creates a ToolPkg execution engine.
    #[allow(non_snake_case)]
    fn newToolPkgExecutionEngine(&self, toolHandler: AIToolHandler) -> Arc<dyn JsExecutionEngine> {
        Arc::new(JsEngine::new(toolHandler))
    }

    /// Executes one JavaScript package tool.
    #[allow(non_snake_case)]
    fn executePackageTool(
        &self,
        packageManager: Arc<Mutex<PackageManager>>,
        toolHandler: AIToolHandler,
        script: &str,
        tool: &AITool,
    ) -> Vec<ToolResult> {
        JsToolManager::getInstance(packageManager, toolHandler).executeScript(script, tool)
    }
}
