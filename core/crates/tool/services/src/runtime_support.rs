use std::collections::{BTreeSet, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use operit_host_api::HostEnvironmentDescriptor;
use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_model::ToolPrompt::SystemToolPromptCategory;
use operit_plugin_sdk::javascript::JsExecutionProvider;

use crate::tools::packTool::RuntimePackageManager::RuntimePackageManager;

/// Future returned by runtime support async boundaries.
pub type ToolRuntimeSupportFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Carries the exact AI turn state required to continue after a Core route change.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
pub struct CoreRouteResumeContext {
    pub runtimeChatHistory: Vec<ChatMessage>,
    pub workspacePath: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub enableThinking: bool,
    /// Identifies the participant selected by the authenticated configuration descriptor.
    pub participantId: String,
    /// Identifies the registered configuration owner, injected solely by the runtime.
    pub extensionOwner: String,
    pub proxySenderName: Option<String>,
    pub notifyReplyOverride: Option<bool>,
    pub chatProviderIdOverride: Option<String>,
    pub chatModelIdOverride: Option<String>,
    pub turnOptions: ChatTurnOptions,
}

/// Handles one completed Core route change and its target-side continuation.
pub type CoreRouteChangeHandler = Arc<
    dyn Fn(
            String,
            String,
            CoreRouteResumeContext,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>>>>
        + Send
        + Sync,
>;

/// Bundled package asset exposed by the runtime crate.
#[derive(Clone, Copy)]
pub struct RuntimePluginAsset {
    pub name: &'static str,
    pub bytes: &'static [u8],
    pub sha256: &'static str,
}

/// Bundled external skill asset exposed by the runtime crate.
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
pub struct RuntimeBundledExternalSkillAsset {
    pub skillName: &'static str,
    pub path: &'static str,
    pub bytes: &'static [u8],
}

/// Cached MCP tool metadata exposed by the runtime crate.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CachedMcpToolInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "inputSchema", default)]
    pub inputSchema: String,
    #[serde(rename = "cachedAt", default)]
    pub cachedAt: i64,
}

/// Skill metadata needed by hidden tool catalog search.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeSkillCatalogEntry {
    pub name: String,
    pub description: String,
}

/// Operation supported by the structured file edit interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeStructuredEditAction {
    REPLACE,
    DELETE,
}

/// One structured file edit operation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct RuntimeStructuredEditOperation {
    pub action: RuntimeStructuredEditAction,
    pub oldContent: String,
    pub newContent: String,
}

/// Chat runtime slot selected by chat management tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeChatSlot {
    MAIN,
    FLOATING,
}

/// Contains native-generated correlation identities scoped to the authenticated calling package.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatTurnHandle {
    pub sequenceId: String,
    pub executionId: String,
    pub chatId: String,
}

/// Reports whether the exact native execution accepted a cancellation request.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatTurnCancelResultData {
    pub executionId: String,
    pub cancelRequested: bool,
}

/// Describes the native aggregate state rather than a plugin-local planner status.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatSequenceStatus {
    Completed,
    Cancelled,
    Failed,
}

/// Finalizes only the exact records accumulated by native commits in one admitted sequence.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatSequenceFinishResultData {
    pub sequenceId: String,
    pub chatId: String,
    pub status: ChatSequenceStatus,
    pub userMessageTimestamp: Option<i64>,
    pub assistants: Vec<operit_plugin_sdk::js_sdk::results::ChatCommittedMessage>,
    pub error: Option<String>,
}


/// Parameters for sending a chat message through the parent runtime.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct RuntimeChatSendRequest {
    pub slot: RuntimeChatSlot,
    pub participantId: Option<String>,
    pub chatId: Option<String>,
    pub message: String,
    pub proxySenderName: Option<String>,
    pub attachments: Vec<operit_model::AttachmentInfo::AttachmentInfo>,
    pub replyToMessageTimestamp: Option<i64>,
    pub turnOptions: ChatTurnOptions,
}

/// Carries one functional model request from the tool layer to the owning runtime.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct RuntimeChatCallRequest {
    pub functionType: operit_model::FunctionType::FunctionType,
    pub turns: Vec<operit_model::PromptTurn::PromptTurn>,
    pub recordTokenUsage: bool,
    pub enableThinking: bool,
}

