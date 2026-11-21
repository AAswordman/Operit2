#![allow(non_snake_case)]
use operit_host_contract::{
    LocalSttInferenceHostRequest, LocalSttInferenceHostResponse, LocalTtsInferenceHostRequest,
    LocalTtsInferenceHostResponse,
};
use operit_local_models::LocalEngineManifest::{LocalEngineArtifact, LocalPlatform};
use operit_local_models::LocalInference::{LocalInferenceError, LocalSttResponse};
use operit_local_models::LocalModelManifest::{LocalModelDriver, LocalModelKind};
use operit_local_models::LocalModelProvider::{LocalModelProviderError, ResolvedLocalModelRuntime};
use operit_local_models::LocalModelRegistry::InstalledLocalModel;
use operit_util::AppLogger::AppLogger;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
const LOCAL_STT_LOG_TAG: &str = "LocalSTT";

/// Executes the desktop speech recognizer after the host resolves its file capabilities.
pub fn transcribe(
    runtime: ResolvedLocalModelRuntime,
    request: LocalSttInferenceHostRequest,
) -> Result<LocalSttResponse, LocalInferenceError> {
    let driver = runtime.model.manifest.driver.as_ref().ok_or_else(|| {
        LocalModelProviderError::DriverMissing(
            runtime.model.manifest.id.clone(),
            runtime.model.manifest.version.clone(),
        )
    })?;
    let LocalModelDriver::SherpaOnnxStreamingTransducer {
        encoder,
        decoder,
        joiner,
        tokens,
        modelType,
    } = driver
    else {
        return Err(LocalModelProviderError::UnsupportedDriver(format!(
            "{:?}",
            runtime.model.manifest.engine
        ))
        .into());
    };
    let executable = engineExecutable(
        &runtime,
        runtime.engine.artifact.sttExecutable.as_deref(),
        "STT",
    )?;
    let modelDirectory = PathBuf::from(&runtime.modelDirectory);
    let audioPath = requiredFile(Path::new(&request.audioPath))?;
    let output = runEngineCommand(
        &runtime,
        &executable,
        &[
            format!(
                "--encoder={}",
                declaredModelFile(&runtime.model, &modelDirectory, encoder)?.display()
            ),
            format!(
                "--decoder={}",
                declaredModelFile(&runtime.model, &modelDirectory, decoder)?.display()
            ),
            format!(
                "--joiner={}",
                declaredModelFile(&runtime.model, &modelDirectory, joiner)?.display()
            ),
            format!(
                "--tokens={}",
                declaredModelFile(&runtime.model, &modelDirectory, tokens)?.display()
            ),
            format!("--model-type={modelType}"),
            "--provider=cpu".to_string(),
            "--num-threads=2".to_string(),
            "--decoding-method=greedy_search".to_string(),
            "--print-args=false".to_string(),
            audioPath.to_string_lossy().to_string(),
        ],
    )?;
    parseSherpaSttOutput(&output).map_err(Into::into)
}

/// Executes the desktop speech synthesizer after the host resolves its file capabilities.
pub fn synthesize(
    runtime: ResolvedLocalModelRuntime,
    request: LocalTtsInferenceHostRequest,
) -> Result<LocalTtsInferenceHostResponse, LocalInferenceError> {
    let speakerId = request.voice.trim().parse::<i32>().map_err(|error| {
        LocalModelProviderError::InvalidRequest(format!(
            "local TTS voice must be a numeric speaker id: {error}"
        ))
    })?;
    let driver = runtime.model.manifest.driver.as_ref().ok_or_else(|| {
        LocalModelProviderError::DriverMissing(
            runtime.model.manifest.id.clone(),
            runtime.model.manifest.version.clone(),
        )
    })?;
    let executable = engineExecutable(
        &runtime,
        runtime.engine.artifact.ttsExecutable.as_deref(),
        "TTS",
    )?;
    let modelDirectory = PathBuf::from(&runtime.modelDirectory);
    let outputPath = request.outputPath.clone();
    let speed = request.speed;
    let mut args = vec![
        format!("--sid={speakerId}"),
        format!("--speed={speed}"),
        "--provider=cpu".to_string(),
        "--num-threads=2".to_string(),
        "--print-args=false".to_string(),
        format!("--output-filename={outputPath}"),
    ];
    appendSherpaTtsDriverArguments(&mut args, driver, speakerId, &runtime, &modelDirectory)?;
    args.push(request.text);
    runEngineCommand(&runtime, &executable, &args)?;
    Ok(LocalTtsInferenceHostResponse {
        audioPath: outputPath,
        outputFormat: "wav".into(),
    })
}

