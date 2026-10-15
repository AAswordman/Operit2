//! Speech recognition performed on the plugin's host.
use super::results::SttTranscribeResultData;
use super::JsFuture;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct SttTranscribeRequest {
    /// Encoded audio file bytes, such as WAV; raw PCM must first be framed.
    pub audio: Vec<u8>,
    pub fileName: String,
    pub contentType: String,
    pub language: Option<String>,
    /// Uses the selected host configuration when omitted.
    pub configId: Option<String>,
}
pub trait SttHost: Send + Sync {
    fn transcribe(&self, request: SttTranscribeRequest) -> JsFuture<SttTranscribeResultData>;
}