/// Describes one device exposed to Core routing tools.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct RuntimeCoreNodeStatus {
    pub nodeId: String,
    pub displayName: String,
    pub userName: String,
    pub platform: String,
    pub model: String,
    pub reachable: bool,
    /// Whether this device can be selected to host Core execution.
    pub canExecute: bool,
    /// Peer Links one routed request traverses, reported only for multi-hop members.
    pub relayHops: Option<u32>,
    /// Hop node ids from the first hop through the member, reported only for multi-hop members.
    pub relayPath: Option<Vec<String>>,
}

/// Describes the current device and every member of its device space.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct RuntimeCoreNodeRouteState {
    pub currentNodeId: String,
    pub nodes: Vec<RuntimeCoreNodeStatus>,
}

/// Outcome of resolving one user-facing device reference against a route snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub enum CoreNodeReferenceMatch<'a> {
    /// Exactly one space member matched the reference.
    Resolved(&'a RuntimeCoreNodeStatus),
    /// No member matched the reference at all.
    Unknown,
    /// More than one member matched; callers must never guess a target.
    Ambiguous(Vec<&'a RuntimeCoreNodeStatus>),
}

impl RuntimeCoreNodeRouteState {
    /// Resolves one device reference to a single space member.
    ///
    /// An exact node id wins first. A device or user name may then match
    /// exactly, then case-insensitively. A node id prefix is accepted last and
    /// only when it is unique. Every stage that matches several members is
    /// reported as ambiguous instead of picking one.
    #[allow(non_snake_case)]
    pub fn resolveNodeReference<'a>(&'a self, reference: &str) -> CoreNodeReferenceMatch<'a> {
        let reference = reference.trim();
        if reference.is_empty() {
            return CoreNodeReferenceMatch::Unknown;
        }
        if let Some(node) = self.nodes.iter().find(|node| node.nodeId == reference) {
            return CoreNodeReferenceMatch::Resolved(node);
        }
        for nameMatch in [
            RuntimeCoreNodeNameMatch::Exact,
            RuntimeCoreNodeNameMatch::AsciiCaseInsensitive,
        ] {
            let matches = self
                .nodes
                .iter()
                .filter(|node| nameMatch.matches(node, reference))
                .collect::<Vec<_>>();
            match matches.len() {
                0 => continue,
                1 => return CoreNodeReferenceMatch::Resolved(matches[0]),
                _ => return CoreNodeReferenceMatch::Ambiguous(matches),
            }
        }
        let prefix = reference.to_ascii_lowercase();
        let matches = self
            .nodes
            .iter()
            .filter(|node| node.nodeId.to_ascii_lowercase().starts_with(&prefix))
            .collect::<Vec<_>>();
        match matches.len() {
            0 => CoreNodeReferenceMatch::Unknown,
            1 => CoreNodeReferenceMatch::Resolved(matches[0]),
            _ => CoreNodeReferenceMatch::Ambiguous(matches),
        }
    }
}

/// Outcome of resolving one path argument that may carry a `node:` prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub enum QualifiedVfsResolution<'a> {
    /// The path has no resolvable node prefix and stays on this device.
    LocalPath(&'a str),
    /// The prefix resolved to another space member; operations target `nodeId`.
    RemoteTarget {
        node: &'a RuntimeCoreNodeStatus,
        /// The reference exactly as typed, so echoed paths round-trip.
        pathPrefix: &'a str,
        innerPath: &'a str,
    },
}

/// Splits one `node:path` reference typed by a model or user into its node
/// prefix and inner path, without resolving the prefix against live state.
///
/// Returns `None` for every string that must keep local path semantics:
/// unqualified paths, prefixes containing separators, and single-letter
/// prefixes, which reserve the Windows drive syntax (`C:/Users`).
#[allow(non_snake_case)]
pub fn splitQualifiedVfsPath(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    let (prefix, innerPath) = text.split_once(':')?;
    if prefix.is_empty() || prefix.len() == 1 {
        return None;
    }
    if prefix.chars().any(|c| c == '/' || c == '\\') {
        return None;
    }
    Some((prefix, innerPath))
}

