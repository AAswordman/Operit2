use serde::{Deserialize, Serialize};

use crate::ChatMessageDisplayMode::ChatMessageDisplayMode;
use crate::ChatMessageTimestampAllocator::ChatMessageTimestampAllocator;
use crate::MessagePart::MessagePart;
use crate::MessagePartCodec::MessagePartCodec;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: String,
    pub parts: Vec<MessagePart>,
    pub timestamp: i64,
    pub roleName: String,
    pub selectedVariantIndex: i32,
    pub variantCount: i32,
    pub provider: String,
    pub modelName: String,
    pub inputTokens: i64,
    pub outputTokens: i64,
    pub cachedInputTokens: i64,
    pub sentAt: i64,
    pub outputDurationMs: i64,
    pub waitDurationMs: i64,
    pub completedAt: i64,
    pub displayMode: ChatMessageDisplayMode,
    pub isFavorite: bool,
    #[serde(skip)]
    pub isVariantPreview: bool,
}

impl PartialEq for ChatMessage {
    fn eq(&self, other: &Self) -> bool {
        self.sender == other.sender
            && self.parts == other.parts
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
    }
}

impl ChatMessage {
    /// Returns text from parts rendered directly in the chat transcript.
    pub fn displayText(&self) -> String {
        MessagePartCodec::visibleText(&self.parts)
    }

    /// Serializes assistant parts at a text-only model protocol boundary.
    pub fn assistantProtocolMarkup(&self) -> String {
        MessagePartCodec::assistantMarkup(&self.parts)
    }

    /// Creates an empty message with one canonical Markdown part.
    pub fn new(sender: String) -> Self {
        Self::new_with_parts(
            sender,
            vec![MessagePart::markdown(
                "part-0".to_string(),
                0,
                String::new(),
            )],
        )
    }

    /// Creates a message containing one markdown part.
    pub fn new_with_markdown(sender: String, content: String) -> Self {
        Self::new_with_parts(
            sender,
            vec![MessagePart::markdown("part-0".to_string(), 0, content)],
        )
    }

    /// Creates a timestamped message containing one markdown part.
    pub fn new_with_markdown_timestamp(sender: String, content: String, timestamp: i64) -> Self {
        Self::new_with_timestamp(
            sender,
            vec![MessagePart::markdown("part-0".to_string(), 0, content)],
            timestamp,
        )
    }

    /// Replaces a user-authored message with one explicit markdown part.
    pub fn replace_with_markdown(&mut self, content: String) {
        self.parts = vec![MessagePart::markdown("part-0".to_string(), 0, content)];
    }

    /// Creates a new message with the supplied structured parts.
    pub fn new_with_parts(sender: String, parts: Vec<MessagePart>) -> Self {
        Self::new_with_timestamp(sender, parts, ChatMessageTimestampAllocator::next())
    }

    /// Creates a timestamped message with the supplied structured parts.
    pub fn new_with_timestamp(sender: String, parts: Vec<MessagePart>, timestamp: i64) -> Self {
        ChatMessageTimestampAllocator::observe(timestamp);
        Self {
            sender,
            parts,
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
