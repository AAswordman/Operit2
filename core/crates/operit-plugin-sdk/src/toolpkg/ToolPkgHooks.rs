use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

/// Registration for one application lifecycle hook.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct ToolPkgAppLifecycleHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub event: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

/// Registration for one message processing plugin.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct ToolPkgMessageProcessingHookRegistration {
    pub containerPackageName: String,
    pub pluginId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

/// Registration for one XML render plugin.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct ToolPkgXmlRenderHookRegistration {
    pub containerPackageName: String,
    pub pluginId: String,
    pub tag: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

/// Registration for one input-menu toggle plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct ToolPkgInputMenuToggleHookRegistration {
    pub containerPackageName: String,
    pub pluginId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

/// Registration for one chat-input hook.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct ToolPkgChatInputHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

/// Registration for one chat-view hook.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct ToolPkgChatViewHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

/// Registration for one tool lifecycle hook.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct ToolPkgToolLifecycleHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

/// Registration for one prompt or summary hook.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct ToolPkgPromptHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

/// Registration for one ToolPkg-backed AI provider.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct ToolPkgAiProviderRegistration {
    pub containerPackageName: String,
    pub providerId: String,
    pub displayName: String,
    pub description: String,
    pub listModelsFunctionName: String,
    pub listModelsFunctionSource: Option<String>,
    pub sendMessageFunctionName: String,
    pub sendMessageFunctionSource: Option<String>,
    pub testConnectionFunctionName: String,
    pub testConnectionFunctionSource: Option<String>,
    pub calculateInputTokensFunctionName: String,
    pub calculateInputTokensFunctionSource: Option<String>,
}

/// Registration for one host-originated event hook.
#[derive(Clone, Debug, serde::Serialize)]
#[allow(non_snake_case)]
pub struct ToolPkgHostEventRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub source: String,
    pub trigger: Value,
    pub functionName: String,
    pub functionSource: Option<String>,
    pub enabled: bool,
}

/// Complete input required to invoke one ToolPkg hook.
#[derive(Clone)]
#[allow(non_snake_case)]
pub struct ToolPkgHookInvocation {
    pub containerPackageName: String,
    pub functionName: String,
    pub event: String,
    pub eventName: Option<String>,
    pub pluginId: Option<String>,
    pub inlineFunctionSource: Option<String>,
    pub eventPayload: Value,
    pub executionContextKey: Option<String>,
    pub runtimeKind: Option<String>,
    pub envOverrides: BTreeMap<String, String>,
    pub timestampMs: i64,
    pub timeoutSec: u64,
    pub dispatchIntermediateOnMain: bool,
    pub onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

/// Dispatches ToolPkg hooks through an SDK-owned package manager.
pub trait ToolPkgHookDispatcher: Send + Sync {
    /// Invokes one ToolPkg hook for the supplied enabled package set.
    #[allow(non_snake_case)]
    fn dispatchToolPkgHook(
        &self,
        enabledPackageNames: &[String],
        invocation: ToolPkgHookInvocation,
    ) -> Result<Option<String>, String>;
}

/// Decodes a hook output as JSON when the output contains valid JSON.
#[allow(non_snake_case)]
pub fn decodeToolPkgHookResult(raw: Option<String>) -> Option<Value> {
    let text = raw?;
    let normalized = text.trim();
    if normalized.is_empty() {
        return Some(Value::String(text));
    }
    match serde_json::from_str::<Value>(normalized) {
        Ok(value) => Some(value),
        Err(_) => Some(Value::String(text)),
    }
}
