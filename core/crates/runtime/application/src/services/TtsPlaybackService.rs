#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::{TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::TtsConfigManager::TtsConfigManager;
use operit_host_api::HostManager::HostManager;
use operit_model::TtsConfig::{
    TtsConfig, TtsHostPlaybackResult, TtsPlaybackResult, TtsProviderType,
};
use operit_util::TtsCleaner::TtsCleaner;

#[derive(Clone)]
/// Coordinates generated audio playback and host TTS playback controls.
pub struct TtsPlaybackService {
    ttsPlaybackHost: Option<Arc<dyn TtsPlaybackHost>>,
    characterCardManager: CharacterCardManager,
    ttsConfigManager: TtsConfigManager,
}

impl TtsPlaybackService {
    /// Creates a playback service from application host context.
    pub fn getInstance(context: &HostManager) -> Result<Self, String> {
        let paths = RuntimeStorePaths::default();
        Ok(Self {
            ttsPlaybackHost: context.ttsPlaybackHost.clone(),
            characterCardManager: CharacterCardManager::new(paths.clone()),
            ttsConfigManager: TtsConfigManager::new(paths),
        })
    }

    /// Plays a generated speech file through the TTS playback host.
    pub fn playAudio(&self, path: &str) -> Result<TtsPlaybackResult, String> {
        let path = path.trim();
        if path.is_empty() {
            return Err("audio path is empty".to_string());
        }
        let status = self
            .ttsPlaybackHost()?
            .playAudio(path)
            .map_err(|error| error.to_string())?;
        Ok(TtsPlaybackResult {
            path: status.path,
            started: status.active,
            details: status.details,
        })
    }

    /// Speaks text with the TTS configuration bound to a character card.
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

    /// Speaks text with a selected TTS configuration.
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

    /// Starts system speech with one already resolved TTS configuration.
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

    /// Pauses host speech playback.
    pub fn pauseSpeech(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .pauseSpeech()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    /// Resumes host speech playback.
    pub fn resumeSpeech(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .resumeSpeech()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    /// Stops host speech playback.
    pub fn stopSpeech(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .stopSpeech()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    /// Reads current host speech playback state.
    pub fn speechState(&self) -> Result<TtsHostPlaybackResult, String> {
        self.ttsPlaybackHost()?
            .speechState()
            .map(ttsHostPlaybackResult)
            .map_err(|error| error.to_string())
    }

    /// Returns the required platform TTS playback host.
    fn ttsPlaybackHost(&self) -> Result<Arc<dyn TtsPlaybackHost>, String> {
        self.ttsPlaybackHost
            .clone()
            .ok_or_else(|| "TtsPlaybackHost is required for TTS playback".to_string())
    }
}

/// Converts a host playback status into the Core proxy result model.
fn ttsHostPlaybackResult(status: TtsPlaybackStatus) -> TtsHostPlaybackResult {
    TtsHostPlaybackResult {
        path: status.path,
        active: status.active,
        paused: status.paused,
        details: status.details,
    }
}
