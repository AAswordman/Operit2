use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use operit_host_api::HostEnvironmentDescriptor;
use operit_host_api::HostManager::HostManager;
use operit_link::{CoreCallRequest, CoreEventStream, CoreWatchRequest};
use operit_plugin_sdk::javascript::{
    JsExecutionHost, JsToolCallRequest, JsToolCallResult, JsToolCallResultData,
    JsToolNameResolutionRequest, JsToolPkgIpcCompletion, JsToolPkgIpcRequest,
    JsToolPkgResourceRequest, JsToolPkgWasmRequest, JsToolPkgWasmResult, ToolPkgConfigScope,
};
use operit_plugin_sdk::js_sdk::chat::ChatSendEvent;
use operit_plugin_sdk::js_sdk::JsAsyncIterable;
use operit_plugin_sdk::js_sdk::tool_types::BuiltinToolName;
use operit_plugin_sdk::package::ToolPackage;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::tools::defaultTool::standard::StandardMediaTools::register_native_media;
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
    ToolCall, ToolExecutionManager, ToolExecutionPolicy, ToolExecutor, ToolInvocationContext, ToolInvocationFuture, ToolParameter, ToolValidationResult, SharedToolExecutor,
};
use operit_util::ChainLogger::{self, TOOL_CHAIN};
use operit_util::LocaleUtils::LocaleUtils;
use operit_util::OperitPaths;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::runtime_support::{ToolRuntimeDependencies, ToolRuntimeSupport};

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

/// Owns one authenticated route stream and its pending-read cancellation signal.
pub(crate) struct CoreRouteWatch {
    stream: tokio::sync::Mutex<CoreEventStream>,
    closed: tokio::sync::Notify,
    value: Mutex<Option<operit_link::CoreValue>>,
}

/// Owns a cancellable input stream without holding the registry lock across network waits.
pub(crate) struct CoreRoutePush {
    session: tokio::sync::Mutex<Option<Box<dyn operit_link::CoreLinkPushSession>>>,
    closed: tokio::sync::Notify,
}

/// Serializes a decorated invocation with an engine-authenticated package owner.
fn toolPkgRouteArguments(owner: Option<String>, method: String, args: Value) -> Result<operit_link::CoreValue, String> {
    let owner = owner.ok_or("Plugin routes require an authenticated engine")?;
    crate::tools::packTool::ToolPkgRouteRuntime::requireSpacePackage(&owner)?;
    let mut args = args.as_object().cloned().ok_or("Plugin route arguments must be an object")?;
    if args.contains_key("packageName") || args.contains_key("methodName") { return Err("Plugin route envelope cannot override authenticated identity".into()); }
    args.insert("packageName".into(), Value::String(owner));
    args.insert("methodName".into(), Value::String(method));
    operit_link::toCoreValue(Value::Object(args)).map_err(|error| error.to_string())
}

#[derive(Clone)]
pub struct AIToolHandler {
    inner: Arc<Mutex<AIToolHandlerState>>,
    /// Contains only the immutable package identity bound by the JavaScript engine.
    pub(crate) authenticatedExtensionOwner: Option<String>,
    pub(crate) storageSession: operit_store::PluginStorage::PluginStorageSession,
    pub(crate) deviceTransportScope: Arc<crate::tools::DeviceTools::DeviceTransportScope>,
    pub(crate) chatObservations: Arc<Mutex<BTreeMap<String, JsAsyncIterable<ChatSendEvent>>>>,
    pub(crate) coreRoutePushes: Arc<Mutex<BTreeMap<String, Arc<CoreRoutePush>>>>,
    pub(crate) coreRouteWatches: Arc<Mutex<BTreeMap<String, Arc<CoreRouteWatch>>>>,
    pub(crate) invocationSnapshot: crate::ToolExecutionManager::ToolInvocationSnapshot,
}

/// Keeps registration identity stable while an invocation owns or waits for its executor.
#[derive(Clone)]
enum ToolExecutorRegistration {
    Exclusive(Arc<tokio::sync::Mutex<RegisteredToolExecutor>>),
    Shared(Arc<dyn SharedToolExecutor>),
}

impl From<RegisteredToolExecutor> for ToolExecutorRegistration {
    /// Preserves explicitly shared contracts and serializes each exclusive instance.
    fn from(executor: RegisteredToolExecutor) -> Self {
        match executor {
            RegisteredToolExecutor::Shared(executor) => Self::Shared(executor),
            executor => Self::Exclusive(Arc::new(tokio::sync::Mutex::new(executor))),
        }
    }
}

/// Releases exclusive ownership automatically on completion, cancellation or unwinding.
enum ToolExecutorLease {
    Exclusive(tokio::sync::OwnedMutexGuard<RegisteredToolExecutor>),
    Shared(RegisteredToolExecutor),
}

impl std::ops::Deref for ToolExecutorLease {
    type Target = RegisteredToolExecutor;
    /// Borrows the executor owned by this invocation.
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Exclusive(executor) => executor,
            Self::Shared(executor) => executor,
        }
    }
}

impl std::ops::DerefMut for ToolExecutorLease {
    /// Mutably borrows the executor owned by this invocation.
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Exclusive(executor) => executor,
            Self::Shared(executor) => executor,
        }
    }
}

/// Ends an AI invocation lifecycle even when its future is cancelled or unwinds.
struct ToolInvocationLifecycle {
    context: ToolInvocationContext,
    tool: AITool,
}

impl Drop for ToolInvocationLifecycle {
    /// Publishes exactly one finish event after all local executor leases have been released.
    fn drop(&mut self) {
        self.context.clone().scopeSync(|| self.context.host().notifyToolExecutionFinished(&self.tool));
    }
}

pub struct AIToolHandlerState {
    toolExecutors: BTreeMap<String, ToolExecutorRegistration>,
    toolVisibility: BTreeMap<String, ToolRegistrationVisibility>,
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

