use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{
    AITool, ToolExecutor, ToolValidationResult,
};
use crate::core::tools::AIToolHook::AIToolHook;
use crate::core::tools::ToolPermissionSystem::ToolPermissionSystem;
use crate::core::tools::ToolRegistration::registerAllTools;
use crate::core::tools::packTool::PackageManager::PackageManager;
use operit_host_api::HostEnvironmentDescriptor;

static INSTANCE: OnceLock<Arc<Mutex<AIToolHandlerState>>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolRegistrationVisibility {
    PUBLIC,
    INTERNAL,
}

#[derive(Clone)]
pub struct AIToolHandler {
    inner: Arc<Mutex<AIToolHandlerState>>,
}

pub struct AIToolHandlerState {
    availableTools: BTreeMap<String, Box<dyn ToolExecutor>>,
    toolVisibility: BTreeMap<String, ToolRegistrationVisibility>,
    defaultToolsRegistered: bool,
    context: OperitApplicationContext,
    hooks: Vec<Arc<dyn AIToolHook>>,
    toolPermissionSystem: ToolPermissionSystem,
    packageManager: Arc<Mutex<PackageManager>>,
}

impl AIToolHandler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AIToolHandlerState {
                availableTools: BTreeMap::new(),
                toolVisibility: BTreeMap::new(),
                defaultToolsRegistered: false,
                context: OperitApplicationContext::new(),
                hooks: Vec::new(),
                toolPermissionSystem: ToolPermissionSystem::getInstance(),
                packageManager: Arc::new(Mutex::new(PackageManager::default())),
            })),
        }
    }

    #[allow(non_snake_case)]
    pub fn getInstance(context: OperitApplicationContext) -> Self {
        let inner = INSTANCE
            .get_or_init(|| {
                Arc::new(Mutex::new(AIToolHandlerState {
                    availableTools: BTreeMap::new(),
                    toolVisibility: BTreeMap::new(),
                    defaultToolsRegistered: false,
                    context: context.clone(),
                    hooks: Vec::new(),
                    toolPermissionSystem: ToolPermissionSystem::getInstance(),
                    packageManager: Arc::new(Mutex::new(PackageManager::default())),
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
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard.availableTools.remove(&toolName);
        guard.toolVisibility.remove(&toolName);
    }

    #[allow(non_snake_case)]
    pub fn getToolPermissionSystem(&self) -> ToolPermissionSystem {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .toolPermissionSystem
            .clone()
    }

    #[allow(non_snake_case)]
    pub fn addToolHook(&mut self, hook: Arc<dyn AIToolHook>) {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        if !guard.hooks.iter().any(|existing| existing.id() == hook.id()) {
            guard.hooks.push(hook);
        }
    }

    #[allow(non_snake_case)]
    pub fn removeToolHook(&mut self, hookId: &str) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .hooks
            .retain(|hook| hook.id() != hookId);
    }

    #[allow(non_snake_case)]
    pub fn clearToolHooks(&mut self) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .hooks
            .clear();
    }

    fn notifyHooks<F>(&self, action: F)
    where
        F: Fn(&dyn AIToolHook),
    {
        let hooks = self
            .inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .hooks
            .clone();
        for hook in hooks {
            action(hook.as_ref());
        }
    }

    #[allow(non_snake_case)]
    pub fn notifyToolCallRequested(&self, tool: &AITool) {
        self.notifyHooks(|hook| hook.onToolCallRequested(tool));
    }

    #[allow(non_snake_case)]
    pub fn notifyToolPermissionChecked(&self, tool: &AITool, granted: bool, reason: Option<&str>) {
        self.notifyHooks(|hook| hook.onToolPermissionChecked(tool, granted, reason));
    }

    #[allow(non_snake_case)]
    pub fn notifyToolExecutionStarted(&self, tool: &AITool) {
        self.notifyHooks(|hook| hook.onToolExecutionStarted(tool));
    }

    #[allow(non_snake_case)]
    pub fn notifyToolExecutionResult(&self, tool: &AITool, result: &ToolResult) {
        self.notifyHooks(|hook| hook.onToolExecutionResult(tool, result));
    }

    #[allow(non_snake_case)]
    pub fn notifyToolExecutionError(&self, tool: &AITool, message: &str) {
        self.notifyHooks(|hook| hook.onToolExecutionError(tool, message));
    }

    #[allow(non_snake_case)]
    pub fn notifyToolExecutionFinished(&self, tool: &AITool) {
        self.notifyHooks(|hook| hook.onToolExecutionFinished(tool));
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
    pub fn getHostEnvironmentDescriptor(&self) -> HostEnvironmentDescriptor {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .context
            .hostEnvironment
            .clone()
    }

    #[allow(non_snake_case)]
    pub fn getOrCreatePackageManager(&self) -> Arc<Mutex<PackageManager>> {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .packageManager
            .clone()
    }

    #[allow(non_snake_case)]
    pub fn getPublicToolNames(&self) -> Vec<String> {
        let guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard
            .toolVisibility
            .iter()
            .filter(|(_, visibility)| **visibility == ToolRegistrationVisibility::PUBLIC)
            .map(|(name, _)| name.clone())
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn getInternalToolNames(&self) -> Vec<String> {
        let guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard
            .toolVisibility
            .iter()
            .filter(|(_, visibility)| **visibility == ToolRegistrationVisibility::INTERNAL)
            .map(|(name, _)| name.clone())
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn registerTool(&mut self, name: String, executor: Box<dyn ToolExecutor>) {
        self.registerToolWithVisibility(name, executor, ToolRegistrationVisibility::PUBLIC);
    }

    #[allow(non_snake_case)]
    pub fn registerInternalTool(&mut self, name: String, executor: Box<dyn ToolExecutor>) {
        self.registerToolWithVisibility(name, executor, ToolRegistrationVisibility::INTERNAL);
    }

    #[allow(non_snake_case)]
    pub fn registerToolWithVisibility(
        &mut self,
        name: String,
        executor: Box<dyn ToolExecutor>,
        visibility: ToolRegistrationVisibility,
    ) {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard.availableTools.insert(name.clone(), executor);
        guard.toolVisibility.insert(name, visibility);
    }

    #[allow(non_snake_case)]
    pub fn getToolVisibility(&self, toolName: &str) -> Option<ToolRegistrationVisibility> {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .toolVisibility
            .get(toolName)
            .copied()
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
    pub fn executeTool(&mut self, tool: AITool) -> ToolResult {
        self.notifyToolCallRequested(&tool);
        self.registerDefaultTools();
        let Some(mut executor) = ({
            self.inner
                .lock()
                .expect("AIToolHandler mutex poisoned")
                .availableTools
                .remove(&tool.name)
        }) else {
            let notFoundResult = ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: String::new(),
                error: Some(format!("Tool not found: {}", tool.name)),
            };
            self.notifyToolExecutionResult(&tool, &notFoundResult);
            self.notifyToolExecutionFinished(&tool);
            return notFoundResult;
        };

        let validationResult = executor.validateParameters(&tool);
        if !validationResult.valid {
            let validationFailedResult = ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: String::new(),
                error: Some(validationResult.errorMessage),
            };
            self.notifyToolExecutionResult(&tool, &validationFailedResult);
            self.notifyToolExecutionFinished(&tool);
            self.inner
                .lock()
                .expect("AIToolHandler mutex poisoned")
                .availableTools
                .insert(tool.name.clone(), executor);
            return validationFailedResult;
        }

        self.notifyToolExecutionStarted(&tool);
        let collected = executor.invokeAndStream(&tool);
        let result = collected.last().cloned().unwrap_or_else(|| ToolResult {
            toolName: tool.name.clone(),
            success: false,
            result: String::new(),
            error: Some("The tool execution returned no results.".to_string()),
        });
        self.notifyToolExecutionResult(&tool, &result);
        self.notifyToolExecutionFinished(&tool);
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .availableTools
            .insert(tool.name.clone(), executor);
        result
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
        guard.toolVisibility.clear();
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
