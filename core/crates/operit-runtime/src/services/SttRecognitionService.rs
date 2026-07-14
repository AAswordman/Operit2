#![allow(non_snake_case)]

use std::fs;
use std::path::PathBuf;

use operit_host_api::HostManager::HostManager;
use operit_model::SttConfig::{SttConfig, SttProviderType, SttRecognitionResult};
use operit_providers::stt::SpeechToTextServiceFactory::SpeechToTextServiceFactory;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::RuntimeStorageLayout::RUNTIME_CLEAN_ON_EXIT_DIR_PATH;
use uuid::Uuid;

use crate::data::preferences::SttConfigManager::SttConfigManager;
use crate::services::LocalProviderService::LocalProviderService;

#[derive(Clone)]
/// Transcribes audio through the configured local or remote STT provider.
pub struct SttRecognitionService {
    configManager: SttConfigManager,
    context: HostManager,
    temporaryAudioDirectory: PathBuf,
}

impl SttRecognitionService {
    /// Creates an STT recognition service from application host context.
    pub fn getInstance(context: &HostManager) -> Self {
        Self::newWithContext(RuntimeStorePaths::default(), context.clone())
    }

    /// Creates an STT recognition service with explicit runtime paths and host context.
    pub fn newWithContext(paths: RuntimeStorePaths, context: HostManager) -> Self {
        let temporaryAudioDirectory = paths.runtime_storage_path(RUNTIME_CLEAN_ON_EXIT_DIR_PATH);
        Self {
            configManager: SttConfigManager::new(paths),
            context,
            temporaryAudioDirectory,
        }
    }

    /// Transcribes one in-memory audio payload with the selected STT configuration.
    pub fn transcribeCurrent(
        &self,
        audioBytes: Vec<u8>,
        fileName: String,
        contentType: String,
        language: Option<String>,
    ) -> Result<SttRecognitionResult, String> {
        let config = self.configManager.getCurrentSttConfig()?;
        self.transcribeWithConfig(config, audioBytes, fileName, contentType, language)
    }

    /// Transcribes one in-memory audio payload with an explicit STT configuration id.
    pub fn transcribeWithConfigId(
        &self,
        configId: String,
        audioBytes: Vec<u8>,
        fileName: String,
        contentType: String,
        language: Option<String>,
    ) -> Result<SttRecognitionResult, String> {
        let config = self.configManager.getSttConfig(&configId)?;
        self.transcribeWithConfig(config, audioBytes, fileName, contentType, language)
    }

    /// Routes one STT request to its configured provider implementation.
    fn transcribeWithConfig(
        &self,
        config: SttConfig,
        audioBytes: Vec<u8>,
        fileName: String,
        contentType: String,
        language: Option<String>,
    ) -> Result<SttRecognitionResult, String> {
        if audioBytes.is_empty() {
            return Err("STT audio payload is empty".to_string());
        }
        let providerType = SttProviderType::normalize(&config.providerType);
        if providerType == SttProviderType::LOCAL_MODEL {
            let response = self.transcribeLocal(config.model, audioBytes, contentType, language)?;
            return Ok(SttRecognitionResult {
                text: response.text,
            });
        }
        SpeechToTextServiceFactory::createService(&config)?.transcribe(
            &config,
            &audioBytes,
            &fileName,
            &contentType,
            language.as_deref(),
        )
    }

    /// Runs local STT from one runtime-owned temporary WAV file and removes it afterward.
    fn transcribeLocal(
        &self,
        modelBinding: String,
        audioBytes: Vec<u8>,
        contentType: String,
        language: Option<String>,
    ) -> Result<operit_local_models::LocalInference::LocalSttResponse, String> {
        if contentType.trim() != "audio/wav" {
            return Err("LOCAL_MODEL STT requires audio/wav input".to_string());
        }
        let model = LocalProviderService::parseModelBinding(modelBinding)?;
        let localProvider = LocalProviderService::getInstance(&self.context)?;
        self.transcribeLocalWithTemporaryInput(model, localProvider, audioBytes, language)
    }

    /// Runs local STT with a target-specific temporary audio path.
    #[cfg(not(target_arch = "wasm32"))]
    fn transcribeLocalWithTemporaryInput(
        &self,
        model: operit_local_models::LocalInference::LocalModelSelection,
        localProvider: LocalProviderService,
        audioBytes: Vec<u8>,
        language: Option<String>,
    ) -> Result<operit_local_models::LocalInference::LocalSttResponse, String> {
        fs::create_dir_all(&self.temporaryAudioDirectory).map_err(errorString)?;
        let audioPath = self
            .temporaryAudioDirectory
            .join(format!("local-stt-{}.wav", Uuid::new_v4()));
        fs::write(&audioPath, audioBytes).map_err(errorString)?;
        let recognition =
            localProvider.transcribeAudio(model, audioPath.to_string_lossy().to_string(), language);
        let cleanup = fs::remove_file(&audioPath).map_err(errorString);
        match (recognition, cleanup) {
            (Ok(response), Ok(())) => Ok(response),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(format!("failed to remove local STT input: {error}")),
            (Err(recognitionError), Err(cleanupError)) => Err(format!(
                "{recognitionError}; failed to remove local STT input: {cleanupError}"
            )),
        }
    }

    /// Runs Web local STT with a runtime-storage temporary audio path.
    #[cfg(target_arch = "wasm32")]
    fn transcribeLocalWithTemporaryInput(
        &self,
        model: operit_local_models::LocalInference::LocalModelSelection,
        localProvider: LocalProviderService,
        audioBytes: Vec<u8>,
        language: Option<String>,
    ) -> Result<operit_local_models::LocalInference::LocalSttResponse, String> {
        let storageHost = self
            .context
            .runtimeStorageHost
            .as_ref()
            .ok_or_else(|| "RuntimeStorageHost is required for LOCAL_MODEL STT".to_string())?;
        let audioPath = format!(
            "{RUNTIME_CLEAN_ON_EXIT_DIR_PATH}/local-stt-{}.wav",
            Uuid::new_v4()
        );
        storageHost
            .writeBytes(&audioPath, &audioBytes)
            .map_err(errorString)?;
        let recognition = localProvider.transcribeAudio(model, audioPath.clone(), language);
        let cleanup = storageHost.delete(&audioPath, false).map_err(errorString);
        match (recognition, cleanup) {
            (Ok(response), Ok(())) => Ok(response),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(format!("failed to remove local STT input: {error}")),
            (Err(recognitionError), Err(cleanupError)) => Err(format!(
                "{recognitionError}; failed to remove local STT input: {cleanupError}"
            )),
        }
    }
}

/// Converts one displayable I/O error into a string.
fn errorString(error: impl std::fmt::Display) -> String {
    error.to_string()
}
