use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MemoryScoreMode {
    BALANCED,
    KEYWORD_FIRST,
    SEMANTIC_FIRST,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MemorySearchConfig {
    pub scoreMode: MemoryScoreMode,
    pub keywordWeight: f32,
    pub tagWeight: f32,
    pub vectorWeight: f32,
    pub edgeWeight: f32,
}

impl Default for MemorySearchConfig {
    fn default() -> Self {
        Self {
            scoreMode: MemoryScoreMode::BALANCED,
            keywordWeight: 1.0,
            tagWeight: 0.7,
            vectorWeight: 1.0,
            edgeWeight: 0.5,
        }
    }
}
