use std::collections::BTreeMap;
use std::sync::Mutex;

use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_LIFECYCLE;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgChatLifecycleHookRegistration;
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgContainerRuntime;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::data::preferences::ApiPreferences::ApiPreferences;
use super::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use super::ToolPkgPreHookTimeout::ToolPkgPreHookTimeout;

// OperitApplication is the process-wide owner; main/floating/detached chat slots share its runtime.
// A successful later application installation explicitly replaces the prior owner for restart, not hook registration.
static CHAT_LIFECYCLE_RUNTIME: Mutex<Option<ToolPkgBridgeRuntime>> = Mutex::new(None);

/// Identifies the sole supported creation lifecycle stage.
pub const CHAT_LIFECYCLE_BEFORE_CREATE: &str = "before_create";

/// Distinguishes a new conversation draft from a branch of an existing source message.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolPkgChatCreationKind {
    /// Creates a new conversation.
    New,
    /// Creates a branch conversation.
    Branch,
}

/// Carries only generic allocated draft identity before configuration resolution or persistence.
#[derive(Clone, Debug, Serialize)]
#[allow(non_snake_case)]
pub struct ToolPkgChatCreationChat {
    pub id: String,
    pub title: String,
    pub workspaceId: Option<String>,
    pub parentChatId: Option<String>,
}

/// Supplies explicit draft context without any character, group, memory or complete extension-map field.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct ToolPkgChatCreationDraft {
    pub creationKind: ToolPkgChatCreationKind,
    pub chat: ToolPkgChatCreationChat,
    pub sourceChatId: Option<String>,
    pub sourceMessageTimestamp: Option<i64>,
    pub input: Option<Map<String, Value>>,
}

/// Owns a detached enabled-runtime snapshot with no package-manager mutex guard retained across await.
pub struct ToolPkgChatLifecycleDispatchContext {
    manager: RuntimePackageManager,
    hooks: Vec<ToolPkgChatLifecycleHookRegistration>,
}

/// Executes registered creation handlers before the caller resolves configuration or commits its chat transaction.
pub struct ToolPkgChatLifecycleHookBridge;

impl ToolPkgChatLifecycleHookBridge {
    /// Explicitly binds the process-owned application runtime, including a legitimate replacement on application restart.
    pub fn register(runtime: ToolPkgBridgeRuntime) -> Result<(), String> {
        let mut current = CHAT_LIFECYCLE_RUNTIME.lock()
            .map_err(|error| format!("Chat lifecycle runtime registration lock failed: {error}"))?;
        *current = Some(runtime);
        Ok(())
    }

    /// Awaits the canonical ready snapshot outside every runtime and package-manager lock before selecting hooks.
    pub async fn snapshot() -> Result<ToolPkgChatLifecycleDispatchContext, String> {
        let runtime = {
            let current = CHAT_LIFECYCLE_RUNTIME.lock()
                .map_err(|error| format!("Chat lifecycle runtime snapshot lock failed: {error}"))?;
            current.as_ref().cloned().ok_or_else(|| "Chat lifecycle runtime is not registered".to_string())?
        };
        let manager = RuntimePackageManager::readySnapshot(runtime.tool_handler().getOrCreatePackageManager()).await?;
        let hooks = collectLifecycleHooks(&manager.getEnabledToolPkgContainerRuntimes())?;
        Ok(ToolPkgChatLifecycleDispatchContext { manager, hooks })
    }

