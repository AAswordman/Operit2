use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ChatHistory::ChatHistory;
use super::ChatMessage::ChatMessage;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatEntity {
    pub id: String,
    pub title: String,
    pub pluginExtensions: BTreeMap<String, serde_json::Value>,
    pub createdAt: i64,
    pub updatedAt: i64,
    pub inputTokens: i64,
    pub outputTokens: i64,
    pub currentWindowSize: i64,
    pub displayOrder: i64,
    pub workspaceId: Option<String>,
    pub parentChatId: Option<String>,
    pub locked: bool,
    pub pinned: bool,
}

impl ChatEntity {
    /// Creates one persisted chat metadata record.
    pub fn new(id: String, title: String, timestamp: i64) -> Self {
        Self {
            id,
            title,
            pluginExtensions: BTreeMap::new(),
            createdAt: timestamp,
            updatedAt: timestamp,
            inputTokens: 0,
            outputTokens: 0,
            currentWindowSize: 0,
            displayOrder: -timestamp,
            workspaceId: None,
            parentChatId: None,
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
            pluginExtensions: self.pluginExtensions.clone(),
            messages,
            createdAt: self.createdAt.to_string(),
            updatedAt: self.updatedAt.to_string(),
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
        }
    }

    /// Converts the runtime chat model into persisted metadata.
    pub fn fromChatHistory(chatHistory: &ChatHistory) -> Self {
        Self {
            id: chatHistory.id.clone(),
            title: chatHistory.title.clone(),
            pluginExtensions: chatHistory.pluginExtensions.clone(),
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
            displayOrder: chatHistory.displayOrder,
            workspaceId: chatHistory.workspaceId.clone(),
            parentChatId: chatHistory.parentChatId.clone(),
            locked: chatHistory.locked,
            pinned: chatHistory.pinned,
        }
    }
}

/// Returns the current epoch timestamp used by generated chat metadata.
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
