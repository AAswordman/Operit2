#![allow(non_snake_case)]

use std::fs;

use uuid::Uuid;

use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use crate::api::voice::VoiceServiceFactory::VoiceServiceFactory;
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::data::model::TtsConfig::TtsSynthesisResult;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::TtsConfigManager::TtsConfigManager;
use crate::util::TtsCleaner::TtsCleaner;
use crate::util::TtsSegmenter::TtsSegmenter;

#[derive(Clone)]
pub struct TtsSynthesisService {
    paths: RuntimeStorePaths,
    characterCardManager: CharacterCardManager,
    ttsConfigManager: TtsConfigManager,
    context: Option<OperitApplicationContext>,
}

impl TtsSynthesisService {
    pub fn getInstance(context: &OperitApplicationContext) -> Self {
        let paths = RuntimeStorePaths::default();
        Self::newWithContext(paths, context.clone())
    }

    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            characterCardManager: CharacterCardManager::new(paths.clone()),
            ttsConfigManager: TtsConfigManager::new(paths.clone()),
            paths,
            context: None,
        }
    }

    pub fn newWithContext(paths: RuntimeStorePaths, context: OperitApplicationContext) -> Self {
        Self {
            characterCardManager: CharacterCardManager::new(paths.clone()),
            ttsConfigManager: TtsConfigManager::new(paths.clone()),
            paths,
            context: Some(context),
        }
    }

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
        let configId = card
            .ttsConfigId
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("character card has no tts config: {characterCardId}"))?;
        let config = self
            .ttsConfigManager
            .getTtsConfig(configId)
            .map_err(|error| error.to_string())?;
        if !config.enabled {
            return Err(format!("tts config is disabled: {}", config.id));
        }
        let voiceService = VoiceServiceFactory::createVoiceService(&config, self.context.as_ref())?;
        let segments = TtsSegmenter::segment(&cleanedText, 4000);
        let audioDir = self.paths.tts_audio_dir();
        fs::create_dir_all(&audioDir).map_err(|error| error.to_string())?;
        let mut audioPaths = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            let bytes = voiceService.synthesize(&config, segment)?;
            let extension = voiceService.outputExtension(&config)?;
            let fileName = format!(
                "{}_{}_{}.{}",
                characterCardId,
                Uuid::new_v4(),
                index,
                extension
            );
            let path = audioDir.join(fileName);
            fs::write(&path, bytes).map_err(|error| error.to_string())?;
            audioPaths.push(path.to_string_lossy().to_string());
        }
        Ok(TtsSynthesisResult { audioPaths })
    }
}
