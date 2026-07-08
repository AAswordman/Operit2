use serde::{Deserialize, Serialize};

use super::ChatMessage::ChatMessage;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageVariantEntity {
    pub variantId: i64,
    pub chatId: String,
    pub messageTimestamp: i64,
    pub variantIndex: i32,
    pub content: String,
    pub roleName: String,
    pub provider: String,
    pub modelName: String,
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub cachedInputTokens: i32,
    pub sentAt: i64,
    pub outputDurationMs: i64,
    pub waitDurationMs: i64,
    pub completedAt: i64,
}

impl MessageVariantEntity {
    pub fn applyTo(&self, baseMessage: ChatMessage, variantCount: i32) -> ChatMessage {
        ChatMessage {
            content: self.content.clone(),
            roleName: if self.roleName.is_empty() {
                baseMessage.roleName
            } else {
                self.roleName.clone()
            },
            selectedVariantIndex: self.variantIndex,
            variantCount,
            provider: self.provider.clone(),
            modelName: self.modelName.clone(),
            inputTokens: self.inputTokens,
            outputTokens: self.outputTokens,
            cachedInputTokens: self.cachedInputTokens,
            sentAt: self.sentAt,
            outputDurationMs: self.outputDurationMs,
            waitDurationMs: self.waitDurationMs,
            completedAt: self.completedAt,
            ..baseMessage
        }
    }

    pub fn fromChatMessage(
        chatId: String,
        messageTimestamp: i64,
        variantIndex: i32,
        message: ChatMessage,
        variantId: i64,
    ) -> MessageVariantEntity {
        MessageVariantEntity {
            variantId,
            chatId,
            messageTimestamp,
            variantIndex,
            content: message.content,
            roleName: message.roleName,
            provider: message.provider,
            modelName: message.modelName,
            inputTokens: message.inputTokens,
            outputTokens: message.outputTokens,
            cachedInputTokens: message.cachedInputTokens,
            sentAt: message.sentAt,
            outputDurationMs: message.outputDurationMs,
            waitDurationMs: message.waitDurationMs,
            completedAt: message.completedAt,
        }
    }
}
