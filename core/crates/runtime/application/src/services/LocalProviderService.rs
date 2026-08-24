#![allow(non_snake_case)]

use std::path::PathBuf;
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    FileSystemHost, LocalInferenceHost, LocalSttInferenceHostRequest, LocalTtsInferenceHostRequest,
    RuntimeStorageHost,
};
use operit_local_models::LocalEngineManifest::{LocalEngineDelivery, LocalPlatform};
use operit_local_models::LocalInference::{
    LocalModelSelection, LocalSttRequest, LocalSttResponse, LocalTtsRequest, LocalTtsResponse,
};
use operit_local_models::LocalModelManifest::LocalModelKind;
use operit_local_models::LocalModelProvider::{LocalModelProvider, ResolvedLocalModelRuntime};
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
/// Executes models selected through the LOCAL_MODEL provider.
pub struct LocalProviderService {
    runtimeRoot: PathBuf,
    runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
    fileSystemHost: Arc<dyn FileSystemHost>,
    localInferenceHost: Option<Arc<dyn LocalInferenceHost>>,
}

impl LocalProviderService {
    /// Creates the local provider service from application host context.
    pub fn getInstance(context: &HostManager) -> Result<Self, String> {
        let runtimeStorageHost = context
            .runtimeStorageHost
            .as_ref()
            .ok_or_else(|| "RuntimeStorageHost is required for LOCAL_MODEL".to_string())?;
        let runtimeRoot = runtimeStorageHost
            .runtimeRootDir()
            .ok_or_else(|| "RuntimeStorageHost runtime root is not configured".to_string())?;
        let fileSystemHost = context
            .fileSystemHost
            .as_ref()
            .ok_or_else(|| "FileSystemHost is required for LOCAL_MODEL".to_string())?;
        Ok(Self {
            runtimeRoot,
            runtimeStorageHost: runtimeStorageHost.clone(),
            fileSystemHost: fileSystemHost.clone(),
            localInferenceHost: context.localInferenceHost.clone(),
        })
    }

    /// Parses one stable local provider model id in `modelId@version` form.
    pub fn parseModelBinding(binding: String) -> Result<LocalModelSelection, String> {
        let (modelId, version) = binding
            .trim()
            .split_once('@')
            .ok_or_else(|| "LOCAL_MODEL model id must use modelId@version".to_string())?;
        if modelId.is_empty() || version.is_empty() {
            return Err("LOCAL_MODEL model id must use non-empty modelId@version".to_string());
        }
        Ok(LocalModelSelection {
            modelId: modelId.to_string(),
            version: version.to_string(),
        })
    }

    /// Transcribes audio with one exact LOCAL_MODEL provider model.
    pub fn transcribeAudio(
        &self,
        model: LocalModelSelection,
        audioPath: String,
        language: Option<String>,
    ) -> Result<LocalSttResponse, String> {
        let provider = self.provider()?;
        let runtime = provider
            .resolve(&model, LocalModelKind::SpeechToText)
            .map_err(errorString)?;
        match runtime.engine.artifact.target.platform {
            LocalPlatform::Android
            | LocalPlatform::Ohos
            | LocalPlatform::Ios
            | LocalPlatform::Web => self.transcribePlatform(runtime, audioPath, language),
            LocalPlatform::Windows | LocalPlatform::Linux | LocalPlatform::Macos => provider
                .transcribe(LocalSttRequest {
                    model,
                    audioPath,
                    language,
                    options: json!({}),
                })
                .map_err(errorString),
        }
    }

    /// Synthesizes WAV audio with one exact LOCAL_MODEL provider model.
    pub fn synthesizeAudio(
        &self,
        model: LocalModelSelection,
        text: String,
        voice: String,
        speed: f64,
    ) -> Result<LocalTtsResponse, String> {
        let provider = self.provider()?;
        let runtime = provider
            .resolve(&model, LocalModelKind::TextToSpeech)
            .map_err(errorString)?;
        match runtime.engine.artifact.target.platform {
            LocalPlatform::Android
            | LocalPlatform::Ohos
            | LocalPlatform::Ios
            | LocalPlatform::Web => self.synthesizePlatform(runtime, text, voice, speed),
            LocalPlatform::Windows | LocalPlatform::Linux | LocalPlatform::Macos => provider
                .synthesize(LocalTtsRequest {
                    model,
                    text,
                    voice,
                    outputFormat: "wav".to_string(),
                    options: json!({ "speed": speed }),
                })
                .map_err(errorString),
        }
    }

    /// Executes mobile STT through the registered platform host.
    fn transcribePlatform(
        &self,
        runtime: ResolvedLocalModelRuntime,
        audioPath: String,
        language: Option<String>,
    ) -> Result<LocalSttResponse, String> {
        let host = self.requiredLocalInferenceHost()?;
        let driver = runtime.model.manifest.driver.as_ref().ok_or_else(|| {
            format!(
                "local model driver is missing: {}",
                runtime.model.manifest.registryKey()
            )
        })?;
        let response = host
            .transcribeLocalSpeech(LocalSttInferenceHostRequest {
                engineLibraryDirectory: self.engineLibraryDirectory(&runtime)?,
                modelDirectory: runtime.modelDirectory.clone(),
                driverJson: serde_json::to_string(driver).map_err(errorString)?,
                audioPath,
                language,
                optionsJson: "{}".to_string(),
            })
            .map_err(errorString)?;
        Ok(LocalSttResponse {
            text: response.text,
            segments: Vec::new(),
        })
    }

