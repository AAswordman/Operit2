use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::future::Future;

use serde::{Deserialize, Serialize};

use operit_tools::tools::climode::CliToolModeSupport::{
    CliToolModeSupport, PROXY_TOOL_NAME, SEARCH_TOOL_NAME,
};
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolResultDataClasses::stringResultData;
use operit_tools::ConversationMarkupManager::{ConversationMarkupManager, ToolResult};
use operit_util::AppLogger::AppLogger;
use operit_util::ChatMarkupRegex::{attr_value, tag_ranges, ChatMarkupRegex};

const TAG: &str = "ToolExecutionManager";
const PACKAGE_PROXY_TOOL_NAME: &str = "package_proxy";
const CLI_PROXY_TOOL_NAME: &str = PROXY_TOOL_NAME;
const CLI_SEARCH_TOOL_NAME: &str = SEARCH_TOOL_NAME;
const PACKAGE_CALLER_NAME_PARAM: &str = "__operit_package_caller_name";
const PACKAGE_CHAT_ID_PARAM: &str = "__operit_package_chat_id";
const PACKAGE_CALLER_PARTICIPANT_ID_PARAM: &str = "__operit_package_caller_participant_id";
const PACKAGE_CALLER_OWNER_PARAM: &str = "__operit_package_caller_owner";

tokio::task_local! {
    static TOOL_RUNTIME_CONTEXT: ToolRuntimeContext;
    static TOOL_INVOCATION_CONTEXT: ToolInvocationContext;
}

/// Selects the tool surface available to a model response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolExposureMode {
    /// Native tool-call markup with the full registered tool catalog.
    FULL,
    /// Command-line proxy mode with a restricted public tool surface.
    CLI,
}

/// Context made available while a batch of tool invocations is executing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRuntimeContext {
    pub callerChatId: Option<String>,
    pub callerParticipantId: Option<String>,
    pub workspacePath: Option<String>,
    pub workspaceFolders: Vec<String>,
    pub toolExposureMode: ToolExposureMode,
    /// Identifies the native-authenticated configuration owner captured for this execution.
    pub extensionOwner: String,
    /// Preserves that owner's complete opaque message snapshot without rereading current bindings.
    pub messageExtension: serde_json::Map<String, serde_json::Value>,
}

impl ToolRuntimeContext {
    /// Rejects unauthenticated or malformed turn identities before entering any policy dispatch.
    pub fn validateSnapshot(&self) -> Result<(), String> {
        if self.extensionOwner.trim().is_empty()
            || self.extensionOwner.trim() != self.extensionOwner
        {
            return Err("Tool execution snapshot requires an exact authenticated extensionOwner".to_string());
        }
        for (field, value) in [
            ("chatId", self.callerChatId.as_deref()),
            ("participantId", self.callerParticipantId.as_deref()),
        ] {
            if let Some(value) = value {
                if value.trim().is_empty() || value.trim() != value {
                    return Err(format!("Tool execution snapshot {field} must be exact nonblank text"));
                }
            }
        }
        Ok(())
    }
}

/// Name-value parameter parsed from a model tool invocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub value: String,
}

/// Parsed model request for one AI tool invocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AITool {
    pub name: String,
    pub parameters: Vec<ToolParameter>,
}

/// Preserves structured parameters through the common invocation pipeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub parameters: BTreeMap<String, serde_json::Value>,
}

