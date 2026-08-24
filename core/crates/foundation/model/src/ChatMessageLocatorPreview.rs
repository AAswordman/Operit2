use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessageLocatorPreview {
    pub messageIndex: Option<i32>,
    pub timestamp: i64,
    pub sender: String,
    pub previewContent: String,
    pub contentLength: i32,
    pub displayMode: String,
    pub isFavorite: bool,
}