/// Formats every space member as a copyable candidate list.
#[allow(non_snake_case)]
fn formatSpaceMembers(state: &RuntimeCoreNodeRouteState) -> String {
    let members = state.nodes.iter().map(|node| coreNodeReferenceLabel(node)).collect::<Vec<_>>();
    if members.is_empty() {
        "the device space has no other members".to_string()
    } else {
        members.join(", ")
    }
}

impl RuntimeCoreNodeRouteState {
    /// Resolves one path argument against live route state.
    ///
    /// A prefix matching this device short-circuits to its inner local path.
    /// Unknown or ambiguous prefixes fail with the member candidate list
    /// instead of guessing; offline members still resolve so callers can
    /// report the device state explicitly.
    #[allow(non_snake_case)]
    pub fn resolveQualifiedVfsPath<'a>(
        &'a self,
        text: &'a str,
    ) -> Result<QualifiedVfsResolution<'a>, String> {
        let Some((prefix, innerPath)) = splitQualifiedVfsPath(text) else {
            return Ok(QualifiedVfsResolution::LocalPath(text.trim()));
        };
        match self.resolveNodeReference(prefix) {
            CoreNodeReferenceMatch::Resolved(node) => {
                if node.nodeId == self.currentNodeId {
                    return Ok(QualifiedVfsResolution::LocalPath(innerPath));
                }
                Ok(QualifiedVfsResolution::RemoteTarget {
                    node,
                    pathPrefix: prefix,
                    innerPath,
                })
            }
            CoreNodeReferenceMatch::Unknown => Err(format!(
                "'{prefix}' is not a member of the current device space; members: {}",
                formatSpaceMembers(self)
            )),
            CoreNodeReferenceMatch::Ambiguous(candidates) => Err(format!(
                "'{prefix}' matches multiple devices; candidates: {}",
                formatCoreNodeReferenceCandidates(&candidates)
            )),
        }
    }
}

/// Selects how one matching stage compares member names.
#[derive(Clone, Copy)]
enum RuntimeCoreNodeNameMatch {
    Exact,
    AsciiCaseInsensitive,
}

impl RuntimeCoreNodeNameMatch {
    /// Reports whether one member's display or user name matches the reference.
    fn matches(self, node: &RuntimeCoreNodeStatus, reference: &str) -> bool {
        [node.displayName.as_str(), node.userName.as_str()]
            .into_iter()
            .filter(|name| !name.trim().is_empty())
            .any(|name| match self {
                Self::Exact => name == reference,
                Self::AsciiCaseInsensitive => name.eq_ignore_ascii_case(reference),
            })
    }
}

/// Formats one member as a user-facing label that always carries its node id.
#[allow(non_snake_case)]
pub fn coreNodeReferenceLabel(node: &RuntimeCoreNodeStatus) -> String {
    let name = node.displayName.trim();
    if name.is_empty() || name == node.nodeId {
        node.nodeId.clone()
    } else {
        format!("{name} ({})", node.nodeId)
    }
}

