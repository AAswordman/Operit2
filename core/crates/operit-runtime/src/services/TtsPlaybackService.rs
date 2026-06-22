#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::AudioPlaybackHost;

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::data::model::TtsConfig::TtsPlaybackResult;

#[derive(Clone)]
pub struct TtsPlaybackService {
    audioPlaybackHost: Arc<dyn AudioPlaybackHost>,
}

impl TtsPlaybackService {
    pub fn getInstance(context: &OperitApplicationContext) -> Result<Self, String> {
        let audioPlaybackHost = context
            .audioPlaybackHost
            .clone()
            .ok_or_else(|| "AudioPlaybackHost is required for TTS playback".to_string())?;
        Ok(Self { audioPlaybackHost })
    }

    pub fn playAudio(&self, path: &str) -> Result<TtsPlaybackResult, String> {
        let path = path.trim();
        if path.is_empty() {
            return Err("audio path is empty".to_string());
        }
        let status = self
            .audioPlaybackHost
            .playAudio(path)
            .map_err(|error| error.to_string())?;
        Ok(TtsPlaybackResult {
            path: status.path,
            started: status.started,
            details: status.details,
        })
    }
}
