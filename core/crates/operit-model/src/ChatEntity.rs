use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ChatHistory::ChatHistory;
use super::ChatMessage::ChatMessage;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatEntity {
    pub id: String,
    pub title: String,
    pub createdAt: i64,
    pub updatedAt: i64,
    pub inputTokens: i64,
    pub outputTokens: i64,
    pub currentWindowSize: i64,
    pub group: Option<String>,
    pub displayOrder: i64,
    pub workspace: Option<String>,
    pub parentChatId: Option<String>,
    pub characterCardName: Option<String>,
    pub characterGroupId: Option<String>,
    pub locked: bool,
    pub pinned: bool,
}

impl ChatEntity {
    /// Creates one persisted chat metadata record.
    pub fn new(id: String, title: String, timestamp: i64) -> Self {
        Self {
            id,
            title,
            createdAt: timestamp,
            updatedAt: timestamp,
            inputTokens: 0,
            outputTokens: 0,
            currentWindowSize: 0,
            group: None,
            displayOrder: -timestamp,
            workspace: None,
            parentChatId: None,
            characterCardName: None,
            characterGroupId: None,
            locked: false,
            pinned: false,
        }
    }

    /// Creates chat metadata with a generated chat identifier.
    pub fn create(title: String) -> Self {
        let timestamp = currentTimeMillis();
        Self::new(Uuid::new_v4().to_string(), title, timestamp)
    }

    /// Projects persisted chat metadata and messages into the runtime model.
    pub fn toChatHistory(&self, messages: Vec<ChatMessage>) -> ChatHistory {
        ChatHistory {
            id: self.id.clone(),
            title: self.title.clone(),
            messages,
            createdAt: self.createdAt.to_string(),
            updatedAt: self.updatedAt.to_string(),
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
        }
    }

    /// Converts the runtime chat model into persisted metadata.
    pub fn fromChatHistory(chatHistory: &ChatHistory) -> Self {
        Self {
            id: chatHistory.id.clone(),
            title: chatHistory.title.clone(),
            createdAt: chatHistory
                .createdAt
                .parse::<i64>()
                .expect("ChatHistory.createdAt must be an epoch millis string"),
            updatedAt: chatHistory
                .updatedAt
                .parse::<i64>()
                .expect("ChatHistory.updatedAt must be an epoch millis string"),
            inputTokens: chatHistory.inputTokens,
            outputTokens: chatHistory.outputTokens,
            currentWindowSize: chatHistory.currentWindowSize,
            group: chatHistory.group.clone(),
            displayOrder: chatHistory.displayOrder,
            workspace: chatHistory.workspace.clone(),
            parentChatId: chatHistory.parentChatId.clone(),
            characterCardName: chatHistory.characterCardName.clone(),
            characterGroupId: chatHistory.characterGroupId.clone(),
            locked: chatHistory.locked,
            pinned: chatHistory.pinned,
        }
    }
}

/// Returns the current epoch timestamp used by generated chat metadata.
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
