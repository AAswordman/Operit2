#![allow(non_snake_case)]

use uuid::Uuid;

use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::RuntimeStorageLayout::RUNTIME_TTS_AUDIO_DIR_PATH;

use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::TtsConfigManager::TtsConfigManager;
use crate::services::LocalProviderService::LocalProviderService;
use operit_host_api::HostManager::HostManager;
use operit_local_models::LocalInference::LocalModelSelection;
use operit_model::TtsConfig::{TtsConfig, TtsProviderType, TtsSynthesisResult};
use operit_providers::tts::VoiceService::VoiceService;
use operit_providers::tts::VoiceServiceFactory::VoiceServiceFactory;
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

/// Adapts the runtime local model provider to the shared TTS voice contract.
struct LocalModelVoiceProvider {
    service: LocalProviderService,
    model: LocalModelSelection,
}

impl LocalModelVoiceProvider {
    /// Creates a voice provider for one exact installed local model binding.
    fn new(config: &TtsConfig, context: &HostManager) -> Result<Self, String> {
        let service = LocalProviderService::getInstance(context)?;
        let model = LocalProviderService::parseModelBinding(config.model.clone())?;
        Ok(Self { service, model })
    }
}

impl VoiceService for LocalModelVoiceProvider {
    /// Synthesizes one text segment through the installed local TTS engine.
    fn synthesize(&self, config: &TtsConfig, text: &str) -> Result<Vec<u8>, String> {
        let response = self.service.synthesizeAudio(
            self.model.clone(),
            text.to_string(),
            config.voice.clone(),
            config.speed,
        )?;
        if response.outputFormat != "wav" {
            return Err(format!(
                "LOCAL_MODEL TTS returned unsupported format: {}",
                response.outputFormat
            ));
        }
        Ok(response.audioBytes)
    }
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

    /// Synthesizes cleaned text and writes every generated segment to runtime storage.
    fn synthesizeWithResolvedConfig(
        &self,
        audioNamePrefix: &str,
        config: &TtsConfig,
        cleanedText: &str,
    ) -> Result<TtsSynthesisResult, String> {
        let voiceService = self.createVoiceService(config)?;
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
            let path = self.paths.runtime_storage_path(&storagePath);
            audioPaths.push(path.to_string_lossy().to_string());
            audioStoragePaths.push(storagePath);
        }
        Ok(TtsSynthesisResult {
            audioPaths,
            audioStoragePaths,
        })
    }

    /// Creates the provider adapter selected by one normalized TTS configuration.
    fn createVoiceService(&self, config: &TtsConfig) -> Result<Box<dyn VoiceService>, String> {
        let providerType = TtsProviderType::normalize(&config.providerType);
        match providerType.as_str() {
            TtsProviderType::LOCAL_MODEL => {
                let context = self.context.as_ref().ok_or_else(|| {
                    "HostManager is required for LOCAL_MODEL TTS synthesis".to_string()
                })?;
                Ok(Box::new(LocalModelVoiceProvider::new(config, context)?))
            }
            _ => VoiceServiceFactory::createVoiceService(config, self.context.as_ref()),
        }
    }
}
