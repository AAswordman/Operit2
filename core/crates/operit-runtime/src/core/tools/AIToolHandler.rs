use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolExecutor, ToolValidationResult};
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::mcp::MCPManager::MCPManager;
use crate::core::tools::mcp::MCPToolExecutor::MCPToolExecutor;
use crate::core::tools::packTool::PackageManager::PackageManager;
use crate::core::tools::AIToolHook::AIToolHook;
use crate::core::tools::ToolPackage::{PackageToolExecutor, ToolPackage};
use crate::core::tools::ToolPermissionSystem::ToolPermissionSystem;
use crate::core::tools::ToolRegistration::registerAllTools;
use crate::core::tools::ToolResultDataClasses::stringResultData;
use crate::util::ChainLogger::{self, TOOL_CHAIN};
use operit_host_api::HostEnvironmentDescriptor;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};

static INSTANCE: OnceLock<Arc<Mutex<AIToolHandlerState>>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    packageManager: Option<Arc<Mutex<PackageManager>>>,
}

impl AIToolHandler {
    fn truncateLogValue(value: &str, maxChars: usize) -> String {
        let mut truncated = String::new();
        for character in value.chars().take(maxChars) {
            truncated.push(character);
        }
        if value.chars().count() > maxChars {
            truncated.push_str("...");
        }
        truncated
    }

