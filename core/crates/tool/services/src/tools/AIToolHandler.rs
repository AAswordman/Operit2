use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use operit_host_api::HostEnvironmentDescriptor;
use operit_host_api::HostManager::HostManager;
use operit_plugin_sdk::javascript::{
    JsExecutionHost, JsToolCallRequest, JsToolCallResult, JsToolCallResultData,
    JsToolNameResolutionRequest, JsToolPkgIpcCompletion, JsToolPkgIpcRequest,
    JsToolPkgResourceRequest, JsToolPkgWasmRequest, JsToolPkgWasmResult, ToolPkgConfigScope,
};
use operit_plugin_sdk::js_sdk::tool_types::BuiltinToolName;
use operit_plugin_sdk::package::ToolPackage;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::tools::mcp::MCPManager::MCPManager;
use operit_tools::tools::mcp::MCPToolExecutor::MCPToolExecutor;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::AIToolHook::{AIToolHook, AIToolHookDecision};
use operit_tools::tools::PackageToolExecutor::PackageToolExecutor;
use operit_tools::tools::ToolPermissionSystem::{AiPermissionMode, ToolPermissionSystem};
use operit_tools::tools::ToolRegistration::registerAllTools;
use operit_tools::tools::ToolResultDataClasses::{stringResultData, ToolResultData};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, AsyncToolExecutor, RegisteredToolExecutor, ToolAccessSpec, ToolBoundary, ToolEffect,
    ToolExecutionManager, ToolExecutor, ToolInvocationFuture, ToolParameter, ToolValidationResult,
};
use operit_util::ChainLogger::{self, TOOL_CHAIN};
use operit_util::LocaleUtils::LocaleUtils;
use operit_util::OperitPaths;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::runtime_support::{ToolRuntimeDependencies, ToolRuntimeSupport};

#[derive(Clone, Copy)]
enum ToolExecutionAccess {
    AiPermission,
    Direct,
}

/// Classifies file-bound invocation failures as policy restrictions or invalid requests.
#[derive(Debug, PartialEq, Eq)]
enum WorkspaceBoundaryError {
    RequiresApproval(String),
    InvalidRequest(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolRegistrationVisibility {
    PUBLIC,
    INTERNAL,
}

#[derive(Clone)]
pub struct AIToolHandler {
    inner: Arc<Mutex<AIToolHandlerState>>,
    executorAvailability: Arc<tokio::sync::Notify>,
}

/// Returns a taken executor on normal completion, cancellation or panic unwinding.
struct ToolExecutorLease {
    name: String,
    executor: Option<RegisteredToolExecutor>,
    state: Arc<Mutex<AIToolHandlerState>>,
    available: Arc<tokio::sync::Notify>,
}

impl std::ops::Deref for ToolExecutorLease {
    type Target = RegisteredToolExecutor;
    /// Borrows the executor while its registry lease remains owned.
    fn deref(&self) -> &Self::Target {
        self.executor
            .as_ref()
            .expect("tool executor lease must remain owned")
    }
}

impl std::ops::DerefMut for ToolExecutorLease {
    /// Mutably borrows the executor while its registry lease remains owned.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.executor
            .as_mut()
            .expect("tool executor lease must remain owned")
    }
}

impl Drop for ToolExecutorLease {
    /// Restores the exact executor and wakes queued invocations after releasing the registry lock.
    fn drop(&mut self) {
        let executor = self
            .executor
            .take()
            .expect("tool executor lease must remain owned");
        {
            let mut state = self.state.lock().expect("AIToolHandler mutex poisoned");
            assert!(
                state.executingTools.remove(&self.name),
                "tool must remain marked executing"
            );
            assert!(
                state
                    .availableTools
                    .insert(self.name.clone(), executor)
                    .is_none(),
                "tool executor must not be replaced during execution"
            );
        }
        self.available.notify_waiters();
    }
}

pub struct AIToolHandlerState {
    availableTools: BTreeMap<String, RegisteredToolExecutor>,
    executingTools: BTreeSet<String>,
    toolVisibility: BTreeMap<String, ToolRegistrationVisibility>,
    unavailableBuiltinTools: BTreeMap<BuiltinToolName, String>,
    defaultToolsRegistered: bool,
    context: HostManager,
    runtimeDependencies: ToolRuntimeDependencies,
    hooks: Vec<Arc<dyn AIToolHook>>,
    toolPermissionSystem: ToolPermissionSystem,
    packageManager: Option<Arc<Mutex<RuntimePackageManager>>>,
}

impl AIToolHandler {
    /// Creates one scope-owned directory through the filesystem Host without package manager locks.
    fn createPackageConfigDirectory(
        &self,
        owner_id: &str,
        plugin_id: &str,
        scope: &str,
    ) -> Result<String, String> {
        let root =
            operit_store::ExtensionStore::ExtensionStore::configPathForScope(owner_id, scope)?;
        let path = if plugin_id == owner_id {
            root
        } else {
            let name = operit_util::OperitPaths::pluginConfigDirName(plugin_id)?;
            format!("{root}/namespaces/{name}")
        };
        let configDir = RuntimeStorePaths::default().runtime_storage_path(&path);
        self.getContext()
            .fileSystemHost
            .as_ref()
            .ok_or("FileSystemHost is required for plugin configuration")?
            .makeDirectory(&configDir.to_string_lossy(), true)
            .map_err(|error| error.to_string())?;
        let relative = path
            .strip_prefix("runtime/")
            .ok_or("Configuration is outside runtime storage")?;
        PathMapper::joinVfsPath("/app/data", relative)
    }

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

