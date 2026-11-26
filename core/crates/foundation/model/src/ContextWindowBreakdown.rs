use serde::{Deserialize, Serialize};

/// Token breakdown across major prompt/context categories for a prepared request window.
#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextWindowBreakdown {
    pub systemPromptTokens: i64,
    pub toolTokens: i64,
    pub fileTokens: i64,
    pub toolResultTokens: i64,
    pub conversationTokens: i64,
}
