#![allow(non_snake_case)]

use std::fs;
use std::sync::Arc;

use operit_host_api::{TtsSynthesisHost, TtsSynthesisRequest};

use crate::tts::VoiceService::VoiceService;
use operit_model::TtsConfig::TtsConfig;

pub struct SystemVoiceProvider {
    host: Arc<dyn TtsSynthesisHost>,
}

impl SystemVoiceProvider {
    pub fn new(host: Arc<dyn TtsSynthesisHost>) -> Self {
        Self { host }
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
        fs::read(&response.audioPath).map_err(|error| error.to_string())
    }
}
