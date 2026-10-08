use super::ChatHistory::ChatHistory;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatHistoryListItem {
    pub id: String,
    pub title: String,
    pub updatedAt: String,
    pub displayOrder: i64,
    pub workspaceId: Option<String>,
    pub workspaceName: Option<String>,
    pub locked: bool,
    pub pinned: bool,
}

impl ChatHistoryListItem {
    /// Projects only host-owned chat metadata for the history list.
    pub fn fromChatHistory(history: &ChatHistory) -> Self {
        Self {
            id: history.id.clone(),
            title: history.title.clone(),
            updatedAt: history.updatedAt.clone(),
            displayOrder: history.displayOrder,
            workspaceId: history.workspaceId.clone(),
            workspaceName: history.workspaceName.clone(),
            locked: history.locked,
            pinned: history.pinned,
        }
    }
}
