use std::cell::RefCell;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::api::chat::enhance::ConversationMarkupManager::{ConversationMarkupManager, ToolResult};
use crate::core::tools::climode::CliToolModeSupport::{
    CliToolModeSupport, PROXY_TOOL_NAME, SEARCH_TOOL_NAME,
};
use crate::core::tools::packTool::PackageManager::PackageManager;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::data::preferences::CharacterCardToolAccessResolver::{
    CharacterCardToolAccessResolver, ResolvedCharacterCardToolAccess,
};
use crate::util::ChatMarkupRegex::{attr_value, tag_ranges, ChatMarkupRegex};

const PACKAGE_PROXY_TOOL_NAME: &str = "package_proxy";
const CLI_PROXY_TOOL_NAME: &str = PROXY_TOOL_NAME;
const CLI_SEARCH_TOOL_NAME: &str = SEARCH_TOOL_NAME;
const PACKAGE_CALLER_NAME_PARAM: &str = "__operit_package_caller_name";
const PACKAGE_CHAT_ID_PARAM: &str = "__operit_package_chat_id";
const PACKAGE_CALLER_CARD_ID_PARAM: &str = "__operit_package_caller_card_id";

thread_local! {
    static TOOL_RUNTIME_CONTEXT: RefCell<Option<ToolRuntimeContext>> = RefCell::new(None);
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolExposureMode {
    FULL,
    CLI,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRuntimeContext {
    pub callerChatId: Option<String>,
    pub callerCardId: Option<String>,
    pub toolExposureMode: ToolExposureMode,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AITool {
    pub name: String,
    pub parameters: Vec<ToolParameter>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub tool: AITool,
    pub rawText: String,
    pub responseLocation: (usize, usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedToolTarget {
    tool: AITool,
    displayName: String,
}

pub struct ToolExecutionManager;

impl ToolExecutionManager {
    pub fn currentToolRuntimeContext() -> Option<ToolRuntimeContext> {
        TOOL_RUNTIME_CONTEXT.with(|value| value.borrow().clone())
    }

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
        invocations
    }

    pub fn executeToolSafely(
        invocation: &ToolInvocation,
        executor: &mut dyn ToolExecutor,
    ) -> Vec<ToolResult> {
        let validationResult = executor.validateParameters(&invocation.tool);
        if !validationResult.valid {
            return vec![ToolResult {
                toolName: invocation.tool.name.clone(),
                success: false,
                result: String::new(),
                error: Some(format!(
                    "Invalid parameters: {}",
                    validationResult.errorMessage
                )),
            }];
        }
        executor.invokeAndStream(&invocation.tool)
    }

    pub fn checkToolPermission(
        toolHandler: &AIToolHandler,
        invocation: &ToolInvocation,
        toolExposureMode: ToolExposureMode,
        roleCardToolAccess: Option<&ResolvedCharacterCardToolAccess>,
    ) -> (bool, Option<ToolResult>) {
        let resolvedTarget = Self::resolveToolTarget(&invocation.tool);
        let permissionTool = if toolExposureMode == ToolExposureMode::CLI
            && invocation.tool.name == CLI_PROXY_TOOL_NAME
        {
            invocation.tool.clone()
        } else {
            resolvedTarget.tool.clone()
        };

        if toolExposureMode == ToolExposureMode::CLI
            && (invocation.tool.name == CLI_SEARCH_TOOL_NAME
                || invocation.tool.name == CLI_PROXY_TOOL_NAME)
        {
            toolHandler.notifyToolPermissionChecked(&permissionTool, true, Some("CLI public tool"));
            return (true, None);
        }

        if let Some(access) = roleCardToolAccess {
            if access.customEnabled && !Self::isInvocationAllowedForRoleCard(invocation, access) {
                return (
                    false,
                    Some(ToolResult {
                        toolName: resolvedTarget.displayName,
                        success: false,
                        result: String::new(),
                        error: Some("Character card tool access denied.".to_string()),
                    }),
                );
            }
        }

        let hasPromptForPermission = !invocation.rawText.contains("deny_tool");
        if hasPromptForPermission {
            let toolPermissionSystem = toolHandler.getToolPermissionSystem();
            match toolPermissionSystem.checkToolPermission(&permissionTool) {
                Ok(true) => {
                    toolHandler.notifyToolPermissionChecked(&permissionTool, true, None);
                    return (true, None);
                }
                Ok(false) => {
                    let error = "User cancelled the tool execution.".to_string();
                    toolHandler.notifyToolPermissionChecked(&permissionTool, false, Some(&error));
                    return (
                        false,
                        Some(ToolResult {
                            toolName: resolvedTarget.displayName,
                            success: false,
                            result: String::new(),
                            error: Some(error),
                        }),
                    );
                }
                Err(error) => {
                    let message = error.to_string();
                    toolHandler.notifyToolPermissionChecked(&permissionTool, false, Some(&message));
                    return (
                        false,
                        Some(ToolResult {
                            toolName: resolvedTarget.displayName,
                            success: false,
                            result: String::new(),
                            error: Some(message),
                        }),
                    );
                }
            }
        }

        toolHandler.notifyToolPermissionChecked(
            &permissionTool,
            true,
            Some("Permission check bypassed by deny_tool tag."),
        );
        if invocation.rawText.contains("deny_tool") {
            return (true, None);
        }

        (true, None)
    }

    pub fn executeInvocations(
        invocations: &[ToolInvocation],
        toolHandler: &mut AIToolHandler,
        packageManager: &PackageManager,
        callerName: Option<String>,
        callerChatId: Option<String>,
        callerCardId: Option<String>,
        toolExposureMode: ToolExposureMode,
    ) -> (Vec<String>, Vec<ToolResult>) {
        let mut emitted = Vec::new();
        let mut results = Vec::new();
        toolHandler.registerDefaultTools();
        let roleCardToolAccess = CharacterCardToolAccessResolver::getInstance().resolve(
            callerCardId.as_deref(),
            packageManager,
            None,
        );
        let previousRuntimeContext = Self::currentToolRuntimeContext();
        TOOL_RUNTIME_CONTEXT.with(|value| {
            *value.borrow_mut() = Some(ToolRuntimeContext {
                callerChatId: callerChatId.clone(),
                callerCardId: callerCardId.clone(),
                toolExposureMode: toolExposureMode.clone(),
            });
        });
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
                    callerCardId.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        for invocation in injectedInvocations {
            if let Some(deniedResult) =
                Self::buildToolExposureDeniedResult(&invocation, toolExposureMode.clone())
            {
                toolHandler.notifyToolExecutionResult(&invocation.tool, &deniedResult);
                emitted.push(ensureEndsWithNewline(
                    &ConversationMarkupManager::formatToolResultForMessage(&deniedResult),
                ));
                results.push(deniedResult);
                continue;
            }

            if roleCardToolAccess.customEnabled
                && !Self::isInvocationAllowedForRoleCard(&invocation, &roleCardToolAccess)
            {
                let deniedResult = ToolResult {
                    toolName: Self::resolveToolTarget(&invocation.tool).displayName,
                    success: false,
                    result: String::new(),
                    error: Some("Character card tool access denied.".to_string()),
                };
                toolHandler.notifyToolExecutionResult(&invocation.tool, &deniedResult);
                emitted.push(ensureEndsWithNewline(
                    &ConversationMarkupManager::formatToolResultForMessage(&deniedResult),
                ));
                results.push(deniedResult);
                continue;
            }

            toolHandler.notifyToolCallRequested(&invocation.tool);
            let (hasPermission, errorResult) = Self::checkToolPermission(
                toolHandler,
                &invocation,
                toolExposureMode.clone(),
                Some(&roleCardToolAccess),
            );
            if !hasPermission {
                if let Some(deniedResult) = errorResult {
                    emitted.push(ensureEndsWithNewline(
                        &ConversationMarkupManager::formatToolResultForMessage(&deniedResult),
                    ));
                    results.push(deniedResult);
                }
                continue;
            }

            let resolved = Self::resolveToolTarget(&invocation.tool);
            let resolvedInvocation = ToolInvocation {
                tool: resolved.tool.clone(),
                rawText: invocation.rawText.clone(),
                responseLocation: invocation.responseLocation,
            };
            if !toolHandler.getToolExecutorOrActivate(&resolvedInvocation.tool.name) {
                let errorMessage = Self::buildToolNotAvailableErrorMessage(&resolved.tool.name);
                let content = ConversationMarkupManager::createToolNotAvailableError(
                    &resolved.tool.name,
                    Some(&errorMessage),
                );
                let deniedResult = ToolResult {
                    toolName: resolved.displayName,
                    success: false,
                    result: String::new(),
                    error: Some(errorMessage),
                };
                toolHandler.notifyToolExecutionResult(&invocation.tool, &deniedResult);
                emitted.push(ensureEndsWithNewline(&content));
                results.push(deniedResult);
                continue;
            }
            toolHandler.notifyToolExecutionStarted(&invocation.tool);
            let Some(collected) =
                toolHandler.executeToolSafelyWithResolvedExecutor(&resolvedInvocation.tool)
            else {
                toolHandler.notifyToolExecutionFinished(&invocation.tool);
                continue;
            };
            for result in &collected {
                toolHandler.notifyToolExecutionResult(&invocation.tool, result);
                emitted.push(ensureEndsWithNewline(
                    &ConversationMarkupManager::formatToolResultForMessage(result),
                ));
            }
            if collected.is_empty() {
                let emptyResult = ToolResult {
                    toolName: resolved.displayName,
                    success: false,
                    result: String::new(),
                    error: Some("The tool execution returned no results.".to_string()),
                };
                toolHandler.notifyToolExecutionResult(&invocation.tool, &emptyResult);
                results.push(emptyResult);
            } else {
                let last = collected.last().expect("collected not empty");
                let combinedResultString = collected
                    .iter()
                    .map(|item| {
                        if item.success {
                            item.result.trim().to_string()
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
                    toolName: resolved.displayName,
                    success: last.success,
                    result: combinedResultString,
                    error: last.error.clone(),
                };
                toolHandler.notifyToolExecutionResult(&invocation.tool, &finalResult);
                results.push(finalResult);
            }
            toolHandler.notifyToolExecutionFinished(&invocation.tool);
        }

        TOOL_RUNTIME_CONTEXT.with(|value| {
            *value.borrow_mut() = previousRuntimeContext;
        });
        (emitted, results)
    }

    fn ensureEndsWithNewline(content: &str) -> String {
        ensureEndsWithNewline(content)
    }

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

    fn addPackageContextParamIfMissing(
        params: &mut Vec<ToolParameter>,
        name: &str,
        value: Option<&str>,
    ) {
        let Some(value) = value else {
            return;
        };
        if value.trim().is_empty() || params.iter().any(|parameter| parameter.name == name) {
            return;
        }
        params.push(ToolParameter {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    fn injectPackageCallContext(
        invocation: &ToolInvocation,
        jsPackageNames: &BTreeSet<String>,
        callerName: Option<&str>,
        callerChatId: Option<&str>,
        callerCardId: Option<&str>,
    ) -> ToolInvocation {
        let resolvedTargetTool = Self::resolveToolTarget(&invocation.tool).tool;
        if !Self::isJsPackageTool(&resolvedTargetTool.name, jsPackageNames) {
            return invocation.clone();
        }

        let mut updatedParams = invocation.tool.parameters.clone();
        Self::addPackageContextParamIfMissing(
            &mut updatedParams,
            PACKAGE_CALLER_NAME_PARAM,
            callerName,
        );
        Self::addPackageContextParamIfMissing(
            &mut updatedParams,
            PACKAGE_CHAT_ID_PARAM,
            callerChatId,
        );
        Self::addPackageContextParamIfMissing(
            &mut updatedParams,
            PACKAGE_CALLER_CARD_ID_PARAM,
            callerCardId,
        );

        if updatedParams.len() == invocation.tool.parameters.len() {
            return invocation.clone();
        }

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

    fn isInvocationAllowedForRoleCard(
        invocation: &ToolInvocation,
        roleCardToolAccess: &ResolvedCharacterCardToolAccess,
    ) -> bool {
        let toolName = invocation.tool.name.trim();
        let resolvedTarget = Self::resolveToolTarget(&invocation.tool).tool;

        if toolName == CLI_SEARCH_TOOL_NAME {
            return true;
        }

        if toolName == CLI_PROXY_TOOL_NAME {
            return Self::isResolvedTargetAllowedForRoleCard(&resolvedTarget, roleCardToolAccess);
        }

        if toolName == "use_package" {
            if !roleCardToolAccess.isBuiltinToolAllowed("use_package") {
                return false;
            }
            let sourceName =
                Self::getParameterValue(&invocation.tool, "package_name").unwrap_or_default();
            return sourceName.is_empty()
                || roleCardToolAccess.isExternalSourceAllowed(&sourceName);
        }

        if toolName == PACKAGE_PROXY_TOOL_NAME {
            if !roleCardToolAccess.isBuiltinToolAllowed("package_proxy") {
                return false;
            }
            let resolvedTargetName = resolvedTarget.name.trim();
            if resolvedTargetName.is_empty() || !resolvedTargetName.contains(':') {
                return true;
            }
            return Self::isResolvedTargetAllowedForRoleCard(&resolvedTarget, roleCardToolAccess);
        }

        if toolName.contains(':') {
            let sourceName = toolName.split(':').next().unwrap_or("").trim();
            return sourceName.is_empty() || roleCardToolAccess.isExternalSourceAllowed(sourceName);
        }

        roleCardToolAccess.isBuiltinToolAllowed(toolName)
    }

    fn isResolvedTargetAllowedForRoleCard(
        resolvedTarget: &AITool,
        roleCardToolAccess: &ResolvedCharacterCardToolAccess,
    ) -> bool {
        let resolvedTargetName = resolvedTarget.name.trim();
        if resolvedTargetName.is_empty() {
            return true;
        }
        if resolvedTargetName == "use_package" {
            let sourceName =
                Self::getParameterValue(resolvedTarget, "package_name").unwrap_or_default();
            return sourceName.is_empty()
                || roleCardToolAccess.isExternalSourceAllowed(&sourceName);
        }
        if resolvedTargetName.contains(':') {
            let sourceName = resolvedTargetName.split(':').next().unwrap_or("").trim();
            return sourceName.is_empty() || roleCardToolAccess.isExternalSourceAllowed(sourceName);
        }
        roleCardToolAccess.isBuiltinToolAllowed(resolvedTargetName)
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
            result: String::new(),
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
    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolValidationResult {
    pub valid: bool,
    pub errorMessage: String,
}

fn ensureEndsWithNewline(content: &str) -> String {
    if content.ends_with('\n') {
        content.to_string()
    } else {
        format!("{content}\n")
    }
}
