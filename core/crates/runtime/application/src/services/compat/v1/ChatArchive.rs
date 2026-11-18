use std::collections::{BTreeMap, HashSet};

use operit_model::ChatMessage::ChatMessage;
use operit_model::ChatMessageDisplayMode::ChatMessageDisplayMode;
use operit_model::MessagePart::MessagePart;
use operit_model::MessagePartCodec::MessagePartCodec;
use operit_model::OperitChatArchive::{
    ARCHIVE_TYPE, CURRENT_FORMAT_VERSION, OperitArchivedChat, OperitArchivedMessage,
    OperitArchivedMessageVariant, OperitChatArchive,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{ChatRuntimeSlot, RuntimeToolSupport};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyArchive {
    archive_type: String,
    format_version: i32,
    #[serde(default)]
    exported_at: i64,
    chats: Vec<LegacyChat>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyChat {
    id: String,
    title: String,
    group: Option<String>,
    messages: Vec<LegacyMessage>,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    current_window_size: i64,
    #[serde(default)]
    display_order: i64,
    parent_chat_id: Option<String>,
    #[serde(default)]
    locked: bool,
    #[serde(default)]
    pinned: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyMessage {
    base_message: LegacyBaseMessage,
    #[serde(default)]
    variants: Vec<LegacyVariant>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyBaseMessage {
    sender: String,
    timestamp: i64,
    #[serde(default)]
    selected_variant_index: i32,
    display_mode: Option<ChatMessageDisplayMode>,
    #[serde(default)]
    is_favorite: bool,
    #[serde(flatten)]
    text: LegacyText,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyVariant {
    variant_index: i32,
    #[serde(flatten)]
    text: LegacyText,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LegacyText {
    content: String,
    role_name: String,
    provider: String,
    model_name: String,
    input_tokens: i64,
    output_tokens: i64,
    cached_input_tokens: i64,
    sent_at: i64,
    output_duration_ms: i64,
    wait_duration_ms: i64,
    completed_at: i64,
}

impl LegacyText {
    /// Projects the shared text and model metadata of a base message or a stored revision.
    fn into_variant(self, sender: &str, variant_index: i32) -> OperitArchivedMessageVariant {
        let parts = if sender == "ai" {
            MessagePartCodec::parseAssistantMarkupForImport(&self.content)
        } else {
            vec![MessagePart::markdown("part-0".into(), 0, self.content)]
        };
        OperitArchivedMessageVariant {
            variantIndex: variant_index,
            parts,
            roleName: self.role_name,
            pluginExtensions: BTreeMap::new(),
            provider: self.provider,
            modelName: self.model_name,
            inputTokens: self.input_tokens,
            outputTokens: self.output_tokens,
            cachedInputTokens: self.cached_input_tokens,
            sentAt: self.sent_at,
            outputDurationMs: self.output_duration_ms,
            waitDurationMs: self.wait_duration_ms,
            completedAt: self.completed_at,
        }
    }
}

impl LegacyMessage {
    /// Retains base and alternate replies separately, with the selected revision and original sender.
    fn into_message(self, timestamp: i64) -> Result<OperitArchivedMessage, String> {
        let base = self.base_message;
        let original = base.text.into_variant(&base.sender, 0);
        let variant_count =
            i32::try_from(self.variants.len() + 1).map_err(|error| error.to_string())?;
        let mut seen = HashSet::new();
        let mut variants = Vec::with_capacity(self.variants.len());
        for variant in self.variants {
            let index = variant.variant_index;
            if index <= 0 || !seen.insert(index) {
                return Err(format!(
                    "Invalid v1 message variant index {index} at {timestamp}"
                ));
            }
            variants.push(variant.text.into_variant(&base.sender, index));
        }
        if base.selected_variant_index != 0 && !seen.contains(&base.selected_variant_index) {
            return Err(format!(
                "Missing selected v1 message variant at {timestamp}"
            ));
        }
        Ok(OperitArchivedMessage {
            baseMessage: ChatMessage {
                sender: base.sender,
                parts: original.parts,
                timestamp,
                roleName: original.roleName,
                pluginExtensions: original.pluginExtensions,
                selectedVariantIndex: base.selected_variant_index,
                variantCount: variant_count,
                provider: original.provider,
                modelName: original.modelName,
                inputTokens: original.inputTokens,
                outputTokens: original.outputTokens,
                cachedInputTokens: original.cachedInputTokens,
                sentAt: original.sentAt,
                outputDurationMs: original.outputDurationMs,
                waitDurationMs: original.waitDurationMs,
                completedAt: original.completedAt,
                completedExecutionGeneration: 0,
                displayMode: base.display_mode.unwrap_or(ChatMessageDisplayMode::NORMAL),
                isFavorite: base.is_favorite,
                isVariantPreview: false,
                contentStream: None,
            },
            variants,
        })
    }
}

impl LegacyChat {
    /// Assigns distinct message identities within the chat without changing other source timestamps.
    fn into_chat(self) -> Result<OperitArchivedChat, String> {
        let reserved: HashSet<_> = self
            .messages
            .iter()
            .map(|message| message.base_message.timestamp)
            .collect();
        let mut seen = HashSet::new();
        let mut messages = Vec::with_capacity(self.messages.len());
        for message in self.messages {
            let mut timestamp = message.base_message.timestamp;
            if !seen.insert(timestamp) {
                loop {
                    timestamp = timestamp
                        .checked_add(1)
                        .ok_or("V1 message timestamp overflow")?;
                    if !reserved.contains(&timestamp) && seen.insert(timestamp) {
                        break;
                    }
                }
            }
            messages.push(message.into_message(timestamp)?);
        }
        let chat = OperitArchivedChat {
            id: self.id,
            title: self.title,
            group: self.group,
            pluginExtensions: BTreeMap::new(),
            messages,
            createdAt: self.created_at,
            updatedAt: self.updated_at,
            inputTokens: self.input_tokens,
            outputTokens: self.output_tokens,
            currentWindowSize: self.current_window_size,
            displayOrder: self.display_order,
            workspaceId: None,
            parentChatId: self.parent_chat_id,
            locked: self.locked,
            pinned: self.pinned,
        };
        Ok(chat)
    }
}

/// Decodes the v1 migrator's format independently of the canonical archive reader.
fn decode_archive(bytes: &[u8]) -> Result<OperitChatArchive, String> {
    let source: LegacyArchive = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if source.archive_type != ARCHIVE_TYPE || source.format_version != 2 {
        return Err("Expected an Operit v1 chat archive (formatVersion 2)".into());
    }
    Ok(OperitChatArchive {
        archiveType: source.archive_type,
        formatVersion: CURRENT_FORMAT_VERSION,
        exportedAt: source.exported_at,
        workspaces: Vec::new(),
        chats: source
            .chats
            .into_iter()
            .map(LegacyChat::into_chat)
            .collect::<Result<_, _>>()?,
    })
}

/// Reads through the runtime VFS and commits through the canonical archive importer.
pub(super) async fn import(runtime: &RuntimeToolSupport, path: String) -> Result<Value, String> {
    let bytes = runtime.hostManager.vfs()?.readFileBytes(&path)?;
    let archive = decode_archive(&bytes)?;
    let imported_ids: Vec<_> = archive.chats.iter().map(|chat| chat.id.clone()).collect();
    let manager = runtime.runtimeChatHistoryManager()?;
    let result = manager
        .importChatArchiveWithProgress(archive, |_, _, _| {})
        .map_err(|error| error.to_string())?;
    let mut holder = runtime.chatRuntimeHolder.lock().await;
    let core = holder.getCore(ChatRuntimeSlot::MAIN);
    let current_chat_id = core.chatHistoryDelegate.currentChatIdFlow.value();
    let refreshed_chat = current_chat_id
        .as_ref()
        .filter(|id| imported_ids.contains(id));
    if let Some(id) = refreshed_chat {
        core.reloadChatMessagesSmart(id.clone());
    }
    let refreshed = refreshed_chat.is_some();
    drop(holder);

    // The archive is already committed. Keep diagnostics separate from the import result.
    let mut summary_timestamps = BTreeMap::new();
    let mut diagnostic_error = None;
    for id in current_chat_id.iter().chain(imported_ids.first()) {
        match manager.getLatestSummaryTimestamp(id.clone()) {
            Ok(timestamp) => {
                summary_timestamps.insert(id.clone(), timestamp);
            }
            Err(error) => {
                diagnostic_error = Some(error.to_string());
            }
        }
    }
    Ok(json!({
        "new": result.new, "updated": result.updated, "skipped": result.skipped,
        "currentChatId": current_chat_id, "refreshed": refreshed,
        "summaryTimestamps": summary_timestamps, "diagnosticError": diagnostic_error,
    }))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use operit_host_native_storage::NativeRuntimeStorageHost;
    use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
    use operit_store::repository::ChatHistoryManager::ChatHistoryManager;
    use std::sync::Arc;

    /// Runs one legacy payload through the real import store and checks its selected reply and summary.
    #[test]
    #[ignore = "Uses isolated global storage hosts and must run in its own test process"]
    fn v1_migrator_persists_variants_and_summaries() {
        let root = std::env::temp_dir().join(format!("operit-migrator-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
            operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
        ));
        operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost(host.clone());
        operit_store::RuntimeStorageHost::setDefaultRuntimeSqliteHost(host.clone());
        CoreNodeIdentityStore::new(host).initialize().unwrap();
        let archive = decode_archive(br#"{
          "archiveType":"operit_chat_archive","formatVersion":2,"exportedAt":1720000000500,
          "chats":[{"id":"sample","title":"Migrated conversation","group":"Imported",
            "createdAt":"2024-07-03T09:00:00.000","updatedAt":"2024-07-03T09:00:01.000",
            "messages":[
              {"baseMessage":{"sender":"user","content":"question","timestamp":1720000000000}},
              {"baseMessage":{"sender":"summary","content":"summary text","timestamp":1720000000001}},
              {"baseMessage":{"sender":"ai","content":"initial answer","timestamp":1720000000001,"selectedVariantIndex":1},
               "variants":[{"variantIndex":1,"content":"revised answer","modelName":"legacy-model","outputTokens":33}]}
            ]}]
        }"#).unwrap();
        let manager = ChatHistoryManager::default().unwrap();
        let initial = manager
            .importChatArchiveWithProgress(archive.clone(), |_, _, _| {})
            .unwrap();
        assert_eq!(initial.new, 1);
        let repeated = manager
            .importChatArchiveWithProgress(archive, |_, _, _| {})
            .unwrap();
        assert_eq!(repeated.updated, 1);
        assert_eq!(
            manager.getLatestSummaryTimestamp("sample".into()).unwrap(),
            Some(1720000000001)
        );
        let messages = manager.loadRuntimeChatMessages("sample".into()).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].displayText(), "summary text");
        assert_eq!(messages[1].displayText(), "revised answer");
        assert_eq!(messages[1].timestamp, 1720000000002);
        assert_eq!(messages[1].selectedVariantIndex, 1);
        assert_eq!(messages[1].modelName, "legacy-model");
        assert_eq!(messages[1].outputTokens, 33);
    }
}