    /// Creates an isolated tool handler with explicit runtime dependencies.
    pub fn new(context: HostManager, runtimeDependencies: ToolRuntimeDependencies) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AIToolHandlerState {
                availableTools: BTreeMap::new(),
                executingTools: BTreeSet::new(),
                toolVisibility: BTreeMap::new(),
                unavailableBuiltinTools: BTreeMap::new(),
                defaultToolsRegistered: false,
                context,
                runtimeDependencies,
                hooks: Vec::new(),
                toolPermissionSystem: ToolPermissionSystem::getInstance(),
                packageManager: None,
            })),
            executorAvailability: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Removes one registered tool and its visibility metadata.
    #[allow(non_snake_case)]
    pub fn unregisterTool(&mut self, toolName: String) {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard.availableTools.remove(&toolName);
        guard.toolVisibility.remove(&toolName);
    }

    /// Removes all tools registered for an MCP server namespace.
    #[allow(non_snake_case)]
    pub fn unregisterMcpServerTools(&mut self, serverName: &str) -> usize {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        let toolNames = guard
            .availableTools
            .keys()
            .filter(|toolName| {
                matches!(toolName.split_once(':'), Some((name, _)) if name == serverName)
            })
            .cloned()
            .collect::<Vec<_>>();
        let count = toolNames.len();
        for toolName in toolNames {
            guard.availableTools.remove(&toolName);
            guard.toolVisibility.remove(&toolName);
        }
        count
    }

    /// Removes the package registration created for an MCP server.
    #[allow(non_snake_case)]
    pub fn unregisterMcpServerPackage(&mut self, serverName: &str) -> bool {
        let packageManager = {
            self.inner
                .lock()
                .expect("AIToolHandler mutex poisoned")
                .packageManager
                .clone()
        };
        let Some(packageManager) = packageManager else {
            return false;
        };
        let mut guard = packageManager
            .lock()
            .expect("package manager mutex poisoned");
        guard.unregisterMCPServerPackage(serverName)
    }

    /// Returns the permission system used before operations that require an approval.
    #[allow(non_snake_case)]
    pub fn getToolPermissionSystem(&self) -> ToolPermissionSystem {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .toolPermissionSystem
            .clone()
    }

    /// Adds a tool lifecycle hook when a hook with the same id is not already registered.
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

    /// Removes a tool lifecycle hook by hook id.
    #[allow(non_snake_case)]
    pub fn removeToolHook(&mut self, hookId: &str) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .hooks
            .retain(|hook| hook.id() != hookId);
    }

    /// Removes every registered tool lifecycle hook.
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

    /// Notifies hooks that a tool call has been requested.
    #[allow(non_snake_case)]
    pub fn notifyToolCallRequested(&self, tool: &AITool) {
        self.notifyHooks(|hook| hook.onToolCallRequested(tool));
    }

    /// Returns the first hook decision for a tool call before execution begins.
    #[allow(non_snake_case)]
    pub async fn checkToolInterception(&self, tool: &AITool) -> AIToolHookDecision {
        let hooks = self
            .inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .hooks
            .clone();
        for hook in hooks {
            match hook.onToolCallInterceptAsync(tool).await {
                AIToolHookDecision::Allow => {}
                decision @ AIToolHookDecision::Block(_) => return decision,
            }
        }
        AIToolHookDecision::Allow
    }

    /// Builds the standard failed result emitted when a hook blocks a tool call.
    #[allow(non_snake_case)]
    pub fn toolInterceptionResult(tool: &AITool, decision: AIToolHookDecision) -> ToolResult {
        let reason = match decision {
            AIToolHookDecision::Allow => "Tool call was not blocked.".to_string(),
            AIToolHookDecision::Block(reason) => reason,
        };
        ToolResult {
            toolName: tool.name.clone(),
            success: false,
            result: stringResultData(""),
            error: Some(reason),
        }
    }

    /// Notifies hooks that permission was checked for a tool call.
    #[allow(non_snake_case)]
    pub fn notifyToolPermissionChecked(&self, tool: &AITool, granted: bool, reason: Option<&str>) {
        self.notifyHooks(|hook| hook.onToolPermissionChecked(tool, granted, reason));
    }

    /// Notifies hooks that tool execution is about to start.
    #[allow(non_snake_case)]
    pub fn notifyToolExecutionStarted(&self, tool: &AITool) {
        self.notifyHooks(|hook| hook.onToolExecutionStarted(tool));
    }

    /// Notifies hooks that tool execution returned a result.
    #[allow(non_snake_case)]
    pub fn notifyToolExecutionResult(&self, tool: &AITool, result: &ToolResult) {
        self.notifyHooks(|hook| hook.onToolExecutionResult(tool, result));
    }

    /// Notifies hooks that tool execution failed before producing a normal result.
    #[allow(non_snake_case)]
    pub fn notifyToolExecutionError(&self, tool: &AITool, message: &str) {
        self.notifyHooks(|hook| hook.onToolExecutionError(tool, message));
    }

    /// Notifies hooks that tool execution has fully finished.
    #[allow(non_snake_case)]
    pub fn notifyToolExecutionFinished(&self, tool: &AITool) {
        self.notifyHooks(|hook| hook.onToolExecutionFinished(tool));
    }

    /// Returns every registered tool name regardless of visibility.
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

    /// Returns the host environment descriptor associated with this handler.
    #[allow(non_snake_case)]
    pub fn getHostEnvironmentDescriptor(&self) -> HostEnvironmentDescriptor {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .context
            .hostEnvironment
            .clone()
    }

    /// Returns the host manager associated with this handler.
    #[allow(non_snake_case)]
    pub fn getContext(&self) -> HostManager {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .context
            .clone()
    }

    /// Installs the live Core command callback after the local application has been assembled.
    #[allow(non_snake_case)]
    pub fn setCoreCommandExecutor(
        &self,
        executor: operit_host_api::HostManager::CoreCommandExecutor,
    ) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .context
            .coreCommandExecutor = Some(executor);
    }

    /// Returns the dependency set associated with this handler.
    #[allow(non_snake_case)]
    pub fn runtimeDependencies(&self) -> ToolRuntimeDependencies {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .runtimeDependencies
            .clone()
    }

    /// Returns the runtime support implementation associated with this handler.
    #[allow(non_snake_case)]
    pub fn runtimeSupport(&self) -> Arc<dyn ToolRuntimeSupport> {
        self.runtimeDependencies().shared_runtime_support()
    }

    /// Returns the shared package manager, creating it with this handler context.
    #[allow(non_snake_case)]
    pub fn getOrCreatePackageManager(&self) -> Arc<Mutex<RuntimePackageManager>> {
        {
            let guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
            if let Some(packageManager) = &guard.packageManager {
                return packageManager.clone();
            }
        }
        let packageManager = Arc::new(Mutex::new(RuntimePackageManager::new(
            RuntimeStorePaths::default(),
            self.clone(),
        )));
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        if let Some(existingPackageManager) = &guard.packageManager {
            return existingPackageManager.clone();
        }
        guard.packageManager = Some(packageManager.clone());
        packageManager
    }

    /// Returns tool names that should be visible to normal callers.
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

    /// Returns tool names reserved for internal runtime calls.
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

    /// Registers a public tool executor.
    #[allow(non_snake_case)]
    pub fn registerTool(&mut self, name: String, executor: Box<dyn ToolExecutor>) {
        self.registerToolWithVisibility(name, executor, ToolRegistrationVisibility::PUBLIC);
    }

    /// Registers an internal tool executor.
    #[allow(non_snake_case)]
    pub fn registerInternalTool(&mut self, name: String, executor: Box<dyn ToolExecutor>) {
        self.registerToolWithVisibility(name, executor, ToolRegistrationVisibility::INTERNAL);
    }

    /// Registers one statically declared built-in tool with explicit visibility.
    #[allow(non_snake_case)]
    pub fn registerBuiltinTool<E: Into<RegisteredToolExecutor>>(
        &mut self,
        name: BuiltinToolName,
        executor: E,
        visibility: ToolRegistrationVisibility,
    ) {
        let toolName = name.as_str().to_string();
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        assert!(
            !guard.availableTools.contains_key(&toolName),
            "Built-in tool is registered more than once: {toolName}"
        );
        assert!(
            !guard.unavailableBuiltinTools.contains_key(&name),
            "Built-in tool cannot be both registered and unavailable: {toolName}"
        );
        guard.availableTools.insert(
            toolName.clone(),
            RegisteredToolExecutor::Asynchronous(Box::new(ContractCheckedBuiltinToolExecutor {
                name,
                executor: executor.into(),
            })),
        );
        guard.toolVisibility.insert(toolName, visibility);
    }

    /// Marks built-in tools as unavailable because their required host capability is absent.
    #[allow(non_snake_case)]
    pub(crate) fn markBuiltinToolsUnavailable(&mut self, names: &[BuiltinToolName], reason: &str) {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        for name in names {
            assert!(
                !guard.availableTools.contains_key(name.as_str()),
                "Registered built-in tool cannot be marked unavailable: {name}"
            );
            assert!(
                guard
                    .unavailableBuiltinTools
                    .insert(*name, reason.to_string())
                    .is_none(),
                "Built-in tool is marked unavailable more than once: {name}"
            );
        }
    }

    /// Registers a tool executor with explicit public or internal visibility.
    #[allow(non_snake_case)]
    pub fn registerToolWithVisibility(
        &mut self,
        name: String,
        executor: Box<dyn ToolExecutor>,
        visibility: ToolRegistrationVisibility,
    ) {
        assert!(
            BuiltinToolName::from_name(&name).is_none(),
            "Built-in tools must use registerBuiltinTool: {name}"
        );
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard
            .availableTools
            .insert(name.clone(), RegisteredToolExecutor::Synchronous(executor));
        guard.toolVisibility.insert(name, visibility);
    }

    /// Registers a public tool with an explicitly asynchronous execution contract.
    pub fn registerAsyncTool(&mut self, name: String, executor: Box<dyn AsyncToolExecutor>) {
        assert!(
            BuiltinToolName::from_name(&name).is_none(),
            "Built-in tools must use their typed registration"
        );
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard
            .availableTools
            .insert(name.clone(), RegisteredToolExecutor::Asynchronous(executor));
        guard
            .toolVisibility
            .insert(name, ToolRegistrationVisibility::PUBLIC);
    }

    /// Returns the configured visibility for one tool.
    #[allow(non_snake_case)]
    pub fn getToolVisibility(&self, toolName: &str) -> Option<ToolRegistrationVisibility> {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .toolVisibility
            .get(toolName)
            .copied()
    }

    /// Registers built-in public and internal tools once for this handler.
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
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        for name in BuiltinToolName::ALL {
            let registered = guard.availableTools.contains_key(name.as_str());
            let unavailable = guard.unavailableBuiltinTools.contains_key(name);
            assert!(
                registered ^ unavailable,
                "Built-in tool must be exactly registered or unavailable: {name}"
            );
        }
        guard.defaultToolsRegistered = true;
    }

    #[allow(non_snake_case)]
    pub fn getToolExecutor(&mut self, _toolName: &str) -> Option<&mut Box<dyn ToolExecutor>> {
        None
    }

    /// Returns whether a tool executor is already registered.
    #[allow(non_snake_case)]
    pub fn hasToolExecutor(&self, toolName: &str) -> bool {
        let guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard.availableTools.contains_key(toolName) || guard.executingTools.contains(toolName)
    }

    /// Acquires one named executor, waiting for an in-flight invocation of that same tool to finish.
    async fn takeToolExecutorForExecution(&self, toolName: &str) -> Option<ToolExecutorLease> {
        loop {
            let notified = self.executorAvailability.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
                if let Some(executor) = guard.availableTools.remove(toolName) {
                    assert!(
                        guard.executingTools.insert(toolName.to_owned()),
                        "Tool executor was already marked executing: {toolName}"
                    );
                    return Some(ToolExecutorLease {
                        name: toolName.to_owned(),
                        executor: Some(executor),
                        state: self.inner.clone(),
                        available: self.executorAvailability.clone(),
                    });
                }
                if !guard.executingTools.contains(toolName) {
                    return None;
                }
            }
            notified.await;
        }
    }

    /// Restores one completed executor and wakes calls queued for the same tool name.
    fn restoreToolExecutorAfterExecution(&self, toolName: String, executor: ToolExecutorLease) {
        assert_eq!(
            toolName, executor.name,
            "the restored tool must own this lease"
        );
        drop(executor);
    }

    /// Ensures default or package tools are registered, then reports whether a tool exists.
    #[allow(non_snake_case)]
    pub async fn getToolExecutorOrActivate(&mut self, toolName: &str) -> bool {
        if let Some((packageName, _)) = toolName.split_once(':') {
            let manager = self.getOrCreatePackageManager();
            if RuntimePackageManager::prepareMcpPackage(&manager, packageName.trim()).await.is_err() {
                return false;
            }
        }
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
        let Some(client) = mcpManager.getClientForMetadata(packageName) else {
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
        packageManager: Arc<Mutex<RuntimePackageManager>>,
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
                self.registerAsyncTool(
                    toolName,
                    Box::new(MCPToolExecutor::new(MCPManager::getInstance(
                        context.clone(),
                    ))),
                );
            } else {
                self.registerAsyncTool(
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

    /// Runs access validation and asynchronously obtains every required approval.
    async fn executeAccessPreflightAsync(
        &self,
        tool: &AITool,
        accessSpec: ToolAccessSpec,
    ) -> Result<ToolAccessSpec, ToolResult> {
        let permissionSystem = self.getToolPermissionSystem();
        let chatId = ToolExecutionManager::currentToolRuntimeContext()
            .and_then(|context| context.callerChatId);
        let mode = permissionSystem
            .getAiPermissionMode()
            .map_err(|error| ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(error.to_string()),
            })?;
        let context = self.getContext();
        let workspaceBoundary = Self::checkWorkspaceBoundary(&mode, tool, &accessSpec, context.runtimeStorageHost.as_ref());
        let policyOverrideReason = match (mode.allowsEffect(accessSpec.effect), workspaceBoundary) {
            (_, Err(WorkspaceBoundaryError::InvalidRequest(error))) => {
                return Err(ToolResult {
                    toolName: tool.name.clone(),
                    success: false,
                    result: stringResultData(""),
                    error: Some(error),
                });
            }
            (false, _) => Some(format!(
                "AI permission mode {} requires approval for {:?} tool execution.",
                mode.name(),
                accessSpec.effect
            )),
            (true, Err(WorkspaceBoundaryError::RequiresApproval(reason))) => Some(reason),
            (true, Ok(())) => None,
        };
        let policyOverrideApproved = if let Some(reason) = policyOverrideReason.as_deref() {
            let approved = permissionSystem
                .checkSandboxEscapeApprovalAsync(tool, chatId.clone())
                .await
                .map_err(|error| ToolResult {
                    toolName: tool.name.clone(),
                    success: false,
                    result: stringResultData(""),
                    error: Some(error.to_string()),
                })?;
            if !approved {
                let error = "User cancelled the tool execution.".to_string();
                self.notifyToolPermissionChecked(tool, false, Some(&error));
                return Err(ToolResult {
                    toolName: tool.name.clone(),
                    success: false,
                    result: stringResultData(""),
                    error: Some(error),
                });
            }
            self.notifyToolPermissionChecked(tool, true, Some(reason));
            true
        } else {
            false
        };

        if mode == AiPermissionMode::WorkspaceWrite
            && accessSpec.effect == ToolEffect::WRITE
            && matches!(accessSpec.boundary, ToolBoundary::None)
            && !policyOverrideApproved
        {
            let approved = permissionSystem
                .checkSandboxEscapeApprovalAsync(tool, chatId.clone())
                .await
                .map_err(|error| ToolResult {
                    toolName: tool.name.clone(),
                    success: false,
                    result: stringResultData(""),
                    error: Some(error.to_string()),
                })?;
            if !approved {
                let error = "User cancelled the tool execution.".to_string();
                self.notifyToolPermissionChecked(tool, false, Some(&error));
                return Err(ToolResult {
                    toolName: tool.name.clone(),
                    success: false,
                    result: stringResultData(""),
                    error: Some(error),
                });
            }
            self.notifyToolPermissionChecked(
                tool,
                true,
                Some("WorkspaceWrite non-file WRITE approved for this session."),
            );
        }

        Ok(accessSpec)
    }

    /// Restricts file writes to the current workspace in workspace-scoped modes.
    fn checkWorkspaceBoundary(
        mode: &AiPermissionMode,
        tool: &AITool,
        accessSpec: &ToolAccessSpec,
        storage: Option<&Arc<dyn operit_host_api::RuntimeStorageHost>>,
    ) -> Result<(), WorkspaceBoundaryError> {
        if *mode == AiPermissionMode::Full {
            return Ok(());
        }
        match &accessSpec.boundary {
            ToolBoundary::None => Ok(()),
            ToolBoundary::FilePath { effect } => {
                Self::checkWorkspaceWritePath(tool, "path", *effect, storage)
            }
            ToolBoundary::FilePair {
                source,
                destination,
            } => {
                Self::checkWorkspaceWritePath(tool, "source", *source, storage)?;
                Self::checkWorkspaceWritePath(tool, "destination", *destination, storage)
            }
        }
    }

    /// Checks that one file write parameter stays inside the current workspace.
    fn checkWorkspaceWritePath(
        tool: &AITool,
        parameterName: &str,
        effect: ToolEffect,
        storage: Option<&Arc<dyn operit_host_api::RuntimeStorageHost>>,
    ) -> Result<(), WorkspaceBoundaryError> {
        if effect == ToolEffect::READ {
            return Ok(());
        }
        let path = tool
            .parameters
            .iter()
            .find(|parameter| parameter.name == parameterName)
            .map(|parameter| parameter.value.trim())
            .ok_or_else(|| {
                WorkspaceBoundaryError::InvalidRequest(format!(
                    "{parameterName} parameter is required"
                ))
            })?;
        if path.is_empty() {
            return Err(WorkspaceBoundaryError::InvalidRequest(format!(
                "{parameterName} parameter is required"
            )));
        }

        let runtimeContext =
            ToolExecutionManager::currentToolRuntimeContext().ok_or_else(|| {
                WorkspaceBoundaryError::RequiresApproval(
                    "File tool execution requires tool runtime context".to_string(),
                )
            })?;
        let workspaceFolders = runtimeContext
            .workspaceFolders
            .iter()
            .map(|folder| folder.trim().to_string())
            .filter(|folder| !folder.is_empty())
            .collect::<Vec<_>>();
        if workspaceFolders.is_empty() {
            return Err(WorkspaceBoundaryError::RequiresApproval(
                "File tool execution requires a current workspace".to_string(),
            ));
        }

        let storage = storage.ok_or_else(|| WorkspaceBoundaryError::RequiresApproval(
            "File tool execution requires its runtime storage Host".to_string()
        ))?;
        let runtimeRoot = storage.runtimeRootDir().ok_or_else(|| WorkspaceBoundaryError::InvalidRequest(
            "Runtime storage Host does not expose its runtime root".to_string()
        ))?;
        let workspaceRoot = storage.workspaceRootDir().ok_or_else(|| WorkspaceBoundaryError::InvalidRequest(
            "Runtime storage Host does not expose its workspace root".to_string()
        ))?;
        let mapper = PathMapper::new(runtimeRoot, workspaceRoot, storage.clone());
        let resolvedPath = mapper
            .resolve(path)
            .map_err(WorkspaceBoundaryError::InvalidRequest)?;
        for folder in &workspaceFolders {
            let resolvedWorkspace = mapper
                .resolve(folder)
                .map_err(WorkspaceBoundaryError::InvalidRequest)?;
            let relative =
                PathMapper::relativePath(&resolvedWorkspace.vfsPath, &resolvedPath.vfsPath)
                    .map_err(WorkspaceBoundaryError::InvalidRequest)?;
            if relative.is_some() {
                return Ok(());
            }
        }
        return Err(WorkspaceBoundaryError::RequiresApproval(format!(
            "{:?} file access is limited to current workspace folders: {}",
            effect,
            workspaceFolders.join(", ")
        )));
    }

    /// Executes an AI-originated tool through permission checks and a resolved executor.
    #[allow(non_snake_case)]
    pub async fn executeToolSafelyWithResolvedExecutor(
        &mut self,
        tool: &AITool,
    ) -> Option<Vec<ToolResult>> {
        self.executeToolWithResolvedExecutor(tool, ToolExecutionAccess::AiPermission)
            .await
    }

    /// Executes a non-AI tool directly through a resolved executor.
    #[allow(non_snake_case)]
    pub async fn executeToolDirectlyWithResolvedExecutor(
        &mut self,
        tool: &AITool,
    ) -> Option<Vec<ToolResult>> {
        self.executeToolWithResolvedExecutor(tool, ToolExecutionAccess::Direct)
            .await
    }

    /// Executes a streamed tool call according to its caller-owned access policy.
    #[allow(non_snake_case)]
    async fn executeToolWithResolvedExecutor(
        &mut self,
        tool: &AITool,
        access: ToolExecutionAccess,
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
        if !self.getToolExecutorOrActivate(&tool.name).await {
            ChainLogger::warn(
                TOOL_CHAIN,
                "tool.stream.not_found",
                &[("tool", tool.name.clone())],
            );
            return None;
        }

        let Some(mut executor) = self.takeToolExecutorForExecution(&tool.name).await else {
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
            match access {
                ToolExecutionAccess::AiPermission => {
                    let accessSpec = executor.accessSpec(tool).map_err(|error| ToolResult {
                        toolName: tool.name.clone(),
                        success: false,
                        result: stringResultData(""),
                        error: Some(format!("Tool access declaration failed: {error}")),
                    });
                    match accessSpec {
                        Ok(accessSpec) => {
                            match self.executeAccessPreflightAsync(tool, accessSpec).await {
                                Ok(accessSpec) => {
                                    ChainLogger::info(
                                        TOOL_CHAIN,
                                        "tool.stream.start",
                                        &[
                                            ("tool", tool.name.clone()),
                                            ("parameters", parameterSummary.clone()),
                                            ("effect", format!("{:?}", accessSpec.effect)),
                                        ],
                                    );
                                    executor.invokeAndStreamAsync(tool).await
                                }
                                Err(errorResult) => vec![errorResult],
                            }
                        }
                        Err(errorResult) => vec![errorResult],
                    }
                }
                ToolExecutionAccess::Direct => {
                    ChainLogger::info(
                        TOOL_CHAIN,
                        "tool.stream.start",
                        &[
                            ("tool", tool.name.clone()),
                            ("parameters", parameterSummary.clone()),
                            ("access", "direct".to_string()),
                        ],
                    );
                    executor.invokeAndStreamAsync(tool).await
                }
            }
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

        self.restoreToolExecutorAfterExecution(tool.name.clone(), executor);
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

    /// Resolves and directly executes a non-AI tool request through the registered tool chain.
    #[allow(non_snake_case)]
    pub async fn executeTool(&mut self, tool: AITool) -> ToolResult {
        ChainLogger::info(
            TOOL_CHAIN,
            "tool.execute.request",
            &[
                ("tool", tool.name.clone()),
                ("parameterCount", tool.parameters.len().to_string()),
            ],
        );
        self.notifyToolCallRequested(&tool);
        let interception = self.checkToolInterception(&tool).await;
        if let AIToolHookDecision::Block(_) = interception {
            let result = Self::toolInterceptionResult(&tool, interception);
            self.notifyToolExecutionResult(&tool, &result);
            self.notifyToolExecutionFinished(&tool);
            return result;
        }
        self.getToolExecutorOrActivate(&tool.name).await;
        let Some(mut executor) = self.takeToolExecutorForExecution(&tool.name).await else {
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
            self.restoreToolExecutorAfterExecution(tool.name.clone(), executor);
            return validationFailedResult;
        }

        self.notifyToolExecutionStarted(&tool);
        ChainLogger::info(
            TOOL_CHAIN,
            "tool.execute.start",
            &[("tool", tool.name.clone())],
        );
        let collected = executor.invokeAndStreamAsync(&tool).await;
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
        self.restoreToolExecutorAfterExecution(tool.name.clone(), executor);
        result
    }

    #[allow(non_snake_case)]
    /// Removes all registered executors and returns their ownership to the caller.
    pub fn takeExecutors(&mut self) -> BTreeMap<String, RegisteredToolExecutor> {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        if !guard.defaultToolsRegistered {
            drop(guard);
            self.registerDefaultTools();
            guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        }
        std::mem::take(&mut guard.availableTools)
    }

    #[allow(non_snake_case)]
    /// Restores a previously removed executor registry.
    pub fn restoreExecutors(&mut self, executors: BTreeMap<String, RegisteredToolExecutor>) {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .availableTools = executors;
    }

    /// Clears registered tools and runtime-only handler state.
    pub fn reset(&mut self) {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard.availableTools.clear();
        guard.toolVisibility.clear();
        guard.unavailableBuiltinTools.clear();
        guard.defaultToolsRegistered = false;
    }
}

impl AIToolHandlerState {
    /// Returns the host manager associated with this handler state.
    #[allow(non_snake_case)]
    pub fn getContext(&self) -> &HostManager {
        &self.context
    }
}

impl JsExecutionHost for AIToolHandler {
    /// Returns executable built-in and package tools with their declared parameters.
    fn get_tool_catalog(&self) -> Result<Value, String> {
        let mut handler = self.clone();
        handler.registerDefaultTools();
        let language = self.package_language()?;
        let useEnglish = language.to_ascii_lowercase().starts_with("en");
        let hostEnvironment = self.getHostEnvironmentDescriptor();
        let categories = self
            .runtimeSupport()
            .buildBuiltinAndInternalCategories(useEnglish, &hostEnvironment);
        let mut tools = Vec::new();
        for category in categories {
            for tool in category.tools {
                let parameters = tool
                    .parametersStructured
                    .unwrap_or_default()
                    .into_iter()
                    .map(|parameter| {
                        json!({
                            "name": parameter.name,
                            "type": parameter.r#type,
                            "description": parameter.description,
                            "required": parameter.required,
                            "default": parameter.default,
                        })
                    })
                    .collect::<Vec<_>>();
                tools.push(json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": parameters,
                    "category": category.categoryName,
                    "source": "builtin",
                }));
            }
        }

        let packageManager = self.getOrCreatePackageManager();
        let packageNames = packageManager
            .lock()
            .map_err(|_| "package manager mutex poisoned".to_string())?
            .getAvailablePackages()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for packageName in packageNames {
            let package = packageManager
                .lock()
                .map_err(|_| "package manager mutex poisoned".to_string())?
                .getEffectivePackageTools(&packageName);
            let Some(package) = package else {
                continue;
            };
            for tool in package.tools.into_iter().filter(|tool| !tool.advice) {
                let parameters = tool
                    .parameters
                    .into_iter()
                    .map(|parameter| {
                        json!({
                            "name": parameter.name,
                            "type": parameter.parameter_type,
                            "description": parameter.description.resolve(useEnglish),
                            "required": parameter.required,
                            "default": Value::Null,
                        })
                    })
                    .collect::<Vec<_>>();
                tools.push(json!({
                    "name": format!("{}:{}", packageName, tool.name),
                    "description": tool.description.resolve(useEnglish),
                    "parameters": parameters,
                    "category": package.category,
                    "source": "package",
                    "packageName": packageName,
                }));
            }
        }
        Ok(json!({ "tools": tools }))
    }

    /// Executes an SDK JavaScript tool request through the registered Operit tool chain.
    fn execute_tool_call(
        &self,
        request: JsToolCallRequest,
    ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<JsToolCallResult> {
        let host = self.clone();
        Box::pin(async move {
            let tool = AITool {
                name: request.qualified_tool_name(),
                parameters: request
                    .parameters
                    .into_iter()
                    .map(|(name, value)| ToolParameter {
                        name,
                        value: match value {
                            serde_json::Value::Null => String::new(),
                            serde_json::Value::String(value) => value,
                            value => value.to_string(),
                        },
                    })
                    .collect(),
            };
            let mut handler = host;
            let result = handler.executeTool(tool).await;
            let data = match result.result {
                ToolResultData::BinaryResultData(data) => JsToolCallResultData::Binary(data.value),
                ToolResultData::StringResultData(data) => {
                    JsToolCallResultData::Value(serde_json::Value::String(data.value))
                }
                ToolResultData::BooleanResultData(data) => {
                    JsToolCallResultData::Value(serde_json::Value::Bool(data.value))
                }
                ToolResultData::IntResultData(data) => JsToolCallResultData::Value(
                    serde_json::Value::Number(serde_json::Number::from(data.value)),
                ),
                data => JsToolCallResultData::Value(
                    serde_json::from_str(&data.toJson())
                        .expect("ToolResultData JSON conversion must succeed"),
                ),
            };
            JsToolCallResult {
                success: result.success,
                data,
                error: result.error,
            }
        })
    }

    /// Returns the current host language.
    fn package_language(&self) -> Result<String, String> {
        LocaleUtils::getCurrentLanguage(&self.getContext(), "")
    }

    /// Reads one runtime environment variable.
    fn read_environment_variable(&self, key: &str) -> Result<Option<String>, String> {
        self.runtimeDependencies()
            .runtime_support()
            .readEnvironmentVariable(key)
    }

    /// Writes one runtime environment variable.
    fn write_environment_variable(&self, key: &str, value: &str) -> Result<(), String> {
        self.runtimeDependencies()
            .runtime_support()
            .writeEnvironmentVariable(key, value)
    }

    /// Resolves unscoped native configuration access explicitly within this device.
    fn plugin_config_dir(&self, plugin_id: &str) -> Result<String, String> {
        let configDir = operit_util::OperitPaths::pluginConfigDir(plugin_id)?;
        let path = configDir.to_string_lossy().replace('\\', "/");
        let absolute = RuntimeStorePaths::default().runtime_storage_path(&path);
        self.getContext()
            .fileSystemHost
            .as_ref()
            .ok_or("FileSystemHost is required for plugin configuration")?
            .makeDirectory(&absolute.to_string_lossy(), true)
            .map_err(|e| e.to_string())?;
        let relative = path
            .strip_prefix("runtime/")
            .ok_or("Configuration is outside runtime storage")?;
        PathMapper::joinVfsPath("/app/data", relative)
    }

    /// Resolves the executing package owner while keeping arbitrary configuration aliases supported.
    fn scoped_plugin_config_dir(&self, owner_id: &str, plugin_id: &str) -> Result<String, String> {
        // Host callbacks must not acquire the package manager lock held by script callers.
        let store = operit_store::ExtensionStore::ExtensionStore::default();
        let owner = store.packageOwner(owner_id)?;
        self.createPackageConfigDirectory(&owner.id, plugin_id, &owner.scope)
    }

    /// Creates registration configuration from its selected scope without querying runtime state.
    fn registration_plugin_config_dir(
        &self,
        owner_id: &str,
        plugin_id: &str,
        scope: ToolPkgConfigScope,
    ) -> Result<String, String> {
        self.createPackageConfigDirectory(owner_id, plugin_id, scope.as_str())
    }

    /// Reads one ToolPkg text resource.
    fn read_toolpkg_text_resource(
        &self,
        target: &str,
        path: &str,
    ) -> Result<Option<String>, String> {
        Ok(self
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .readToolPkgTextResource(target, path, true))
    }

    /// Materializes one ToolPkg binary resource.
    fn materialize_toolpkg_resource(
        &self,
        request: JsToolPkgResourceRequest,
    ) -> Result<String, String> {
        crate::tools::ToolJsRuntime::materializeToolPkgResource(self, request)
    }

    /// Calls one ToolPkg WASM export through the active package manager.
    fn call_toolpkg_wasm(
        &self,
        request: JsToolPkgWasmRequest,
    ) -> Result<JsToolPkgWasmResult, String> {
        self.getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .callToolPkgWasm(request, true)
    }

    /// Dispatches one Compose DSL controller command.
    fn handle_compose_webview_controller_command(
        &self,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.getContext()
            .composeDslWebViewHost
            .as_ref()
            .ok_or_else(|| "ComposeDslWebViewHost is not registered".to_string())?
            .handleControllerCommand(payload)
            .map_err(|error| error.to_string())
    }

    /// Validates and forwards one structured file-picker request to the registered host owner.
    fn open_compose_file_picker(
        &self,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let request = operit_host_api::ComposeDslFilePickerRequest::fromValue(payload.clone())
            .map_err(|error| error.to_string())?;
        self.getContext()
            .composeDslWebViewHost
            .as_ref()
            .ok_or_else(|| "ComposeDslWebViewHost is not registered".to_string())?
            .openFilePicker(request)
            .map_err(|error| error.to_string())
    }

    /// Returns whether one package is imported.
    fn is_package_imported(&self, package_name: &str) -> Result<bool, String> {
        Ok(self
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .isPackageEnabled(package_name))
    }

    /// Imports one package.
    fn import_package(&self, package_name: &str) -> Result<String, String> {
        Ok(self
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .enablePackage(package_name))
    }

    /// Removes one package.
    fn remove_package(&self, package_name: &str) -> Result<String, String> {
        Ok(self
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .disablePackage(package_name))
    }

    /// Activates one package.
    fn use_package(&self, package_name: &str) -> Result<String, String> {
        Ok(self
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .usePackage(package_name))
    }

    /// Lists imported packages.
    fn list_imported_packages(&self) -> Result<Vec<String>, String> {
        Ok(self
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .getEnabledPackageNames())
    }

    /// Resolves one package-aware tool name.
    fn resolve_tool_name(&self, request: JsToolNameResolutionRequest) -> Result<String, String> {
        crate::tools::ToolJsRuntime::resolveJsToolName(self, request)
    }

    /// Submits one ToolPkg IPC request to the host scheduler.
    fn invoke_toolpkg_ipc_async(
        &self,
        request: JsToolPkgIpcRequest,
        completion: JsToolPkgIpcCompletion,
    ) -> Result<(), String> {
        let packageTarget = request.package_target.clone();
        let channel = request.channel.clone();
        let targetContextKey = request.target_context_key.clone().unwrap_or_default();
        let targetRuntime = request.target_runtime.clone().unwrap_or_default();
        let toolHandler = self.clone();
        let scheduler = self
            .getContext()
            .hostRuntimeTaskSchedulerHost
            .clone()
            .ok_or_else(|| {
                "HostRuntimeTaskSchedulerHost is required for ToolPkg IPC".to_string()
            })?;
        ChainLogger::info(
            TOOL_CHAIN,
            "toolpkg.ipc.submit",
            &[
                ("package", packageTarget.clone()),
                ("channel", channel.clone()),
                ("targetContext", targetContextKey.clone()),
                ("targetRuntime", targetRuntime.clone()),
            ],
        );
        scheduler
            .scheduleHostRuntimeAsyncTask(
                "operit-toolpkg-ipc",
                Box::new(move || {
                    Box::pin(async move {
                        let startedMs = operit_host_api::TimeUtils::currentTimeMillis();
                        ChainLogger::info(
                            TOOL_CHAIN,
                            "toolpkg.ipc.target_start",
                            &[
                                ("package", packageTarget.clone()),
                                ("channel", channel.clone()),
                                ("targetContext", targetContextKey.clone()),
                                ("targetRuntime", targetRuntime.clone()),
                            ],
                        );
                        let result =
                            crate::tools::ToolJsRuntime::invokeToolPkgIpc(&toolHandler, request)
                                .await;
                        let elapsedMs = operit_host_api::TimeUtils::currentTimeMillis()
                            .saturating_sub(startedMs);
                        let outcome = if result.is_ok() { "success" } else { "error" };
                        ChainLogger::info(
                            TOOL_CHAIN,
                            "toolpkg.ipc.target_finish",
                            &[
                                ("package", packageTarget),
                                ("channel", channel),
                                ("targetContext", targetContextKey),
                                ("targetRuntime", targetRuntime),
                                ("elapsedMs", elapsedMs.to_string()),
                                ("outcome", outcome.to_string()),
                            ],
                        );
                        completion(result);
                    })
                }),
            )
            .map_err(|error| error.to_string())
    }

    /// Waits for the runtime scheduler to advance JavaScript work on a later host turn.
    fn wait_for_javascript_runtime_turn(&self) -> operit_host_api::HostRuntimeTurnFuture {
        self.getContext()
            .hostRuntimeTaskSchedulerHost
            .as_ref()
            .expect("HostRuntimeTaskSchedulerHost is required for JavaScript execution")
            .waitForHostRuntimeTaskTurn()
    }
}

pub struct FnToolExecutor {
    pub invoke: Arc<dyn Fn(&AITool) -> ToolResult + Send + Sync>,
    pub validate: Arc<dyn Fn(&AITool) -> ToolValidationResult + Send + Sync>,
    pub effect: ToolEffect,
}

/// Executes a proxy whose nested package invocation returns an asynchronous result.
pub struct AsyncFnToolExecutor {
    pub invoke: Arc<
        dyn Fn(AITool) -> operit_plugin_sdk::javascript::JsExecutionCompletion<ToolResult>
            + Send
            + Sync,
    >,
    pub validate: Arc<dyn Fn(&AITool) -> ToolValidationResult + Send + Sync>,
    pub effect: ToolEffect,
}

impl AsyncToolExecutor for AsyncFnToolExecutor {
    /// Validates the proxy request before beginning execution.
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        (self.validate)(tool)
    }
    /// Declares the proxy's access effect.
    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        Ok(ToolAccessSpec {
            effect: self.effect,
            boundary: ToolBoundary::None,
        })
    }
    /// Awaits the nested invocation on its registered asynchronous path.
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool) -> ToolInvocationFuture<'a> {
        let invocation = (self.invoke)(tool.clone());
        Box::pin(async move { vec![invocation.await] })
    }
}