    fn summarizeToolParameters(tool: &AITool) -> String {
        if tool.parameters.is_empty() {
            return String::new();
        }
        let mut parts = Vec::new();
        for parameter in tool.parameters.iter().take(3) {
            parts.push(format!(
                "{}={}",
                parameter.name,
                Self::truncateLogValue(&parameter.value, 120)
            ));
        }
        let mut summary = parts.join(", ");
        if tool.parameters.len() > 3 {
            summary.push_str(", ...");
        }
        Self::truncateLogValue(&summary, 320)
    }

    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AIToolHandlerState {
                availableTools: BTreeMap::new(),
                toolVisibility: BTreeMap::new(),
                defaultToolsRegistered: false,
                context: OperitApplicationContext::new(),
                hooks: Vec::new(),
                toolPermissionSystem: ToolPermissionSystem::getInstance(),
                packageManager: None,
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
                    packageManager: None,
                }))
            })
            .clone();
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
        if !guard
            .hooks
            .iter()
            .any(|existing| existing.id() == hook.id())
        {
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
    pub fn getContext(&self) -> OperitApplicationContext {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .context
            .clone()
    }

    #[allow(non_snake_case)]
    pub fn getOrCreatePackageManager(&self) -> Arc<Mutex<PackageManager>> {
        let context = {
            let guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
            if let Some(packageManager) = &guard.packageManager {
                return packageManager.clone();
            }
            guard.context.clone()
        };
        let packageManager = Arc::new(Mutex::new(PackageManager::newWithContext(
            RuntimeStorePaths::default(),
            context,
        )));
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        if let Some(existingPackageManager) = &guard.packageManager {
            return existingPackageManager.clone();
        }
        guard.packageManager = Some(packageManager.clone());
        packageManager
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
    pub fn hasToolExecutor(&self, toolName: &str) -> bool {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .availableTools
            .contains_key(toolName)
    }

    #[allow(non_snake_case)]
    pub fn getToolExecutorOrActivate(&mut self, toolName: &str) -> bool {
        if self.hasToolExecutor(toolName) {
            if let Some((packageName, _)) = toolName.split_once(':') {
                let packageName = packageName.trim();
                if !packageName.is_empty() {
                    let packageManager = self.getOrCreatePackageManager();
                    let isMcpAvailable = packageManager
                        .lock()
                        .expect("package manager mutex poisoned")
                        .getAvailableServerPackages()
                        .contains_key(packageName);
                    if isMcpAvailable && !self.isMcpServiceActive(packageName) {
                        let _ = packageManager
                            .lock()
                            .expect("package manager mutex poisoned")
                            .usePackage(packageName);
                    }
                }
            }
            return self.hasToolExecutor(toolName);
        }

        self.registerDefaultTools();
        if self.hasToolExecutor(toolName) {
            return true;
        }

        if toolName.contains(':') {
            let packageName = toolName
                .split_once(':')
                .map(|(name, _)| name.trim())
                .unwrap_or("");
            if !packageName.is_empty() {
                let packageManager = self.getOrCreatePackageManager();
                let selectedPackage = {
                    let mut guard = packageManager
                        .lock()
                        .expect("package manager mutex poisoned");
                    let isPackageAvailable = guard.getAvailablePackages().contains_key(packageName);
                    let isMcpAvailable =
                        guard.getAvailableServerPackages().contains_key(packageName);
                    if isPackageAvailable || isMcpAvailable {
                        let _ = guard.usePackage(packageName);
                        guard
                            .getEffectivePackageTools(packageName)
                            .filter(|package| !guard.isToolPkgContainer(&package.name))
                    } else {
                        None
                    }
                };
                if let Some(selectedPackage) = selectedPackage {
                    self.registerPackageTools(packageManager, selectedPackage);
                }
            }
        }

        self.hasToolExecutor(toolName)
    }

    #[allow(non_snake_case)]
    fn isMcpServiceActive(&self, packageName: &str) -> bool {
        let mcpManager = MCPManager::getInstance(self.getContext());
        let Some(client) = mcpManager.getOrCreateClient(packageName) else {
            return false;
        };
        client
            .getServiceInfo()
            .map(|info| info.active && info.ready)
            .unwrap_or(false)
    }

    #[allow(non_snake_case)]
    fn registerPackageTools(
        &mut self,
        packageManager: Arc<Mutex<PackageManager>>,
        toolPackage: ToolPackage,
    ) {
        let isMcpPackage = toolPackage.category == "MCP"
            || toolPackage
                .tools
                .first()
                .map(|tool| tool.script.contains("/* MCPJS"))
                .unwrap_or(false);
        let executableTools = toolPackage
            .tools
            .iter()
            .filter(|packageTool| !packageTool.advice)
            .cloned()
            .collect::<Vec<_>>();
        let context = self.getContext();
        for packageTool in executableTools {
            let toolName = format!("{}:{}", toolPackage.name, packageTool.name);
            if isMcpPackage {
                self.registerTool(
                    toolName,
                    Box::new(MCPToolExecutor::new(MCPManager::getInstance(
                        context.clone(),
                    ))),
                );
            } else {
                self.registerTool(
                    toolName,
                    Box::new(PackageToolExecutor::new(
                        toolPackage.clone(),
                        packageManager.clone(),
                        self.clone(),
                    )),
                );
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn executeToolSafelyWithResolvedExecutor(
        &mut self,
        tool: &AITool,
    ) -> Option<Vec<ToolResult>> {
        let parameterSummary = Self::summarizeToolParameters(tool);
        ChainLogger::info(
            TOOL_CHAIN,
            "tool.stream.request",
            &[
                ("tool", tool.name.clone()),
                ("parameterCount", tool.parameters.len().to_string()),
                ("parameters", parameterSummary.clone()),
            ],
        );
        if !self.getToolExecutorOrActivate(&tool.name) {
            ChainLogger::warn(
                TOOL_CHAIN,
                "tool.stream.not_found",
                &[("tool", tool.name.clone())],
            );
            return None;
        }

        let Some(mut executor) = ({
            self.inner
                .lock()
                .expect("AIToolHandler mutex poisoned")
                .availableTools
                .remove(&tool.name)
        }) else {
            ChainLogger::warn(
                TOOL_CHAIN,
                "tool.stream.not_registered",
                &[("tool", tool.name.clone())],
            );
            return None;
        };

        let validationResult = executor.validateParameters(tool);
        let startMs = operit_host_api::TimeUtils::currentTimeMillis();
        let collected = if validationResult.valid {
            ChainLogger::info(
                TOOL_CHAIN,
                "tool.stream.start",
                &[
                    ("tool", tool.name.clone()),
                    ("parameters", parameterSummary.clone()),
                ],
            );
            executor.invokeAndStream(tool)
        } else {
            ChainLogger::warn(
                TOOL_CHAIN,
                "tool.stream.validation_failed",
                &[
                    ("tool", tool.name.clone()),
                    ("error", validationResult.errorMessage.clone()),
                ],
            );
            vec![ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(format!(
                    "Invalid parameters: {}",
                    validationResult.errorMessage
                )),
            }]
        };

        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .availableTools
            .insert(tool.name.clone(), executor);
        let elapsedMs = operit_host_api::TimeUtils::currentTimeMillis().saturating_sub(startMs);
        let finalResult = collected.last().cloned();
        if let Some(finalResult) = finalResult {
            if finalResult.success {
                ChainLogger::info(
                    TOOL_CHAIN,
                    "tool.stream.done",
                    &[
                        ("tool", tool.name.clone()),
                        ("resultCount", collected.len().to_string()),
                        ("elapsedMs", elapsedMs.to_string()),
                        (
                            "resultChars",
                            ChainLogger::lenField(&finalResult.result.toString()),
                        ),
                    ],
                );
            } else if let Some(error) = finalResult.error.clone() {
                ChainLogger::error(
                    TOOL_CHAIN,
                    "tool.stream.error",
                    &[
                        ("tool", tool.name.clone()),
                        ("resultCount", collected.len().to_string()),
                        ("elapsedMs", elapsedMs.to_string()),
                        ("parameters", parameterSummary.clone()),
                        ("error", error),
                        (
                            "resultChars",
                            ChainLogger::lenField(&finalResult.result.toString()),
                        ),
                    ],
                );
            } else {
                ChainLogger::error(
                    TOOL_CHAIN,
                    "tool.stream.error",
                    &[
                        ("tool", tool.name.clone()),
                        ("resultCount", collected.len().to_string()),
                        ("elapsedMs", elapsedMs.to_string()),
                        ("parameters", parameterSummary.clone()),
                        (
                            "resultChars",
                            ChainLogger::lenField(&finalResult.result.toString()),
                        ),
                    ],
                );
            }
        } else {
            ChainLogger::error(
                TOOL_CHAIN,
                "tool.stream.empty_result",
                &[
                    ("tool", tool.name.clone()),
                    ("elapsedMs", elapsedMs.to_string()),
                ],
            );
        }
        if elapsedMs >= 3000 {
            ChainLogger::warn(
                TOOL_CHAIN,
                "tool.stream.slow",
                &[
                    ("tool", tool.name.clone()),
                    ("elapsedMs", elapsedMs.to_string()),
                    ("parameters", parameterSummary.clone()),
                ],
            );
        }
        Some(collected)
    }

    #[allow(non_snake_case)]
    pub fn executeTool(&mut self, tool: AITool) -> ToolResult {
        ChainLogger::info(
            TOOL_CHAIN,
            "tool.execute.request",
            &[
                ("tool", tool.name.clone()),
                ("parameterCount", tool.parameters.len().to_string()),
            ],
        );
        self.notifyToolCallRequested(&tool);
        self.getToolExecutorOrActivate(&tool.name);
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
                result: stringResultData(""),
                error: Some(format!("Tool not found: {}", tool.name)),
            };
            self.notifyToolExecutionResult(&tool, &notFoundResult);
            self.notifyToolExecutionFinished(&tool);
            ChainLogger::warn(
                TOOL_CHAIN,
                "tool.execute.not_found",
                &[("tool", tool.name.clone())],
            );
            return notFoundResult;
        };

        let validationResult = executor.validateParameters(&tool);
        if !validationResult.valid {
            let validationError = validationResult.errorMessage;
            let validationFailedResult = ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(validationError.clone()),
            };
            self.notifyToolExecutionResult(&tool, &validationFailedResult);
            self.notifyToolExecutionFinished(&tool);
            ChainLogger::warn(
                TOOL_CHAIN,
                "tool.execute.validation_failed",
                &[("tool", tool.name.clone()), ("error", validationError)],
            );
            self.inner
                .lock()
                .expect("AIToolHandler mutex poisoned")
                .availableTools
                .insert(tool.name.clone(), executor);
            return validationFailedResult;
        }

        self.notifyToolExecutionStarted(&tool);
        ChainLogger::info(
            TOOL_CHAIN,
            "tool.execute.start",
            &[("tool", tool.name.clone())],
        );
        let collected = executor.invokeAndStream(&tool);
        if collected.is_empty() {
            ChainLogger::error(
                TOOL_CHAIN,
                "tool.execute.empty_result",
                &[("tool", tool.name.clone())],
            );
        }
        let result = collected
            .last()
            .cloned()
            .expect("ToolExecutor.invokeAndStream must return at least one ToolResult");
        self.notifyToolExecutionResult(&tool, &result);
        self.notifyToolExecutionFinished(&tool);
        if result.success {
            ChainLogger::info(
                TOOL_CHAIN,
                "tool.execute.done",
                &[
                    ("tool", tool.name.clone()),
                    (
                        "resultChars",
                        ChainLogger::lenField(&result.result.toString()),
                    ),
                ],
            );
        } else {
            if let Some(error) = result.error.as_ref() {
                ChainLogger::error(
                    TOOL_CHAIN,
                    "tool.execute.error",
                    &[
                        ("tool", tool.name.clone()),
                        ("error", error.clone()),
                        (
                            "resultChars",
                            ChainLogger::lenField(&result.result.toString()),
                        ),
                    ],
                );
            } else {
                ChainLogger::error(
                    TOOL_CHAIN,
                    "tool.execute.error",
                    &[
                        ("tool", tool.name.clone()),
                        (
                            "resultChars",
                            ChainLogger::lenField(&result.result.toString()),
                        ),
                    ],
                );
            }
        }
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
    pub invoke: Arc<dyn Fn(&AITool) -> ToolResult + Send + Sync>,
    pub validate: Arc<dyn Fn(&AITool) -> ToolValidationResult + Send + Sync>,
}

impl ToolExecutor for FnToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        (self.validate)(tool)
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![(self.invoke)(tool)]
    }
}