    /// Awaits every selected owner in sequence and returns extensions only after all handlers succeed strictly.
    #[allow(non_snake_case)]
    pub async fn dispatchBeforeCreate(
        context: &ToolPkgChatLifecycleDispatchContext,
        draft: &ToolPkgChatCreationDraft,
        sourceExtensions: &BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, Value>, String> {
        validateCreationDraft(draft)?;
        if context.hooks.is_empty() { return Ok(BTreeMap::new()); }
        let seconds = ApiPreferences::getInstance().getToolPkgPreHookTimeoutSeconds()
            .map_err(|error| format!("Chat lifecycle timeout preference read failed: {error}"))?;
        let budget = ToolPkgPreHookTimeout::fromSeconds(seconds);
        let mut extensions = BTreeMap::new();
        for hook in &context.hooks {
            let payload = creationPayload(draft, &hook.containerPackageName, sourceExtensions)?;
            let timeoutMillis = budget.remainingTimeoutMillis()
                .ok_or_else(|| format!("Chat lifecycle before_create timed out: {}:{}", hook.containerPackageName, hook.hookId))?;
            let raw = context.manager.runToolPkgMainHookWithTimeoutMillis(
                &hook.containerPackageName, &hook.functionName, TOOLPKG_EVENT_CHAT_LIFECYCLE,
                Some(CHAT_LIFECYCLE_BEFORE_CREATE), Some(&hook.hookId), hook.functionSource.as_deref(),
                payload, None, None, None, timeoutMillis,
            ).await.map_err(|error| format!("Chat lifecycle before_create failed for {}:{}: {error}", hook.containerPackageName, hook.hookId))?;
            if budget.hasExpired() {
                return Err(format!("Chat lifecycle before_create timed out: {}:{}", hook.containerPackageName, hook.hookId));
            }
            let extension = parseCreationResult(raw)
                .map_err(|error| format!("Chat lifecycle before_create invalid result for {}:{}: {error}", hook.containerPackageName, hook.hookId))?;
            if let Some(value) = extension {
                if extensions.insert(hook.containerPackageName.clone(), Value::Object(value)).is_some() {
                    return Err(format!("Duplicate chat lifecycle hook owner: {}", hook.containerPackageName));
                }
            }
        }
        Ok(extensions)
    }
}

impl ToolPkgChatLifecycleDispatchContext {
    /// Exposes this exact ready snapshot for subsequent configuration resolution without repeating catalog selection.
    #[allow(non_snake_case)]
    pub fn packageManager(&self) -> &RuntimePackageManager { &self.manager }
}

/// Rejects inconsistent generic new or branch draft context before even an empty registration set can succeed.
#[allow(non_snake_case)]
fn validateCreationDraft(draft: &ToolPkgChatCreationDraft) -> Result<(), String> {
    if draft.chat.id.trim().is_empty() { return Err("Chat creation draft id is required".to_string()); }
    for (label, value) in [("workspaceId", &draft.chat.workspaceId), ("parentChatId", &draft.chat.parentChatId), ("sourceChatId", &draft.sourceChatId)] {
        if value.as_ref().is_some_and(|id| id.trim().is_empty()) {
            return Err(format!("Chat creation {label} must be nonblank when present"));
        }
    }
    if draft.sourceChatId.as_ref() == Some(&draft.chat.id) {
        return Err("Chat creation source and draft ids must be distinct".to_string());
    }
    match draft.creationKind {
        ToolPkgChatCreationKind::New => {
            if draft.sourceMessageTimestamp.is_some() || draft.chat.parentChatId.is_some() {
                return Err("New chat creation must not provide branch parent or message timestamp".to_string());
            }
        }
        ToolPkgChatCreationKind::Branch => {
            let source = draft.sourceChatId.as_ref().ok_or_else(|| "Branch creation requires sourceChatId".to_string())?;
            let timestamp = draft.sourceMessageTimestamp.ok_or_else(|| "Branch creation requires sourceMessageTimestamp".to_string())?;
            if timestamp <= 0 { return Err("Branch creation sourceMessageTimestamp must be positive".to_string()); }
            if draft.chat.parentChatId.as_ref() != Some(source) {
                return Err("Branch creation parentChatId must match sourceChatId".to_string());
            }
        }
    }
    Ok(())
}

/// Selects only actual enabled runtime registrations and rejects owner or handler ambiguity before execution.
#[allow(non_snake_case)]
fn collectLifecycleHooks(containers: &[ToolPkgContainerRuntime]) -> Result<Vec<ToolPkgChatLifecycleHookRegistration>, String> {
    let mut hooks = BTreeMap::new();
    for container in containers {
        if container.chatLifecycleHooks.is_empty() { continue; }
        if container.packageName.trim().is_empty() { return Err("Chat lifecycle registered owner is required".to_string()); }
        if container.chatLifecycleHooks.len() != 1 {
            return Err(format!("Duplicate chat lifecycle hook owner: {}", container.packageName));
        }
        let hook = &container.chatLifecycleHooks[0];
        if hook.id.trim().is_empty() || hook.function.trim().is_empty() {
            return Err(format!("Chat lifecycle hook id and exported function are required: {}", container.packageName));
        }
        let registered = ToolPkgChatLifecycleHookRegistration {
            containerPackageName: container.packageName.clone(), hookId: hook.id.clone(),
            functionName: hook.function.clone(), functionSource: hook.functionSource.clone(),
        };
        if hooks.insert(container.packageName.clone(), registered).is_some() {
            return Err(format!("Duplicate chat lifecycle hook owner: {}", container.packageName));
        }
    }
    Ok(hooks.into_values().collect())
}

/// Copies only the selected real owner's source extension into the exact locked creation payload.
#[allow(non_snake_case)]
fn creationPayload(draft: &ToolPkgChatCreationDraft, owner: &str, sourceExtensions: &BTreeMap<String, Value>) -> Result<Value, String> {
    let sourceExtension = match sourceExtensions.get(owner) {
        Some(Value::Object(value)) => Some(value.clone()),
        Some(_) => return Err(format!("Chat lifecycle source extension must be an object for owner: {owner}")),
        None => None,
    };
    Ok(serde_json::json!({
        "eventName": CHAT_LIFECYCLE_BEFORE_CREATE, "creationKind": draft.creationKind, "chat": draft.chat,
        "sourceChatId": draft.sourceChatId, "sourceMessageTimestamp": draft.sourceMessageTimestamp,
        "input": draft.input, "sourceExtension": sourceExtension,
    }))
}

/// Requires exactly one extension field with an object or explicit null, never a null or undefined top-level success.
#[allow(non_snake_case)]
fn parseCreationResult(raw: Option<String>) -> Result<Option<Map<String, Value>>, String> {
    let text = raw.ok_or_else(|| "Lifecycle handler returned undefined".to_string())?;
    let parsed: Value = serde_json::from_str(&text).map_err(|error| format!("Lifecycle handler must return JSON: {error}"))?;
    let Value::Object(mut result) = parsed else { return Err("Lifecycle handler must return an object with extension".to_string()); };
    if result.len() != 1 { return Err("Lifecycle handler must return exactly {extension: object|null}".to_string()); }
    let extension = result.remove("extension").ok_or_else(|| "Lifecycle handler extension field is required".to_string())?;
    match extension {
        Value::Object(value) => Ok(Some(value)),
        Value::Null => Ok(None),
        _ => Err("Lifecycle handler extension must be an object or null".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgFunctionHookRuntime;

    /// Supplies only generic allocated draft data to the actual production payload builder.
    fn draft() -> ToolPkgChatCreationDraft {
        ToolPkgChatCreationDraft {
            creationKind: ToolPkgChatCreationKind::Branch,
            chat: ToolPkgChatCreationChat { id: "draft-id".to_string(), title: "Draft title".to_string(), workspaceId: Some("workspace".to_string()), parentChatId: Some("source".to_string()) },
            sourceChatId: Some("source".to_string()), sourceMessageTimestamp: Some(1720000000000),
            input: Some(Map::from_iter([("opaque".to_string(), Value::Bool(true))])),
        }
    }

    /// Loads a nonempty source from the actual v27 SQLite fixture instead of testing only source-free creation.
    #[test]
    fn new_with_persisted_source_keeps_workspace_and_isolated_owner_event() {
        let source = rusqlite::Connection::open_in_memory().unwrap();
        source.execute_batch(include_str!("../../../../../persistence/store/src/db/fixtures/version27-chat-records.sql")).unwrap();
        source.execute_batch("ALTER TABLE chats ADD COLUMN pluginExtensions TEXT NOT NULL DEFAULT '{}'").unwrap();
        let namespaces = serde_json::json!({
            "alpha": {"opaque": ["source-alpha", 1]},
            "beta": {"private": "source-beta"}
        });
        source.execute("UPDATE chats SET pluginExtensions = ?1 WHERE id = ?2", rusqlite::params![namespaces.to_string(), "v27-chat"]).unwrap();
        let (source_id, workspace_id, extensions): (String, Option<String>, String) = source.query_row(
            "SELECT id, workspaceId, pluginExtensions FROM chats WHERE id = ?1",
            ["v27-chat"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        let extensions: BTreeMap<String, Value> = serde_json::from_str(&extensions).unwrap();
        let mut new = draft();
        new.creationKind = ToolPkgChatCreationKind::New;
        new.sourceChatId = Some(source_id.clone());
        new.sourceMessageTimestamp = None;
        new.chat.parentChatId = None;
        new.chat.workspaceId = workspace_id.clone();
        validateCreationDraft(&new).unwrap();
        let payload = creationPayload(&new, "alpha", &extensions).unwrap();
        assert_eq!(payload["creationKind"], "new");
        assert_eq!(payload["sourceChatId"], source_id);
        assert_eq!(payload["chat"]["workspaceId"], "workspace-kept");
        assert_eq!(payload["chat"]["parentChatId"], Value::Null);
        assert_eq!(payload["sourceMessageTimestamp"], Value::Null);
        assert_eq!(payload["sourceExtension"], extensions["alpha"]);
        assert!(payload.get("pluginExtensions").is_none());
        assert!(payload["sourceExtension"].get("private").is_none());
        assert_eq!(source.query_row("SELECT workspaceId FROM chats WHERE id = ?1", ["v27-chat"], |row| row.get::<_, Option<String>>(0)).unwrap(), workspace_id);
        assert_eq!(source.query_row("SELECT COUNT(*) FROM chats WHERE id = ?1", [&new.chat.id], |row| row.get::<_, i64>(0)).unwrap(), 0);
    }

    /// Allows a genuinely first conversation while rejecting branch-only context on sourced new conversations.
    #[test]
    fn new_creation_accepts_explicit_source_or_none_without_branch_fields() {
        let mut new = draft();
        new.creationKind = ToolPkgChatCreationKind::New;
        new.sourceMessageTimestamp = None;
        new.chat.parentChatId = None;
        validateCreationDraft(&new).unwrap();
        new.sourceChatId = None;
        validateCreationDraft(&new).unwrap();
        new.sourceChatId = Some("source".to_string());
        new.chat.parentChatId = Some("source".to_string());
        assert!(validateCreationDraft(&new).is_err());
        new.chat.parentChatId = None;
        new.sourceMessageTimestamp = Some(1720000000000);
        assert!(validateCreationDraft(&new).is_err());
        new.sourceMessageTimestamp = None;
        new.sourceChatId = Some(new.chat.id.clone());
        assert!(validateCreationDraft(&new).is_err());
        new.sourceChatId = Some(" ".to_string());
        assert!(validateCreationDraft(&new).is_err());
    }

    /// Requires the exact source, matching parent and genuine positive branch point for every branch draft.
    #[test]
    fn branch_creation_requires_complete_distinct_source_context() {
        validateCreationDraft(&draft()).unwrap();
        let mut missing_source = draft();
        missing_source.sourceChatId = None;
        assert!(validateCreationDraft(&missing_source).is_err());
        let mut missing_timestamp = draft();
        missing_timestamp.sourceMessageTimestamp = None;
        assert!(validateCreationDraft(&missing_timestamp).is_err());
        for timestamp in [0, -1] {
            let mut invalid_timestamp = draft();
            invalid_timestamp.sourceMessageTimestamp = Some(timestamp);
            assert!(validateCreationDraft(&invalid_timestamp).is_err());
        }
        let mut missing_parent = draft();
        missing_parent.chat.parentChatId = None;
        assert!(validateCreationDraft(&missing_parent).is_err());
        missing_parent.chat.parentChatId = Some("unrelated".to_string());
        assert!(validateCreationDraft(&missing_parent).is_err());
        let mut same_id = draft();
        same_id.chat.id = "source".to_string();
        assert!(validateCreationDraft(&same_id).is_err());
    }

    /// Supplies registered function metadata, not a successful native host or persistence implementation.
    fn registered(owner: &str) -> ToolPkgContainerRuntime {
        ToolPkgContainerRuntime {
            packageName: owner.to_string(), chatLifecycleHooks: vec![ToolPkgFunctionHookRuntime {
                id: "creation".to_string(), function: "beforeCreate".to_string(), functionSource: None,
            }], ..Default::default()
        }
    }

    /// Proves each owner sees only its own namespace and that neither draft nor source map is modified.
    #[test]
    fn source_extensions_are_strictly_isolated_by_real_owner() {
        let sources = BTreeMap::from([
            ("alpha".to_string(), serde_json::json!({ "opaque": "alpha-only" })),
            ("beta".to_string(), serde_json::json!({ "private": "beta-only" })),
        ]);
        let before = sources.clone();
        let alpha = creationPayload(&draft(), "alpha", &sources).unwrap();
        let beta = creationPayload(&draft(), "beta", &sources).unwrap();
        assert_eq!(alpha["sourceExtension"], sources["alpha"]); assert_eq!(beta["sourceExtension"], sources["beta"]);
        assert_eq!(alpha.as_object().unwrap().len(), 7); assert_eq!(sources, before);
        assert_eq!(alpha["eventName"], "before_create"); assert_eq!(alpha["creationKind"], "branch");
        assert_eq!(alpha["chat"].as_object().unwrap().len(), 4);
        assert_eq!(alpha["chat"]["id"], "draft-id"); assert_eq!(alpha["sourceChatId"], "source");
    }

    /// Represents absent owner metadata as protocol null without inspecting unrelated source namespaces.
    #[test]
    fn missing_own_namespace_is_null_and_foreign_bad_metadata_is_not_read() {
        let sources = BTreeMap::from([("foreign".to_string(), Value::String("not this owner's data".to_string()))]);
        assert_eq!(creationPayload(&draft(), "own", &sources).unwrap()["sourceExtension"], Value::Null);
        assert!(creationPayload(&draft(), "foreign", &sources).is_err());
    }

    /// Rejects all undeclared result forms, including void, top-level null, owner injection and non-object extensions.
    #[test]
    fn lifecycle_results_require_exact_explicit_extension() {
        assert!(parseCreationResult(None).is_err());
        for raw in ["", "null", "undefined", "[]", "false", "{}", "{\"owner\":\"foreign\"}", "{\"extension\":{},\"owner\":\"foreign\"}", "{\"extension\":[]}", "{\"extension\":42}", "{\"extension\":false}"] {
            assert!(parseCreationResult(Some(raw.to_string())).is_err(), "{raw}");
        }
        assert_eq!(parseCreationResult(Some("{\"extension\":null}".to_string())).unwrap(), None);
        assert_eq!(parseCreationResult(Some("{\"extension\":{\"opaque\":[null,true,1.5]}}".to_string())).unwrap(), Some(Map::from_iter([("opaque".to_string(), serde_json::json!([null, true, 1.5]))])));
    }

    /// Rejects multiple creation handlers for one owner even when their individual registration ids differ.
    #[test]
    fn duplicate_owner_handlers_are_errors_before_any_dispatch() {
        let first = registered("alpha");
        let mut duplicate = first.clone();
        duplicate.chatLifecycleHooks.push(ToolPkgFunctionHookRuntime { id: "another".to_string(), function: "other".to_string(), functionSource: None });
        assert_eq!(collectLifecycleHooks(&[duplicate]).unwrap_err(), "Duplicate chat lifecycle hook owner: alpha");
        assert_eq!(collectLifecycleHooks(&[first.clone(), first]).unwrap_err(), "Duplicate chat lifecycle hook owner: alpha");
    }

    /// Retains arbitrary real owners and stable ordering while allowing an actually empty active registration set.
    #[test]
    fn independent_owners_and_no_participants_are_valid_registration_snapshots() {
        let hooks = collectLifecycleHooks(&[registered("org.example.beta"), registered("com.example.alpha")]).unwrap();
        assert_eq!(hooks.iter().map(|hook| hook.containerPackageName.as_str()).collect::<Vec<_>>(), vec!["com.example.alpha", "org.example.beta"]);
        assert!(collectLifecycleHooks(&[]).unwrap().is_empty());
    }
}
