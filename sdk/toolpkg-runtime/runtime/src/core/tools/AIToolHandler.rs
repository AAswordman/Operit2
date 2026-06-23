use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::HostEnvironmentDescriptor;

use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{AITool, ToolExecutor};
use operit_context::OperitApplicationContext::OperitApplicationContext;
use operit_tools::ToolResultDataClasses::stringResultData;

static INSTANCE: OnceLock<Arc<Mutex<AIToolHandlerState>>> = OnceLock::new();

#[derive(Clone)]
pub struct AIToolHandler {
    inner: Arc<Mutex<AIToolHandlerState>>,
}

pub trait ExternalToolInvocationBridge: Send + Sync {
    #[allow(non_snake_case)]
    fn invokeTool(&self, tool: &AITool) -> Vec<ToolResult>;
}

struct AIToolHandlerState {
    context: OperitApplicationContext,
    externalToolBridge: Option<Arc<dyn ExternalToolInvocationBridge>>,
    executors: BTreeMap<String, Box<dyn ToolExecutor>>,
}

impl AIToolHandler {
    pub fn new() -> Self {
        Self::withContext(OperitApplicationContext::new())
    }

    #[allow(non_snake_case)]
    pub fn withContext(context: OperitApplicationContext) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AIToolHandlerState {
                context,
                externalToolBridge: None,
                executors: BTreeMap::new(),
            })),
        }
    }

    #[allow(non_snake_case)]
    pub fn getInstance(context: OperitApplicationContext) -> Self {
        let inner = INSTANCE
            .get_or_init(|| {
                Arc::new(Mutex::new(AIToolHandlerState {
                    context,
                    externalToolBridge: None,
                    executors: BTreeMap::new(),
                }))
            })
            .clone();
        Self { inner }
    }

    #[allow(non_snake_case)]
    pub fn setExternalToolBridge(&mut self, bridge: Option<Arc<dyn ExternalToolInvocationBridge>>) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .externalToolBridge = bridge;
    }

    #[allow(non_snake_case)]
    pub fn getContext(&self) -> OperitApplicationContext {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .context
            .clone()
    }

    #[allow(non_snake_case)]
    pub fn getHostEnvironmentDescriptor(&self) -> HostEnvironmentDescriptor {
        self.getContext().hostEnvironment
    }

    #[allow(non_snake_case)]
    pub fn registerTool(&mut self, name: String, executor: Box<dyn ToolExecutor>) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .executors
            .insert(name, executor);
    }

    #[allow(non_snake_case)]
    pub fn executeTool(&mut self, tool: AITool) -> ToolResult {
        let mut executor = {
            self.inner
                .lock()
                .expect("AIToolHandler mutex poisoned")
                .executors
                .remove(&tool.name)
        };
        if let Some(executorRef) = executor.as_mut() {
            let validation = executorRef.validateParameters(&tool);
            let result = if validation.valid {
                executorRef.invokeAndStream(&tool).last().cloned()
            } else {
                Some(ToolResult {
                    toolName: tool.name.clone(),
                    success: false,
                    result: stringResultData(""),
                    error: Some(validation.errorMessage),
                })
            };
            self.inner
                .lock()
                .expect("AIToolHandler mutex poisoned")
                .executors
                .insert(tool.name.clone(), executor.take().expect("executor available"));
            return result.unwrap_or_else(|| ToolResult {
                toolName: tool.name,
                success: false,
                result: stringResultData(""),
                error: Some("Tool executor returned no result.".to_string()),
            });
        }

        let bridge = self
            .inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .externalToolBridge
            .clone();
        let Some(bridge) = bridge else {
            return ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(format!("Tool not found: {}", tool.name)),
            };
        };
        bridge.invokeTool(&tool).last().cloned().unwrap_or_else(|| ToolResult {
            toolName: tool.name,
            success: false,
            result: stringResultData(""),
            error: Some("External tool bridge returned no result.".to_string()),
        })
    }
}

impl Default for AIToolHandler {
    fn default() -> Self {
        Self::new()
    }
}
