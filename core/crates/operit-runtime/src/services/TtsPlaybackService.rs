#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::{AudioPlaybackHost, TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use operit_context::OperitApplicationContext::OperitApplicationContext;
use crate::data::model::TtsConfig::{
    TtsConfig, TtsHostPlaybackResult, TtsPlaybackResult, TtsProviderType,
};
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::TtsConfigManager::TtsConfigManager;
use crate::util::TtsCleaner::TtsCleaner;

#[derive(Clone)]
pub struct TtsPlaybackService {
    audioPlaybackHost: Option<Arc<dyn AudioPlaybackHost>>,
    ttsPlaybackHost: Option<Arc<dyn TtsPlaybackHost>>,
    characterCardManager: CharacterCardManager,
    ttsConfigManager: TtsConfigManager,
}

impl TtsPlaybackService {
    pub fn getInstance(context: &OperitApplicationContext) -> Result<Self, String> {
        let paths = RuntimeStorePaths::default();
        Ok(Self {
            audioPlaybackHost: context.audioPlaybackHost.clone(),
            ttsPlaybackHost: context.ttsPlaybackHost.clone(),
            characterCardManager: CharacterCardManager::new(paths.clone()),
            ttsConfigManager: TtsConfigManager::new(paths),
        })
    }

    pub fn playAudio(&self, path: &str) -> Result<TtsPlaybackResult, String> {
        let path = path.trim();
        if path.is_empty() {
            return Err("audio path is empty".to_string());
        }
        let audioPlaybackHost = self
            .audioPlaybackHost
            .clone()
            .ok_or_else(|| "AudioPlaybackHost is required for audio playback".to_string())?;
        let status = audioPlaybackHost
            .playAudio(path)
            .map_err(|error| error.to_string())?;
        Ok(TtsPlaybackResult {
            path: status.path,
            started: status.started,
            details: status.details,
        })
    }

    pub fn speakForCharacter(
        &self,
        characterCardId: &str,
        text: &str,
        interrupt: bool,
    ) -> Result<TtsHostPlaybackResult, String> {
        let characterCardId = characterCardId.trim();
        if characterCardId.is_empty() {
            return Err("character card id is empty".to_string());
        }
        let cleanedText = TtsCleaner::clean(text);
        if cleanedText.is_empty() {
            return Err("tts text is empty".to_string());
        }
        let card = self
            .characterCardManager
            .getCharacterCard(characterCardId)
            .map_err(|error| error.to_string())?;
        let config = match card.ttsConfigId.as_ref().map(|value| value.trim()) {
            Some(configId) if !configId.is_empty() => self
                .ttsConfigManager
                .getTtsConfig(configId)
                .map_err(|error| error.to_string())?,
            _ => self
                .ttsConfigManager
                .getCurrentTtsConfig()
                .map_err(|error| error.to_string())?,
        };
        self.speakWithResolvedConfig(config, cleanedText, interrupt)
    }

    pub fn speakWithConfig(
        &self,
        ttsConfigId: &str,
        text: &str,
        interrupt: bool,
    ) -> Result<TtsHostPlaybackResult, String> {
        let ttsConfigId = ttsConfigId.trim();
        if ttsConfigId.is_empty() {
            return Err("tts config id is empty".to_string());
        }
        let cleanedText = TtsCleaner::clean(text);
        if cleanedText.is_empty() {
            return Err("tts text is empty".to_string());
        }
        let config = self
            .ttsConfigManager
            .getTtsConfig(ttsConfigId)
            .map_err(|error| error.to_string())?;
        self.speakWithResolvedConfig(config, cleanedText, interrupt)
    }

    fn speakWithResolvedConfig(
        &self,
        config: TtsConfig,
        cleanedText: String,
        interrupt: bool,
    ) -> Result<TtsHostPlaybackResult, String> {
        let providerType = TtsProviderType::normalize(&config.providerType);
        if providerType != TtsProviderType::SYSTEM_TTS {
            return Err(format!(
                "tts playback host only accepts SYSTEM_TTS config: {providerType}"
            ));
        }
        let host = self.ttsPlaybackHost()?;
        let status = host
            .speakText(TtsPlaybackRequest {
                text: cleanedText,
                voice: config.voice,
                locale: config.model,
                speed: config.speed,
                pitch: 1.0,
                interrupt,
            })
            .map_err(|error| error.to_string())?;
        Ok(ttsHostPlaybackResult(status))
    }

    pub fn pauseSpeech(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .pauseSpeech()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    pub fn resumeSpeech(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .resumeSpeech()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    pub fn stopSpeech(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .stopSpeech()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    pub fn speechState(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .speechState()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    fn ttsPlaybackHost(&self) -> Result<Arc<dyn TtsPlaybackHost>, String> {
        self.ttsPlaybackHost
            .clone()
            .ok_or_else(|| "TtsPlaybackHost is required for system TTS playback".to_string())
    }
}

fn ttsHostPlaybackResult(status: TtsPlaybackStatus) -> TtsHostPlaybackResult {
    TtsHostPlaybackResult {
        path: status.path,
        active: status.active,
        paused: status.paused,
        details: status.details,
    }
}