impl ToolCall {
    /// Resolves an explicit proxy envelope once before the sole lifecycle; nested proxies are rejected.
    pub(crate) fn resolveTarget(&self, context: &ToolInvocationContext) -> Result<Self, String> {
        if !matches!(self.name.as_str(), PACKAGE_PROXY_TOOL_NAME | CLI_PROXY_TOOL_NAME) { return Ok(self.clone()); }
        if self.name == CLI_PROXY_TOOL_NAME && context.turnSnapshot().map(|turn| &turn.toolExposureMode) != Some(&ToolExposureMode::CLI) {
            return Err(CliToolModeSupport::buildCliModeUnavailableMessage(false));
        }
        for key in self.parameters.keys() {
            if !matches!(key.as_str(), "tool_name" | "params" | PACKAGE_CALLER_NAME_PARAM | PACKAGE_CHAT_ID_PARAM | PACKAGE_CALLER_PARTICIPANT_ID_PARAM | PACKAGE_CALLER_OWNER_PARAM) {
                return Err(format!("Unexpected proxy parameter: {key}"));
            }
        }
        let name = self.parameters.get("tool_name").and_then(serde_json::Value::as_str)
            .ok_or("Proxy tool_name must be text")?;
        if name.trim().is_empty() || name.trim() != name { return Err("Proxy tool_name must be exact nonblank text".into()); }
        if CliToolModeSupport::isReservedProxyTarget(name) { return Err(format!("Proxy cannot target another proxy or catalog entry: {name}")); }
        if self.name == PACKAGE_PROXY_TOOL_NAME {
            match name.split_once(':') {
                Some((package, method)) if !package.is_empty() && !method.is_empty() && !method.contains(':') => {},
                _ => return Err("tool_name must use packageName:toolName format".into()),
            }
        }
        let value = self.parameters.get("params").ok_or("Proxy params is required")?;
        let object = match value {
            serde_json::Value::String(text) => serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(text).map_err(|error| error.to_string())?,
            serde_json::Value::Object(object) => object.clone(),
            _ => return Err("Proxy params must be a JSON object or its legacy text encoding".into()),
        };
        let mut parameters = object.into_iter().collect::<BTreeMap<_, _>>();
        for key in [PACKAGE_CALLER_NAME_PARAM, PACKAGE_CHAT_ID_PARAM, PACKAGE_CALLER_PARTICIPANT_ID_PARAM, PACKAGE_CALLER_OWNER_PARAM] {
            parameters.remove(key);
            if let Some(value) = self.parameters.get(key) { parameters.insert(key.to_string(), value.clone()); }
        }
        Ok(Self { name: name.to_string(), parameters })
    }

    /// Adapts values only at the legacy executor and text-hook contract boundary.
    pub fn toLegacyTool(&self) -> AITool {
        AITool {
            name: self.name.clone(),
            parameters: self.parameters.iter().map(|(name, value)| ToolParameter {
                name: name.clone(),
                value: match value {
                    serde_json::Value::String(text) => text.clone(),
                    value => value.to_string(),
                },
            }).collect(),
        }
    }
}

impl From<&AITool> for ToolCall {
    /// Adapts model text without interpreting its contents as JSON or caller identity.
    fn from(tool: &AITool) -> Self {
        Self {
            name: tool.name.clone(),
            parameters: tool.parameters.iter().map(|parameter| (
                parameter.name.clone(), serde_json::Value::String(parameter.value.clone()),
            )).collect(),
        }
    }
}

/// Carries native-owned caller state independently of public tool parameters and shared registrations.
#[derive(Clone)]
pub struct ToolInvocationContext {
    host: AIToolHandler,
    snapshot: ToolInvocationSnapshot,
}

/// Captures transport-independent turn and hook state before any Host scheduling boundary.
#[derive(Clone, Default)]
pub(crate) struct ToolInvocationSnapshot {
    pub turnSnapshot: Option<ToolRuntimeContext>,
    activeHooks: BTreeSet<String>,
}

impl ToolInvocationContext {
    /// Captures the actual caller-bound handler, turn snapshot and inherited hook scope.
    pub(crate) fn capture(host: AIToolHandler) -> Self {
        let snapshot = match TOOL_INVOCATION_CONTEXT.try_with(|context| context.snapshot.clone()) {
            Ok(snapshot) => snapshot,
            Err(_) => match TOOL_RUNTIME_CONTEXT.try_with(Clone::clone) {
                Ok(turn) => ToolInvocationSnapshot { turnSnapshot: Some(turn), activeHooks: BTreeSet::new() },
                Err(_) => host.invocationSnapshot.clone(),
            },
        };
        Self { host, snapshot }
    }

    /// Clones the immutable snapshot for transfer to an engine-bound native host.
    pub(crate) fn snapshot(&self) -> ToolInvocationSnapshot { self.snapshot.clone() }

    /// Borrows the immutable caller-bound handler carrying its authenticated owner and storage session.
    pub fn host(&self) -> &AIToolHandler { &self.host }