/// Formats matched members as a copyable candidate list for tool errors.
#[allow(non_snake_case)]
pub fn formatCoreNodeReferenceCandidates(candidates: &[&RuntimeCoreNodeStatus]) -> String {
    candidates
        .iter()
        .map(|node| coreNodeReferenceLabel(node))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Supplies live Core routing state without coupling tools to the transport implementation.
pub trait CoreNodeToolRuntime: Send + Sync {
    /// Returns one consistent snapshot of current device reachability.
    #[allow(non_snake_case)]
    fn coreNodeRouteState(&self) -> Result<RuntimeCoreNodeRouteState, String>;
}

/// Provides runtime-owned services that the tools crate must not own.
pub trait ToolRuntimeSupport: Send + Sync {
    fn bindDeviceToolRuntime(&self, _runtime: Arc<dyn crate::tools::DeviceTools::DeviceToolRuntime>) -> Result<(), String> {
        Err("Device tool routing is not supported by this runtime".into())
    }
    fn closeDeviceTransports(&self, _owner: &str) {}

    fn executeDeviceTool(&self, _request: operit_plugin_sdk::javascript::JsToolCallRequest)
        -> operit_plugin_sdk::javascript::JsExecutionCompletion<operit_plugin_sdk::javascript::JsToolCallResult> {
        Box::pin(async { crate::tools::DeviceTools::failure("Device tool routing is not initialized") })
    }

    /// Installs the live Core routing capability owned by the outer proxy.
    #[allow(non_snake_case)]
    fn bindCoreNodeToolRuntime(&self, runtime: Arc<dyn CoreNodeToolRuntime>) -> Result<(), String>;

    /// Returns live device reachability for Core routing tools.
    #[allow(non_snake_case)]
    fn coreNodeRouteState(&self) -> Result<RuntimeCoreNodeRouteState, String>;

    /// Installs the runtime-owned route change controller.
    #[allow(non_snake_case)]
    fn bindCoreRouteChangeHandler(&self, handler: CoreRouteChangeHandler) -> Result<(), String>;

    /// Requests a route change after the current AI round has been committed.
    #[allow(non_snake_case)]
    fn requestCoreRouteChange<'a>(
        &'a self,
        chatId: String,
        targetNodeId: String,
        resumeContext: CoreRouteResumeContext,
    ) -> ToolRuntimeSupportFuture<'a, Result<(), String>>;

    /// Reads complete configured model summaries directly from the canonical configuration manager.
    #[allow(non_snake_case)]
    fn listModelSummaries(
        &self,
    ) -> Result<Vec<operit_plugin_sdk::js_sdk::models::ModelsSummary>, String>;
    /// Synthesizes encoded audio through the configured provider on this host.
    fn synthesizeSpeech(&self, _request: operit_plugin_sdk::js_sdk::tts::TtsSynthesizeRequest)
        -> Result<operit_plugin_sdk::js_sdk::results::TtsSynthesizeResultData, String> {
        Err("Speech synthesis is unavailable on this host".into())
    }
    /// Starts system speech through this runtime's shared playback host.
    #[allow(non_snake_case)]
    fn speakSpeech(&self, request: operit_plugin_sdk::js_sdk::tts::TtsSpeakRequest)
        -> Result<operit_plugin_sdk::js_sdk::results::TtsSpeakResultData, String>;
    /// Recognizes supplied audio through the configured provider on this host.
    fn transcribeSpeech(&self, _request: operit_plugin_sdk::js_sdk::stt::SttTranscribeRequest)
        -> Result<operit_plugin_sdk::js_sdk::results::SttTranscribeResultData, String> {
        Err("Speech recognition is unavailable on this host".into())
    }
    /// Reads complete configured speech records directly from the canonical configuration manager.
    #[allow(non_snake_case)]
    fn listTtsConfigs(
        &self,
    ) -> Result<Vec<operit_plugin_sdk::js_sdk::tts::TtsConfig>, String>;
    /// Lists complete independent named appearance snapshots from the ordinary canonical preference manager.
    #[allow(non_snake_case)]
    fn listThemeConfigs(&self) -> Result<Vec<operit_plugin_sdk::js_sdk::theme::ThemeConfig>, String>;
    /// Applies one exact existing named appearance snapshot and its active ID in the canonical transaction.
    #[allow(non_snake_case)]
    fn applyThemeConfig(&self, id: String) -> Result<operit_plugin_sdk::js_sdk::theme::ThemeConfig, String>;
    /// Reads the actual current canonical speech ID without resolving any plugin-owned usage.
    #[allow(non_snake_case)]
    fn getCurrentTtsConfigId(&self) -> Result<String, String>;
    /// Selects only the exact existing canonical speech ID and propagates validation or persistence errors.
    #[allow(non_snake_case)]
    fn setCurrentTtsConfigId(&self, id: String) -> Result<String, String>;
    /// Filters the hidden model-facing catalog using actual registered tool-prompt hooks and execution context.
    #[allow(non_snake_case)]
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
    >;

    /// Reads actual tool source registries after the one initial registry load has completed.
    #[allow(non_snake_case)]
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
    >;

    /// Reads one stored environment variable.
    #[allow(non_snake_case)]
    fn readEnvironmentVariable(&self, packageName: &str, key: &str) -> Result<Option<String>, String>;

    /// Writes one stored environment variable.
    #[allow(non_snake_case)]
    fn writeEnvironmentVariable(&self, packageName: &str, key: &str, value: &str) -> Result<(), String>;

    /// Removes one stored environment variable.
    #[allow(non_snake_case)]
    fn removeEnvironmentVariable(&self, packageName: &str, key: &str) -> Result<(), String>;

    /// Returns built-in and internal tool prompt categories for hidden catalog search.
    #[allow(non_snake_case)]
    fn buildBuiltinAndInternalCategories(
        &self,
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> Vec<SystemToolPromptCategory>;

    /// Returns AI-visible built-in tool names for hidden catalog source labeling.
    #[allow(non_snake_case)]
    fn buildBuiltinToolNameSet(
        &self,
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> BTreeSet<String>;

    /// Returns AI-visible skill package metadata.
    #[allow(non_snake_case)]
    fn aiVisibleSkillPackages(&self) -> Vec<RuntimeSkillCatalogEntry>;

    /// Returns cached MCP tool descriptions for a server.
    #[allow(non_snake_case)]
    fn cachedMcpTools(&self, serverName: &str) -> Vec<CachedMcpToolInfo>;

    /// Returns built-in package assets owned by the runtime.
    #[allow(non_snake_case)]
    fn builtinPluginAssets(&self) -> &'static [RuntimePluginAsset];

    /// Returns bundled external package assets owned by the runtime.
    #[allow(non_snake_case)]
    fn bundledExternalPluginAssets(&self) -> &'static [RuntimePluginAsset];

    /// Returns bundled external skill assets owned by the runtime.
    #[allow(non_snake_case)]
    fn bundledExternalSkillAssets(&self) -> &'static [RuntimeBundledExternalSkillAsset];

    /// Returns whether a skill is visible to AI package activation.
    #[allow(non_snake_case)]
    fn isSkillVisibleToAi(&self, skillName: &str) -> bool;

    /// Updates AI visibility for a skill package.
    #[allow(non_snake_case)]
    fn setSkillVisibleToAi(&self, skillName: &str, visible: bool) -> Result<(), String>;

    /// Generates an MCP plugin description using parent-owned model services.
    #[allow(non_snake_case)]
    fn generateMcpPluginDescription<'a>(
        &'a self,
        pluginName: &'a str,
        toolDescriptions: &'a [String],
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>>;

    /// Starts parent-owned chat services.
    #[allow(non_snake_case)]
    fn startChatServices(&self) -> Result<(), String>;

    /// Stops parent-owned chat services.
    #[allow(non_snake_case)]
    fn stopChatServices(&self) -> Result<(), String>;

    /// Returns whether the requested chat is currently processing.
    #[allow(non_snake_case)]
    fn isChatProcessing(&self, chatId: &str) -> Result<bool, String>;

    /// Switches the parent-owned main chat runtime to a chat id.
    #[allow(non_snake_case)]
    fn switchMainChat<'a>(
        &'a self,
        chatId: &'a str,
    ) -> ToolRuntimeSupportFuture<'a, Result<(), String>>;

    /// Creates a workspace conversation with explicit source and opaque creation input.
    #[allow(non_snake_case)]
    fn createChatRuntime<'a>(
        &'a self,
        setAsCurrentChat: bool,
        sourceChatId: Option<String>,
        parentChatId: Option<String>,
        input: Option<serde_json::Value>,
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>>;

    /// Reads one authenticated owner's object extension directly from an existing record.
    fn readChatExtension(
        &self,
        owner: &str,
        target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
    ) -> Result<Option<operit_plugin_sdk::js_sdk::core::JsonObject>, String>;

    /// Replaces one authenticated owner's object extension directly on an existing record.
    fn writeChatExtension(
        &self,
        owner: &str,
        target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
        value: operit_plugin_sdk::js_sdk::core::JsonObject,
    ) -> Result<operit_plugin_sdk::js_sdk::core::JsonObject, String>;

    /// Deletes one authenticated owner's extension directly from an existing record.
    fn deleteChatExtension(
        &self,
        owner: &str,
        target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
    ) -> Result<bool, String>;

    /// Admits one authenticated message and returns its bounded semantic observation before generation awaits.
    fn openPluginChatMessage(
        &self, owner: &str, request: operit_plugin_sdk::js_sdk::chat::ChatSendRequest, observeParts: bool,
    ) -> Result<operit_plugin_sdk::js_sdk::JsAsyncIterable<operit_plugin_sdk::js_sdk::chat::ChatSendEvent>, String>;

    /// Captures and requests cancellation of this owner's current execution before any asynchronous model access.
    fn requestPluginChatCancellation(
        &self, owner: &str, chatId: &str,
    ) -> Result<operit_plugin_sdk::js_sdk::chat::ChatCancelResult, String>;

    /// Sends a message through the parent-owned chat runtime.
    #[allow(non_snake_case)]
    fn sendChatMessage<'a>(
        &'a self,
        request: RuntimeChatSendRequest,
    ) -> ToolRuntimeSupportFuture<'a, Result<operit_plugin_sdk::js_sdk::results::MessageSendResultData, String>>;

    /// Calls a functional model without adding messages to chat history.
    #[allow(non_snake_case)]
    fn callChatModel<'a>(
        &'a self,
        request: RuntimeChatCallRequest,
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>>;

    /// Applies structured edits to file content.
    #[allow(non_snake_case)]
    fn processFileBindingOperations(
        &self,
        originalContent: &str,
        operations: &[RuntimeStructuredEditOperation],
    ) -> (String, String);

    /// Generates a unified diff for file apply results.
    #[allow(non_snake_case)]
    fn generateUnifiedDiff(&self, oldContent: &str, newContent: &str) -> String;
}

