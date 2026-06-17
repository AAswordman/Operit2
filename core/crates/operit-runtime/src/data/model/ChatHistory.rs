use super::ChatMessage::ChatMessage;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatHistory {
    pub id: String,
    pub title: String,
    pub messages: Vec<ChatMessage>,
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
