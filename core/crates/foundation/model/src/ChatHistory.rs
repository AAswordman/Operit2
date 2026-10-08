use std::collections::BTreeMap;

use super::ChatMessage::ChatMessage;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatHistory {
    pub id: String,
    pub title: String,
    pub pluginExtensions: BTreeMap<String, serde_json::Value>,
    pub messages: Vec<ChatMessage>,
    pub createdAt: String,
    pub updatedAt: String,
    pub inputTokens: i64,
    pub outputTokens: i64,
    pub currentWindowSize: i64,
    pub displayOrder: i64,
    pub workspaceId: Option<String>,
    pub workspaceName: Option<String>,
    pub workspacePrimaryPath: Option<String>,
    pub parentChatId: Option<String>,
    pub locked: bool,
    pub pinned: bool,
}
