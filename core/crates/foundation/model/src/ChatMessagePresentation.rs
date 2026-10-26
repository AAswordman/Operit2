use serde::{Deserialize, Serialize};

use crate::MessagePart::MessagePart;

/// Identifies the native assistant presentation namespace in persisted message extensions.
pub const PRESENTATION_EXTENSION: &str = "host.response.presentation";

/// Stores only delivered display bubbles alongside the canonical assistant response.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatMessagePresentation {
    pub bubbles: Vec<Vec<MessagePart>>,
    pub pending: bool,
}
