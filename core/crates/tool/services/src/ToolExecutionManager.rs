use std::collections::BTreeSet;
use std::future::Future;

use serde::{Deserialize, Serialize};

use operit_plugin_sdk::js_sdk::tool_types::BuiltinToolName;
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

tokio::task_local! {
    static TOOL_RUNTIME_CONTEXT: ToolRuntimeContext;
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

/// Resolved executable tool target and the name shown in tool results.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedToolTarget {
    tool: AITool,
    displayName: String,
}

/// Parses model tool calls, checks permissions, and executes tools through handlers.
pub struct ToolExecutionManager;

impl ToolExecutionManager {
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

    /// Validates and invokes one tool using the supplied executor.
    pub fn executeToolSafely(
        invocation: &ToolInvocation,
        executor: &mut dyn ToolExecutor,
    ) -> Vec<ToolResult> {
        let validationResult = executor.validateParameters(&invocation.tool);
        if !validationResult.valid {
            return vec![ToolResult {
                toolName: invocation.tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(format!(
                    "Invalid parameters: {}",
                    validationResult.errorMessage
                )),
            }];
        }
        executor.invokeAndStream(&invocation.tool)
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
            if let Some(deniedResult) =
                Self::buildToolExposureDeniedResult(&invocation, toolExposureMode.clone())
            {
                toolHandler.notifyToolExecutionResult(&invocation.tool, &deniedResult);
                emitted.push(ensureEndsWithNewline(
                    &ConversationMarkupManager::formatToolResultForMessage(&deniedResult),
                ));
                results.push(deniedResult);
                AppLogger::w(
                    TAG,
                    &format!(
                        "tool.execution.denied tool={} reason=exposure",
                        displayToolName
                    ),
                );
                continue;
            }

            toolHandler.notifyToolCallRequested(&invocation.tool);
            let interception = toolHandler.checkToolInterception(&invocation.tool).await?;
            if let operit_tools::tools::AIToolHook::AIToolHookDecision::Block(_) = interception {
                let blockedResult =
                    AIToolHandler::toolInterceptionResult(&invocation.tool, interception);
                toolHandler.notifyToolExecutionResult(&invocation.tool, &blockedResult);
                emitted.push(ensureEndsWithNewline(
                    &ConversationMarkupManager::formatToolResultForMessage(&blockedResult),
                ));
                results.push(blockedResult);
                continue;
            }
            if !toolHandler.getToolExecutorOrActivate(&invocation.tool.name).await {
                let errorMessage = Self::buildToolNotAvailableErrorMessage(&invocation.tool.name);
                let content = ConversationMarkupManager::createToolNotAvailableError(
                    &invocation.tool.name,
                    Some(&errorMessage),
                );
                let deniedResult = ToolResult {
                    toolName: displayToolName.clone(),
                    success: false,
                    result: stringResultData(""),
                    error: Some(errorMessage),
                };
                toolHandler.notifyToolExecutionResult(&invocation.tool, &deniedResult);
                emitted.push(ensureEndsWithNewline(&content));
                results.push(deniedResult);
                AppLogger::w(
                    TAG,
                    &format!(
                        "tool.execution.denied tool={} reason=unavailable",
                        displayToolName
                    ),
                );
                continue;
            }
            toolHandler.notifyToolExecutionStarted(&invocation.tool);
            let Some(collected) = toolHandler
                .executeToolSafelyWithResolvedExecutor(&invocation.tool)
                .await
            else {
                toolHandler.notifyToolExecutionFinished(&invocation.tool);
                AppLogger::w(
                    TAG,
                    &format!("tool.execution.no_executor_result tool={}", displayToolName),
                );
                continue;
            };
            AppLogger::d(
                TAG,
                &format!(
                    "tool.execution.raw_results tool={} rawResultCount={}",
                    displayToolName,
                    collected.len()
                ),
            );
            for result in &collected {
                toolHandler.notifyToolExecutionResult(&invocation.tool, result);
                emitted.push(ensureEndsWithNewline(
                    &ConversationMarkupManager::formatToolResultForMessage(result),
                ));
            }
            if collected.is_empty() {
                let emptyResult = ToolResult {
                    toolName: displayToolName.clone(),
                    success: false,
                    result: stringResultData(""),
                    error: Some("The tool execution returned no results.".to_string()),
                };
                toolHandler.notifyToolExecutionResult(&invocation.tool, &emptyResult);
                results.push(emptyResult);
                AppLogger::w(
                    TAG,
                    &format!("tool.execution.empty_result tool={}", displayToolName),
                );
            } else {
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
                toolHandler.notifyToolExecutionResult(&invocation.tool, &finalResult);
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
            toolHandler.notifyToolExecutionFinished(&invocation.tool);
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
        if Self::resolveToolTarget(tool).tool.name.trim() != "switch_core"
            || result.toolName.trim() != "switch_core"
            || !result.success
        {
            return None;
        }
        Some(RouteChangeIntent {
            targetNodeId: result.result.toString().trim().to_string(),
        })
    }

    /// Returns the concrete target carried by a proxy tool invocation.
    pub(crate) fn resolveProxyTargetTool(tool: &AITool) -> AITool {
        Self::resolveToolTarget(tool).tool
    }

    /// Resolves one proxy wrapper into its concrete target and display name.
    fn resolveToolTarget(tool: &AITool) -> ResolvedToolTarget {
        if tool.name != PACKAGE_PROXY_TOOL_NAME && tool.name != CLI_PROXY_TOOL_NAME {
            return ResolvedToolTarget {
                tool: tool.clone(),
                displayName: tool.name.clone(),
            };
        }

        let targetToolName = tool
            .parameters
            .iter()
            .find(|parameter| parameter.name == "tool_name")
            .map(|parameter| parameter.value.trim().to_string())
            .unwrap_or_default();
        if targetToolName.is_empty() {
            return ResolvedToolTarget {
                tool: tool.clone(),
                displayName: tool.name.clone(),
            };
        }

        let forwardedParameters = Self::resolveProxyParameters(tool);
        ResolvedToolTarget {
            tool: AITool {
                name: targetToolName.clone(),
                parameters: forwardedParameters,
            },
            displayName: targetToolName,
        }
    }

    fn resolveDisplayToolName(tool: &AITool) -> String {
        Self::resolveToolTarget(tool).displayName
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
        let resolvedTargetTool = Self::resolveToolTarget(&invocation.tool).tool;
        if !Self::isJsPackageTool(&resolvedTargetTool.name, jsPackageNames) {
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

    fn resolveProxyParameters(tool: &AITool) -> Vec<ToolParameter> {
        let paramsRaw = tool
            .parameters
            .iter()
            .find(|parameter| parameter.name == "params")
            .map(|parameter| parameter.value.trim().to_string())
            .unwrap_or_default();
        if paramsRaw.is_empty() {
            return Vec::new();
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(&paramsRaw) else {
            return Vec::new();
        };
        let Some(object) = value.as_object() else {
            return Vec::new();
        };

        object
            .iter()
            .map(|(key, value)| ToolParameter {
                name: key.clone(),
                value: match value {
                    serde_json::Value::Null => "null".to_string(),
                    serde_json::Value::String(text) => text.clone(),
                    _ => value.to_string(),
                },
            })
            .collect()
    }

    fn buildToolExposureDeniedResult(
        invocation: &ToolInvocation,
        toolExposureMode: ToolExposureMode,
    ) -> Option<ToolResult> {
        let toolName = invocation.tool.name.trim();
        let denied = match toolExposureMode {
            ToolExposureMode::CLI if !Self::isCliPublicTool(toolName) => Some(format!(
                "{}",
                CliToolModeSupport::buildCliTopLevelRestrictionErrorMessage(
                    &Self::resolveDisplayToolName(&invocation.tool),
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
                Self::resolveDisplayToolName(&invocation.tool)
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
    fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool) -> ToolInvocationFuture<'a>;
}

/// Records the execution contract explicitly selected when a tool is registered.
pub enum RegisteredToolExecutor {
    Synchronous(Box<dyn ToolExecutor>),
    Asynchronous(Box<dyn AsyncToolExecutor>),
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
    /// Validates a request using its registered contract.
    pub fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        match self {
            Self::Synchronous(executor) => executor.validateParameters(tool),
            Self::Asynchronous(executor) => executor.validateParameters(tool),
        }
    }
    /// Returns the declared access boundary of the registered tool.
    pub fn accessSpec(&self, tool: &AITool) -> Result<ToolAccessSpec, String> {
        match self {
            Self::Synchronous(executor) => executor.accessSpec(tool),
            Self::Asynchronous(executor) => executor.accessSpec(tool),
        }
    }
    /// Dispatches exactly the contract registered for this tool, without probing or retrying.
    pub fn invokeAndStreamAsync<'a>(&'a mut self, tool: &'a AITool) -> ToolInvocationFuture<'a> {
        match self {
            Self::Synchronous(executor) => {
                Box::pin(std::future::ready(executor.invokeAndStream(tool)))
            }
            Self::Asynchronous(executor) => executor.invokeAndStreamAsync(tool),
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