    /// Requires the immutable engine-bound owner to remain registered and enabled.
    pub fn requirePluginOwner(&self) -> Result<String, String> {
        let owner = self.host.authenticatedExtensionOwner.as_deref()
            .ok_or_else(|| "Plugin operations require a real ToolPkg execution owner".to_string())?;
        if owner.trim().is_empty() || owner.trim() != owner { return Err("Plugin execution owner is not exact nonblank text".into()); }
        let manager = self.host.getOrCreatePackageManager();
        let registry = manager.lock().map_err(|_| "Package manager mutex poisoned")?;
        registry.requireEnabledToolPkgContainer(owner)?;
        Ok(owner.to_string())
    }

    /// Borrows the exact captured model turn independently of the executing plugin's ownership.
    pub fn turnSnapshot(&self) -> Option<&ToolRuntimeContext> { self.snapshot.turnSnapshot.as_ref() }

    /// Returns whether this invocation is already executing the named hook.
    pub fn isHookActive(&self, id: &str) -> bool { self.snapshot.activeHooks.contains(id) }

    /// Captures a nested scope before awaiting the named hook.
    pub fn withHook(&self, id: &str) -> Self {
        let mut nested = self.clone();
        nested.snapshot.activeHooks.insert(id.to_owned());
        nested
    }

    /// Reinstalls native caller state for synchronous hook dispatch and cancellation cleanup.
    pub fn scopeSync<T>(self, action: impl FnOnce() -> T) -> T {
        let turn = self.snapshot.turnSnapshot.clone();
        let scoped = || TOOL_INVOCATION_CONTEXT.sync_scope(self, action);
        match turn {
            Some(turn) => TOOL_RUNTIME_CONTEXT.sync_scope(turn, scoped),
            None => scoped(),
        }
    }

    /// Keeps native caller state available throughout this asynchronous invocation.
    pub async fn scope<T, F: Future<Output = T>>(self, future: F) -> T {
        let turn = self.snapshot.turnSnapshot.clone();
        let scoped = TOOL_INVOCATION_CONTEXT.scope(self, future);
        match turn {
            Some(turn) => TOOL_RUNTIME_CONTEXT.scope(turn, scoped).await,
            None => scoped.await,
        }
    }
}

/// Parsed tool invocation plus its source text location in the assistant response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub tool: AITool,
    pub rawText: String,
    pub responseLocation: (usize, usize),
}

/// Captures a completed request to move one chat to another CoreNode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteChangeIntent {
    pub targetNodeId: String,
}

/// Parses model tool calls, checks permissions, and executes tools through handlers.
pub struct ToolExecutionManager;

/// Selects native-owned admission policy without exposing permission controls to JavaScript parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolExecutionPolicy {
    /// Dispatches AI lifecycle hooks and applies the model's declared exposure and permission preflight.
    Ai { exposure: ToolExposureMode },
    /// Executes an ordinary JavaScript or application request without AI lifecycle hooks or permission preflight.
    Direct,
    /// Applies receiver-owned permission preflight and lifecycle to an authenticated device request.
    Device,
}

impl ToolExecutionManager {
    /// Returns the native context captured for the current invocation and its awaited hooks.
    pub fn currentToolInvocationContext() -> Option<ToolInvocationContext> {
        TOOL_INVOCATION_CONTEXT.try_with(Clone::clone).ok()
    }

    /// Returns the runtime context for the currently executing tool batch.
    pub fn currentToolRuntimeContext() -> Option<ToolRuntimeContext> {
        TOOL_RUNTIME_CONTEXT.try_with(Clone::clone).ok()
    }

    /// Scopes an owned turn snapshot to one asynchronous execution, including every awaited hook.
    pub async fn scopeToolRuntimeContext<T, F>(context: ToolRuntimeContext, future: F) -> T
    where
        F: Future<Output = T>,
    {
        TOOL_RUNTIME_CONTEXT.scope(context, future).await
    }