/// Enforces the registered name and successful result contract for one built-in executor.
struct ContractCheckedBuiltinToolExecutor {
    name: BuiltinToolName,
    executor: RegisteredToolExecutor,
}

impl AsyncToolExecutor for ContractCheckedBuiltinToolExecutor {
    /// Validates parameters only for the exact built-in name bound during registration.
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        assert_eq!(
            tool.name,
            self.name.as_str(),
            "Built-in executor received a different tool name"
        );
        self.executor.validateParameters(tool)
    }

    /// Delegates access classification after verifying the registered built-in name.
    fn accessSpec(&self, tool: &AITool) -> Result<ToolAccessSpec, String> {
        assert_eq!(
            tool.name,
            self.name.as_str(),
            "Built-in executor received a different tool name"
        );
        self.executor.accessSpec(tool)
    }

    /// Verifies every successful result emitted by the wrapped built-in executor.
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool) -> ToolInvocationFuture<'a> {
        Box::pin(async move {
            assert_eq!(
                tool.name,
                self.name.as_str(),
                "Built-in executor received a different tool name"
            );
            let results = self.executor.invokeAndStreamAsync(tool).await;
            for result in &results {
                if self.name != BuiltinToolName::PackageProxy {
                    assert_eq!(
                        result.toolName,
                        self.name.as_str(),
                        "Built-in executor emitted a result for a different tool name"
                    );
                }
                assert!(
                    !result.success || self.name.accepts_runtime_result(&result.result),
                    "Built-in executor emitted an incompatible successful result: {}",
                    self.name
                );
                assert!(
                    result.success || result.error.is_some(),
                    "Built-in executor emitted a failed result without an error: {}",
                    self.name
                );
            }
            results
        })
    }
}

