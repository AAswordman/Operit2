use serde::{Deserialize, Serialize};

/// Identifies one existing user-message record for a generic continuation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct ChatTurnContinuation {
    /// Contains the actual committed user timestamp, never a selected participant or namespace.
    pub userMessageTimestamp: i64,
}

/// Observes authoritative parsed message snapshots without controlling model execution.
pub trait ChatOutputObserver: std::fmt::Debug + Send + Sync {
    /// Replaces the entire semantic state of the identified assistant message.
    fn snapshot(&self, messageTimestamp: i64, parts: &[crate::MessagePart::MessagePart]);
    /// Reports an actual semantic observation failure without inventing a message part.
    fn failed(&self, error: String);
}

/// Controls one initial submit or an explicitly identified continuation through the same execution pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatTurnOptions {
    pub persistTurn: bool,
    pub notifyReply: Option<bool>,
    pub hideUserMessage: bool,
    pub disableWarning: bool,
    /// Identifies the existing committed user turn when this operation is a continuation.
    pub continuation: Option<ChatTurnContinuation>,
    /// Contains a native-generated execution identity that cannot be supplied by serialized caller options.
    #[serde(skip)]
    pub nativeExecutionId: Option<String>,
    /// Carries a native-only observer that cannot be provided through serialized request options.
    #[serde(skip)]
    pub outputObserver: Option<std::sync::Arc<dyn ChatOutputObserver>>,
    /// Keeps sequence-level lease ownership and completion notifications with the native transport.
    #[serde(skip)]
    pub deferSequenceCompletion: bool,
    #[serde(default)]
    pub chatInputSubmitRequestedHandled: bool,
}

impl Default for ChatTurnOptions {
    /// Creates ordinary initial-submit controls with no implicit continuation identity.
    fn default() -> Self {
        Self {
            persistTurn: true,
            notifyReply: None,
            hideUserMessage: false,
            disableWarning: false,
            continuation: None,
            nativeExecutionId: None,
            outputObserver: None,
            deferSequenceCompletion: false,
            chatInputSubmitRequestedHandled: false,
        }
    }
}
