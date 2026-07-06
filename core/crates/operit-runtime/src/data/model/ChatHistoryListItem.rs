use super::ChatHistory::ChatHistory;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatHistoryListItem {
    pub id: String,
    pub title: String,
    pub updatedAt: String,
    pub group: Option<String>,
    pub displayOrder: i64,
    pub characterCardName: Option<String>,
    pub characterGroupId: Option<String>,
    pub locked: bool,
    pub pinned: bool,
}

impl ChatHistoryListItem {
    pub fn fromChatHistory(history: &ChatHistory) -> Self {
        Self {
            id: history.id.clone(),
            title: history.title.clone(),
            updatedAt: history.updatedAt.clone(),
            group: history.group.clone(),
            displayOrder: history.displayOrder,
            characterCardName: history.characterCardName.clone(),
            characterGroupId: history.characterGroupId.clone(),
            locked: history.locked,
            pinned: history.pinned,
        }
    }
}
