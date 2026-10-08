use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatDisplayWindowState {
    pub hasOlderDisplayHistory: bool,
    pub hasNewerDisplayHistory: bool,
    pub isLoadingDisplayWindow: bool,
}

/// A bounded, volatile display projection; the complete conversation stays on Core.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatDisplayWindow {
    pub messages: Vec<ChatDisplayMessage>,
    pub older: Option<ChatDisplayCursor>,
    pub error: Option<String>,
}

/// UTF-8 byte offset within the visible text of a timestamped message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatDisplayCursor {
    pub timestamp: i64,
    pub offset: u32,
}

/// Presentation-only fields: token statistics, variants and raw tool parts do
/// not belong in a small display's watch snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatDisplayMessage {
    pub sender: String,
    pub timestamp: i64,
    pub text: String,
    pub contentStream: Option<operit_link::CoreStream<operit_util::MarkdownRenderStream::MarkdownStreamEvent>>,
}
