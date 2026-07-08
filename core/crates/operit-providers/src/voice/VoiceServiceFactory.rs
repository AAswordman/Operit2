#![allow(non_snake_case)]

use crate::voice::HttpVoiceProvider::HttpVoiceProvider;
use crate::voice::SystemVoiceProvider::SystemVoiceProvider;
use crate::voice::VoiceService::VoiceService;
use operit_host_api::HostManager::HostManager;
use operit_model::TtsCatalog::TtsCatalog;
use operit_model::TtsConfig::{TtsConfig, TtsProviderType};

pub struct VoiceServiceFactory;

impl VoiceServiceFactory {
    pub fn createVoiceService(
        config: &TtsConfig,
        context: Option<&HostManager>,
    ) -> Result<Box<dyn VoiceService>, String> {
        let providerType = TtsProviderType::normalize(&config.providerType);
        match providerType.as_str() {
            TtsProviderType::SYSTEM_TTS => {
                let host = context
                    .and_then(|context| context.ttsSynthesisHost.clone())
                    .ok_or_else(|| "TtsSynthesisHost is required for SYSTEM_TTS".to_string())?;
                Ok(Box::new(SystemVoiceProvider::new(host)))
            }
            _ => {
                TtsCatalog::provider(&providerType)?;
                Ok(Box::new(HttpVoiceProvider::new()))
            }
        }
    }
}