    /// Creates persistent local plugin data through the filesystem Host and returns its public VFS path.
    fn createPackageDataDirectory(&self, owner_id: &str, scope: &str) -> Result<String, String> {
        let path = operit_store::ExtensionStore::ExtensionStore::dataPathForScope(owner_id, scope)?;
        let directory = RuntimeStorePaths::default().runtime_storage_path(&path);
        self.getContext()
            .fileSystemHost
            .as_ref()
            .ok_or("FileSystemHost is required for local plugin data")?
            .makeDirectory(&directory.to_string_lossy(), true)
            .map_err(|error| error.to_string())?;
        let relative = path.strip_prefix("runtime/").ok_or("Local plugin data is outside runtime storage")?;
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
                toolExecutors: BTreeMap::new(),
                toolVisibility: BTreeMap::new(),
                defaultToolsRegistered: false,
                context,
                runtimeDependencies,
                hooks: Vec::new(),
                toolPermissionSystem: ToolPermissionSystem::getInstance(),
                packageManager: None,
            })),
            authenticatedExtensionOwner: None,
            invocationSnapshot: crate::ToolExecutionManager::ToolInvocationSnapshot::default(),
            deviceTransportScope: Default::default(),
            chatObservations: Arc::new(Mutex::new(BTreeMap::new())),
            coreRoutePushes: Default::default(),
            coreRouteWatches: Arc::new(Mutex::new(BTreeMap::new())),
            storageSession: operit_store::PluginStorage::PluginStorageSession::default(),
        }
    }

    /// Removes one registered tool and its visibility metadata.
    #[allow(non_snake_case)]
    pub fn unregisterTool(&mut self, toolName: String) {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard.toolExecutors.remove(&toolName);
        guard.toolVisibility.remove(&toolName);
    }

    /// Removes all tools registered for an MCP server namespace.
    #[allow(non_snake_case)]
    pub fn unregisterMcpServerTools(&mut self, serverName: &str) -> usize {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        let toolNames = guard
            .toolExecutors
            .keys()
            .filter(|toolName| {
                matches!(toolName.split_once(':'), Some((name, _)) if name == serverName)
            })
            .cloned()
            .collect::<Vec<_>>();
        let count = toolNames.len();
        for toolName in toolNames {
            guard.toolExecutors.remove(&toolName);
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

    /// Delivers notifications outside already active lifecycle hooks.
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
        let context = ToolExecutionManager::currentToolInvocationContext()
            .expect("Lifecycle notifications require the native invocation context");
        for hook in hooks {
            if context.isHookActive(hook.id()) {
                continue;
            }
            context.withHook(hook.id()).scopeSync(|| action(hook.as_ref()));
        }
    }

    /// Notifies hooks that a tool call has been requested.
    #[allow(non_snake_case)]
    fn notifyToolCallRequested(&self, tool: &AITool) {
        self.notifyHooks(|hook| hook.onToolCallRequested(tool));
    }

    /// Returns an explicit policy decision or the original invoked failure before tool execution begins.
    #[allow(non_snake_case)]
    async fn checkToolInterception(&self, tool: &AITool) -> Result<AIToolHookDecision, String> {
        let hooks = self
            .inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .hooks
            .clone();
        for hook in hooks {
            let context = ToolExecutionManager::currentToolInvocationContext()
                .ok_or_else(|| "Tool interception requires the native invocation context".to_string())?;
            if context.isHookActive(hook.id()) { continue; }
            match context.withHook(hook.id()).scope(async { hook.onToolCallInterceptAsync(tool).await }).await? {
                AIToolHookDecision::Allow => {}
                decision @ AIToolHookDecision::Block(_) => return Ok(decision),
            }
        }
        Ok(AIToolHookDecision::Allow)
    }

    /// Builds the standard failed result emitted when a hook blocks a tool call.
    #[allow(non_snake_case)]
    fn toolInterceptionResult(tool: &AITool, decision: AIToolHookDecision) -> ToolResult {
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
    fn notifyToolPermissionChecked(&self, tool: &AITool, granted: bool, reason: Option<&str>) {
        self.notifyHooks(|hook| hook.onToolPermissionChecked(tool, granted, reason));
    }

    /// Notifies hooks that tool execution is about to start.
    #[allow(non_snake_case)]
    fn notifyToolExecutionStarted(&self, tool: &AITool) {
        self.notifyHooks(|hook| hook.onToolExecutionStarted(tool));
    }

    /// Notifies hooks that tool execution returned a result.
    #[allow(non_snake_case)]
    fn notifyToolExecutionResult(&self, tool: &AITool, result: &ToolResult) {
        self.notifyHooks(|hook| hook.onToolExecutionResult(tool, result));
    }

    /// Notifies hooks that tool execution failed before producing a normal result.
    #[allow(non_snake_case)]
    fn notifyToolExecutionError(&self, tool: &AITool, message: &str) {
        self.notifyHooks(|hook| hook.onToolExecutionError(tool, message));
    }

    /// Notifies hooks that tool execution has fully finished.
    #[allow(non_snake_case)]
    fn notifyToolExecutionFinished(&self, tool: &AITool) {
        self.notifyHooks(|hook| hook.onToolExecutionFinished(tool));
    }

    /// Returns every registered tool name regardless of visibility.
    #[allow(non_snake_case)]
    pub fn getAllToolNames(&self) -> Vec<String> {
        self.inner
            .lock()
            .expect("AIToolHandler mutex poisoned")
            .toolExecutors
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
            !guard.toolExecutors.contains_key(&toolName),
            "Built-in tool is registered more than once: {toolName}"
        );
        guard.toolExecutors.insert(toolName.clone(), ToolExecutorRegistration::from(executor.into()));
        guard.toolVisibility.insert(toolName, visibility);
    }

    /// Replaces the registration without changing executors already owned by invocations.
    fn publishToolExecutor(&mut self, name: String, executor: RegisteredToolExecutor, visibility: ToolRegistrationVisibility) {
        let mut state = self.inner.lock().expect("AIToolHandler mutex poisoned");
        state.toolExecutors.insert(name.clone(), executor.into());
        state.toolVisibility.insert(name, visibility);
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
        self.publishToolExecutor(name, RegisteredToolExecutor::Synchronous(executor), visibility);
    }

    /// Registers a public tool with an explicitly asynchronous execution contract.
    pub fn registerAsyncTool(&mut self, name: String, executor: Box<dyn AsyncToolExecutor>) {
        assert!(
            BuiltinToolName::from_name(&name).is_none(),
            "Built-in tools must use their typed registration"
        );
        self.publishToolExecutor(name, RegisteredToolExecutor::Asynchronous(executor), ToolRegistrationVisibility::PUBLIC);
    }

    /// Registers an internal tool with an explicitly asynchronous execution contract.
    pub fn registerInternalAsyncTool(&mut self, name: String, executor: Box<dyn AsyncToolExecutor>) {
        assert!(
            BuiltinToolName::from_name(&name).is_none(),
            "Built-in tools must use their typed registration"
        );
        self.publishToolExecutor(
            name,
            RegisteredToolExecutor::Asynchronous(executor),
            ToolRegistrationVisibility::INTERNAL,
        );
    }

    /// Registers an explicitly concurrent operation on the same public or internal executor registry.
    pub fn registerSharedTool(&mut self, name: String, executor: Arc<dyn SharedToolExecutor>, visibility: ToolRegistrationVisibility) {
        assert!(BuiltinToolName::from_name(&name).is_none(), "Built-in tools must use their typed registration");
        self.publishToolExecutor(name, RegisteredToolExecutor::Shared(executor), visibility);
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
        guard.defaultToolsRegistered = true;
    }

    /// Returns whether a tool executor is already registered.
    #[allow(non_snake_case)]
    pub fn hasToolExecutor(&self, toolName: &str) -> bool {
        let guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard.toolExecutors.contains_key(toolName)
    }

    /// Acquires the registered instance without holding the registry while waiting or executing.
    async fn acquireToolExecutor(&self, toolName: &str) -> Option<ToolExecutorLease> {
        let registration = self.inner.lock().expect("AIToolHandler mutex poisoned")
            .toolExecutors.get(toolName).cloned()?;
        Some(match registration {
            ToolExecutorRegistration::Exclusive(executor) => ToolExecutorLease::Exclusive(executor.lock_owned().await),
            ToolExecutorRegistration::Shared(executor) => ToolExecutorLease::Shared(RegisteredToolExecutor::Shared(executor)),
        })
    }

    /// Ensures default or package tools are registered, then reports whether a tool exists.
    #[allow(non_snake_case)]
    pub(super) async fn getToolExecutorOrActivate(&mut self, toolName: &str) -> Result<bool, String> {
        if let Some((packageName, _)) = toolName.split_once(':') {
            let manager = self.getOrCreatePackageManager();
            RuntimePackageManager::prepareMcpPackage(&manager, packageName.trim()).await?;
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
            return Ok(self.hasToolExecutor(toolName));
        }

        self.registerDefaultTools();
        if self.hasToolExecutor(toolName) {
            return Ok(true);
        }

        if let Some((packageName, _)) = toolName.split_once(':') {
            let packageName = packageName.trim();
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

        Ok(self.hasToolExecutor(toolName))
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

    /// Registers package targets once without capturing an execution caller.
    #[allow(non_snake_case)]
    pub(crate) fn registerPackageTools(
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
        if operit_tools::runtime_support::splitQualifiedVfsPath(path).is_some() {
            // Qualified `node:` paths execute on the target device under its own
            // capability mode, so the local workspace boundary does not apply.
            // Cross-node transfers are rejected by the file tools themselves.
            return Ok(());
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

    /// Executes every caller through one pipeline, with hooks and permissions selected by native AI policy.
    #[allow(non_snake_case)]
    pub async fn executeToolInvocation(
        &mut self,
        request: &ToolCall,
        policy: ToolExecutionPolicy,
    ) -> Result<Vec<ToolResult>, String> {
        let context = ToolInvocationContext::capture(self.clone());
        operit_util::AppLogger::AppLogger::batch_future_file_writes(
            context.clone().scope(self.executeInvocationInContext(request, policy, &context)),
        ).await
    }

    /// Executes under the native caller context and dispatches lifecycle hooks only for AI requests.
    async fn executeInvocationInContext(&mut self, request: &ToolCall, policy: ToolExecutionPolicy, context: &ToolInvocationContext) -> Result<Vec<ToolResult>, String> {
        let entryRequest = request;
        let resolved = request.resolveTarget(context)?;
        let request = &resolved;
        let legacyTool = std::sync::OnceLock::new();
        let tool = || legacyTool.get_or_init(|| request.toLegacyTool());
        let isAiInvocation = matches!(policy, ToolExecutionPolicy::Ai { .. });
        let logExecution = if isAiInvocation {
            ChainLogger::info
        } else {
            ChainLogger::verbose
        };
        let _lifecycle = if isAiInvocation {
            let lifecycle = ToolInvocationLifecycle {
                context: context.clone(),
                tool: tool().clone(),
            };
            self.notifyToolCallRequested(tool());
            Some(lifecycle)
        } else {
            None
        };
        let startMs = operit_host_api::TimeUtils::currentTimeMillis();
        ChainLogger::verbose_with_level(
            TOOL_CHAIN,
            "tool.execute.request",
            &[("tool", request.name.clone()), ("policy", format!("{policy:?}"))],
            operit_util::AppLogger::VERBOSE_LEVEL_3,
        );
        let execution = async {
            if let ToolExecutionPolicy::Ai { exposure } = &policy {
                if let Some(denied) = ToolExecutionManager::buildToolExposureDeniedResult(&entryRequest.toLegacyTool(), exposure.clone()) {
                    return Ok(vec![denied]);
                }
            }
            if isAiInvocation {
                let interception = self.checkToolInterception(tool()).await?;
                if let AIToolHookDecision::Block(_) = interception {
                    return Ok(vec![Self::toolInterceptionResult(tool(), interception)]);
                }
            }
            if !self.getToolExecutorOrActivate(&request.name).await? {
                return Ok(vec![Self::failedInvocation(tool(), format!("Tool not found: {}", request.name))]);
            }
            let mut executor = self.acquireToolExecutor(&request.name).await
                .ok_or_else(|| format!("Registered executor disappeared: {}", request.name))?;
            let prepared = match executor.prepare(request, context, isAiInvocation) {
                Ok(prepared) => prepared,
                Err(error) => return Ok(vec![Self::failedInvocation(tool(), format!("Invalid tool invocation: {error}"))]),
            };
            if isAiInvocation {
                if let Some(access) = prepared.access {
                    if let Err(denied) = self.executeAccessPreflightAsync(tool(), access).await {
                        return Ok(vec![denied]);
                    }
                }
                self.notifyToolExecutionStarted(tool());
            }
            let collected = prepared.invoke.await;
            if let Some(name) = BuiltinToolName::from_name(&request.name) {
                for result in &collected {
                    assert!(result.toolName == request.name,
                        "Built-in executor returned another tool's result");
                    assert!(!result.success || name.accepts_runtime_result(&result.result),
                        "Built-in executor returned an incompatible successful result: {name}");
                    assert!(result.success || result.error.is_some(), "Failed result must carry its execution error");
                }
            }
            if collected.is_empty() {
                return Err(format!("Tool executor returned no results: {}", request.name));
            }
            Ok(collected)
        }.await;
        match &execution {
            Ok(results) => {
                if isAiInvocation {
                    for result in results {
                        self.notifyToolExecutionResult(tool(), result);
                    }
                }
                logExecution(
                    TOOL_CHAIN,
                    "tool.execute.completed",
                    &[("tool", request.name.clone()), ("resultCount", results.len().to_string()),
                      ("elapsedMs", operit_host_api::TimeUtils::currentTimeMillis().saturating_sub(startMs).to_string())],
                );
            }
            Err(error) => {
                if isAiInvocation {
                    self.notifyToolExecutionError(tool(), error);
                }
                ChainLogger::error(TOOL_CHAIN, "tool.execute.error", &[("tool", request.name.clone()), ("error", error.clone())]);
            }
        }
        execution
    }

    /// Creates an explicit failed result for a rejected invocation.
    fn failedInvocation(tool: &AITool, error: String) -> ToolResult {
        ToolResult {
            toolName: tool.name.clone(),
            success: false,
            result: stringResultData(""),
            error: Some(error),
        }
    }

    /// Adapts a direct caller's terminal result from the common invocation pipeline.
    #[allow(non_snake_case)]
    pub async fn executeTool(&mut self, tool: AITool) -> ToolResult {
        match self.executeToolInvocation(&ToolCall::from(&tool), ToolExecutionPolicy::Direct).await {
            Ok(results) => results.into_iter().last().expect("The invocation pipeline rejects empty results"),
            Err(error) => Self::failedInvocation(&tool, error),
        }
    }

    /// Clears registered tools and runtime-only handler state.
    pub fn reset(&mut self) {
        let mut guard = self.inner.lock().expect("AIToolHandler mutex poisoned");
        guard.toolExecutors.clear();
        guard.toolVisibility.clear();
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
    /// Captures native turn and hook scopes before JavaScript execution crosses its Host scheduler.
    fn capture_tool_invocation_host(&self) -> Arc<dyn JsExecutionHost> {
        let mut host = self.clone();
        host.invocationSnapshot = ToolInvocationContext::capture(self.clone()).snapshot();
        Arc::new(host)
    }

    /// Authenticates the actual registered container before binding an immutable per-engine owner.
    fn for_toolpkg_execution_context(
        &self,
        context: &operit_plugin_sdk::javascript::ToolPkgExecutionContext,
    ) -> Result<Arc<dyn JsExecutionHost>, String> {
        if context.context_key.trim().is_empty() || context.container_package_name.trim().is_empty()
        {
            return Err("ToolPkg extension execution context is incomplete".to_string());
        }
        let manager = self.getOrCreatePackageManager();
        let registry = manager.lock().expect("package manager mutex poisoned");
        registry.packageRegistryReadiness().require_ready()?;
        registry
            .getToolPkgContainerRuntime(&context.container_package_name)
            .ok_or_else(|| {
                format!(
                    "ToolPkg extension owner is not registered: {}",
                    context.container_package_name
                )
            })?;
        if !registry.getEnabledToolPkgContainerRuntimes().iter()
            .any(|runtime| runtime.packageName == context.container_package_name)
        {
            return Err(format!(
                "ToolPkg extension owner is disabled: {}",
                context.container_package_name
            ));
        }
        if let Some(owner) = &self.authenticatedExtensionOwner {
            if owner != &context.container_package_name { return Err("Cannot rebind an authenticated engine to another package".to_owned()); }
            return Ok(Arc::new(self.clone()));
        }
        let mut host = self.clone();
        host.authenticatedExtensionOwner = Some(context.container_package_name.clone());
        host.storageSession = operit_store::PluginStorage::PluginStorageSession::default();
        host.deviceTransportScope = Default::default();
        host.chatObservations = Arc::new(Mutex::new(BTreeMap::new()));
        host.coreRoutePushes = Default::default();
        host.coreRouteWatches = Arc::new(Mutex::new(BTreeMap::new()));
        Ok(Arc::new(host))
    }

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

    /// Executes a direct SDK request without AI interception, permissions or lifecycle hooks.
    fn execute_tool_call(
        &self,
        mut request: JsToolCallRequest,
    ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<JsToolCallResult> {
        if request.device.as_ref().is_some_and(|device| device.action == operit_plugin_sdk::javascript::JsDeviceToolAction::Transport) {
            self.deviceTransportScope.bind(&self.runtimeSupport());
            request.parameters.insert("__transport_owner".into(), serde_json::Value::String(self.deviceTransportScope.id.clone()));
        }
        // Device selection changes transport only, never the direct SDK admission policy.
        if request.device.is_some() {
            return self.runtimeSupport().executeDeviceTool(request);
        }
        let host = self.clone();
        Box::pin(async move {
            let call = ToolCall {
                name: request.qualified_tool_name(),
                parameters: request.parameters,
            };
            let mut handler = host;
            let result = match handler.executeToolInvocation(&call, ToolExecutionPolicy::Direct).await {
                Ok(results) => results.into_iter().last().expect("The invocation pipeline rejects empty results"),
                Err(error) => Self::failedInvocation(&call.toLegacyTool(), error),
            };
            let data = match result.result {
                ToolResultData::JsonResultData(data) => JsToolCallResultData::Value(data.value),
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
                    serde_json::to_value(data)
                        .expect("ToolResultData structured conversion must succeed"),
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

    /// Registers media for the synchronous v1 image globals and returns the link JSON.
    fn register_media(&self, request_json: &str) -> Result<String, String> {
        register_native_media(&self.getContext(), request_json)
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

    /// Resolves the current package container for its persistent device-local data directory.
    fn plugin_local_data_dir(&self, owner_id: &str) -> Result<String, String> {
        let owner = operit_store::ExtensionStore::ExtensionStore::default().packageOwner(owner_id)?;
        self.createPackageDataDirectory(&owner.id, "device")
    }

    /// Creates registration-local data using the owner validated by the engine without package manager locks.
    fn registration_plugin_local_data_dir(&self, owner_id: &str) -> Result<String, String> {
        self.createPackageDataDirectory(owner_id, "device")
    }

    /// Resolves the current package's shared data without changing its installation scope.
    fn plugin_space_data_dir(&self, owner_id: &str) -> Result<String, String> {
        let owner = operit_store::ExtensionStore::ExtensionStore::default().packageOwner(owner_id)?;
        self.createPackageDataDirectory(&owner.id, "space")
    }

    /// Creates shared data for registration without accessing installed package state.
    fn registration_plugin_space_data_dir(&self, owner_id: &str) -> Result<String, String> {
        self.createPackageDataDirectory(owner_id, "space")
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

    /// Validates and forwards an export to the registered host file-save owner.
    fn save_compose_file(&self, payload: &serde_json::Value) -> Result<serde_json::Value, String> {
        let request = operit_host_api::ComposeDslFileSaveRequest::fromValue(payload.clone())
            .map_err(|error| error.to_string())?;
        self.getContext()
            .composeDslWebViewHost
            .as_ref()
            .ok_or_else(|| "ComposeDslWebViewHost is not registered".to_string())?
            .saveFile(request)
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
        scheduler
            .scheduleHostRuntimeAsyncTask(
                "operit-toolpkg-ipc",
                Box::new(move || {
                    Box::pin(async move {
                        let result =
                            crate::tools::ToolJsRuntime::invokeToolPkgIpc(&toolHandler, request)
                                .await;
                        if let Err(error) = &result {
                            ChainLogger::error(
                                TOOL_CHAIN,
                                "toolpkg.ipc.target_finish",
                                &[
                                    ("package", packageTarget),
                                    ("channel", channel),
                                    ("targetContext", targetContextKey),
                                    ("targetRuntime", targetRuntime),
                                    ("outcome", "error".to_string()),
                                    ("error", error.clone()),
                                ],
                            );
                        }
                        completion(result);
                    })
                }),
            )
            .map_err(|error| error.to_string())
    }

    /// Resolves the package installation and delegates the Binding decision to the Rust route runtime.
    fn should_route_toolpkg(&self, binding: String, streaming: bool) -> Result<bool, String> {
        let owner = self.authenticatedExtensionOwner.as_ref().ok_or("Plugin routes require an authenticated engine")?;
        let record = operit_store::ExtensionStore::ExtensionStore::default().record("package", owner)?;
        if record.scope == "device" { return Ok(false); }
        let args = operit_link::toCoreValue(serde_json::json!({ "bindingKey": binding })).map_err(|error| error.to_string())?;
        let runtime = operit_link::coreRouteRuntime().ok_or("Core route runtime is not installed")?;
        if streaming {
            let remote = runtime.shouldRouteWatch("watchToolPkgRoute", &args).map_err(|error| error.message)?;
            let local = runtime.shouldUseLocalWatchSource("watchToolPkgRoute", &args).map_err(|error| error.message)?;
            Ok(remote || local)
        } else { runtime.shouldRoute("callToolPkgRoute", &args).map_err(|error| error.message) }
    }

    /// Adds the authenticated package identity to the private route argument envelope.
    fn call_core_route(&self, target: String, method: String, args: Value)
        -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        let owner = self.authenticatedExtensionOwner.clone();
        Box::pin(async move {
            if target != "callToolPkgRoute" { return Err("Invalid plugin Call entry point".into()); }
            let args = toolPkgRouteArguments(owner, method, args)?;
            let runtime = operit_link::coreRouteRuntime().ok_or("Core route runtime is not installed")?;
            let request = CoreCallRequest::new(operit_link::nextCoreRouteRequestId(&target), operit_link::CORE_INTERNAL_TARGET, target, args);
            let value = runtime.call(request).await.result.map_err(|error| error.message)?;
            operit_link::fromCoreValue(value).map_err(|error| error.to_string())
        })
    }

    /// Obtains a normal Core embedded stream and opens its existing Watch descriptor.
    fn open_core_route_watch(&self, target: String, method: String, args: Value)
        -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        let owner = self.authenticatedExtensionOwner.clone();
        let watches = self.coreRouteWatches.clone();
        Box::pin(async move {
            if target != "watchToolPkgRoute" { return Err("Invalid plugin Watch entry point".into()); }
            let args = toolPkgRouteArguments(owner, method, args)?;
            let runtime = operit_link::coreRouteRuntime().ok_or("Core route runtime is not installed")?;
            let value = runtime.call(CoreCallRequest::new(operit_link::nextCoreRouteRequestId(&target), operit_link::CORE_INTERNAL_TARGET, target.clone(), args.clone())).await.result.map_err(|error| error.message)?;
            let descriptor = operit_link::fromCoreValue::<operit_link::CoreStream<Value>>(value).map_err(|error| error.to_string())?.descriptor;
            let openArgs = operit_rslink_runtime::core_route_embedded_stream_open_args(
                descriptor.args, "call", target, args,
            ).map_err(|error| error.message)?;
            let request = CoreWatchRequest::new(operit_link::nextCoreRouteRequestId("plugin-watch"), descriptor.target, descriptor.propertyName, openArgs);
            let stream = runtime.watch(request).await.map_err(|error| error.message)?;
            let id = Uuid::new_v4().to_string();
            watches.lock().map_err(|_| "Core route watch registry is poisoned")?.insert(id.clone(), Arc::new(CoreRouteWatch {
                stream: tokio::sync::Mutex::new(stream), closed: tokio::sync::Notify::new(), value: Mutex::new(None),
            }));
            Ok(serde_json::json!({ "subscriptionId": id }))
        })
    }

    /// Opens one ordered input stream through the existing Core Link Push abstraction.
    fn open_toolpkg_route_push(&self, method: String, args: Value)
        -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        let owner = self.authenticatedExtensionOwner.clone();
        let pushes = self.coreRoutePushes.clone();
        Box::pin(async move {
            let args = toolPkgRouteArguments(owner, method, args)?;
            let runtime = operit_link::coreRouteRuntime().ok_or("Core route runtime is not installed")?;
            let request = operit_link::CorePushRequest::new(operit_link::nextCoreRouteRequestId("plugin-push"), operit_link::CORE_INTERNAL_TARGET, "pushToolPkgRoute").withArgs(args);
            let session = runtime.openPush(request).await.map_err(|error| error.message)?;
            let id = Uuid::new_v4().to_string();
            pushes.lock().map_err(|_| "Core Push registry is poisoned")?.insert(id.clone(), Arc::new(CoreRoutePush {
                session: tokio::sync::Mutex::new(Some(session)), closed: tokio::sync::Notify::new(),
            }));
            Ok(serde_json::json!({ "subscriptionId": id }))
        })
    }

    /// Sends exactly one item and preserves Link backpressure for the producer.
    fn send_toolpkg_route_push(&self, id: String, value: Value)
        -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        let pushes = self.coreRoutePushes.clone();
        Box::pin(async move {
            let push = pushes.lock().map_err(|_| "Core Push registry is poisoned")?.get(&id).cloned().ok_or("Plugin Push session is not active")?;
            let mut guard = push.session.try_lock().map_err(|_| "Plugin Push already has a pending send")?;
            let value = operit_link::toCoreValue(value).map_err(|error| error.to_string())?;
            let session = guard.as_mut().ok_or("Plugin Push session is closed")?;
            tokio::select! {
                biased;
                _ = push.closed.notified() => { guard.take(); Err("Plugin Push was aborted".into()) },
                sent = session.send(value) => { sent.map_err(|error| error.message)?; Ok(Value::Null) },
            }
        })
    }

    /// Completes or aborts one authenticated producer and removes its owned session.
    fn close_toolpkg_route_push(&self, id: String, abort: bool)
        -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        let pushes = self.coreRoutePushes.clone();
        Box::pin(async move {
            let push = pushes.lock().map_err(|_| "Core Push registry is poisoned")?.remove(&id).ok_or("Plugin Push session is not active")?;
            if abort { push.closed.notify_one(); }
            let session = push.session.lock().await.take();
            if !abort { session.ok_or("Plugin Push session is closed")?.close().await.map_err(|error| error.message)?; }
            Ok(Value::Null)
        })
    }

    /// Waits for a route event or explicit cancellation of the authenticated subscription.
    fn next_core_route_watch(&self, subscription_id: String)
        -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        let watches = self.coreRouteWatches.clone();
        Box::pin(async move {
            let watch = watches.lock()
                .map_err(|_| "Core route watch registry is poisoned".to_string())?
                .get(&subscription_id).cloned()
                .ok_or_else(|| "Core route watch subscription is not active".to_string())?;
            let mut stream = watch.stream.try_lock()
                .map_err(|_| "Core route watch already has a pending read".to_string())?;
            let event = tokio::select! {
                biased;
                _ = watch.closed.notified() => None,
                event = stream.recv() => event,
            };
            match event {
                Some(event) => {
                    let mut previous = watch.value.lock().map_err(|_| "Core route watch value is poisoned")?;
                    let value = if event.kind == operit_link::CoreEventKind::Delta {
                        previous.take().ok_or("Core stream received a delta before its initial value")?.intoIncrementalDelta(&event.value)?
                    } else { event.value };
                    *previous = Some(value.clone());
                    let value: Value = operit_link::fromCoreValue(value).map_err(|error| error.to_string())?;
                    if let Some(error) = value.get("error").and_then(Value::as_str) { return Err(error.into()); }
                    Ok(value)
                },
                None => {
                    watches.lock().map_err(|_| "Core route watch registry is poisoned".to_string())?
                        .remove(&subscription_id);
                    Ok(Value::Null)
                }
            }
        })
    }

    /// Cancels a pending watch read and releases its route stream.
    fn close_core_route_watch(&self, subscription_id: String) -> Result<(), String> {
        let watch = self.coreRouteWatches.lock()
            .map_err(|_| "Core route watch registry is poisoned".to_string())?
            .remove(&subscription_id);
        if let Some(watch) = watch { watch.closed.notify_one(); }
        Ok(())
    }

    /// Observes an exact key using the current engine's authenticated storage handles.
    fn wait_plugin_storage_record(
        &self,
        handle: String,
        collection: String,
        key: String,
        after: Option<operit_host_api::PluginStorage::StorageRecordWatchState>,
    ) -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        let session = self.storageSession.clone();
        Box::pin(async move {
            let entry = session.waitForRecord(handle, collection, key, after).await
                .map_err(|error| error.to_string())?;
            serde_json::to_value(entry).map_err(|error| error.to_string())
        })
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
        dyn Fn(AITool, ToolInvocationContext) -> operit_plugin_sdk::javascript::JsExecutionCompletion<ToolResult>
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
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool, context: &'a crate::ToolExecutionManager::ToolInvocationContext) -> ToolInvocationFuture<'a> {
        let invocation = (self.invoke)(tool.clone(), context.clone());
        Box::pin(async move { vec![invocation.await] })
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
    use crate::runtime_support::*;
    use crate::ToolExecutionManager::{PreparedToolInvocation, ToolExposureMode, ToolRuntimeContext};
    use operit_model::ToolPrompt::SystemToolPromptCategory;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};


    /// Verifies accepted storage paths resolve to one canonical identity and reject escapes.
    #[test]
    fn directory_paths_resolve_to_canonical_owned_storage() {
        let roots = RuntimeStorePaths::new(std::path::PathBuf::from("storage-root/runtime"), std::path::PathBuf::from("storage-root/workspaces"));
        for scope_path in ["device/example/cache.sqlite", "space/example/memory.sqlite"] {
            let relative = format!("plugin_data/{scope_path}");
            let expected = format!("runtime/{relative}");
            for path in [format!("/app/data/{relative}"), expected.clone(), format!("storage-root/runtime/{relative}")] {
                assert_eq!(crate::tools::defaultTool::standard::StandardStorageTools::resolve_plugin_storage_path(&path, &roots).unwrap(), expected);
            }
        }
        for path in ["/app/workspaces/example/cache.sqlite", "/app/data/../../escape.sqlite", "storage-root/other/cache.sqlite", "/app/database/cache.sqlite"] {
            assert!(crate::tools::defaultTool::standard::StandardStorageTools::resolve_plugin_storage_path(path, &roots).is_err(), "{path}");
        }
    }


    /// Rejects business dependencies so lifecycle tests cannot accidentally execute application services.
    struct UnusedRuntimeSupport {
        package_assets: Option<&'static [RuntimePluginAsset]>,
    }
    impl ToolRuntimeSupport for UnusedRuntimeSupport {
        /// Rejects unexpected application service access in execution pipeline tests.
    fn bindCoreNodeToolRuntime(&self, runtime: Arc<dyn CoreNodeToolRuntime>) -> Result<(), String> { panic!("Unexpected runtime service: bindCoreNodeToolRuntime") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn coreNodeRouteState(&self) -> Result<RuntimeCoreNodeRouteState, String> { panic!("Unexpected runtime service: coreNodeRouteState") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn bindCoreRouteChangeHandler(&self, handler: CoreRouteChangeHandler) -> Result<(), String> { panic!("Unexpected runtime service: bindCoreRouteChangeHandler") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn requestCoreRouteChange<'a>(
        &'a self,
        chatId: String,
        targetNodeId: String,
        resumeContext: CoreRouteResumeContext,
    ) -> ToolRuntimeSupportFuture<'a, Result<(), String>> { panic!("Unexpected runtime service: requestCoreRouteChange") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn listModelSummaries(
        &self,
    ) -> Result<Vec<operit_plugin_sdk::js_sdk::models::ModelsSummary>, String> { panic!("Unexpected runtime service: listModelSummaries") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn listTtsConfigs(
        &self,
    ) -> Result<Vec<operit_plugin_sdk::js_sdk::tts::TtsConfig>, String> { panic!("Unexpected runtime service: listTtsConfigs") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn listThemeConfigs(&self) -> Result<Vec<operit_plugin_sdk::js_sdk::theme::ThemeConfig>, String> { panic!("Unexpected runtime service: listThemeConfigs") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn applyThemeConfig(&self, id: String) -> Result<operit_plugin_sdk::js_sdk::theme::ThemeConfig, String> { panic!("Unexpected runtime service: applyThemeConfig") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn getCurrentTtsConfigId(&self) -> Result<String, String> { panic!("Unexpected runtime service: getCurrentTtsConfigId") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn setCurrentTtsConfigId(&self, id: String) -> Result<String, String> { panic!("Unexpected runtime service: setCurrentTtsConfigId") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn filterToolCatalog<'a>(
        &'a self,
        entries: Vec<crate::tools::climode::CliToolModeSupport::HiddenToolCatalogEntry>,
        context: crate::ToolExecutionManager::ToolRuntimeContext,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Vec<crate::tools::climode::CliToolModeSupport::HiddenToolCatalogEntry>,
                        String,
                    >,
                > + Send
                + 'a,
        >,
    > { panic!("Unexpected runtime service: filterToolCatalog") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn readToolSourceCatalog<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        operit_plugin_sdk::js_sdk::catalog::CatalogToolSources,
                        String,
                    >,
                > + Send
                + 'a,
        >,
    > { panic!("Unexpected runtime service: readToolSourceCatalog") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn readEnvironmentVariable(&self, key: &str) -> Result<Option<String>, String> { panic!("Unexpected runtime service: readEnvironmentVariable") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn writeEnvironmentVariable(&self, key: &str, value: &str) -> Result<(), String> { panic!("Unexpected runtime service: writeEnvironmentVariable") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn removeEnvironmentVariable(&self, key: &str) -> Result<(), String> { panic!("Unexpected runtime service: removeEnvironmentVariable") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn buildBuiltinAndInternalCategories(
        &self,
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> Vec<SystemToolPromptCategory> { panic!("Unexpected runtime service: buildBuiltinAndInternalCategories") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn buildBuiltinToolNameSet(
        &self,
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> BTreeSet<String> { panic!("Unexpected runtime service: buildBuiltinToolNameSet") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn aiVisibleSkillPackages(&self) -> Vec<RuntimeSkillCatalogEntry> { panic!("Unexpected runtime service: aiVisibleSkillPackages") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn cachedMcpTools(&self, serverName: &str) -> Vec<CachedMcpToolInfo> { panic!("Unexpected runtime service: cachedMcpTools") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn builtinPluginAssets(&self) -> &'static [RuntimePluginAsset] {
        self.package_assets.expect("Unexpected runtime service: builtinPluginAssets")
    }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn bundledExternalPluginAssets(&self) -> &'static [RuntimePluginAsset] {
        self.package_assets.expect("Unexpected runtime service: bundledExternalPluginAssets");
        &[]
    }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn bundledExternalSkillAssets(&self) -> &'static [RuntimeBundledExternalSkillAsset] { panic!("Unexpected runtime service: bundledExternalSkillAssets") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn isSkillVisibleToAi(&self, skillName: &str) -> bool { panic!("Unexpected runtime service: isSkillVisibleToAi") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn setSkillVisibleToAi(&self, skillName: &str, visible: bool) -> Result<(), String> { panic!("Unexpected runtime service: setSkillVisibleToAi") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn generateMcpPluginDescription<'a>(
        &'a self,
        pluginName: &'a str,
        toolDescriptions: &'a [String],
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>> { panic!("Unexpected runtime service: generateMcpPluginDescription") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn startChatServices(&self) -> Result<(), String> { panic!("Unexpected runtime service: startChatServices") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn stopChatServices(&self) -> Result<(), String> { panic!("Unexpected runtime service: stopChatServices") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn isChatProcessing(&self, chatId: &str) -> Result<bool, String> { panic!("Unexpected runtime service: isChatProcessing") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn switchMainChat<'a>(
        &'a self,
        chatId: &'a str,
    ) -> ToolRuntimeSupportFuture<'a, Result<(), String>> { panic!("Unexpected runtime service: switchMainChat") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn createChatRuntime<'a>(
        &'a self,
        setAsCurrentChat: bool,
        sourceChatId: Option<String>,
        parentChatId: Option<String>,
        input: Option<serde_json::Value>,
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>> { panic!("Unexpected runtime service: createChatRuntime") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn readChatExtension(
        &self,
        owner: &str,
        target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
    ) -> Result<Option<operit_plugin_sdk::js_sdk::core::JsonObject>, String> { panic!("Unexpected runtime service: readChatExtension") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn writeChatExtension(
        &self,
        owner: &str,
        target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
        value: operit_plugin_sdk::js_sdk::core::JsonObject,
    ) -> Result<operit_plugin_sdk::js_sdk::core::JsonObject, String> { panic!("Unexpected runtime service: writeChatExtension") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn deleteChatExtension(
        &self,
        owner: &str,
        target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
    ) -> Result<bool, String> { panic!("Unexpected runtime service: deleteChatExtension") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn openPluginChatMessage(
        &self, owner: &str, request: operit_plugin_sdk::js_sdk::chat::ChatSendRequest, observeParts: bool,
    ) -> Result<operit_plugin_sdk::js_sdk::JsAsyncIterable<operit_plugin_sdk::js_sdk::chat::ChatSendEvent>, String> { panic!("Unexpected runtime service: openPluginChatMessage") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn requestPluginChatCancellation(
        &self, owner: &str, chatId: &str,
    ) -> Result<operit_plugin_sdk::js_sdk::chat::ChatCancelResult, String> { panic!("Unexpected runtime service: requestPluginChatCancellation") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn sendChatMessage<'a>(
        &'a self,
        request: RuntimeChatSendRequest,
    ) -> ToolRuntimeSupportFuture<'a, Result<operit_plugin_sdk::js_sdk::results::MessageSendResultData, String>> { panic!("Unexpected runtime service: sendChatMessage") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn callChatModel<'a>(
        &'a self,
        request: RuntimeChatCallRequest,
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>> { panic!("Unexpected runtime service: callChatModel") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn processFileBindingOperations(
        &self,
        originalContent: &str,
        operations: &[RuntimeStructuredEditOperation],
    ) -> (String, String) { panic!("Unexpected runtime service: processFileBindingOperations") }
        /// Rejects unexpected application service access in execution pipeline tests.
    fn generateUnifiedDiff(&self, oldContent: &str, newContent: &str) -> String { panic!("Unexpected runtime service: generateUnifiedDiff") }
    }

    struct PipelineFixture {
        handler: AIToolHandler,
        root: std::path::PathBuf,
    }
    impl PipelineFixture {
        /// Builds only real registry, permission and storage components with rejecting business services.
        fn new() -> Self {
            Self::with_package_assets(None)
        }

        /// Supplies optional real package sources for registry isolation tests.
        fn with_package_assets(package_assets: Option<&'static [RuntimePluginAsset]>) -> Self {
            static SCHEDULER: std::sync::Once = std::sync::Once::new();
            SCHEDULER.call_once(|| operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
                operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
            )));
            let root = std::env::temp_dir().join(format!("operit-tool-pipeline-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            let storage = Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
                root.join("runtime"), root.join("workspaces"),
            ));
            let permissions = ToolPermissionSystem::newWithStorage(storage.clone());
            permissions.saveAiPermissionMode(AiPermissionMode::ReadOnly).unwrap();
            let handler = AIToolHandler {
                inner: Arc::new(Mutex::new(AIToolHandlerState {
                    toolExecutors: BTreeMap::new(),
                    toolVisibility: BTreeMap::new(), defaultToolsRegistered: true,
                    context: HostManager { runtimeStorageHost: Some(storage.clone()), runtimeSqliteHost: Some(storage),
                        fileSystemHost: Some(Arc::new(operit_host_native_filesystem::PosixFileSystemHost::new())),
                        hostRuntimeTaskSchedulerHost: Some(operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()),
                        ..Default::default() },
                    runtimeDependencies: ToolRuntimeDependencies::new(
                        Arc::new(UnusedRuntimeSupport { package_assets }),
                        Arc::new(operit_js_bridge::javascript::JsExecutionProvider::HostJsExecutionProvider::new()),
                    ),
                    hooks: Vec::new(), toolPermissionSystem: permissions, packageManager: None,
                })),
                    authenticatedExtensionOwner: Some("engine-owner".into()),
                invocationSnapshot: Default::default(),
                storageSession: Default::default(), chatObservations: Default::default(), coreRouteWatches: Default::default(), coreRoutePushes: Default::default(), deviceTransportScope: Default::default(),
            };
            Self { handler, root }
        }
    }
    impl Drop for PipelineFixture {
        /// Removes only this test's checked unique temporary storage root.
        fn drop(&mut self) {
            assert_eq!(self.root.parent(), Some(std::env::temp_dir().as_path()));
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }

    /// A broken theme archive stays visible as an issue while unrelated packages and command snapshots work.
    #[tokio::test(flavor = "current_thread")]
    async fn broken_theme_does_not_block_healthy_packages_or_registry_commands() {
        use std::io::{Cursor, Write};
        use crate::tools::packTool::RuntimePackageManager::RuntimePackageManager;
        use operit_store::RuntimeStorageHost::{setDefaultRuntimeStorageHost, setDefaultRuntimeSqliteHost};

        fn archive(manifest: serde_json::Value) -> &'static [u8] {
            let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
            zip.start_file("manifest.json", zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(&serde_json::to_vec(&manifest).unwrap()).unwrap();
            zip.start_file("data.txt", zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(b"healthy resource").unwrap();
            Box::leak(zip.finish().unwrap().into_inner().into_boxed_slice())
        }
        let assets = Box::leak(vec![
            RuntimePluginAsset { name: "healthy.toolpkg", sha256: "", bytes: archive(serde_json::json!({
                "toolpkg_id":"healthy_plugin", "resources":[{"key":"text","path":"data.txt","mime":"text/plain"}]
            })) },
            RuntimePluginAsset { name: "broken_theme.toolpkg", sha256: "", bytes: archive(serde_json::json!({
                "toolpkg_id":"broken_theme", "themes":[{"id":"default","path":"theme.json","preview":"preview.png","files":[
                    {"path":"theme.json","mediaType":"application/json"}, {"path":"preview.png","mediaType":"image/png"}
                ]}]
            })) },
        ].into_boxed_slice());
        operit_host_api::HostManager::setDefaultHostJavaScriptRuntimeHost(Arc::new(
            operit_host_native_scheduler::NativeHostJavaScriptRuntimeHost::new(),
        ));
        let fixture = PipelineFixture::with_package_assets(Some(assets));
        let context = fixture.handler.getContext();
        setDefaultRuntimeStorageHost(context.runtimeStorageHost.clone().unwrap());
        setDefaultRuntimeSqliteHost(context.runtimeSqliteHost.clone().unwrap());
        let paths = RuntimeStorePaths::new(fixture.root.join("runtime"), fixture.root.join("workspaces"));
        let mut manager = RuntimePackageManager::new(paths, fixture.handler.clone());
        assert!(manager.claimInitialPackageRegistryLoad().unwrap());
        let readiness = manager.packageRegistryReadiness();
        let waiter = tokio::spawn(async move { readiness.wait_until_ready().await });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        manager.loadAvailablePackages();
        assert_eq!(waiter.await.unwrap(), Ok(()));
        assert!(manager.getAvailablePackages().contains_key("healthy_plugin"));
        assert!(!manager.getAvailablePackages().contains_key("broken_theme"));
        let issues = manager.getToolPkgLoadIssues();
        assert_eq!(issues.len(), 1, "unexpected package failures: {issues:?}");
        assert!(issues[0].sourcePath.contains("broken_theme.toolpkg"));
        let manager = Arc::new(Mutex::new(manager));
        let snapshot = RuntimePackageManager::readySnapshot(manager.clone()).await.unwrap();
        assert!(snapshot.getAvailablePackages().contains_key("healthy_plugin"));
        assert_eq!(snapshot.getToolPkgLoadIssues().len(), 1);
        // Cached failures and subsequent rescans must preserve the same isolation.
        manager.lock().unwrap().loadAvailablePackages();
        let snapshot = RuntimePackageManager::readySnapshot(manager).await.unwrap();
        assert!(snapshot.getAvailablePackages().contains_key("healthy_plugin"));
        assert_eq!(snapshot.getToolPkgLoadIssues().len(), 1);
    }

    #[derive(Default)]
    struct LifecycleProbe { events: Mutex<Vec<&'static str>> }
    impl LifecycleProbe {
        /// Records an event while checking that dispatch enters the actual hook scope.
        fn record(&self, event: &'static str) {
            let context = ToolExecutionManager::currentToolInvocationContext().unwrap();
            assert!(context.isHookActive(self.id()));
            self.events.lock().unwrap().push(event);
        }
        /// Takes the complete event sequence for one invocation.
        fn take(&self) -> Vec<&'static str> { std::mem::take(&mut *self.events.lock().unwrap()) }
    }
    impl AIToolHook for LifecycleProbe {
        /// Identifies the probe so recursive dispatch can be detected.
        fn id(&self) -> &str { "test.lifecycle" }
        /// Records request notification in its native scope.
        fn onToolCallRequested(&self, _: &AITool) { self.record("requested"); }
        /// Records admission interception in its native scope.
        fn onToolCallIntercept(&self, _: &AITool) -> AIToolHookDecision { self.record("intercept"); AIToolHookDecision::Allow }
        /// Records only real AI permission decisions.
        fn onToolPermissionChecked(&self, _: &AITool, _: bool, _: Option<&str>) { self.record("permission"); }
        /// Records actual executor start.
        fn onToolExecutionStarted(&self, _: &AITool) { self.record("started"); }
        /// Records one returned result.
        fn onToolExecutionResult(&self, _: &AITool, _: &ToolResult) { self.record("result"); }
        /// Records pipeline failure.
        fn onToolExecutionError(&self, _: &AITool, _: &str) { self.record("error"); }
        /// Records terminal cleanup, including cancellation outside the future's poll scope.
        fn onToolExecutionFinished(&self, _: &AITool) { self.record("finished"); }
    }

    #[derive(Default)]
    struct SharedProbe {
        accessCalls: AtomicUsize,
        calls: AtomicUsize,
        closed: tokio::sync::Notify,
    }
    impl SharedToolExecutor for SharedProbe {
        /// Prepares a probe once and counts access reads only under the native AI policy.
        fn prepare<'a>(&'a self, request: &'a ToolCall, context: &'a ToolInvocationContext, checkAccess: bool)
            -> Result<crate::ToolExecutionManager::PreparedToolInvocation<'a>, String> {
            let access = checkAccess.then(|| {
                self.accessCalls.fetch_add(1, Ordering::SeqCst);
                ToolAccessSpec { effect: ToolEffect::WRITE, boundary: ToolBoundary::None }
            });
            let invoke = Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                match request.parameters.get("op").and_then(Value::as_str) {
                    Some("next") => self.closed.notified().await,
                    Some("close") => self.closed.notify_one(),
                    _ => {}
                }
                let value = json!({
                    "parameters": request.parameters,
                    "owner": context.host().authenticatedExtensionOwner,
                    "turn": context.turnSnapshot(),
                    "hookActive": context.isHookActive("outer-hook"),
                });
                vec![ToolResult { toolName: request.name.clone(), success: true,
                    result: ToolResultData::JsonResultData(operit_plugin_sdk::js_sdk::results::JsonResultData { value }), error: None }]
            });
            Ok(crate::ToolExecutionManager::PreparedToolInvocation { access, invoke })
        }
    }

    /// Creates the exact structured request consumed by the production JS execution adapter.
    fn probeRequest(parameters: Value) -> JsToolCallRequest {
        JsToolCallRequest { tool_type: "default".into(), tool_name: "pipeline_probe".into(),
            parameters: serde_json::from_value(parameters).unwrap(), device: None }
    }

    /// Proves AI admission remains active while the same executor is direct and hook-free for JS.
    #[tokio::test]
    async fn ai_and_js_share_executors_but_not_lifecycle_or_permissions() {
        let mut fixture = PipelineFixture::new();
        let probe = Arc::new(SharedProbe::default());
        let hooks = Arc::new(LifecycleProbe::default());
        fixture.handler.registerSharedTool("pipeline_probe".into(), probe.clone(), ToolRegistrationVisibility::PUBLIC);
        fixture.handler.addToolHook(hooks.clone());
        let approvals = Arc::new(AtomicUsize::new(0));
        let approvalsInCallback = approvals.clone();
        fixture.handler.getToolPermissionSystem().setAsyncPermissionRequester(move |_, _, _| {
            approvalsInCallback.fetch_add(1, Ordering::SeqCst);
            async { crate::tools::ToolPermissionSystem::PermissionRequestResult::DENY }
        });
        let payload = json!({"null":null,"array":[1,false,{"value":null}],"text":"{\"key\":null}",
            "integer":{"kind":"integer","value":"9223372036854775807"},"blob":{"kind":"blob","value":[0,255]}});
        let request = probeRequest(payload.clone());
        let denied = fixture.handler.executeToolInvocation(&ToolCall { name: request.qualified_tool_name(), parameters: request.parameters.clone() },
            ToolExecutionPolicy::Ai { exposure: ToolExposureMode::FULL }).await.unwrap();
        assert!(!denied[0].success);
        assert_eq!(probe.calls.load(Ordering::SeqCst), 0);
        assert_eq!(approvals.load(Ordering::SeqCst), 1);
        assert_eq!(hooks.take(), ["requested","intercept","permission","result","finished"]);
        let result = fixture.handler.execute_tool_call(request).await;
        assert!(result.success, "{:?}", result.error);
        let JsToolCallResultData::Value(value) = result.data else { panic!("Expected structured JSON") };
        assert_eq!(value["parameters"], payload);
        assert_eq!(probe.accessCalls.load(Ordering::SeqCst), 1);
        assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
        assert_eq!(approvals.load(Ordering::SeqCst), 1);
        assert!(hooks.take().is_empty());
        fixture.handler.getToolPermissionSystem().saveAiPermissionMode(AiPermissionMode::Full).unwrap();
        let request = probeRequest(payload);
        let allowed = fixture.handler.executeToolInvocation(
            &ToolCall { name: request.qualified_tool_name(), parameters: request.parameters },
            ToolExecutionPolicy::Ai { exposure: ToolExposureMode::FULL },
        ).await.unwrap();
        assert!(allowed[0].success);
        assert_eq!(hooks.take(), ["requested", "intercept", "started", "result", "finished"]);
        assert_eq!(probe.calls.load(Ordering::SeqCst), 2);
        assert_eq!(probe.accessCalls.load(Ordering::SeqCst), 2);
        assert_eq!(approvals.load(Ordering::SeqCst), 1);
    }

    /// A captured AI turn must not promote direct JS requests into AI admission.
    #[tokio::test]
    async fn js_tools_in_ai_turn_skip_all_lifecycle_and_permissions() {
        let mut fixture = PipelineFixture::new();
        let hooks = Arc::new(LifecycleProbe::default());
        let probe = Arc::new(SharedProbe::default());
        fixture.handler.addToolHook(hooks.clone());
        fixture.handler.registerSharedTool("pipeline_probe".into(), probe.clone(), ToolRegistrationVisibility::INTERNAL);
        fixture.handler.getToolPermissionSystem().setAsyncPermissionRequester(|_, _, _| async {
            panic!("Direct JS must never request AI approval");
        });
        let turn = ToolRuntimeContext {
            callerChatId: Some("direct-in-ai-turn".into()), callerParticipantId: None,
            workspacePath: None, workspaceFolders: Vec::new(), toolExposureMode: ToolExposureMode::CLI,
            extensionOwner: "configuration-owner".into(), executionData: Default::default(),
        };
        let host = ToolExecutionManager::scopeToolRuntimeContext(turn.clone(), async {
            fixture.handler.capture_tool_invocation_host()
        }).await;
        for payload in [json!({"value":null}), json!({"value":42})] {
            let result = host.execute_tool_call(probeRequest(payload.clone())).await;
            assert!(result.success, "{:?}", result.error);
            let JsToolCallResultData::Value(value) = result.data else { panic!("Expected structured JSON") };
            assert_eq!(value["turn"], serde_json::to_value(&turn).unwrap());
            assert_eq!(value["parameters"], payload);
        }
        assert_eq!(probe.calls.load(Ordering::SeqCst), 2);
        assert_eq!(probe.accessCalls.load(Ordering::SeqCst), 0);
        assert!(hooks.take().is_empty());
    }

    /// Proves a pending shared pull cannot occupy the registry and prevent the same executor's close call.
    #[tokio::test]
    async fn shared_pull_does_not_block_close_through_the_same_registry() {
        let mut fixture = PipelineFixture::new();
        fixture.handler.registerSharedTool("pipeline_probe".into(), Arc::new(SharedProbe::default()), ToolRegistrationVisibility::INTERNAL);
        let mut next = fixture.handler.execute_tool_call(probeRequest(json!({"op":"next"})));
        assert!(futures_util::poll!(&mut next).is_pending());
        let mut close = fixture.handler.execute_tool_call(probeRequest(json!({"op":"close"})));
        assert!(futures_util::poll!(&mut close).is_ready());
        assert!(futures_util::poll!(&mut next).is_ready());
    }

    /// Proves Direct cancellation releases an exclusive executor without dispatching lifecycle hooks.
    #[tokio::test]
    async fn direct_cancellation_releases_exclusive_executor_without_hooks() {
        let mut fixture = PipelineFixture::new();
        let hooks = Arc::new(LifecycleProbe::default());
        fixture.handler.addToolHook(hooks.clone());
        fixture.handler.registerAsyncTool("pipeline_probe".into(), Box::new(AsyncFnToolExecutor {
            validate: Arc::new(|_| ToolValidationResult { valid:true,errorMessage:String::new() }),
            invoke: Arc::new(|_, _| Box::pin(std::future::pending())), effect: ToolEffect::WRITE,
        }));
        for _ in 0..2 {
            let mut pending = fixture.handler.execute_tool_call(probeRequest(json!({})));
            assert!(futures_util::poll!(&mut pending).is_pending());
            let ToolExecutorRegistration::Exclusive(registration) = fixture.handler.inner.lock().unwrap()
                .toolExecutors.get("pipeline_probe").unwrap().clone() else { panic!("Expected exclusive executor") };
            assert!(registration.try_lock().is_err());
            drop(pending);
            assert!(fixture.handler.inner.lock().unwrap().toolExecutors.contains_key("pipeline_probe"));
            assert!(registration.try_lock().is_ok());
            assert!(hooks.take().is_empty());
        }
        assert!(ToolExecutionManager::currentToolInvocationContext().is_none());
    }

    /// Proves re-registration during execution neither panics nor overwrites the replacement on release.
    #[tokio::test]
    async fn replacing_an_active_registration_preserves_each_instance() {
        let mut fixture = PipelineFixture::new();
        fixture.handler.registerAsyncTool("pipeline_probe".into(), Box::new(AsyncFnToolExecutor {
            validate: Arc::new(|_| ToolValidationResult { valid: true, errorMessage: String::new() }),
            invoke: Arc::new(|_, _| Box::pin(std::future::pending())), effect: ToolEffect::WRITE,
        }));
        let old = fixture.handler.inner.lock().unwrap().toolExecutors.get("pipeline_probe").unwrap().clone();
        let mut pending = fixture.handler.execute_tool_call(probeRequest(json!({})));
        assert!(futures_util::poll!(&mut pending).is_pending());
        fixture.handler.registerAsyncTool("pipeline_probe".into(), Box::new(AsyncFnToolExecutor {
            validate: Arc::new(|_| ToolValidationResult { valid: true, errorMessage: String::new() }),
            invoke: Arc::new(|_, _| Box::pin(async { ToolResult {
                toolName: "pipeline_probe".into(), success: true,
                result: ToolResultData::JsonResultData(operit_plugin_sdk::js_sdk::results::JsonResultData { value: json!({"generation": 2}) }),
                error: None,
            } })), effect: ToolEffect::WRITE,
        }));
        let result = fixture.handler.execute_tool_call(probeRequest(json!({"generation": 2}))).await;
        assert!(result.success, "{:?}", result.error);
        let JsToolCallResultData::Value(value) = result.data else { panic!("Expected structured JSON") };
        assert_eq!(value["generation"], 2);
        drop(pending);
        let ToolExecutorRegistration::Exclusive(old) = old else { panic!("Expected exclusive registration") };
        assert!(old.try_lock().is_ok());
        let ToolExecutorRegistration::Exclusive(replacement) = fixture.handler.inner.lock().unwrap()
            .toolExecutors.get("pipeline_probe").unwrap().clone() else { panic!("Expected exclusive replacement") };
        assert!(!Arc::ptr_eq(&old, &replacement));
        assert!(fixture.handler.execute_tool_call(probeRequest(json!({}))).await.success);
    }

    /// Proves queued leases retain identity and releasing removed instances cannot republish them.
    #[tokio::test]
    async fn unregistering_an_active_executor_does_not_resurrect_it() {
        let mut fixture = PipelineFixture::new();
        fixture.handler.registerAsyncTool("pipeline_probe".into(), Box::new(AsyncFnToolExecutor {
            validate: Arc::new(|_| ToolValidationResult { valid: true, errorMessage: String::new() }),
            invoke: Arc::new(|_, _| Box::pin(std::future::pending())), effect: ToolEffect::WRITE,
        }));
        let lease = fixture.handler.acquireToolExecutor("pipeline_probe").await.unwrap();
        let mut queued = Box::pin(fixture.handler.acquireToolExecutor("pipeline_probe"));
        assert!(futures_util::poll!(&mut queued).is_pending());
        fixture.handler.clone().unregisterTool("pipeline_probe".into());
        assert!(!fixture.handler.hasToolExecutor("pipeline_probe"));
        drop(lease);
        let queued = queued.await.unwrap();
        assert!(!fixture.handler.hasToolExecutor("pipeline_probe"));
        drop(queued);
        assert!(fixture.handler.acquireToolExecutor("pipeline_probe").await.is_none());
    }

    /// Proves captured hosts retain turn and hook state independently across a scheduler boundary.
    #[tokio::test]
    async fn captured_callers_share_executors_without_sharing_identity_or_snapshot() {
        let mut fixture = PipelineFixture::new();
        let hooks = Arc::new(LifecycleProbe::default());
        fixture.handler.addToolHook(hooks.clone());
        fixture.handler.registerSharedTool("pipeline_probe".into(), Arc::new(SharedProbe::default()), ToolRegistrationVisibility::INTERNAL);
        let turn = ToolRuntimeContext { callerChatId: Some("chat-A".into()), callerParticipantId:None,
            workspacePath:None,workspaceFolders:Vec::new(),toolExposureMode:ToolExposureMode::FULL,
            extensionOwner:"configuration-owner".into(), executionData: Default::default() };
        let a = ToolExecutionManager::scopeToolRuntimeContext(turn.clone(), async {
            ToolInvocationContext::capture(fixture.handler.clone()).withHook("outer-hook")
                .scope(async { fixture.handler.capture_tool_invocation_host() }).await
        }).await;
        let mut b = fixture.handler.clone();
        b.authenticatedExtensionOwner = Some("engine-B".into());
        b.storageSession = Default::default();
        b.chatObservations = Default::default();
        assert!(Arc::ptr_eq(&fixture.handler.inner, &b.inner));
        assert!(!Arc::ptr_eq(&fixture.handler.chatObservations, &b.chatObservations));
        let b = b.capture_tool_invocation_host();
        assert!(ToolExecutionManager::currentToolInvocationContext().is_none());
        for (host, owner, expectedTurn, activeHook) in [
            (a,"engine-owner",serde_json::to_value(&turn).unwrap(),true),
            (b,"engine-B",Value::Null,false),
        ] {
            let result = host.execute_tool_call(probeRequest(json!({"owner":"forged"}))).await;
            assert!(result.success);
            let JsToolCallResultData::Value(value) = result.data else { panic!("Expected structured JSON") };
            assert_eq!(value["owner"],owner);
            assert_eq!(value["turn"],expectedTurn);
            assert_eq!(value["hookActive"],activeHook);
            assert!(hooks.take().is_empty());
        }
    }


    /// Proves registered typed execution decodes once, defers business work until admission, and preserves null.
    #[tokio::test]
    async fn typed_registration_prepares_once_and_never_executes_before_permission() {
        static DECODED: AtomicUsize = AtomicUsize::new(0);
        static EXECUTED: AtomicUsize = AtomicUsize::new(0);
        let mut fixture = PipelineFixture::new();
        let hooks = Arc::new(LifecycleProbe::default());
        fixture.handler.addToolHook(hooks.clone());
        fixture.handler.getToolPermissionSystem().setAsyncPermissionRequester(|_, _, _| async {
            crate::tools::ToolPermissionSystem::PermissionRequestResult::DENY
        });
        struct TypedProbe;
        impl SharedToolExecutor for TypedProbe {
            fn prepare<'a>(&'a self, request: &'a ToolCall, _: &'a ToolInvocationContext, checkAccess: bool)
                -> Result<PreparedToolInvocation<'a>, String> {
                DECODED.fetch_add(1, Ordering::SeqCst);
                let args = request.parameters.clone();
                Ok(crate::tools::ToolInvocation::prepareInvocation(request, checkAccess, ToolEffect::WRITE, async move {
                    EXECUTED.fetch_add(1, Ordering::SeqCst);
                    Ok(ToolResultData::JsonResultData(operit_plugin_sdk::js_sdk::results::JsonResultData { value: serde_json::to_value(args).unwrap() }))
                }))
            }
        }
        fixture.handler.registerSharedTool("typed_probe".into(), Arc::new(TypedProbe), ToolRegistrationVisibility::INTERNAL);
        let request = ToolCall { name:"typed_probe".into(),parameters:BTreeMap::from([("value".into(),Value::Null)]) };
        let denied = fixture.handler.executeToolInvocation(&request, ToolExecutionPolicy::Ai { exposure:ToolExposureMode::FULL }).await.unwrap();
        assert!(!denied[0].success);
        assert_eq!(DECODED.load(Ordering::SeqCst),1);
        assert_eq!(EXECUTED.load(Ordering::SeqCst),0);
        assert_eq!(hooks.take(),["requested","intercept","permission","result","finished"]);
        let allowed = fixture.handler.executeToolInvocation(&request,ToolExecutionPolicy::Direct).await.unwrap();
        assert!(allowed[0].success);
        assert_eq!(DECODED.load(Ordering::SeqCst),2);
        assert_eq!(EXECUTED.load(Ordering::SeqCst),1);
        assert!(hooks.take().is_empty());
    }

    /// Ensures every Tools method selects a built-in checked by the registration build contract.
    #[test]
    fn sdk_bindings_use_the_canonical_builtin_catalog() {
        use operit_plugin_sdk::js_sdk::runtime_bindings::JS_TOOL_BINDINGS;
        for binding in JS_TOOL_BINDINGS {
            assert_eq!(BuiltinToolName::from_name(binding.target.as_str()), Some(binding.target));
        }
    }

    /// A Direct SDK call reaches the common executor validation without dispatching AI hooks.
    #[tokio::test]
    async fn sdk_builtin_tools_use_common_validation_without_ai_hooks() {
        let mut fixture = PipelineFixture::new();
        let hooks = Arc::new(LifecycleProbe::default());
        fixture.handler.addToolHook(hooks.clone());
        fixture.handler.registerBuiltinTool(BuiltinToolName::TtsSetCurrentConfigId,
            RegisteredToolExecutor::Shared(Arc::new(crate::tools::defaultTool::standard::StandardTtsTools::TtsToolExecutor { operation: crate::tools::defaultTool::standard::StandardTtsTools::TtsToolOperation::SetCurrentConfigId })),
            ToolRegistrationVisibility::INTERNAL);
        let result = fixture.handler.execute_tool_call(JsToolCallRequest {
            tool_type: "default".into(), tool_name: "tts_set_current_config_id".into(),
            parameters: BTreeMap::from([("id".into(), json!({"nested":[null,true]}))]),
            device: None,
        }).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid tool invocation"));
        assert!(hooks.take().is_empty());
        assert!(fixture.handler.hasToolExecutor("tts_set_current_config_id"));
    }

    /// Proves CLI aliases resolve once, with hooks and permission admission restricted to AI calls.
    #[tokio::test]
    async fn proxy_resolution_does_not_execute_a_second_lifecycle() {
        let mut fixture = PipelineFixture::new();
        let hooks = Arc::new(LifecycleProbe::default());
        let probe = Arc::new(SharedProbe::default());
        fixture.handler.addToolHook(hooks.clone());
        fixture.handler.registerSharedTool("pipeline_probe".into(),probe.clone(),ToolRegistrationVisibility::PUBLIC);
        fixture.handler.getToolPermissionSystem().setAsyncPermissionRequester(|_, _, _| async {
            crate::tools::ToolPermissionSystem::PermissionRequestResult::DENY
        });
        let turn = ToolRuntimeContext { callerChatId:Some("chat".into()),callerParticipantId:None,
            workspacePath:None,workspaceFolders:Vec::new(),toolExposureMode:ToolExposureMode::CLI,
            extensionOwner:"config-owner".into(), executionData: Default::default() };
        let request = ToolCall { name:"proxy".into(),parameters:BTreeMap::from([
            ("tool_name".into(),Value::String("pipeline_probe".into())),("params".into(),json!({"nullable":null}))
        ]) };
        let denied = ToolExecutionManager::scopeToolRuntimeContext(turn.clone(), fixture.handler.executeToolInvocation(
            &request,ToolExecutionPolicy::Ai { exposure:ToolExposureMode::CLI })).await.unwrap();
        assert!(!denied[0].success);
        assert_eq!(denied[0].toolName,"pipeline_probe");
        assert_eq!(hooks.take(),["requested","intercept","permission","result","finished"]);
        assert_eq!(probe.calls.load(Ordering::SeqCst),0);
        let allowed = ToolExecutionManager::scopeToolRuntimeContext(turn, fixture.handler.executeToolInvocation(&request,ToolExecutionPolicy::Direct)).await.unwrap();
        assert!(allowed[0].success);
        assert!(hooks.take().is_empty());
        assert_eq!(probe.calls.load(Ordering::SeqCst),1);
        assert_eq!(probe.accessCalls.load(Ordering::SeqCst),1);
    }

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
