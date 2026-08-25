use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::MessagePart::{MessagePart, MessagePartKind};

/// Persists one structured message part for a selected message revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagePartEntity {
    pub chatId: String,
    pub messageTimestamp: i64,
    pub variantIndex: i32,
    pub partId: String,
    pub sequence: i32,
    pub kind: MessagePartKind,
    pub content: String,
    pub toolCallId: Option<String>,
    pub toolName: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

impl MessagePartEntity {
    /// Converts the stored row into the shared message-part model.
    pub fn toMessagePart(&self) -> MessagePart {
        MessagePart {
            partId: self.partId.clone(),
            sequence: self.sequence,
            kind: self.kind.clone(),
            content: self.content.clone(),
            toolCallId: self.toolCallId.clone(),
            toolName: self.toolName.clone(),
            attributes: self.attributes.clone(),
        }
    }

    /// Converts a message part into a row scoped to one message revision.
    pub fn fromMessagePart(
        chatId: String,
        messageTimestamp: i64,
        variantIndex: i32,
        part: MessagePart,
    ) -> Self {
        Self {
            chatId,
            messageTimestamp,
            variantIndex,
            partId: part.partId,
            sequence: part.sequence,
            kind: part.kind,
            content: part.content,
            toolCallId: part.toolCallId,
            toolName: part.toolName,
            attributes: part.attributes,
        }
    }
}
