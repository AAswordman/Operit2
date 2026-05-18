use std::collections::BTreeMap;

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{
    AITool, ToolExecutor, ToolValidationResult,
};
use crate::core::tools::ToolRegistration::registerAllTools;

pub struct AIToolHandler {
    availableTools: BTreeMap<String, Box<dyn ToolExecutor>>,
    defaultToolsRegistered: bool,
}

impl AIToolHandler {
    pub fn new() -> Self {
        Self {
            availableTools: BTreeMap::new(),
            defaultToolsRegistered: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn unregisterTool(&mut self, toolName: String) {
        self.availableTools.remove(&toolName);
    }

    #[allow(non_snake_case)]
    pub fn getAllToolNames(&self) -> Vec<String> {
        self.availableTools.keys().cloned().collect()
    }

    #[allow(non_snake_case)]
    pub fn registerTool(&mut self, name: String, executor: Box<dyn ToolExecutor>) {
        self.availableTools.insert(name, executor);
    }

    #[allow(non_snake_case)]
    pub fn registerDefaultTools(&mut self) {
        if self.defaultToolsRegistered {
            return;
        }
        registerAllTools(self);
        self.defaultToolsRegistered = true;
    }

    #[allow(non_snake_case)]
    pub fn getToolExecutor(&mut self, toolName: &str) -> Option<&mut Box<dyn ToolExecutor>> {
        self.availableTools.get_mut(toolName)
    }

    #[allow(non_snake_case)]
    pub fn takeExecutors(&mut self) -> BTreeMap<String, Box<dyn ToolExecutor>> {
        std::mem::take(&mut self.availableTools)
    }

    #[allow(non_snake_case)]
    pub fn restoreExecutors(&mut self, executors: BTreeMap<String, Box<dyn ToolExecutor>>) {
        self.availableTools = executors;
    }

    pub fn reset(&mut self) {
        self.availableTools.clear();
        self.defaultToolsRegistered = false;
    }
}

impl Default for AIToolHandler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FnToolExecutor {
    pub name: String,
    pub invoke: fn(&AITool) -> ToolResult,
    pub validate: fn(&AITool) -> ToolValidationResult,
}

impl ToolExecutor for FnToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        (self.validate)(tool)
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![(self.invoke)(tool)]
    }
}