    /// Extracts XML-like tool invocations from an assistant response.
    pub fn extractToolInvocations(response: &str) -> Vec<ToolInvocation> {
        let mut invocations = Vec::new();
        for tool_match in ChatMarkupRegex::tool_call_matches(response) {
            let mut parameters = Vec::new();
            for (start, end) in tag_ranges(&tool_match.body, "param") {
                let raw = &tool_match.body[start..end];
                let paramName = attr_value(raw, "name").unwrap_or_default();
                let paramValue = raw
                    .split_once('>')
                    .and_then(|(_, tail)| tail.rsplit_once("</").map(|(body, _)| body))
                    .map(Self::unescapeXml)
                    .unwrap_or_default();
                parameters.push(ToolParameter {
                    name: paramName,
                    value: paramValue,
                });
            }
            invocations.push(ToolInvocation {
                tool: AITool {
                    name: tool_match.name,
                    parameters,
                },
                rawText: response[tool_match.start..tool_match.end].to_string(),
                responseLocation: (tool_match.start, tool_match.end),
            });
        }
        let toolNames = invocations
            .iter()
            .map(|invocation| invocation.tool.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        AppLogger::d(
            TAG,
            &format!(
                "tool.parse.complete responseChars={} toolCount={} tools=[{}]",
                response.len(),
                invocations.len(),
                toolNames
            ),
        );
        invocations
    }

    /// Executes a batch and returns emitted markup and results.
    pub async fn executeInvocations(
        invocations: &[ToolInvocation],
        toolHandler: &mut AIToolHandler,
        packageManager: &RuntimePackageManager,
        callerName: Option<String>,
        callerChatId: Option<String>,
        callerParticipantId: Option<String>,
        workspacePath: Option<String>,
        workspaceFolders: Vec<String>,
        toolExposureMode: ToolExposureMode,
        extensionOwner: String,
        messageExtension: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(Vec<String>, Vec<ToolResult>, Option<RouteChangeIntent>), String> {
        let runtimeContext = ToolRuntimeContext {
            callerChatId: callerChatId.clone(),
            callerParticipantId: callerParticipantId.clone(),
            workspacePath: workspacePath.clone(),
            workspaceFolders: workspaceFolders.clone(),
            toolExposureMode: toolExposureMode.clone(),
            extensionOwner,
            messageExtension,
        };
        runtimeContext.validateSnapshot()?;
        Self::scopeToolRuntimeContext(runtimeContext, async move {
        let mut emitted = Vec::new();
        let mut results = Vec::new();
        let mut routeChangeIntent = None;
        let requestedToolNames = invocations
            .iter()
            .map(|invocation| invocation.tool.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        AppLogger::d(
            TAG,
            &format!(
                "tool.execution.batch_start requestedCount={} tools=[{}] callerNameSet={} callerChatIdSet={} callerParticipantIdSet={} exposure={:?}",
                invocations.len(),
                requestedToolNames,
                callerName.is_some(),
                callerChatId.is_some(),
                callerParticipantId.is_some(),
                toolExposureMode
            ),
        );
        toolHandler.registerDefaultTools();
        let jsPackageNames = packageManager
            .getAvailablePackages()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let injectedInvocations = invocations
            .iter()
            .map(|invocation| {
                Self::injectPackageCallContext(
                    invocation,
                    &jsPackageNames,
                    callerName.as_deref(),
                    callerChatId.as_deref(),
                    callerParticipantId.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        for invocation in injectedInvocations.iter().cloned() {
            let displayToolName = Self::resolveDisplayToolName(&invocation.tool);
            AppLogger::d(
                TAG,
                &format!(
                    "tool.execution.start tool={} params={} rawChars={}",
                    displayToolName,
                    invocation.tool.parameters.len(),
                    invocation.rawText.len()
                ),
            );
            let collected = toolHandler
                .executeToolInvocation(
                    &ToolCall::from(&invocation.tool),
                    ToolExecutionPolicy::Ai { exposure: toolExposureMode.clone() },
                )
                .await?;
            AppLogger::d(
                TAG,
                &format!(
                    "tool.execution.raw_results tool={} rawResultCount={}",
                    displayToolName,
                    collected.len()
                ),
            );
            for result in &collected {
                emitted.push(ensureEndsWithNewline(
                    &ConversationMarkupManager::formatToolResultForMessage(result),
                ));
            }
            {
                let last = collected.last().expect("collected not empty");
                let combinedResultString = collected
                    .iter()
                    .map(|item| {
                        if item.success {
                            item.result.toString().trim().to_string()
                        } else {
                            format!(
                                "Step error: {}",
                                item.error
                                    .clone()
                                    .unwrap_or_else(|| "Unknown error".to_string())
                            )
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string();
                let finalResult = ToolResult {
                    toolName: displayToolName.clone(),
                    success: last.success,
                    result: stringResultData(combinedResultString),
                    error: last.error.clone(),
                };
                if let Some(intent) = Self::routeChangeIntentForResult(&invocation.tool, last) {
                    routeChangeIntent = Some(intent);
                }
                AppLogger::d(
                    TAG,
                    &format!(
                        "tool.execution.complete tool={} success={} rawResultCount={} combinedChars={}",
                        displayToolName,
                        finalResult.success,
                        collected.len(),
                        finalResult.result.toString().len()
                    ),
                );
                results.push(finalResult);
            }
        }

        let emittedChars = emitted.iter().map(|content| content.len()).sum::<usize>();
        AppLogger::d(
            TAG,
            &format!(
                "tool.execution.batch_complete requestedCount={} emittedMessages={} emittedChars={} resultCount={}",
                invocations.len(),
                emitted.len(),
                emittedChars,
                results.len()
            ),
        );
        Ok((emitted, results, routeChangeIntent))
        }).await
    }

    fn ensureEndsWithNewline(content: &str) -> String {
        ensureEndsWithNewline(content)
    }

    /// Builds a route-change intent from the successful switch_core tool result.
    #[allow(non_snake_case)]
    fn routeChangeIntentForResult(tool: &AITool, result: &ToolResult) -> Option<RouteChangeIntent> {
        if Self::resolveDisplayToolName(tool).trim() != "switch_core"
            || result.toolName.trim() != "switch_core"
            || !result.success
        {
            return None;
        }
        Some(RouteChangeIntent {
            targetNodeId: result.result.toString().trim().to_string(),
        })
    }

    /// Returns only a presentation name; execution arguments are resolved strictly by ToolCall.
    fn resolveDisplayToolName(tool: &AITool) -> String {
        match tool.name.as_str() {
            PACKAGE_PROXY_TOOL_NAME | CLI_PROXY_TOOL_NAME => {
                match tool.parameters.iter().find(|parameter| parameter.name == "tool_name") {
                    Some(parameter) => parameter.value.clone(),
                    None => tool.name.clone(),
                }
            },
            _ => tool.name.clone(),
        }
    }

    fn isJsPackageTool(toolName: &str, jsPackageNames: &BTreeSet<String>) -> bool {
        let parts = toolName.splitn(2, ':').collect::<Vec<_>>();
        parts.len() == 2 && jsPackageNames.contains(parts[0])
    }

    /// Replaces every untrusted reserved value with the authenticated host context, including its absence.
    fn setPackageContextParameter(
        params: &mut Vec<ToolParameter>,
        name: &str,
        value: Option<&str>,
    ) {
        params.retain(|parameter| parameter.name != name);
        if let Some(value) = value {
            params.push(ToolParameter {
                name: name.to_string(),
                value: value.to_string(),
            });
        }
    }

    /// Injects canonical context into a selected JavaScript tool or its proxy wrapper.
    fn injectPackageCallContext(
        invocation: &ToolInvocation,
        jsPackageNames: &BTreeSet<String>,
        callerName: Option<&str>,
        callerChatId: Option<&str>,
        callerParticipantId: Option<&str>,
    ) -> ToolInvocation {
        let targetName = Self::resolveDisplayToolName(&invocation.tool);
        if !Self::isJsPackageTool(&targetName, jsPackageNames) {
            return invocation.clone();
        }

        let mut updatedParams = invocation.tool.parameters.clone();
        Self::setPackageContextParameter(&mut updatedParams, PACKAGE_CALLER_NAME_PARAM, callerName);
        Self::setPackageContextParameter(&mut updatedParams, PACKAGE_CHAT_ID_PARAM, callerChatId);
        Self::setPackageContextParameter(
            &mut updatedParams,
            PACKAGE_CALLER_PARTICIPANT_ID_PARAM,
            callerParticipantId,
        );

        let context = Self::currentToolRuntimeContext();
        Self::setPackageContextParameter(
            &mut updatedParams,
            PACKAGE_CALLER_OWNER_PARAM,
            context.as_ref().map(|context| context.extensionOwner.as_str()),
        );

        ToolInvocation {
            tool: AITool {
                name: invocation.tool.name.clone(),
                parameters: updatedParams,
            },
            rawText: invocation.rawText.clone(),
            responseLocation: invocation.responseLocation,
        }
    }

    fn getParameterValue(tool: &AITool, name: &str) -> Option<String> {
        tool.parameters
            .iter()
            .find(|parameter| parameter.name == name)
            .map(|parameter| parameter.value.trim().to_string())
    }

    /// Rejects tools outside the model surface before the common pipeline resolves an executor.
    pub(crate) fn buildToolExposureDeniedResult(
        tool: &AITool,
        toolExposureMode: ToolExposureMode,
    ) -> Option<ToolResult> {
        let toolName = tool.name.trim();
        let denied = match toolExposureMode {
            ToolExposureMode::CLI if !Self::isCliPublicTool(toolName) => Some(format!(
                "{}",
                CliToolModeSupport::buildCliTopLevelRestrictionErrorMessage(
                    &Self::resolveDisplayToolName(tool),
                    true,
                )
            )),
            ToolExposureMode::FULL if Self::isCliPublicTool(toolName) => {
                Some(CliToolModeSupport::buildCliModeUnavailableMessage(true))
            }
            _ => None,
        }?;

        Some(ToolResult {
            toolName: if toolExposureMode == ToolExposureMode::CLI
                && !Self::isCliPublicTool(toolName)
            {
                Self::resolveDisplayToolName(tool)
            } else {
                toolName.to_string()
            },
            success: false,
            result: stringResultData(""),
            error: Some(denied),
        })
    }

    fn isCliPublicTool(toolName: &str) -> bool {
        toolName == CLI_SEARCH_TOOL_NAME || toolName == CLI_PROXY_TOOL_NAME
    }

    fn buildToolNotAvailableErrorMessage(toolName: &str) -> String {
        if toolName.contains('.') && !toolName.contains(':') {
            let parts = toolName.splitn(2, '.').collect::<Vec<_>>();
            return format!(
                "Tool invocation syntax error: for tools inside a package, use the 'packName:toolName' format instead of '{}'. You may want to call '{}:{}'.",
                toolName,
                parts.get(0).copied().unwrap_or(""),
                parts.get(1).copied().unwrap_or("")
            );
        }

        if toolName.contains(':') {
            let parts = toolName.splitn(2, ':').collect::<Vec<_>>();
            let packName = parts[0];
            let toolNamePart = parts.get(1).copied().unwrap_or("");
            return format!(
                "Tool package '{}' is not activated. Auto-activation was attempted but failed, or tool '{}' does not exist. Please use 'use_package' with package name '{}' to check available tools.",
                packName, toolNamePart, packName
            );
        }

        format!(
            "Tool '{}' is unavailable or does not exist. If this is a tool inside a package, call it using the 'packName:toolName' format.",
            toolName
        )
    }

    fn unescapeXml(input: &str) -> String {
        let mut result = input.to_string();
        if result.starts_with("<![CDATA[") && result.ends_with("]]>") {
            result = result[9..result.len() - 3].to_string();
        }
        if result.ends_with("]]>") {
            result.truncate(result.len() - 3);
        }
        if result.starts_with("<![CDATA[") {
            result = result[9..].to_string();
        }
        result
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
    }
}

pub trait ToolExecutor: Send {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult;
    fn accessSpec(&self, tool: &AITool) -> Result<ToolAccessSpec, String>;
    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult>;
}

/// Represents a tool result produced without blocking its calling executor.
pub type ToolInvocationFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Vec<ToolResult>> + Send + 'a>>;

/// Executes tools that own asynchronous package calls; synchronous tools retain ToolExecutor.
pub trait AsyncToolExecutor: Send {
    /// Validates a request before its execution begins.
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult;
    /// Declares the access boundary of the asynchronous tool.
    fn accessSpec(&self, tool: &AITool) -> Result<ToolAccessSpec, String>;
    /// Executes one invocation on its asynchronous result path.
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool, context: &'a ToolInvocationContext) -> ToolInvocationFuture<'a>;
}

/// Holds one validated invocation and its optional native AI access declaration.
pub struct PreparedToolInvocation<'a> {
    pub access: Option<ToolAccessSpec>,
    pub invoke: ToolInvocationFuture<'a>,
}

/// Prepares concurrent operations without retaining an exclusive registry lease.
pub trait SharedToolExecutor: Send + Sync {
    /// Decodes and validates once, declaring access only when native AI admission requires it.
    fn prepare<'a>(&'a self, request: &'a ToolCall, context: &'a ToolInvocationContext, checkAccess: bool)
        -> Result<PreparedToolInvocation<'a>, String>;
}

/// Records the execution contract explicitly selected when a tool is registered.
pub enum RegisteredToolExecutor {
    /// Declares an invocation envelope resolved before execution, never an executable forwarding tool.
    Alias,
    Synchronous(Box<dyn ToolExecutor>),
    Asynchronous(Box<dyn AsyncToolExecutor>),
    Shared(Arc<dyn SharedToolExecutor>),
}

impl<T: ToolExecutor + 'static> From<Box<T>> for RegisteredToolExecutor {
    /// Records a synchronous executor without changing its invocation contract.
    fn from(executor: Box<T>) -> Self {
        Self::Synchronous(executor)
    }
}

impl From<Box<dyn ToolExecutor>> for RegisteredToolExecutor {
    /// Records an already erased synchronous executor at registration.
    fn from(executor: Box<dyn ToolExecutor>) -> Self {
        Self::Synchronous(executor)
    }
}

impl RegisteredToolExecutor {
    /// Prepares every registered contract once; only legacy adapters convert structured arguments to text.
    pub fn prepare<'a>(&'a mut self, request: &'a ToolCall, context: &'a ToolInvocationContext, checkAccess: bool)
        -> Result<PreparedToolInvocation<'a>, String> {
        match self {
            Self::Alias => Err("Invocation aliases must be resolved before acquiring an executor".into()),
            Self::Shared(executor) => executor.prepare(request, context, checkAccess),
            Self::Synchronous(executor) => {
                let tool = request.toLegacyTool();
                let validation = executor.validateParameters(&tool);
                if !validation.valid { return Err(validation.errorMessage); }
                let access = if checkAccess { Some(executor.accessSpec(&tool)?) } else { None };
                Ok(PreparedToolInvocation { access, invoke: Box::pin(async move { executor.invokeAndStream(&tool) }) })
            },
            Self::Asynchronous(executor) => {
                let tool = request.toLegacyTool();
                let validation = executor.validateParameters(&tool);
                if !validation.valid { return Err(validation.errorMessage); }
                let access = if checkAccess { Some(executor.accessSpec(&tool)?) } else { None };
                Ok(PreparedToolInvocation { access, invoke: Box::pin(async move { executor.invokeAndStreamAsync(&tool, context).await }) })
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolEffect {
    READ,
    WRITE,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolBoundary {
    None,
    FilePath {
        effect: ToolEffect,
    },
    FilePair {
        source: ToolEffect,
        destination: ToolEffect,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolAccessSpec {
    pub effect: ToolEffect,
    pub boundary: ToolBoundary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolValidationResult {
    pub valid: bool,
    pub errorMessage: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates an explicit opaque turn snapshot for task-local tests without a package-manager fixture.
    fn turn_snapshot(owner: &str) -> ToolRuntimeContext {
        ToolRuntimeContext {
            callerChatId: Some(format!("chat:{owner}")),
            callerParticipantId: Some(format!("participant:{owner}")),
            workspacePath: None,
            workspaceFolders: Vec::new(),
            toolExposureMode: ToolExposureMode::FULL,
            extensionOwner: owner.to_string(),
            messageExtension: serde_json::json!({"turn": owner}).as_object().unwrap().clone(),
        }
    }

    /// Verifies concurrent awaited scopes retain different snapshots and leave no context in their parent task.
    #[tokio::test]
    async fn concurrent_execution_snapshots_remain_task_local() {
        let left = turn_snapshot("package.left");
        let right = turn_snapshot("package.right");
        let (first, second) = tokio::join!(
            ToolExecutionManager::scopeToolRuntimeContext(left.clone(), async {
                tokio::task::yield_now().await;
                ToolExecutionManager::currentToolRuntimeContext().unwrap()
            }),
            ToolExecutionManager::scopeToolRuntimeContext(right.clone(), async {
                tokio::task::yield_now().await;
                ToolExecutionManager::currentToolRuntimeContext().unwrap()
            }),
        );
        assert_eq!(first, left);
        assert_eq!(second, right);
        assert!(ToolExecutionManager::currentToolRuntimeContext().is_none());
    }

    /// Verifies nested invocations restore their parent snapshot after success and after original policy failure.
    #[tokio::test]
    async fn nested_execution_snapshots_restore_the_parent_on_failure() {
        let parent = turn_snapshot("package.parent");
        let child = turn_snapshot("package.child");
        ToolExecutionManager::scopeToolRuntimeContext(parent.clone(), async {
            let result: Result<(), String> = ToolExecutionManager::scopeToolRuntimeContext(child.clone(), async {
                assert_eq!(ToolExecutionManager::currentToolRuntimeContext(), Some(child));
                tokio::task::yield_now().await;
                Err(" exact rejected policy ".to_string())
            }).await;
            assert_eq!(result, Err(" exact rejected policy ".to_string()));
            assert_eq!(ToolExecutionManager::currentToolRuntimeContext(), Some(parent));
        }).await;
        assert!(ToolExecutionManager::currentToolRuntimeContext().is_none());
    }

    /// Rejects noncanonical owner identities and malformed optional participant identifiers without normalization.
    #[test]
    fn execution_snapshot_requires_exact_authenticated_identity() {
        let valid = turn_snapshot("package.owner");
        assert!(valid.validateSnapshot().is_ok());
        for owner in ["", " ", " package.owner", "package.owner "] {
            let mut invalid = valid.clone();
            invalid.extensionOwner = owner.to_string();
            assert!(invalid.validateSnapshot().is_err());
        }
        let mut invalid = valid;
        invalid.callerParticipantId = Some(" participant ".to_string());
        assert!(invalid.validateSnapshot().is_err());
    }

    #[test]
    fn route_change_intent_uses_switch_core_result_text() {
        let tool = AITool {
            name: " switch_core ".to_string(),
            parameters: Vec::new(),
        };
        let result = ToolResult {
            toolName: "switch_core".to_string(),
            success: true,
            result: stringResultData(" core-target-1 \n"),
            error: None,
        };

        let intent = ToolExecutionManager::routeChangeIntentForResult(&tool, &result)
            .expect("switch_core success must produce route intent");

        assert_eq!(intent.targetNodeId, "core-target-1");
    }

    #[test]
    fn route_change_intent_recognizes_executed_cli_proxy_target() {
        let tool = AITool {
            name: "proxy".to_string(),
            parameters: vec![ToolParameter {
                name: "tool_name".to_string(),
                value: "switch_core".to_string(),
            }],
        };
        let result = ToolResult {
            toolName: "switch_core".to_string(),
            success: true,
            result: stringResultData("core-target-2"),
            error: None,
        };
        assert_eq!(
            ToolExecutionManager::routeChangeIntentForResult(&tool, &result)
                .expect("executed switch_core via CLI proxy must produce route intent")
                .targetNodeId,
            "core-target-2",
        );

        let mut failed = result.clone();
        failed.success = false;
        assert!(ToolExecutionManager::routeChangeIntentForResult(&tool, &failed).is_none());
        let mut mismatched = result;
        mismatched.toolName = "other_tool".to_string();
        assert!(ToolExecutionManager::routeChangeIntentForResult(&tool, &mismatched).is_none());
    }
}

fn ensureEndsWithNewline(content: &str) -> String {
    if content.ends_with('\n') {
        content.to_string()
    } else {
        format!("{content}\n")
    }
}
