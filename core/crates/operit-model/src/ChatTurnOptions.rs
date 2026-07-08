use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatTurnOptions {
    pub persistTurn: bool,
    pub notifyReply: Option<bool>,
    pub hideUserMessage: bool,
    pub disableWarning: bool,
}

impl Default for ChatTurnOptions {
    fn default() -> Self {
        Self {
            persistTurn: true,
            notifyReply: None,
            hideUserMessage: false,
            disableWarning: false,
        }
    }
}
