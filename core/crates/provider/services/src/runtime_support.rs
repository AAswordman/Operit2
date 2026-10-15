use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Contains only JSON object properties in provider configuration requests and snapshots.
pub type JsonObject = serde_json::Map<String, Value>;

use operit_model::FunctionType::FunctionType;
use operit_model::ModelConfigData::{ProviderProfile, ResolvedModelConfig};
use operit_model::PromptFunctionType::PromptFunctionType;

/// Future returned by provider runtime continuation boundaries.
pub type ProviderRuntimeSupportFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Describes the model binding selected for one runtime function.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ProviderFunctionModelBinding {
    pub providerId: String,
    pub modelId: String,
}

/// Describes a ToolPkg-backed AI provider registration.
pub type ProviderToolPkgAiProviderRegistration =
    operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgAiProviderRegistration;

/// Captures a timestamp for provider request timing logs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderMessageTiming {
    pub startedAtMs: u64,
}

/// Describes a package surfaced in provider prompt composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderPackageInfo {
    pub name: String,
    pub description: String,
}

/// Distinguishes presentation-only resolution from an authenticated model, speech or tool execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatConfigurationPurpose {
    /// Resolves conversation presentation without selecting a model execution profile.
    Display,
    /// Resolves an authenticated participant and snapshot for an actual execution.
    Execution,
}

/// Requests one plugin-resolved chat configuration using opaque context and participant keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatConfigurationRequest {
    /// Must be present on the wire; presentation never selects an execution participant implicitly.
    pub purpose: ChatConfigurationPurpose,
    pub chatId: Option<String>,
    /// Contains this configuration owner's conversation extension, read from the canonical record.
    pub chatExtension: Option<JsonObject>,
    /// Contains the exact historical message revision snapshot supplied by the owning runtime.
    pub messageExtension: Option<JsonObject>,
    pub participantId: Option<String>,
    pub promptFunctionType: PromptFunctionType,
    pub defaultModelBinding: ProviderFunctionModelBinding,
    pub defaultTtsConfigId: String,
}

impl ChatConfigurationRequest {
    /// Rejects cross-purpose calls before reading records or invoking a registered API handler.
    pub fn requirePurpose(&self, purpose: ChatConfigurationPurpose) -> Result<(), String> {
        if self.purpose != purpose {
            return Err(format!("Chat configuration purpose mismatch: expected {purpose:?}, received {:?}", self.purpose));
        }
        if purpose == ChatConfigurationPurpose::Display
            && (self.participantId.is_some() || self.messageExtension.is_some()) {
            return Err("Display configuration cannot request an execution participant or message snapshot".to_string());
        }
        Ok(())
    }
}

/// Describes the plugin's generic conversation identity without selecting an execution profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatDisplayIdentity {
    pub title: String,
    pub avatarUri: Option<String>,
}

/// Describes one participant in the exact presentation order explicitly returned by the plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatDisplayParticipant {
    pub id: String,
    pub name: String,
    pub avatarUri: Option<String>,
}

/// Describes one explicit initial assistant message; a display response never requires an execution snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatInitialMessage {
    pub content: String,
    pub displayName: String,
    pub messageExtension: Option<JsonObject>,
}

/// Contains only display data and explicit ordered initialization messages, never an executable profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatConfigurationDisplayResult {
    pub contextKey: Option<String>,
    pub identity: Option<ChatDisplayIdentity>,
    pub participants: Vec<ChatDisplayParticipant>,
    pub initialMessages: Vec<ChatInitialMessage>,
    /// Identifies the catalog owner authenticated by the host, not a field accepted from plugin JSON.
    #[serde(skip)]
    pub extensionOwner: String,
}

impl ChatConfigurationDisplayResult {
    /// Validates presentation data independently of model, speech, tool bindings and execution snapshot requirements.
    pub fn validate(&self) -> Result<(), String> {
        if self.extensionOwner.trim().is_empty() {
            return Err("Display configuration extensionOwner is not authenticated".to_string());
        }
        match (&self.contextKey, &self.identity) {
            (None,None) => {
                if !self.participants.is_empty() || !self.initialMessages.is_empty() {
                    return Err("Unbound display configuration cannot declare participants or initial messages".to_string());
                }
            }
            (Some(key),Some(identity)) => {
                if key.trim().is_empty() || identity.title.trim().is_empty() {
                    return Err("Display configuration identity or contextKey is empty".to_string());
                }
            }
            _ => return Err("Display contextKey and identity must be bound together or explicitly null".to_string()),
        }
        let mut ids=std::collections::BTreeSet::new();
        for participant in &self.participants {
            if participant.id.trim().is_empty() || participant.name.trim().is_empty() {
                return Err("Display participant identity is empty".to_string());
            }
            if !ids.insert(&participant.id) {
                return Err(format!("Duplicate display participant: {}",participant.id));
            }
        }
        Ok(())
    }
}

