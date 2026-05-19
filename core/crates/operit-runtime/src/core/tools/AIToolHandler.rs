use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{
    AITool, ToolExecutor, ToolValidationResult,
};
use crate::core::tools::ToolRegistration::registerAllTools;

static INSTANCE: OnceLock<Arc<Mutex<AIToolHandlerState>>> = OnceLock::new();

#[derive(Clone)]
pub struct AIToolHandler {
    inner: Arc<Mutex<AIToolHandlerState>>,
}

pub struct AIToolHandlerState {
    availableTools: BTreeMap<String, Box<dyn ToolExecutor>>,
    defaultToolsRegistered: bool,
    context: OperitApplicationContext,
}

impl AIToolHandler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AIToolHandlerState {
                availableTools: BTreeMap::new(),
                defaultToolsRegistered: false,
                context: OperitApplicationContext::new(),
            })),
        }
    }

    #[allow(non_snake_case)]
    pub fn getInstance(context: OperitApplicationContext) -> Self {
        let inner = INSTANCE
            .get_or_init(|| {
                Arc::new(Mutex::new(AIToolHandlerState {
                    availableTools: BTreeMap::new(),
                    defaultToolsRegistered: false,
                    context: context.clone(),
                }))
            })
            .clone();
        {
            let mut guard = inner.lock().expect("AIToolHandler mutex poisoned");
            guard.context = context;
        }
        Self { inner }
    }

    #[allow(non_snake_case)]
    pub fn unregisterTool(&mut self, toolName: String) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .availableTools
            .remove(&toolName);
    }

    #[allow(non_snake_case)]
    pub fn getAllToolNames(&self) -> Vec<String> {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .availableTools
            .keys()
            .cloned()
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn registerTool(&mut self, name: String, executor: Box<dyn ToolExecutor>) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .availableTools
            .insert(name, executor);
    }

    #[allow(non_snake_case)]
    pub fn registerDefaultTools(&mut self) {
        {
            let guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
            if guard.defaultToolsRegistered {
                return;
            }
        }
        let context = {
            self.inner
                .lock()
                .expect("AIToolHandler mutex poisoned")
                .context
                .clone()
        };
        registerAllTools(self, &context);
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .defaultToolsRegistered = true;
    }

    #[allow(non_snake_case)]
    pub fn getToolExecutor(&mut self, _toolName: &str) -> Option<&mut Box<dyn ToolExecutor>> {
        None
    }

    #[allow(non_snake_case)]
    pub fn takeExecutors(&mut self) -> BTreeMap<String, Box<dyn ToolExecutor>> {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        if !guard.defaultToolsRegistered {
            drop(guard);
            self.registerDefaultTools();
            guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        }
        std::mem::take(&mut guard.availableTools)
    }

    #[allow(non_snake_case)]
    pub fn restoreExecutors(&mut self, executors: BTreeMap<String, Box<dyn ToolExecutor>>) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .availableTools = executors;
    }

    pub fn reset(&mut self) {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard.availableTools.clear();
        guard.defaultToolsRegistered = false;
    }
}

impl AIToolHandlerState {
    #[allow(non_snake_case)]
    pub fn getContext(&self) -> &OperitApplicationContext {
        &self.context
    }
}

impl Default for AIToolHandler {
    fn default() -> Self {
        if let Some(inner) = INSTANCE.get() {
            return Self {
                inner: inner.clone(),
            };
        }
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
