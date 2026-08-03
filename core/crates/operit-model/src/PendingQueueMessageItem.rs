use serde::{Deserialize, Serialize};

/// Identifies one user message waiting for the active chat turn to finish.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingQueueMessageItem {
    pub id: i64,
    pub text: String,
}
