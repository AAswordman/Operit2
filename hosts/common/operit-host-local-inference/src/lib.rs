#![allow(non_snake_case)]
use operit_host_contract::*;
use operit_local_models::LocalModelProvider::ResolvedLocalModelRuntime;
use std::sync::Arc;
mod chat;
mod speech;

/// Owns native inference and optionally delegates speech to an owner platform.
#[derive(Clone, Default)]
pub struct NativeLocalInferenceHost {
    speech: Option<Arc<dyn LocalInferenceHost>>,
}
impl NativeLocalInferenceHost {
    /// Creates a desktop inference host with embedded chat and installed speech engines.
    pub fn new() -> Self {
        Self::default()
    }
    /// Binds mobile speech execution while retaining native chat ownership here.
    pub fn withSpeech(speech: Arc<dyn LocalInferenceHost>) -> Self {
        Self {
            speech: Some(speech),
        }
    }
}

/// Decodes engine metadata after the consuming host receives native capabilities.
fn runtime(options: &str, model: &str, engine: &str) -> HostResult<ResolvedLocalModelRuntime> {
    let mut runtime: ResolvedLocalModelRuntime =
        serde_json::from_str(options).map_err(|error| HostError::new(error.to_string()))?;
    runtime.modelDirectory = model.into();
    runtime.engineRuntimeDirectory = engine.into();
    Ok(runtime)
}
/// Selects platform library metadata within the already resolved engine capability.
fn speechLibraryDirectory(options: &str, directory: &str) -> HostResult<String> {
    use operit_local_models::LocalEngineManifest::{LocalEngineDelivery, LocalPlatform};
    let runtime: ResolvedLocalModelRuntime =
        serde_json::from_str(options).map_err(|error| HostError::new(error.to_string()))?;
    if runtime.engine.artifact.delivery == LocalEngineDelivery::Embedded {
        return Ok(directory.into());
    }
    let artifact = runtime.engine.artifact;
    let relative = match artifact.target.platform {
        LocalPlatform::Android => artifact.androidLibraryDir,
        LocalPlatform::Ohos => artifact.ohosLibraryDir,
        LocalPlatform::Ios => artifact.iosFrameworkDir,
        LocalPlatform::Web => artifact.webRuntimeDir,
        LocalPlatform::Windows | LocalPlatform::Linux | LocalPlatform::Macos => {
            return Err(HostError::new(
                "Desktop engines execute in their platform host",
            ))
        }
    }
    .ok_or_else(|| HostError::new("Platform engine library directory is not declared"))?;
    let path = std::path::Path::new(&relative);
    if path.is_absolute()
        || path.components().any(|part| {
            !matches!(
                part,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return Err(HostError::new(
            "Engine library path escapes its installation directory",
        ));
    }
    let path = std::path::Path::new(directory).join(path);
    if !path.is_dir() {
        return Err(HostError::new(
            "Platform engine library directory is missing",
        ));
    }
    Ok(path.to_string_lossy().into_owned())
}
impl LocalInferenceHost for NativeLocalInferenceHost {
    /// Executes speech through the configured platform backend.
    fn transcribeLocalSpeech(
        &self,
        request: LocalSttInferenceHostRequest,
    ) -> HostResult<LocalSttInferenceHostResponse> {
        if let Some(host) = &self.speech {
            let mut request = request;
            request.engineLibraryDirectory =
                speechLibraryDirectory(&request.optionsJson, &request.engineLibraryDirectory)?;
            return host.transcribeLocalSpeech(request);
        }
        let runtime = runtime(
            &request.optionsJson,
            &request.modelDirectory,
            &request.engineLibraryDirectory,
        )?;
        let result = speech::transcribe(runtime, request)
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(LocalSttInferenceHostResponse {
            text: result.text,
            resultJson: "{}".into(),
        })
    }
    /// Synthesizes into the output capability selected by Core.
    fn synthesizeLocalSpeech(
        &self,
        request: LocalTtsInferenceHostRequest,
    ) -> HostResult<LocalTtsInferenceHostResponse> {
        if let Some(host) = &self.speech {
            let mut request = request;
            request.engineLibraryDirectory =
                speechLibraryDirectory(&request.optionsJson, &request.engineLibraryDirectory)?;
            return host.synthesizeLocalSpeech(request);
        }
        let runtime = runtime(
            &request.optionsJson,
            &request.modelDirectory,
            &request.engineLibraryDirectory,
        )?;
        speech::synthesize(runtime, request).map_err(|error| HostError::new(error.to_string()))
    }
    /// Loads a model only after resolving the file inside this platform host.
    fn generateLocalChat(
        &self,
        engine: &str,
        model: &dyn FileSource,
        request: &str,
        settings: &str,
    ) -> HostResult<String> {
        let body =
            serde_json::from_str(request).map_err(|error| HostError::new(error.to_string()))?;
        let settings =
            serde_json::from_str(settings).map_err(|error| HostError::new(error.to_string()))?;
        chat::validateChatRequest(&body, &settings).map_err(HostError::new)?;
        let modelPath = model.nativePath()?;
        if !std::path::Path::new(&modelPath).is_file() {
            return Err(HostError::new("Local model file is missing"));
        }
        let response =
            chat::nativeGenerate(engine, &modelPath, &body, &settings).map_err(HostError::new)?;
        serde_json::to_string(&response).map_err(|error| HostError::new(error.to_string()))
    }
    /// Releases native cached weights through their owning platform backend.
    fn unloadLocalChat(&self) -> HostResult<()> {
        chat::unloadLocalChat();
        Ok(())
    }
}