/// Carries all runtime-specific dependencies required by one tool runtime.
#[derive(Clone)]
pub struct ToolRuntimeDependencies {
    runtime_support: Arc<dyn ToolRuntimeSupport>,
    js_execution_provider: Arc<dyn JsExecutionProvider>,
}

impl ToolRuntimeDependencies {
    /// Creates one tool runtime dependency set.
    pub fn new(
        runtime_support: Arc<dyn ToolRuntimeSupport>,
        js_execution_provider: Arc<dyn JsExecutionProvider>,
    ) -> Self {
        Self {
            runtime_support,
            js_execution_provider,
        }
    }

    /// Returns the runtime support implementation for this tool runtime.
    pub fn runtime_support(&self) -> &dyn ToolRuntimeSupport {
        self.runtime_support.as_ref()
    }

    /// Clones the shared runtime support implementation.
    pub fn shared_runtime_support(&self) -> Arc<dyn ToolRuntimeSupport> {
        self.runtime_support.clone()
    }

    /// Returns the JavaScript execution provider for this tool runtime.
    pub fn js_execution_provider(&self) -> &dyn JsExecutionProvider {
        self.js_execution_provider.as_ref()
    }

    /// Clones the JavaScript execution provider for this tool runtime.
    pub fn shared_js_execution_provider(&self) -> Arc<dyn JsExecutionProvider> {
        self.js_execution_provider.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(nodeId: &str, displayName: &str, userName: &str, reachable: bool) -> RuntimeCoreNodeStatus {
        RuntimeCoreNodeStatus {
            nodeId: nodeId.to_string(),
            displayName: displayName.to_string(),
            userName: userName.to_string(),
            platform: "android".to_string(),
            model: "M2007J1SC".to_string(),
            reachable,
            canExecute: true,
            relayHops: None,
            relayPath: None,
        }
    }

    fn state(nodes: Vec<RuntimeCoreNodeStatus>) -> RuntimeCoreNodeRouteState {
        RuntimeCoreNodeRouteState {
            currentNodeId: nodes.first().map(|node| node.nodeId.clone()).unwrap_or_default(),
            nodes,
        }
    }

    fn resolvedNodeId(matchResult: CoreNodeReferenceMatch<'_>) -> String {
        match matchResult {
            CoreNodeReferenceMatch::Resolved(node) => node.nodeId.clone(),
            other => panic!("expected a resolved node, got {other:?}"),
        }
    }

    /// An exact node id wins even when another member is named like it.
    #[test]
    fn exactNodeIdOutranksNamesAndPrefixes() {
        let state = state(vec![
            status("core-1-cas", "cas", "harmoon", true),
            status("core-2-device", "core-1-cas", "harmoon", true),
        ]);
        assert_eq!(resolvedNodeId(state.resolveNodeReference("core-1-cas")), "core-1-cas");
    }

    /// Device and user names resolve exactly, then case-insensitively.
    #[test]
    fn deviceAndUserNamesResolveThroughEveryExactStage() {
        let state = state(vec![
            status("core-1-cas", "cas", "harmoon", true),
            status("core-2-elish", "Elish", "lin", true),
        ]);
        assert_eq!(resolvedNodeId(state.resolveNodeReference("cas")), "core-1-cas");
        assert_eq!(resolvedNodeId(state.resolveNodeReference("elish")), "core-2-elish");
        assert_eq!(resolvedNodeId(state.resolveNodeReference("  cas  ")), "core-1-cas");
        assert_eq!(resolvedNodeId(state.resolveNodeReference("lin")), "core-2-elish");
    }

    /// A unique node id prefix resolves, including the timestamp-only form.
    #[test]
    fn uniqueNodeIdPrefixesResolve() {
        let state = state(vec![
            status("core-1791589326795-13765763d4b74d4ab76648f26ee874be", "cas", "harmoon", true),
            status("core-1791589400000-58f0d4ee0efc47f59a1f1b4d514b7af0", "elish", "lin", true),
        ]);
        assert_eq!(
            resolvedNodeId(state.resolveNodeReference("core-1791589326795")),
            "core-1791589326795-13765763d4b74d4ab76648f26ee874be"
        );
        assert_eq!(
            resolvedNodeId(state.resolveNodeReference("CORE-17915894")),
            "core-1791589400000-58f0d4ee0efc47f59a1f1b4d514b7af0"
        );
    }

    /// Repeated names and prefixes never pick a target by accident, while an
    /// exactly spelled name still outranks its case-insensitive neighbours.
    #[test]
    fn ambiguousReferencesReportEveryCandidate() {
        let state = state(vec![
            status("core-1", "cas", "harmoon", true),
            status("core-2", "cas", "harmoon", true),
            status("core-3", "Mac", "lin", true),
            status("core-4", "mac", "lin", true),
        ]);
        for reference in ["cas", "harmoon", "MAC", "core-"] {
            match state.resolveNodeReference(reference) {
                CoreNodeReferenceMatch::Ambiguous(candidates) => {
                    assert!(candidates.len() >= 2, "{reference}");
                }
                other => panic!("ambiguous reference {reference} resolved as {other:?}"),
            }
        }
        assert_eq!(resolvedNodeId(state.resolveNodeReference("mac")), "core-4");
        match state.resolveNodeReference("cas") {
            CoreNodeReferenceMatch::Ambiguous(candidates) => {
                assert_eq!(
                    formatCoreNodeReferenceCandidates(&candidates),
                    "cas (core-1), cas (core-2)"
                );
            }
            other => panic!("expected candidates, got {other:?}"),
        }
    }

    /// Unknown, empty, and name-less members stay unresolved instead of guessing.
    #[test]
    fn unknownAndEmptyReferencesStayUnknown() {
        let state = state(vec![
            status("core-1", "cas", "", true),
            status("core-2", "", "", true),
        ]);
        for reference in ["", "   ", "linux", "core-9"] {
            assert_eq!(state.resolveNodeReference(reference), CoreNodeReferenceMatch::Unknown, "{reference}");
        }
    }

    /// Labels always carry the stable node id for copyable error messages.
    #[test]
    fn referenceLabelsFallBackToTheNodeId() {
        assert_eq!(coreNodeReferenceLabel(&status("core-1", "cas", "", true)), "cas (core-1)");
        assert_eq!(coreNodeReferenceLabel(&status("core-1", "", "", true)), "core-1");
        assert_eq!(coreNodeReferenceLabel(&status("core-1", "core-1", "", true)), "core-1");
    }

    /// Only bare prefixes without separators split as qualified paths; local
    /// VFS paths and Windows drive syntax keep local semantics.
    #[test]
    fn qualifiedPathSplittingKeepsLocalPathsIntact() {
        assert_eq!(splitQualifiedVfsPath("cas:/mnt/nas/data"), Some(("cas", "/mnt/nas/data")));
        assert_eq!(splitQualifiedVfsPath("cas:C:/Users/report.docx"), Some(("cas", "C:/Users/report.docx")));
        assert_eq!(splitQualifiedVfsPath("  cas:x/y "), Some(("cas", "x/y")));
        // Display names may contain spaces; the colon split runs first.
        assert_eq!(
            splitQualifiedVfsPath("EliteMini Series:/Users/x"),
            Some(("EliteMini Series", "/Users/x"))
        );
        for local in ["/app/workspaces/chat-a", "/mnt/linux/home", "", "relative/path"] {
            assert_eq!(splitQualifiedVfsPath(local), None, "{local}");
        }
        // Single-letter prefixes reserve the Windows drive syntax.
        assert_eq!(splitQualifiedVfsPath("C:/Users"), None);
        assert_eq!(splitQualifiedVfsPath("c:data"), None);
        // Prefixes with separators are path text, not node references.
        assert_eq!(splitQualifiedVfsPath("/mnt/linux/foo:bar"), None);
        assert_eq!(splitQualifiedVfsPath(":no-prefix"), None);
    }

    /// Qualified paths resolve through the shared node reference stages and
    /// never guess on unknown or ambiguous prefixes.
    #[test]
    fn qualifiedPathResolutionMatchesSpaceMembers() {
        let state = state(vec![
            status("core-2-local", "desk", "harmoon", true),
            status("core-1-cas", "cas", "harmoon", true),
        ]);
        match state.resolveQualifiedVfsPath("cas:/mnt/nas/data").unwrap() {
            QualifiedVfsResolution::RemoteTarget { node, pathPrefix, innerPath } => {
                assert_eq!(node.nodeId, "core-1-cas");
                assert_eq!(pathPrefix, "cas");
                assert_eq!(innerPath, "/mnt/nas/data");
            }
            other => panic!("expected a remote target, got {other:?}"),
        }
        // This device's own name short-circuits to the inner local path.
        match state.resolveQualifiedVfsPath("desk:/mnt/linux/home").unwrap() {
            QualifiedVfsResolution::LocalPath(inner) => assert_eq!(inner, "/mnt/linux/home"),
            other => panic!("expected a local path, got {other:?}"),
        }
        // The exact node id and a unique id prefix resolve like switch_core.
        match state.resolveQualifiedVfsPath("core-1:/sdcard/Download").unwrap() {
            QualifiedVfsResolution::RemoteTarget { node, .. } => assert_eq!(node.nodeId, "core-1-cas"),
            other => panic!("expected a remote target, got {other:?}"),
        }
        // Unqualified input passes through unchanged.
        assert_eq!(
            state.resolveQualifiedVfsPath("/app/workspaces/chat-a").unwrap(),
            QualifiedVfsResolution::LocalPath("/app/workspaces/chat-a")
        );
    }

    /// Unknown and ambiguous path prefixes fail with candidate lists.
    #[test]
    fn qualifiedPathFailuresListCandidates() {
        let state = state(vec![
            status("core-1", "cas", "harmoon", true),
            status("core-2", "cas", "harmoon", true),
        ]);
        let unknown = state.resolveQualifiedVfsPath("lighthouse:/mnt/nas").unwrap_err();
        assert!(unknown.contains("'lighthouse' is not a member"), "{unknown}");
        assert!(unknown.contains("cas (core-1)"), "{unknown}");
        let ambiguous = state.resolveQualifiedVfsPath("harmoon:/sdcard").unwrap_err();
        assert!(ambiguous.contains("matches multiple devices"), "{ambiguous}");
        assert!(ambiguous.contains("cas (core-1), cas (core-2)"), "{ambiguous}");
    }

    /// Offline members still resolve so callers can report the device state
    /// explicitly instead of treating the device as unknown.
    #[test]
    fn offlineMembersResolveForExplicitOfflineErrors() {
        let state = state(vec![
            status("core-1-local", "desk", "harmoon", true),
            status("core-2-off", "phone", "harmoon", false),
        ]);
        match state.resolveQualifiedVfsPath("phone:/sdcard/Download").unwrap() {
            QualifiedVfsResolution::RemoteTarget { node, .. } => assert!(!node.reachable),
            other => panic!("expected a remote target, got {other:?}"),
        }
    }
}
