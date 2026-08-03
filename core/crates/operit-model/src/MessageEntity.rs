use serde::{Deserialize, Serialize};

use super::ChatMessage::ChatMessage;
use super::ChatMessageDisplayMode::ChatMessageDisplayMode;
use super::MessagePart::MessagePart;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEntity {
    pub messageId: i64,
    pub chatId: String,
    pub sender: String,
    pub timestamp: i64,
    pub orderIndex: i32,
    pub roleName: String,
    pub selectedVariantIndex: i32,
    pub provider: String,
    pub modelName: String,
    pub inputTokens: i64,
    pub outputTokens: i64,
    pub cachedInputTokens: i64,
    pub sentAt: i64,
    pub outputDurationMs: i64,
    pub waitDurationMs: i64,
    pub completedAt: i64,
    pub displayMode: String,
    pub isFavorite: bool,
}

impl MessageEntity {
    /// Builds a chat message from stored metadata and ordered parts.
    pub fn toChatMessage(&self, parts: Vec<MessagePart>) -> ChatMessage {
        ChatMessage {
            sender: self.sender.clone(),
            parts,
            timestamp: self.timestamp,
            roleName: self.roleName.clone(),
            selectedVariantIndex: self.selectedVariantIndex,
            variantCount: 1,
            provider: self.provider.clone(),
            modelName: self.modelName.clone(),
            inputTokens: self.inputTokens,
            outputTokens: self.outputTokens,
            cachedInputTokens: self.cachedInputTokens,
            sentAt: self.sentAt,
            outputDurationMs: self.outputDurationMs,
            waitDurationMs: self.waitDurationMs,
            completedAt: self.completedAt,
            displayMode: match self.displayMode.as_str() {
                "NORMAL" => ChatMessageDisplayMode::NORMAL,
                "HIDDEN_PLACEHOLDER" => ChatMessageDisplayMode::HIDDEN_PLACEHOLDER,
                other => panic!("unknown ChatMessageDisplayMode: {other}"),
            },
            isFavorite: self.isFavorite,
            isVariantPreview: false,
        }
    }

    pub fn fromChatMessage(
        chatId: String,
        message: ChatMessage,
        orderIndex: i32,
        messageId: i64,
    ) -> Self {
        Self {
            messageId,
            chatId,
            sender: message.sender,
            timestamp: message.timestamp,
            orderIndex,
            roleName: message.roleName,
            selectedVariantIndex: message.selectedVariantIndex,
            provider: message.provider,
            modelName: message.modelName,
            inputTokens: message.inputTokens,
            outputTokens: message.outputTokens,
            cachedInputTokens: message.cachedInputTokens,
            sentAt: message.sentAt,
            outputDurationMs: message.outputDurationMs,
            waitDurationMs: message.waitDurationMs,
            completedAt: message.completedAt,
            displayMode: format!("{:?}", message.displayMode),
            isFavorite: message.isFavorite,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessageCount {
    pub chatId: String,
    pub count: i32,
}
