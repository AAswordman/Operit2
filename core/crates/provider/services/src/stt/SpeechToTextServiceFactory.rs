use operit_model::SttConfig::{SttConfig, SttProviderType};

use crate::stt::HttpSpeechToTextProvider::HttpSpeechToTextProvider;
use crate::stt::SpeechToTextService::SpeechToTextService;

pub struct SpeechToTextServiceFactory;

impl SpeechToTextServiceFactory {
    /// Creates the remote provider implementation selected by one STT configuration.
    pub fn createService(config: &SttConfig) -> Result<Box<dyn SpeechToTextService>, String> {
        let providerType = SttProviderType::normalize(&config.providerType);
        if providerType == SttProviderType::LOCAL_MODEL {
            return Err("LOCAL_MODEL STT is created by the runtime local provider".to_string());
        }
        Ok(Box::new(HttpSpeechToTextProvider::new()))
    }
}
