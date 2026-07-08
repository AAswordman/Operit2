use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromptTag {
    pub id: String,
    pub name: String,
    pub description: String,
    pub promptContent: String,
    pub tagType: TagType,
    pub createdAt: i64,
    pub updatedAt: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TagType {
    TONE,
    CHARACTER,
    FUNCTION,
    CUSTOM,
}

impl Default for TagType {
    fn default() -> Self {
        Self::CUSTOM
    }
}
