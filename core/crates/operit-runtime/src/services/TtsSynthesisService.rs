#![allow(non_snake_case)]

use uuid::Uuid;

use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::RuntimeStorageLayout::RUNTIME_TTS_AUDIO_DIR_PATH;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use operit_providers::voice::VoiceServiceFactory::VoiceServiceFactory;
use operit_host_api::HostManager::HostManager;
use operit_model::TtsConfig::{TtsConfig, TtsSynthesisResult};
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::TtsConfigManager::TtsConfigManager;
use operit_util::TtsCleaner::TtsCleaner;
use operit_util::TtsSegmenter::TtsSegmenter;

#[derive(Clone)]
/// Synthesizes speech audio using configured character or TTS profiles.
pub struct TtsSynthesisService {
    paths: RuntimeStorePaths,
    characterCardManager: CharacterCardManager,
    ttsConfigManager: TtsConfigManager,
    context: Option<HostManager>,
}

impl TtsSynthesisService {
    /// Creates a synthesis service from application host context.
    pub fn getInstance(context: &HostManager) -> Self {
        let paths = RuntimeStorePaths::default();
        Self::newWithContext(paths, context.clone())
    }

    /// Creates a synthesis service for explicit runtime paths.
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            characterCardManager: CharacterCardManager::new(paths.clone()),
            ttsConfigManager: TtsConfigManager::new(paths.clone()),
            paths,
            context: None,
        }
    }

    /// Creates a synthesis service with runtime paths and host context.
    pub fn newWithContext(paths: RuntimeStorePaths, context: HostManager) -> Self {
        Self {
            characterCardManager: CharacterCardManager::new(paths.clone()),
            ttsConfigManager: TtsConfigManager::new(paths.clone()),
            paths,
            context: Some(context),
        }
    }

    /// Synthesizes text with the TTS configuration bound to a character card.
    pub fn synthesizeForCharacter(
        &self,
        characterCardId: &str,
        text: &str,
    ) -> Result<TtsSynthesisResult, String> {
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
        self.synthesizeWithResolvedConfig(characterCardId, &config, &cleanedText)
    }

    /// Synthesizes text with a selected TTS configuration.
    pub fn synthesizeWithConfig(
        &self,
        ttsConfigId: &str,
        text: &str,
    ) -> Result<TtsSynthesisResult, String> {
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
        self.synthesizeWithResolvedConfig(ttsConfigId, &config, &cleanedText)
    }

    fn synthesizeWithResolvedConfig(
        &self,
        audioNamePrefix: &str,
        config: &TtsConfig,
        cleanedText: &str,
    ) -> Result<TtsSynthesisResult, String> {
        let voiceService = VoiceServiceFactory::createVoiceService(config, self.context.as_ref())?;
        let segments = TtsSegmenter::segment(&cleanedText, 4000);
        let storageHost = defaultRuntimeStorageHost();
        let mut audioPaths = Vec::new();
        let mut audioStoragePaths = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            let bytes = voiceService.synthesize(config, segment)?;
            let extension = voiceService.outputExtension(config)?;
            let fileName = format!(
                "{}_{}_{}.{}",
                audioNamePrefix,
                Uuid::new_v4(),
                index,
                extension
            );
            let storagePath = format!("{RUNTIME_TTS_AUDIO_DIR_PATH}/{fileName}");
            storageHost
                .writeBytes(&storagePath, &bytes)
                .map_err(|error| error.message)?;
            let path = self.paths.root_dir().join(&storagePath);
            audioPaths.push(path.to_string_lossy().to_string());
            audioStoragePaths.push(storagePath);
        }
        Ok(TtsSynthesisResult {
            audioPaths,
            audioStoragePaths,
        })
    }
}