impl ToolExecutor for FnToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        (self.validate)(tool)
    }

    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        Ok(ToolAccessSpec {
            effect: self.effect,
            boundary: ToolBoundary::None,
        })
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        vec![(self.invoke)(tool)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a file-path access declaration for workspace boundary tests.
    fn filePathAccessSpec(effect: ToolEffect) -> ToolAccessSpec {
        ToolAccessSpec {
            effect,
            boundary: ToolBoundary::FilePath { effect },
        }
    }

    /// Builds a file tool invocation with one VFS path parameter.
    fn pathTool(path: &str) -> AITool {
        AITool {
            name: "list_files".to_string(),
            parameters: vec![ToolParameter {
                name: "path".to_string(),
                value: path.to_string(),
            }],
        }
    }

    /// Confirms Full mode does not require a runtime workspace for file paths.
    #[test]
    fn fullModeDoesNotRequireRuntimeWorkspaceForFilePaths() {
        let result = AIToolHandler::checkWorkspaceBoundary(
            &AiPermissionMode::Full,
            &pathTool("/mnt/android/sdcard"),
            &filePathAccessSpec(ToolEffect::READ),
            None,
        );

        assert_eq!(result, Ok(()));
    }

    /// Confirms workspace-scoped modes allow external file reads without a runtime workspace.
    #[test]
    fn workspaceScopedModesAllowExternalFileReadsWithoutRuntimeWorkspace() {
        let readOnlyResult = AIToolHandler::checkWorkspaceBoundary(
            &AiPermissionMode::ReadOnly,
            &pathTool("/mnt/android/sdcard"),
            &filePathAccessSpec(ToolEffect::READ),
            None,
        );
        let workspaceWriteResult = AIToolHandler::checkWorkspaceBoundary(
            &AiPermissionMode::WorkspaceWrite,
            &pathTool("/mnt/android/sdcard"),
            &filePathAccessSpec(ToolEffect::READ),
            None,
        );

        assert_eq!(readOnlyResult, Ok(()));
        assert_eq!(workspaceWriteResult, Ok(()));
    }

    /// Confirms WorkspaceWrite still requires a runtime workspace for file writes.
    #[test]
    fn workspaceWriteRequiresRuntimeWorkspaceForFileWrites() {
        let result = AIToolHandler::checkWorkspaceBoundary(
            &AiPermissionMode::WorkspaceWrite,
            &pathTool("/mnt/android/sdcard"),
            &filePathAccessSpec(ToolEffect::WRITE),
            None,
        );

        assert_eq!(
            result,
            Err(WorkspaceBoundaryError::RequiresApproval(
                "File tool execution requires tool runtime context".to_string()
            ))
        );
    }
}
