#![allow(non_snake_case)]

use crate::data::model::TtsConfig::TtsConfig;

pub trait VoiceService: Send + Sync {
    fn synthesize(&self, config: &TtsConfig, text: &str) -> Result<Vec<u8>, String>;

    fn outputExtension(&self, config: &TtsConfig) -> Result<&'static str, String> {
        normalizedAudioExtension(&config.responseFormat)
    }
}

pub fn normalizedAudioExtension(responseFormat: &str) -> Result<&'static str, String> {
    let trimmed = responseFormat.trim();
    match trimmed {
        "mp3" => Ok("mp3"),
        "opus" => Ok("opus"),
        "aac" => Ok("aac"),
        "flac" => Ok("flac"),
        "wav" => Ok("wav"),
        "pcm" => Ok("pcm"),
        value => match value.split_once('_').map(|(prefix, _)| prefix) {
            Some("mp3") => Ok("mp3"),
            Some("opus") => Ok("opus"),
            Some("aac") => Ok("aac"),
            Some("flac") => Ok("flac"),
            Some("wav") => Ok("wav"),
            Some("pcm") => Ok("pcm"),
            _ => Err(format!("unsupported tts response format: {value}")),
        },
    }
}