    /// Executes mobile TTS through the registered platform host.
    fn synthesizePlatform(
        &self,
        runtime: ResolvedLocalModelRuntime,
        text: String,
        voice: String,
        speed: f64,
    ) -> Result<LocalTtsResponse, String> {
        let host = self.requiredLocalInferenceHost()?;
        let driver = runtime.model.manifest.driver.as_ref().ok_or_else(|| {
            format!(
                "local model driver is missing: {}",
                runtime.model.manifest.registryKey()
            )
        })?;
        let outputPath = self.platformTtsOutputPath(&runtime)?;
        let response = host
            .synthesizeLocalSpeech(LocalTtsInferenceHostRequest {
                engineLibraryDirectory: self.engineLibraryDirectory(&runtime)?,
                modelDirectory: runtime.modelDirectory.clone(),
                driverJson: serde_json::to_string(driver).map_err(errorString)?,
                text,
                voice,
                speed,
                outputPath: outputPath.clone(),
                optionsJson: "{}".to_string(),
            })
            .map_err(errorString)?;
        let audioBytes = self.readPlatformTtsOutput(&response.audioPath)?;
        Ok(LocalTtsResponse {
            audioBytes,
            outputFormat: response.outputFormat,
        })
    }

    /// Resolves the platform native library directory from installed engine metadata.
    fn engineLibraryDirectory(
        &self,
        runtime: &ResolvedLocalModelRuntime,
    ) -> Result<String, String> {
        if runtime.engine.artifact.delivery == LocalEngineDelivery::Embedded {
            return Ok(format!(
                "embedded:{}@{}",
                runtime.engine.manifest.id, runtime.engine.manifest.version
            ));
        }
        let relativePath = match runtime.engine.artifact.target.platform {
            LocalPlatform::Android => runtime.engine.artifact.androidLibraryDir.as_ref(),
            LocalPlatform::Ohos => runtime.engine.artifact.ohosLibraryDir.as_ref(),
            LocalPlatform::Ios => runtime.engine.artifact.iosFrameworkDir.as_ref(),
            LocalPlatform::Web => runtime.engine.artifact.webRuntimeDir.as_ref(),
            LocalPlatform::Windows | LocalPlatform::Linux | LocalPlatform::Macos => None,
        }
        .ok_or_else(|| "platform engine library directory is not declared".to_string())?;
        let directory = joinDeclaredRuntimePath(
            &PathBuf::from(&runtime.engineRuntimeDirectory),
            relativePath,
        );
        let path = storagePathString(&directory);
        let entry = self.fileSystemHost.fileExists(&path).map_err(errorString)?;
        if !entry.exists || !entry.isDirectory {
            return Err(format!(
                "platform engine library directory is missing: {path}"
            ));
        }
        Ok(directory.to_string_lossy().to_string())
    }

    /// Returns the registered platform local inference host.
    fn requiredLocalInferenceHost(&self) -> Result<Arc<dyn LocalInferenceHost>, String> {
        self.localInferenceHost
            .clone()
            .ok_or_else(|| "LocalInferenceHost is required on this platform".to_string())
    }

    /// Builds an output path for one platform TTS request.
    fn platformTtsOutputPath(&self, runtime: &ResolvedLocalModelRuntime) -> Result<String, String> {
        let outputPath = self
            .runtimeRoot
            .join("temp")
            .join("clean_on_exit")
            .join(format!("local-tts-{}.wav", Uuid::new_v4()));
        let outputParent = outputPath
            .parent()
            .ok_or_else(|| "local TTS output path has no parent".to_string())?;
        let outputParent = storagePathString(outputParent);
        self.fileSystemHost
            .makeDirectory(&outputParent, true)
            .map_err(errorString)?;
        Ok(storagePathString(&outputPath))
    }

    /// Reads and removes one platform TTS output file.
    fn readPlatformTtsOutput(&self, audioPath: &str) -> Result<Vec<u8>, String> {
        let audioBytes = self
            .fileSystemHost
            .readFileBytes(audioPath)
            .map_err(errorString)?;
        self.fileSystemHost
            .deleteFile(audioPath, false)
            .map_err(errorString)?;
        Ok(audioBytes)
    }

    /// Creates a local inference provider for the configured runtime root.
    fn provider(&self) -> Result<LocalModelProvider, String> {
        LocalModelProvider::forRuntimeStorage(self.runtimeStorageHost.clone()).map_err(errorString)
    }
}

/// Converts one displayable provider error into a string.
fn errorString(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Converts a path into the virtual runtime-storage representation.
fn storagePathString(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Joins a declared runtime path while preserving current-directory semantics.
fn joinDeclaredRuntimePath(root: &std::path::Path, relativePath: &str) -> PathBuf {
    if relativePath.trim() == "." {
        return root.to_path_buf();
    }
    root.join(relativePath)
}
