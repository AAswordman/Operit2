#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::{FileSystemHost, TtsSynthesisHost, TtsSynthesisRequest};

use crate::tts::VoiceService::VoiceService;
use operit_model::TtsConfig::TtsConfig;

pub struct SystemVoiceProvider {
    host: Arc<dyn TtsSynthesisHost>,
    fileSystemHost: Arc<dyn FileSystemHost>,
}

impl SystemVoiceProvider {
    /// Creates a system voice provider with explicit synthesis and file hosts.
    pub fn new(host: Arc<dyn TtsSynthesisHost>, fileSystemHost: Arc<dyn FileSystemHost>) -> Self {
        Self {
            host,
            fileSystemHost,
        }
    }
}

impl VoiceService for SystemVoiceProvider {
    fn synthesize(&self, config: &TtsConfig, text: &str) -> Result<Vec<u8>, String> {
        let response = self
            .host
            .synthesizeSpeech(TtsSynthesisRequest {
                text: text.to_string(),
                voice: config.voice.clone(),
                locale: config.model.clone(),
                speed: config.speed,
                pitch: 1.0,
                outputFormat: config.responseFormat.clone(),
            })
            .map_err(|error| error.to_string())?;
        self.fileSystemHost
            .readFileBytes(&response.audioPath)
            .map_err(|error| error.to_string())
    }
}
