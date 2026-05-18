use serde::{Deserialize, Serialize};

use super::ChatMessageDisplayMode::ChatMessageDisplayMode;
use super::ChatMessageTimestampAllocator::ChatMessageTimestampAllocator;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub roleName: String,
    pub selectedVariantIndex: i32,
    pub variantCount: i32,
    pub provider: String,
    pub modelName: String,
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub cachedInputTokens: i32,
    pub sentAt: i64,
    pub outputDurationMs: i64,
    pub waitDurationMs: i64,
    pub completedAt: i64,
    pub displayMode: ChatMessageDisplayMode,
    pub isFavorite: bool,
    #[serde(skip)]
    pub isVariantPreview: bool,
}

impl ChatMessage {
    pub fn new(sender: String) -> Self {
        Self::new_with_content(sender, String::new())
    }

    pub fn new_with_content(sender: String, content: String) -> Self {
        Self::new_with_timestamp(sender, content, ChatMessageTimestampAllocator::next())
    }

    pub fn new_with_timestamp(sender: String, content: String, timestamp: i64) -> Self {
        ChatMessageTimestampAllocator::observe(timestamp);
        Self {
            sender,
            content,
            timestamp,
            roleName: String::new(),
            selectedVariantIndex: 0,
            variantCount: 1,
            provider: String::new(),
            modelName: String::new(),
            inputTokens: 0,
            outputTokens: 0,
            cachedInputTokens: 0,
            sentAt: 0,
            outputDurationMs: 0,
            waitDurationMs: 0,
            completedAt: 0,
            displayMode: ChatMessageDisplayMode::NORMAL,
            isFavorite: false,
            isVariantPreview: false,
        }
    }
}
