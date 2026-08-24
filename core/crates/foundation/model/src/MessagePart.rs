use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Identifies the semantic role of one ordered message part.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagePartKind {
    Markdown,
    Thinking,
    ToolCall,
    ToolResult,
    Status,
}

/// Stores one semantic unit of a chat message without embedding protocol markup in text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagePart {
    pub partId: String,
    pub sequence: i32,
    pub kind: MessagePartKind,
    pub content: String,
    pub toolCallId: Option<String>,
    pub toolName: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

impl MessagePart {
    /// Creates a markdown part at the supplied position.
    pub fn markdown(partId: String, sequence: i32, content: String) -> Self {
        Self::new(partId, sequence, MessagePartKind::Markdown, content)
    }

    /// Creates a thinking part at the supplied position.
    pub fn thinking(partId: String, sequence: i32, content: String) -> Self {
        Self::new(partId, sequence, MessagePartKind::Thinking, content)
    }

    /// Creates a status part with its XML attributes represented as fields.
    pub fn status(
        partId: String,
        sequence: i32,
        content: String,
        attributes: BTreeMap<String, String>,
    ) -> Self {
        let mut part = Self::new(partId, sequence, MessagePartKind::Status, content);
        part.attributes = attributes;
        part
    }

    /// Creates a tool-call part with tool parameters represented as attributes.
    pub fn toolCall(
        partId: String,
        sequence: i32,
        toolCallId: String,
        toolName: String,
        parameters: BTreeMap<String, String>,
    ) -> Self {
        Self {
            toolCallId: Some(toolCallId),
            toolName: Some(toolName),
            attributes: parameters,
            ..Self::new(partId, sequence, MessagePartKind::ToolCall, String::new())
        }
    }

    /// Creates a tool-result part with structured status and payload fields.
    pub fn toolResult(
        partId: String,
        sequence: i32,
        toolCallId: Option<String>,
        toolName: String,
        status: String,
        content: String,
    ) -> Self {
        let attributes = BTreeMap::from([("status".to_string(), status)]);
        Self {
            toolCallId,
            toolName: Some(toolName),
            attributes,
            ..Self::new(partId, sequence, MessagePartKind::ToolResult, content)
        }
    }

    /// Creates a semantic message part.
    pub fn new(partId: String, sequence: i32, kind: MessagePartKind, content: String) -> Self {
        Self {
            partId,
            sequence,
            kind,
            content,
            toolCallId: None,
            toolName: None,
            attributes: BTreeMap::new(),
        }
    }
}