/// Resolves one required engine executable from installed runtime metadata.
fn engineExecutable(
    runtime: &ResolvedLocalModelRuntime,
    relativePath: Option<&str>,
    capability: &str,
) -> Result<PathBuf, LocalModelProviderError> {
    let relativePath = relativePath
        .ok_or_else(|| LocalModelProviderError::EngineExecutableMissing(capability.to_string()))?;
    let executable =
        requiredFile(&PathBuf::from(&runtime.engineRuntimeDirectory).join(relativePath))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let prepare = || -> std::io::Result<()> {
            let mut permissions = std::fs::metadata(&executable)?.permissions();
            let mode = permissions.mode();
            if mode & 0o100 == 0 {
                permissions.set_mode(mode | 0o100);
                std::fs::set_permissions(&executable, permissions)?;
            }
            Ok(())
        };
        prepare().map_err(|error| {
            LocalModelProviderError::Process(format!(
                "cannot prepare executable {}: {error}",
                executable.display()
            ))
        })?;
    }
    Ok(executable)
}

/// Resolves one declared model file and verifies that it exists.
fn declaredModelFile(
    model: &InstalledLocalModel,
    modelDirectory: &Path,
    relativePath: &str,
) -> Result<PathBuf, LocalModelProviderError> {
    let declared = model
        .manifest
        .files
        .iter()
        .find(|file| file.relativePath == relativePath)
        .ok_or_else(|| LocalModelProviderError::RuntimeFileMissing(relativePath.to_string()))?;
    requiredFile(&modelDirectory.join(&declared.relativePath))
}

/// Resolves one declared model directory and verifies that it exists.
fn declaredModelDirectory(
    model: &InstalledLocalModel,
    modelDirectory: &Path,
    relativePath: &str,
) -> Result<PathBuf, LocalModelProviderError> {
    let requested = Path::new(relativePath.trim());
    let hasDeclaredChild = model
        .manifest
        .files
        .iter()
        .any(|file| Path::new(&file.relativePath).starts_with(requested));
    if !hasDeclaredChild {
        return Err(LocalModelProviderError::RuntimeFileMissing(
            relativePath.to_string(),
        ));
    }
    requiredDirectory(&modelDirectory.join(requested))
}

/// Verifies one required runtime file and returns its owned path.
fn requiredFile(path: &Path) -> Result<PathBuf, LocalModelProviderError> {
    if !path.is_file() {
        return Err(LocalModelProviderError::RuntimeFileMissing(
            path.to_string_lossy().to_string(),
        ));
    }
    Ok(path.to_path_buf())
}

/// Verifies one required runtime directory and returns its owned path.
fn requiredDirectory(path: &Path) -> Result<PathBuf, LocalModelProviderError> {
    if !path.is_dir() {
        return Err(LocalModelProviderError::RuntimeFileMissing(
            path.to_string_lossy().to_string(),
        ));
    }
    Ok(path.to_path_buf())
}

