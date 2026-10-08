use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::ChatHistory::ChatHistory;
use super::ChatMessage::ChatMessage;
use super::MessagePart::MessagePart;
use super::MessageVariantEntity::MessageVariantEntity;
use super::Workspace::Workspace;

pub const ARCHIVE_TYPE: &str = "operit_chat_archive";
pub const CURRENT_FORMAT_VERSION: i32 = 5;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperitChatArchive {
    pub archiveType: String,
    pub formatVersion: i32,
    pub exportedAt: i64,
    pub workspaces: Vec<Workspace>,
    pub chats: Vec<OperitArchivedChat>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperitArchivedChat {
    pub id: String,
    pub title: String,
    pub pluginExtensions: BTreeMap<String, serde_json::Value>,
    pub messages: Vec<OperitArchivedMessage>,
    pub createdAt: String,
    pub updatedAt: String,
    pub inputTokens: i64,
    pub outputTokens: i64,
    pub currentWindowSize: i64,
    pub displayOrder: i64,
    pub workspaceId: Option<String>,
    pub parentChatId: Option<String>,
    pub locked: bool,
    pub pinned: bool,
}

impl OperitArchivedChat {
    #[allow(non_snake_case)]
    /// Converts runtime chat state into the portable archive representation.
    pub fn fromChatHistory(
        history: ChatHistory,
        messages: Vec<OperitArchivedMessage>,
    ) -> Result<Self, String> {
        Ok(Self {
            id: history.id,
            title: history.title,
            pluginExtensions: history.pluginExtensions,
            messages,
            createdAt: millisStringToLocalDateTimeString(&history.createdAt)?,
            updatedAt: millisStringToLocalDateTimeString(&history.updatedAt)?,
            inputTokens: history.inputTokens,
            outputTokens: history.outputTokens,
            currentWindowSize: history.currentWindowSize,
            displayOrder: history.displayOrder,
            workspaceId: history.workspaceId,
            parentChatId: history.parentChatId,
            locked: history.locked,
            pinned: history.pinned,
        })
    }

    #[allow(non_snake_case)]
    /// Converts one archived chat into runtime chat state.
    pub fn toChatHistory(&self) -> Result<ChatHistory, String> {
        Ok(ChatHistory {
            id: self.id.clone(),
            title: self.title.clone(),
            pluginExtensions: self.pluginExtensions.clone(),
            messages: self
                .messages
                .iter()
                .map(|message| message.baseMessage.clone())
                .collect(),
            createdAt: localDateTimeStringToMillisString(&self.createdAt)?,
            updatedAt: localDateTimeStringToMillisString(&self.updatedAt)?,
            inputTokens: self.inputTokens,
            outputTokens: self.outputTokens,
            currentWindowSize: self.currentWindowSize,
            displayOrder: self.displayOrder,
            workspaceId: self.workspaceId.clone(),
            workspaceName: None,
            workspacePrimaryPath: None,
            parentChatId: self.parentChatId.clone(),
            locked: self.locked,
            pinned: self.pinned,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperitArchivedMessage {
    pub baseMessage: ChatMessage,
    pub variants: Vec<OperitArchivedMessageVariant>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperitArchivedMessageVariant {
    pub variantIndex: i32,
    pub parts: Vec<MessagePart>,
    pub roleName: String,
    pub pluginExtensions: BTreeMap<String, serde_json::Value>,
    pub provider: String,
    pub modelName: String,
    pub inputTokens: i64,
    pub outputTokens: i64,
    pub cachedInputTokens: i64,
    pub sentAt: i64,
    pub outputDurationMs: i64,
    pub waitDurationMs: i64,
    pub completedAt: i64,
}

impl OperitArchivedMessageVariant {
    #[allow(non_snake_case)]
    /// Creates an archived revision from its metadata and ordered parts.
    pub fn fromEntity(entity: MessageVariantEntity, parts: Vec<MessagePart>) -> Self {
        Self {
            variantIndex: entity.variantIndex,
            parts,
            roleName: entity.roleName,
            pluginExtensions: entity.pluginExtensions,
            provider: entity.provider,
            modelName: entity.modelName,
            inputTokens: entity.inputTokens,
            outputTokens: entity.outputTokens,
            cachedInputTokens: entity.cachedInputTokens,
            sentAt: entity.sentAt,
            outputDurationMs: entity.outputDurationMs,
            waitDurationMs: entity.waitDurationMs,
            completedAt: entity.completedAt,
        }
    }

    #[allow(non_snake_case)]
    /// Converts archived revision metadata into a persistence entity.
    pub fn toEntity(&self, chatId: String, messageTimestamp: i64) -> MessageVariantEntity {
        MessageVariantEntity {
            variantId: 0,
            chatId,
            messageTimestamp,
            variantIndex: self.variantIndex,
            roleName: self.roleName.clone(),
            pluginExtensions: self.pluginExtensions.clone(),
            provider: self.provider.clone(),
            modelName: self.modelName.clone(),
            inputTokens: self.inputTokens,
            outputTokens: self.outputTokens,
            cachedInputTokens: self.cachedInputTokens,
            sentAt: self.sentAt,
            outputDurationMs: self.outputDurationMs,
            waitDurationMs: self.waitDurationMs,
            completedAt: self.completedAt,
        }
    }
}

#[allow(non_snake_case)]
fn millisStringToLocalDateTimeString(value: &str) -> Result<String, String> {
    let millis = value.parse::<i64>().map_err(|error| error.to_string())?;
    let datetime = chrono::Local
        .timestamp_millis_opt(millis)
        .single()
        .ok_or_else(|| format!("invalid epoch millis: {millis}"))?;
    Ok(datetime
        .naive_local()
        .format("%Y-%m-%dT%H:%M:%S%.3f")
        .to_string())
}

#[allow(non_snake_case)]
fn localDateTimeStringToMillisString(value: &str) -> Result<String, String> {
    let datetime = parseLocalDateTime(value)?;
    let local = chrono::Local
        .from_local_datetime(&datetime)
        .single()
        .ok_or_else(|| format!("invalid local date time: {value}"))?;
    Ok(local.timestamp_millis().to_string())
}

#[allow(non_snake_case)]
fn parseLocalDateTime(value: &str) -> Result<chrono::NaiveDateTime, String> {
    let format = if value.contains('.') {
        "%Y-%m-%dT%H:%M:%S%.f"
    } else {
        "%Y-%m-%dT%H:%M:%S"
    };
    chrono::NaiveDateTime::parse_from_str(value, format).map_err(|error| error.to_string())
}