/// Carries resource access already resolved by the owning plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatResourceRoute {
    pub key: String,
    pub readable: bool,
    pub writable: bool,
}

/// Describes explicit tool source policy without referring to a stored domain entity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatToolAccess {
    pub enabled: bool,
    pub allowedBuiltinTools: Vec<String>,
    pub allowedPackages: Vec<String>,
    pub allowedSkills: Vec<String>,
    pub allowedMcpServers: Vec<String>,
}

/// Supplies one complete execution identity, prompt and native configuration binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatParticipantProfile {
    pub id: String,
    pub name: String,
    pub avatarUri: Option<String>,
    pub introPrompt: String,
    pub userPreferencesText: String,
    pub openingStatement: String,
    pub modelBinding: ProviderFunctionModelBinding,
    pub ttsConfigId: String,
    pub toolAccess: ChatToolAccess,
    pub resources: Vec<ChatResourceRoute>,
}

/// Returns one execution participant and its plugin-owned conversation context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatConfigurationResult {
    pub contextKey: String,
    pub profile: ChatParticipantProfile,
    pub participants: Vec<ChatParticipantProfile>,
    /// Contains the plugin-authored opaque execution snapshot persisted with this generation.
    pub messageExtension: JsonObject,
    /// Identifies the actual registered owner injected by Rust after authenticated dispatch.
    #[serde(skip)]
    pub extensionOwner: String,
}

impl ChatConfigurationResult {
    /// Rejects malformed plugin descriptors before any model, speech or tool execution begins.
    pub fn validate(&self) -> Result<(), String> {
        if self.contextKey.trim().is_empty() {
            return Err("Chat configuration contextKey is empty".to_string());
        }
        if self.extensionOwner.trim().is_empty() {
            return Err("Chat configuration extensionOwner is not authenticated".to_string());
        }
        if self.participants.is_empty() {
            return Err("Chat configuration participants are empty".to_string());
        }
        if self.messageExtension.is_empty() {
            return Err("Chat execution message snapshot is empty".to_string());
        }
        let mut ids = std::collections::BTreeSet::new();
        for participant in &self.participants {
            if participant.id.trim().is_empty() || participant.name.trim().is_empty() {
                return Err("Chat participant identity is empty".to_string());
            }
            if !ids.insert(&participant.id) {
                return Err(format!("Duplicate chat participant: {}", participant.id));
            }
            if participant.modelBinding.providerId.trim().is_empty()
                || participant.modelBinding.modelId.trim().is_empty()
                || participant.ttsConfigId.trim().is_empty()
            {
                return Err(format!(
                    "Chat participant configuration is empty: {}",
                    participant.id
                ));
            }
            let mut routes = std::collections::BTreeSet::new();
            for route in &participant.resources {
                if route.key.trim().is_empty() || !routes.insert(&route.key) {
                    return Err(format!("Invalid chat resource route: {}", route.key));
                }
            }
        }
        if !self
            .participants
            .iter()
            .any(|participant| participant == &self.profile)
        {
            return Err("Selected chat profile is not an exact registered participant".to_string());
        }
        Ok(())
    }
}

/// Supplies runtime-owned data and plugin operations to provider code.
pub trait ProviderRuntimeSupport: Send + Sync {
    /// Returns the root directory used by provider runtime data.
    fn dataDir(&self) -> Result<PathBuf, String>;

    /// Returns the current thinking quality level.
    fn thinkingQualityLevel(&self) -> Result<i32, String>;

    /// Records provider/model token usage.
    fn updateTokensForProviderModel(
        &self,
        providerModel: &str,
        inputTokens: i64,
        outputTokens: i64,
        cachedInputTokens: i64,
    ) -> Result<(), String>;

    /// Returns whether profile-document descriptions are excluded from prompt composition.
    fn disableUserPreferenceDescription(&self) -> Result<bool, String>;

    /// Returns the application's explicit current speech configuration binding.
    fn defaultTtsConfigId(&self) -> Result<String, String>;

