#![allow(non_snake_case)]
use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    FileSource, LocalInferenceHost, LocalSttInferenceHostRequest, LocalTtsInferenceHostRequest,
    RuntimeStorageHost,
};
use operit_local_models::LocalEngineManifest::LocalEngineDelivery;
use operit_local_models::LocalInference::{
    LocalModelSelection, LocalSttResponse, LocalTtsResponse,
};
use operit_local_models::LocalModelManifest::LocalModelKind;
use operit_local_models::LocalModelProvider::{LocalModelProvider, ResolvedLocalModelRuntime};
use operit_vfs::{VfsFile, VisualFileSystem::VisualFileSystem};
use std::sync::Arc;
use uuid::Uuid;

/// Executes selected local models using virtual artifacts and platform capabilities.
#[derive(Clone)]
pub struct LocalProviderService {
    storage: Arc<dyn RuntimeStorageHost>,
    filesystem: VisualFileSystem,
    host: Arc<dyn LocalInferenceHost>,
}
impl LocalProviderService {
    /// Binds inference to the same runtime that owns its model files.
    pub fn getInstance(context: &HostManager) -> Result<Self, String> {
        Ok(Self {
            storage: context
                .runtimeStorageHost
                .clone()
                .ok_or("Local inference requires runtime storage")?,
            filesystem: context.vfs()?,
            host: context
                .localInferenceHost
                .clone()
                .ok_or("LocalInferenceHost is not configured")?,
        })
    }
    /// Parses one exact model release binding.
    pub fn parseModelBinding(binding: String) -> Result<LocalModelSelection, String> {
        let (modelId, version) = binding
            .trim()
            .split_once('@')
            .ok_or("LOCAL_MODEL model id must use modelId@version")?;
        if modelId.is_empty() || version.is_empty() {
            return Err("LOCAL_MODEL model id must use non-empty modelId@version".into());
        }
        Ok(LocalModelSelection {
            modelId: modelId.into(),
            version: version.into(),
        })
    }
    /// Resolves a declared model release using virtual installation addresses.
    fn resolve(
        &self,
        model: &LocalModelSelection,
        kind: LocalModelKind,
    ) -> Result<ResolvedLocalModelRuntime, String> {
        LocalModelProvider::forRuntimeStorage(self.storage.clone())
            .map_err(|error| error.to_string())?
            .resolve(model, kind)
            .map_err(|error| error.to_string())
    }
    /// Returns engine artifacts without exposing their native locations to Core.
    fn engine(&self, runtime: &ResolvedLocalModelRuntime) -> Result<Option<VfsFile>, String> {
        if runtime.engine.artifact.delivery == LocalEngineDelivery::Embedded {
            return Ok(None);
        }
        self.filesystem
            .file(&runtime.engineRuntimeDirectory)
            .map(Some)
    }
    /// Transcribes a virtual audio file through the owning platform host.
    pub fn transcribeAudio(
        &self,
        model: LocalModelSelection,
        audioPath: String,
        language: Option<String>,
    ) -> Result<LocalSttResponse, String> {
        let runtime = self.resolve(&model, LocalModelKind::SpeechToText)?;
        let driver = runtime
            .model
            .manifest
            .driver
            .as_ref()
            .ok_or("Local model driver is missing")?;
        let model = self.filesystem.file(&runtime.modelDirectory)?;
        let engine = self.engine(&runtime)?;
        let audio = self.filesystem.file(&audioPath)?;
        let response = self
            .host
            .transcribeLocalFiles(
                LocalSttInferenceHostRequest {
                    engineLibraryDirectory: format!(
                        "embedded:{}@{}",
                        runtime.engine.manifest.id, runtime.engine.manifest.version
                    ),
                    modelDirectory: runtime.modelDirectory.clone(),
                    driverJson: serde_json::to_string(driver).map_err(|error| error.to_string())?,
                    audioPath,
                    language,
                    optionsJson: serde_json::to_string(&runtime)
                        .map_err(|error| error.to_string())?,
                },
                &model,
                engine.as_ref().map(|file| file as &dyn FileSource),
                &audio,
            )
            .map_err(|error| error.to_string())?;
        Ok(LocalSttResponse {
            text: response.text,
            segments: Vec::new(),
        })
    }
    /// Synthesizes and consumes a temporary VFS output on every platform.
    pub fn synthesizeAudio(
        &self,
        model: LocalModelSelection,
        text: String,
        voice: String,
        speed: f64,
    ) -> Result<LocalTtsResponse, String> {
        let runtime = self.resolve(&model, LocalModelKind::TextToSpeech)?;
        let driver = runtime
            .model
            .manifest
            .driver
            .as_ref()
            .ok_or("Local model driver is missing")?;
        let model = self.filesystem.file(&runtime.modelDirectory)?;
        let engine = self.engine(&runtime)?;
        let parent = "/operitapp/data/temp/clean_on_exit";
        self.filesystem.makeDirectory(parent, true)?;
        let outputPath = format!("{parent}/local-tts-{}.wav", Uuid::new_v4());
        let output = self.filesystem.file(&outputPath)?;
        self.host
            .synthesizeLocalFiles(
                LocalTtsInferenceHostRequest {
                    engineLibraryDirectory: format!(
                        "embedded:{}@{}",
                        runtime.engine.manifest.id, runtime.engine.manifest.version
                    ),
                    modelDirectory: runtime.modelDirectory.clone(),
                    driverJson: serde_json::to_string(driver).map_err(|error| error.to_string())?,
                    text,
                    voice,
                    speed,
                    outputPath: outputPath.clone(),
                    optionsJson: serde_json::to_string(&runtime)
                        .map_err(|error| error.to_string())?,
                },
                &model,
                engine.as_ref().map(|file| file as &dyn FileSource),
                &output,
            )
            .map_err(|error| error.to_string())?;
        let audioBytes = self.filesystem.readFileBytes(&outputPath);
        let cleanup = self.filesystem.deleteFile(&outputPath, false);
        let audioBytes = audioBytes?;
        cleanup?;
        Ok(LocalTtsResponse {
            audioBytes,
            outputFormat: "wav".into(),
        })
    }
}
