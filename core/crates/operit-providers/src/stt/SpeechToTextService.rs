#![allow(non_snake_case)]

use operit_model::SttConfig::{SttConfig, SttRecognitionResult};

pub trait SpeechToTextService: Send + Sync {
    /// Transcribes one in-memory audio payload with the supplied provider configuration.
    fn transcribe(
        &self,
        config: &SttConfig,
        audioBytes: &[u8],
        fileName: &str,
        contentType: &str,
        language: Option<&str>,
    ) -> Result<SttRecognitionResult, String>;
}
