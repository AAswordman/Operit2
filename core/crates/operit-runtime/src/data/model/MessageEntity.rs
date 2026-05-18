use serde::{Deserialize, Serialize};

use super::ChatMessage::ChatMessage;
use super::ChatMessageDisplayMode::ChatMessageDisplayMode;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEntity {
    pub messageId: i64,
    pub chatId: String,
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub orderIndex: i32,
    pub roleName: String,
    pub selectedVariantIndex: i32,
    pub provider: String,
    pub modelName: String,
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub cachedInputTokens: i32,
    pub sentAt: i64,
    pub outputDurationMs: i64,
    pub waitDurationMs: i64,
    pub completedAt: i64,
    pub displayMode: String,
    pub isFavorite: bool,
}

impl MessageEntity {
    pub fn toChatMessage(&self) -> ChatMessage {
        ChatMessage {
            sender: self.sender.clone(),
            content: self.content.clone(),
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
            content: message.content,
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
