use serde::{Deserialize, Serialize};

/// Identifies one persisted conversation or one exact message revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum PluginExtensionTarget {
    #[serde(rename = "chat")]
    Chat { chatId: String },
    #[serde(rename = "message")]
    Message {
        chatId: String,
        messageTimestamp: i64,
        variantIndex: i32,
    },
}

impl PluginExtensionTarget {
    /// Returns the owning chat identifier without interpreting any plugin namespace.
    pub fn chatId(&self) -> &str {
        match self {
            Self::Chat { chatId } | Self::Message { chatId, .. } => chatId,
        }
    }

    /// Rejects empty chat identifiers and invalid revision indexes before storage access.
    pub fn validate(&self) -> Result<(), String> {
        if self.chatId().trim().is_empty() {
            return Err("Plugin extension target requires a nonempty chatId".to_string());
        }
        if let Self::Message { variantIndex, .. } = self {
            if *variantIndex < 0 {
                return Err("Plugin extension target variantIndex must be nonnegative".to_string());
            }
        }
        Ok(())
    }
}
