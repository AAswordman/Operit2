use chrono::TimeZone;
use serde::{Deserialize, Serialize};

use super::ChatHistory::ChatHistory;
use super::ChatMessage::ChatMessage;
use super::MessageVariantEntity::MessageVariantEntity;

pub const ARCHIVE_TYPE: &str = "operit_chat_archive";
pub const CURRENT_FORMAT_VERSION: i32 = 2;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperitChatArchive {
    pub archiveType: String,
    pub formatVersion: i32,
    pub exportedAt: i64,
    pub chats: Vec<OperitArchivedChat>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OperitArchivedChat {
    pub id: String,
    pub title: String,
    pub messages: Vec<OperitArchivedMessage>,
    pub createdAt: String,
    pub updatedAt: String,
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub currentWindowSize: i32,
    pub group: Option<String>,
    pub displayOrder: i64,
    pub workspace: Option<String>,
    pub parentChatId: Option<String>,
    pub characterCardName: Option<String>,
    pub characterGroupId: Option<String>,
    pub locked: bool,
    pub pinned: bool,
}

impl OperitArchivedChat {
    #[allow(non_snake_case)]
    pub fn fromChatHistory(
        history: ChatHistory,
        messages: Vec<OperitArchivedMessage>,
    ) -> Result<Self, String> {
        Ok(Self {
            id: history.id,
            title: history.title,
            messages,
            createdAt: millisStringToLocalDateTimeString(&history.createdAt)?,
            updatedAt: millisStringToLocalDateTimeString(&history.updatedAt)?,
            inputTokens: history.inputTokens,
            outputTokens: history.outputTokens,
            currentWindowSize: history.currentWindowSize,
            group: history.group,
            displayOrder: history.displayOrder,
            workspace: history.workspace,
            parentChatId: history.parentChatId,
            characterCardName: history.characterCardName,
            characterGroupId: history.characterGroupId,
            locked: history.locked,
            pinned: history.pinned,
        })
    }

    #[allow(non_snake_case)]
    pub fn toChatHistory(&self) -> Result<ChatHistory, String> {
        Ok(ChatHistory {
            id: self.id.clone(),
            title: self.title.clone(),
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
            group: self.group.clone(),
            displayOrder: self.displayOrder,
            workspace: self.workspace.clone(),
            parentChatId: self.parentChatId.clone(),
            characterCardName: self.characterCardName.clone(),
            characterGroupId: self.characterGroupId.clone(),
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
    pub content: String,
    pub roleName: String,
    pub provider: String,
    pub modelName: String,
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub cachedInputTokens: i32,
    pub sentAt: i64,
    pub outputDurationMs: i64,
    pub waitDurationMs: i64,
    pub completedAt: i64,
}

impl OperitArchivedMessageVariant {
    #[allow(non_snake_case)]
    pub fn fromEntity(entity: MessageVariantEntity) -> Self {
        Self {
            variantIndex: entity.variantIndex,
            content: entity.content,
            roleName: entity.roleName,
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
    pub fn toEntity(&self, chatId: String, messageTimestamp: i64) -> MessageVariantEntity {
        MessageVariantEntity {
            variantId: 0,
            chatId,
            messageTimestamp,
            variantIndex: self.variantIndex,
            content: self.content.clone(),
            roleName: self.roleName.clone(),
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