/// Runs one installed desktop engine process with a deterministic library path.
fn runEngineCommand(
    runtime: &ResolvedLocalModelRuntime,
    executable: &Path,
    args: &[String],
) -> Result<Output, LocalModelProviderError> {
    let runtimeDirectory = PathBuf::from(&runtime.engineRuntimeDirectory);
    let mut command = Command::new(executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.args(args).current_dir(&runtimeDirectory);
    configureEngineLibraryPath(&mut command, &runtime.engine.artifact, &runtimeDirectory)?;
    let output = command.output().map_err(|error| {
        LocalModelProviderError::Process(format!(
            "cannot start executable {}: {error}",
            executable.display()
        ))
    })?;
    if !output.status.success() {
        return Err(LocalModelProviderError::Process(format!(
            "status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output)
}

/// Configures the shared-library search path for one desktop engine process.
fn configureEngineLibraryPath(
    command: &mut Command,
    artifact: &LocalEngineArtifact,
    runtimeDirectory: &Path,
) -> Result<(), LocalModelProviderError> {
    let libraryDirectory = runtimeDirectory.join("lib");
    match artifact.target.platform {
        LocalPlatform::Windows => {
            let mut paths = vec![runtimeDirectory.join("bin"), libraryDirectory];
            if let Some(existing) = std::env::var_os("PATH") {
                paths.extend(std::env::split_paths(&existing));
            }
            let joined = std::env::join_paths(paths)
                .map_err(|error| LocalModelProviderError::Process(error.to_string()))?;
            command.env("PATH", joined);
        }
        LocalPlatform::Linux => {
            command.env("LD_LIBRARY_PATH", libraryDirectory);
        }
        LocalPlatform::Macos => {
            command.env("DYLD_LIBRARY_PATH", libraryDirectory);
        }
        LocalPlatform::Android | LocalPlatform::Ohos | LocalPlatform::Ios | LocalPlatform::Web => {
            return Err(LocalModelProviderError::UnsupportedDriver(
                "platform engines are host-driven".to_string(),
            ));
        }
    }
    Ok(())
}

/// Parses the structured JSON record emitted by Sherpa streaming STT.
fn parseSherpaSttOutput(output: &Output) -> Result<LocalSttResponse, LocalModelProviderError> {
    let stderr = std::str::from_utf8(&output.stderr)
        .map_err(|error| LocalModelProviderError::InvalidOutput(error.to_string()))?;
    let records = stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .collect::<Vec<_>>();
    let record = records
        .last()
        .ok_or_else(|| LocalModelProviderError::InvalidOutput(stderr.to_string()))?;
    let text = record
        .as_object()
        .and_then(|object| object.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| LocalModelProviderError::InvalidOutput(record.to_string()))?;
    if text.trim().is_empty() {
        AppLogger::w(
            LOCAL_STT_LOG_TAG,
            &format!(
                "Sherpa STT completed without recognized text\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        );
    }
    Ok(LocalSttResponse {
        text: text.to_string(),
        segments: Vec::new(),
    })
}

/// Reads one finite numeric option from a JSON object.
fn jsonNumberOption(options: &Value, key: &str) -> Result<f64, LocalModelProviderError> {
    let object = options.as_object().ok_or_else(|| {
        LocalModelProviderError::InvalidRequest("local inference options must be an object".into())
    })?;
    let value = object
        .get(key)
        .ok_or_else(|| LocalModelProviderError::InvalidRequest(format!("{key} is required")))?
        .as_f64()
        .ok_or_else(|| LocalModelProviderError::InvalidRequest(format!("{key} must be numeric")))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(LocalModelProviderError::InvalidRequest(format!(
            "{key} must be finite and positive"
        )));
    }
    Ok(value)
}

/// Appends Sherpa ONNX TTS model arguments for one exact driver.
fn appendSherpaTtsDriverArguments(
    args: &mut Vec<String>,
    driver: &LocalModelDriver,
    speakerId: i32,
    runtime: &ResolvedLocalModelRuntime,
    modelDirectory: &Path,
) -> Result<(), LocalModelProviderError> {
    match driver {
        LocalModelDriver::SherpaOnnxVits {
            model,
            lexicon,
            tokens,
            ruleFsts,
            ruleFars,
            speakerCount,
        } => {
            validateSpeakerId(speakerId, *speakerCount, "VITS")?;
            args.push(format!(
                "--vits-model={}",
                declaredModelFile(&runtime.model, modelDirectory, model)?.display()
            ));
            args.push(format!(
                "--vits-lexicon={}",
                declaredModelFile(&runtime.model, modelDirectory, lexicon)?.display()
            ));
            args.push(format!(
                "--vits-tokens={}",
                declaredModelFile(&runtime.model, modelDirectory, tokens)?.display()
            ));
            appendRuleFileArgument(
                args,
                "--tts-rule-fsts",
                ruleFsts,
                &runtime.model,
                modelDirectory,
            )?;
            appendRuleFileArgument(
                args,
                "--tts-rule-fars",
                ruleFars,
                &runtime.model,
                modelDirectory,
            )
        }
        LocalModelDriver::SherpaOnnxMatcha {
            acousticModel,
            vocoder,
            lexicon,
            tokens,
            ruleFsts,
            ruleFars,
            speakerCount,
        } => {
            validateSpeakerId(speakerId, *speakerCount, "Matcha")?;
            args.push(format!(
                "--matcha-acoustic-model={}",
                declaredModelFile(&runtime.model, modelDirectory, acousticModel)?.display()
            ));
            args.push(format!(
                "--matcha-vocoder={}",
                declaredModelFile(&runtime.model, modelDirectory, vocoder)?.display()
            ));
            args.push(format!(
                "--matcha-lexicon={}",
                declaredModelFile(&runtime.model, modelDirectory, lexicon)?.display()
            ));
            args.push(format!(
                "--matcha-tokens={}",
                declaredModelFile(&runtime.model, modelDirectory, tokens)?.display()
            ));
            appendRuleFileArgument(
                args,
                "--tts-rule-fsts",
                ruleFsts,
                &runtime.model,
                modelDirectory,
            )?;
            appendRuleFileArgument(
                args,
                "--tts-rule-fars",
                ruleFars,
                &runtime.model,
                modelDirectory,
            )
        }
        LocalModelDriver::SherpaOnnxKitten {
            model,
            voices,
            tokens,
            dataDir,
            speakerCount,
        } => {
            validateSpeakerId(speakerId, *speakerCount, "Kitten")?;
            args.push(format!(
                "--kitten-model={}",
                declaredModelFile(&runtime.model, modelDirectory, model)?.display()
            ));
            args.push(format!(
                "--kitten-voices={}",
                declaredModelFile(&runtime.model, modelDirectory, voices)?.display()
            ));
            args.push(format!(
                "--kitten-tokens={}",
                declaredModelFile(&runtime.model, modelDirectory, tokens)?.display()
            ));
            args.push(format!(
                "--kitten-data-dir={}",
                declaredModelDirectory(&runtime.model, modelDirectory, dataDir)?.display()
            ));
            Ok(())
        }
        _ => Err(LocalModelProviderError::UnsupportedDriver(format!(
            "{:?}",
            runtime.model.manifest.engine
        ))),
    }
}

/// Validates a requested numeric TTS speaker id.
fn validateSpeakerId(
    speakerId: i32,
    speakerCount: i32,
    driverName: &str,
) -> Result<(), LocalModelProviderError> {
    if speakerId < 0 || speakerId >= speakerCount {
        return Err(LocalModelProviderError::InvalidRequest(format!(
            "local {driverName} speaker id must be between 0 and {}",
            speakerCount - 1
        )));
    }
    Ok(())
}

/// Appends one comma-separated set of declared rule files to engine arguments.
fn appendRuleFileArgument(
    args: &mut Vec<String>,
    flag: &str,
    relativePaths: &[String],
    model: &InstalledLocalModel,
    modelDirectory: &Path,
) -> Result<(), LocalModelProviderError> {
    if relativePaths.is_empty() {
        return Ok(());
    }
    let mut paths = Vec::new();
    for relativePath in relativePaths {
        paths.push(
            declaredModelFile(model, modelDirectory, relativePath)?
                .to_string_lossy()
                .to_string(),
        );
    }
    args.push(format!("{flag}={}", paths.join(",")));
    Ok(())
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use super::*;
    use operit_local_models::LocalEngineCatalog::LocalEngineCatalog;
    use operit_local_models::LocalEngineManifest::LocalPlatformTarget;
    use operit_local_models::LocalModelCatalog::LocalModelCatalog;
    use operit_local_models::LocalModelRegistry::InstalledLocalEngine;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    struct SpeechRuntime {
        directory: PathBuf,
        runtime: ResolvedLocalModelRuntime,
    }

    impl SpeechRuntime {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "operit speech permissions {}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(directory.join("bin")).unwrap();
            let manifest = LocalEngineCatalog::sherpaOnnx();
            let artifact = manifest
                .artifactForTarget(&LocalPlatformTarget::current().unwrap())
                .unwrap()
                .clone();
            let runtime = ResolvedLocalModelRuntime {
                model: InstalledLocalModel {
                    manifest: LocalModelCatalog::sherpaOnnxStreamingStt(),
                    storagePath: String::new(),
                    installedAtMs: 0,
                    verifiedAtMs: None,
                },
                engine: InstalledLocalEngine {
                    manifest,
                    artifact,
                    storagePath: String::new(),
                    installedAtMs: 0,
                    verifiedAtMs: None,
                },
                modelDirectory: directory.to_string_lossy().into_owned(),
                engineDirectory: directory.to_string_lossy().into_owned(),
                engineRuntimeDirectory: directory.to_string_lossy().into_owned(),
            };
            Self { directory, runtime }
        }
    }

    impl Drop for SpeechRuntime {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).unwrap();
        }
    }

    #[test]
    fn installedSpeechExecutablesLaunchAfterStorageDropsExecutePermissions() {
        let fixture = SpeechRuntime::new();
        for (relative, capability) in [
            (
                fixture.runtime.engine.artifact.sttExecutable.as_deref(),
                "STT",
            ),
            (
                fixture.runtime.engine.artifact.ttsExecutable.as_deref(),
                "TTS",
            ),
        ] {
            let path = fixture.directory.join(relative.unwrap());
            fs::write(&path, "#!/bin/sh\nprintf '%s' speech-engine-ready\n").unwrap();
            for mode in [0o644, 0o600, 0o755, 0o555] {
                fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
                if mode & 0o100 == 0 {
                    assert_eq!(
                        Command::new(&path).output().unwrap_err().kind(),
                        std::io::ErrorKind::PermissionDenied
                    );
                }
                for _ in 0..2 {
                    let executable =
                        engineExecutable(&fixture.runtime, relative, capability).unwrap();
                    let output = runEngineCommand(&fixture.runtime, &executable, &[]).unwrap();
                    assert_eq!(output.stdout, b"speech-engine-ready");
                    assert_eq!(
                        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                        mode | 0o100
                    );
                }
            }
        }
    }
}