    /// Invokes the unique enabled chat configuration API through authenticated host dispatch.
    fn resolveChatConfiguration(
        &self,
        request: ChatConfigurationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ChatConfigurationResult, String>> + Send + '_>>;

    /// Returns deployed skill package descriptions for provider prompt composition.
    fn aiVisibleSkillPackages(&self) -> Result<Vec<ProviderPackageInfo>, String>;

    /// Returns the model binding for a function.
    fn modelBindingForFunction(
        &self,
        rootDir: PathBuf,
        functionType: FunctionType,
    ) -> Result<ProviderFunctionModelBinding, String>;

    /// Returns the resolved model config for a provider/model pair.
    fn resolvedModelConfig(
        &self,
        rootDir: PathBuf,
        providerId: &str,
        modelId: &str,
    ) -> Result<ResolvedModelConfig, String>;

    /// Returns the saved ChatGPT Codex session.
    fn loadCodexTokens(
        &self,
    ) -> Result<crate::chat::llmprovider::CodexOAuth::CodexOAuthTokens, String> {
        Err("Codex authorization is required".to_string())
    }

    /// Persists a refreshed ChatGPT Codex session.
    fn saveCodexTokens(
        &self,
        _tokens: crate::chat::llmprovider::CodexOAuth::CodexOAuthTokens,
    ) -> Result<(), String> {
        Err("Codex authorization cannot be saved".to_string())
    }

    /// Returns the provider profile for a provider id.
    fn providerProfile(
        &self,
        rootDir: PathBuf,
        providerId: &str,
    ) -> Result<ProviderProfile, String>;

    /// Returns whether a tool package AI provider is registered.
    fn hasToolPkgAiProvider(&self, providerId: &str) -> bool;

    /// Returns a tool package AI provider registration.
    fn toolPkgAiProvider(&self, providerId: &str) -> Option<ProviderToolPkgAiProviderRegistration>;

    /// Invokes a tool package AI provider function.
    fn runToolPkgAiProviderHook(
        &self,
        containerPackageName: &str,
        functionName: &str,
        functionSource: Option<&str>,
        event: &str,
        tag: Option<String>,
        sourceKey: Option<String>,
        eventPayload: Value,
        runtimeContextKey: Option<String>,
        executionKind: Option<String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send>>;

    /// Decodes a tool package hook result.
    fn decodeToolPkgHookResult(&self, raw: Option<String>) -> Option<Value>;

    /// Returns a provider timing snapshot.
    fn messageTimingNow(&self) -> ProviderMessageTiming;

    /// Writes a provider timing log entry.
    fn logMessageTiming(
        &self,
        stage: &str,
        startTimeMs: ProviderMessageTiming,
        details: Option<String>,
    );
}

/// Carries one runtime-specific provider support implementation.
#[derive(Clone)]
pub struct ProviderRuntimeContext {
    support: Arc<dyn ProviderRuntimeSupport>,
}

impl ProviderRuntimeContext {
    /// Creates a provider context from a caller-owned support implementation.
    pub fn new(support: Arc<dyn ProviderRuntimeSupport>) -> Self {
        Self { support }
    }

    /// Returns the runtime support implementation for this context.
    pub fn support(&self) -> &dyn ProviderRuntimeSupport {
        self.support.as_ref()
    }

    /// Clones the shared runtime support implementation.
    pub fn shared_support(&self) -> Arc<dyn ProviderRuntimeSupport> {
        self.support.clone()
    }
}

#[cfg(test)]
mod chat_configuration_descriptor_tests {
    use super::{
        ChatConfigurationDisplayResult, ChatConfigurationPurpose, ChatConfigurationRequest,
        ChatConfigurationResult, PromptFunctionType, ProviderFunctionModelBinding,
    };
    use serde_json::{json, Value};

    /// Builds a complete neutral fixture with different opaque context and participant identities.
    fn descriptor_json() -> Value {
        let profile = json!({
            "id": "participant-1", "name": "Participant", "avatarUri": null,
            "introPrompt": "intro", "userPreferencesText": "preferences", "openingStatement": "opening",
            "modelBinding": { "providerId": "provider", "modelId": "model" }, "ttsConfigId": "voice",
            "toolAccess": { "enabled": true, "allowedBuiltinTools": ["explicit"], "allowedPackages": [], "allowedSkills": [], "allowedMcpServers": [] },
            "resources": [{ "key": "opaque/resource", "readable": true, "writable": false }]
        });
        json!({ "contextKey": "opaque/conversation", "profile": profile, "participants": [profile], "messageExtension": { "actor": "participant-1", "voice": "voice", "profileRevision": "7" } })
    }

    /// Attaches an explicit native fixture owner before testing descriptor-specific validation errors.
    fn authenticated_descriptor(raw: Value) -> ChatConfigurationResult {
        let mut descriptor: ChatConfigurationResult = serde_json::from_value(raw).unwrap();
        descriptor.extensionOwner = "org.example.fixture".to_string();
        descriptor
    }

    /// Round-trips the full strict neutral descriptor without requiring domain prefixes.
    #[test]
    fn accepts_full_opaque_descriptor() {
        let raw = descriptor_json();
        let mut descriptor: ChatConfigurationResult = serde_json::from_value(raw.clone()).unwrap();
        assert!(descriptor.extensionOwner.is_empty());
        descriptor.extensionOwner = "org.example.fixture".to_string();
        descriptor.validate().unwrap();
        assert_eq!(serde_json::to_value(descriptor).unwrap(), raw);
    }

    /// Rejects missing or non-object execution snapshots before runtime owner injection.
    #[test]
    fn rejects_invalid_message_snapshots() {
        let mut raw = descriptor_json();
        raw.as_object_mut().unwrap().remove("messageExtension");
        assert!(serde_json::from_value::<ChatConfigurationResult>(raw).is_err());
        for value in [json!(null), json!([]), json!("snapshot")] {
            let mut raw = descriptor_json();
            raw["messageExtension"] = value;
            assert!(serde_json::from_value::<ChatConfigurationResult>(raw).is_err());
        }
    }

    /// Rejects descriptors that have not acquired an authenticated runtime owner.
    #[test]
    fn rejects_unauthenticated_execution_owner() {
        let descriptor: ChatConfigurationResult =
            serde_json::from_value(descriptor_json()).unwrap();
        assert!(descriptor.validate().is_err());
    }

    /// Rejects empty contexts, missing participants and duplicate execution identities.
    #[test]
    fn rejects_invalid_context_or_participants() {
        let mut raw = descriptor_json();
        raw["contextKey"] = json!(" ");
        assert!(authenticated_descriptor(raw).validate().is_err());
        let mut raw = descriptor_json();
        raw["participants"] = json!([]);
        assert!(authenticated_descriptor(raw).validate().is_err());
        let mut raw = descriptor_json();
        let profile = raw["profile"].clone();
        raw["participants"] = json!([profile, profile]);
        assert!(authenticated_descriptor(raw).validate().is_err());
    }

    /// Rejects a selected profile that is not an exact declared participant.
    #[test]
    fn rejects_mismatched_selected_profile() {
        let mut raw = descriptor_json();
        raw["profile"]["ttsConfigId"] = json!("another-voice");
        assert!(authenticated_descriptor(raw).validate().is_err());
    }

    /// Rejects empty native execution bindings and duplicate opaque resource routes.
    #[test]
    fn rejects_invalid_native_bindings_and_routes() {
        let mut raw = descriptor_json();
        raw["participants"][0]["modelBinding"]["modelId"] = json!("");
        assert!(authenticated_descriptor(raw).validate().is_err());
        let mut raw = descriptor_json();
        raw["participants"][0]["resources"] = json!([
            {"key": "opaque", "readable": true, "writable": false},
            {"key": "opaque", "readable": false, "writable": true}
        ]);
        assert!(authenticated_descriptor(raw).validate().is_err());
    }

    /// Serializes owner-scoped conversation and historical snapshots without a second association token.
    #[test]
    fn configuration_request_rejects_legacy_selection() {
        let chatExtension = json!({"selection":{"kind":"opaque","id":"identity"}})
            .as_object()
            .unwrap()
            .clone();
        let messageExtension = json!({"actor":"historical","voice":"stored-voice"})
            .as_object()
            .unwrap()
            .clone();
        let request = ChatConfigurationRequest {
            purpose: ChatConfigurationPurpose::Execution,
            chatId: Some("chat-a".to_string()),
            chatExtension: Some(chatExtension.clone()),
            messageExtension: Some(messageExtension.clone()),
            participantId: Some("participant-a".to_string()),
            promptFunctionType: PromptFunctionType::CHAT,
            defaultModelBinding: ProviderFunctionModelBinding {
                providerId: "provider".to_string(),
                modelId: "model".to_string(),
            },
            defaultTtsConfigId: "voice".to_string(),
        };
        let mut raw = serde_json::to_value(request).unwrap();
        let decoded: ChatConfigurationRequest = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(decoded.chatExtension, Some(chatExtension));
        assert_eq!(decoded.messageExtension, Some(messageExtension));
        assert!(raw.get("selection").is_none());
        raw["selection"] = json!("legacy-token");
        assert!(serde_json::from_value::<ChatConfigurationRequest>(raw).is_err());
    }

    /// Supplies display-only initialization data without any execution profile, model, voice or tool binding.
    fn display_json() -> Value {
        json!({"contextKey":"opaque/display", "identity":{"title":"Conversation","avatarUri":null},
            "participants":[{"id":"b","name":"Second","avatarUri":null},{"id":"a","name":"First","avatarUri":null}],
            "initialMessages":[{"content":"Explicit first","displayName":"Opening B","messageExtension":null},
                {"content":"Explicit second","displayName":"Opening A","messageExtension":{"opaque":"fresh"}}]})
    }

    /// Preserves plugin-declared participant and message order without requiring an execution snapshot.
    #[test]
    fn display_preserves_explicit_order_and_nullable_initial_snapshots() {
        let raw=display_json();
        let mut display: ChatConfigurationDisplayResult=serde_json::from_value(raw.clone()).unwrap();
        display.extensionOwner="arbitrary.owner".into();
        display.validate().unwrap();
        assert_eq!(display.participants[0].id,"b");
        assert_eq!(display.initialMessages[0].content,"Explicit first");
        assert!(display.initialMessages[0].messageExtension.is_none());
        assert_eq!(serde_json::to_value(&display).unwrap(),raw);
        assert!(serde_json::from_value::<ChatConfigurationResult>(raw).is_err());
        assert!(serde_json::from_value::<ChatConfigurationDisplayResult>(descriptor_json()).is_err());
    }

    /// Allows explicit unbound display and empty plugin-declared initialization lists without inventing defaults.
    #[test]
    fn display_accepts_explicit_unbound_and_empty_initialization() {
        for raw in [json!({"contextKey":null,"identity":null,"participants":[],"initialMessages":[]}),
            json!({"contextKey":"opaque","identity":{"title":"Empty","avatarUri":null},"participants":[],"initialMessages":[]})] {
            let mut display: ChatConfigurationDisplayResult=serde_json::from_value(raw).unwrap();
            assert!(display.validate().is_err());
            display.extensionOwner="native.owner".into();
            display.validate().unwrap();
        }
    }

    /// Rejects malformed display descriptors instead of accepting them as execution or unbound success.
    #[test]
    fn display_rejects_inconsistent_identity_and_duplicate_participants() {
        for (field,value) in [("contextKey",json!(null)),("identity",json!(null)),("contextKey",json!(" ")),
            ("participants",json!([{"id":"same","name":"A","avatarUri":null},{"id":"same","name":"B","avatarUri":null}]))] {
            let mut raw=display_json();raw[field]=value;
            let mut display: ChatConfigurationDisplayResult=serde_json::from_value(raw).unwrap();
            display.extensionOwner="native.owner".into();
            assert!(display.validate().is_err(),"{field}");
        }
        let mut raw=display_json();raw.as_object_mut().unwrap().remove("initialMessages");
        assert!(serde_json::from_value::<ChatConfigurationDisplayResult>(raw).is_err());
    }

    /// Requires an exact explicit purpose and rejects display calls carrying execution identities or snapshots.
    #[test]
    fn request_purpose_is_required_and_cross_purpose_calls_are_errors() {
        let raw=json!({"purpose":"display","chatId":"actual-chat","chatExtension":null,"messageExtension":null,
            "participantId":null,"promptFunctionType":"CHAT","defaultModelBinding":{"providerId":"provider","modelId":"model"},"defaultTtsConfigId":"voice"});
        let mut request: ChatConfigurationRequest=serde_json::from_value(raw.clone()).unwrap();
        request.requirePurpose(ChatConfigurationPurpose::Display).unwrap();
        assert!(request.requirePurpose(ChatConfigurationPurpose::Execution).is_err());
        request.participantId=Some("opaque".into());
        assert!(request.requirePurpose(ChatConfigurationPurpose::Display).is_err());
        request.participantId=None;request.messageExtension=Some(serde_json::Map::new());
        assert!(request.requirePurpose(ChatConfigurationPurpose::Display).is_err());
        let mut missing=raw.clone();missing.as_object_mut().unwrap().remove("purpose");
        assert!(serde_json::from_value::<ChatConfigurationRequest>(missing).is_err());
        let mut wrong=raw;wrong["purpose"]=json!("unknown");
        assert!(serde_json::from_value::<ChatConfigurationRequest>(wrong).is_err());
        let mut empty=authenticated_descriptor(descriptor_json());empty.messageExtension.clear();
        assert!(empty.validate().is_err());
    }

    /// Rejects unknown structured JSON fields before a configuration can reach production execution.
    #[test]
    fn rejects_unknown_descriptor_json_fields() {
        let mut raw = descriptor_json();
        raw["profile"]["unregisteredField"] = json!(true);
        assert!(serde_json::from_value::<ChatConfigurationResult>(raw).is_err());
    }
}
