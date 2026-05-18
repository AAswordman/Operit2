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
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub currentWindowSize: i32,
    pub group: Option<String>,
    pub displayOrder: i64,
    pub workspace: Option<String>,
    pub workspaceEnv: Option<String>,
    pub parentChatId: Option<String>,
    pub characterCardName: Option<String>,
    pub characterGroupId: Option<String>,
    pub locked: bool,
}

impl ChatEntity {
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
            workspaceEnv: None,
            parentChatId: None,
            characterCardName: None,
            characterGroupId: None,
            locked: false,
        }
    }

    pub fn create(title: String) -> Self {
        let timestamp = currentTimeMillis();
        Self::new(Uuid::new_v4().to_string(), title, timestamp)
    }

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
            workspaceEnv: self.workspaceEnv.clone(),
            parentChatId: self.parentChatId.clone(),
            characterCardName: self.characterCardName.clone(),
            characterGroupId: self.characterGroupId.clone(),
            locked: self.locked,
        }
    }

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
            workspaceEnv: chatHistory.workspaceEnv.clone(),
            parentChatId: chatHistory.parentChatId.clone(),
            characterCardName: chatHistory.characterCardName.clone(),
            characterGroupId: chatHistory.characterGroupId.clone(),
            locked: chatHistory.locked,
        }
    }
}

fn currentTimeMillis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}
