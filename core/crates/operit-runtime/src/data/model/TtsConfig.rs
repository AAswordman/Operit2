use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsConfig {
    pub id: String,
    pub name: String,
    pub providerType: String,
    pub endpoint: String,
    pub apiKey: String,
    pub model: String,
    pub voice: String,
    pub responseFormat: String,
    pub speed: f64,
    pub enabled: bool,
    pub createdAt: i64,
    pub updatedAt: i64,
}

pub struct TtsProviderType;

impl TtsProviderType {
    pub const SYSTEM_TTS: &'static str = "SYSTEM_TTS";
    pub const OPENAI_COMPATIBLE: &'static str = "OPENAI_COMPATIBLE";

    pub fn normalize(providerType: &str) -> String {
        let trimmed = providerType.trim();
        if trimmed.eq_ignore_ascii_case(Self::SYSTEM_TTS) {
            Self::SYSTEM_TTS.to_string()
        } else if trimmed.eq_ignore_ascii_case(Self::OPENAI_COMPATIBLE) {
            Self::OPENAI_COMPATIBLE.to_string()
        } else {
            trimmed.to_string()
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsSynthesisResult {
    pub audioPaths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsPlaybackResult {
    pub path: String,
    pub started: bool,
    pub details: String,
}
