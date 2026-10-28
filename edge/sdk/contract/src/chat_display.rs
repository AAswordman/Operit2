//! Presentation contract shared by Core projections and constrained displays.
use serde::{Deserialize, Serialize};

/// A bounded display window; business messages and tools remain at the owning Core.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatDisplayWindow {
    pub messages: Vec<ChatDisplayMessage>,
    pub older: Option<ChatDisplayCursor>,
    pub error: Option<String>,
}

/// UTF-8 byte offset within one timestamped message's projected display text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatDisplayCursor {
    pub timestamp: i64,
    pub offset: u32,
}

/// Plain display text, including tool names rendered by Core as ordinary text.
#[allow(non_snake_case)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatDisplayMessage {
    pub sender: String,
    pub timestamp: i64,
    pub text: String,
    pub contentStream: Option<operit_link::CoreStream<ChatDisplayEvent>>,
}

/// Display edits and errors cannot represent calls, arguments, results or tool states.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum ChatDisplayEvent {
    Chunk { value: String },
    Replace { value: String },
    Error { message: String },
}
