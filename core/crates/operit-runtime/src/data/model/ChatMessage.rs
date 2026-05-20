use serde::{Deserialize, Serialize};

use crate::api::chat::llmprovider::AIService::SharedAiResponseStream;
use crate::util::stream::HotStream::SharedStream;
use super::ChatMessageDisplayMode::ChatMessageDisplayMode;
use super::ChatMessageTimestampAllocator::ChatMessageTimestampAllocator;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    #[serde(skip)]
    pub contentStream: Option<SharedAiResponseStream>,
}

impl PartialEq for ChatMessage {
    fn eq(&self, other: &Self) -> bool {
        let sameContentStream = match (&self.contentStream, &other.contentStream) {
            (Some(left), Some(right)) => left.replay_cache() == right.replay_cache(),
            (None, None) => true,
            _ => false,
        };
        self.sender == other.sender
            && self.content == other.content
            && self.timestamp == other.timestamp
            && self.roleName == other.roleName
            && self.selectedVariantIndex == other.selectedVariantIndex
            && self.variantCount == other.variantCount
            && self.provider == other.provider
            && self.modelName == other.modelName
            && self.inputTokens == other.inputTokens
            && self.outputTokens == other.outputTokens
            && self.cachedInputTokens == other.cachedInputTokens
            && self.sentAt == other.sentAt
            && self.outputDurationMs == other.outputDurationMs
            && self.waitDurationMs == other.waitDurationMs
            && self.completedAt == other.completedAt
            && self.displayMode == other.displayMode
            && self.isFavorite == other.isFavorite
            && self.isVariantPreview == other.isVariantPreview
            && sameContentStream
    }
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
            contentStream: None,
        }
    }
}
