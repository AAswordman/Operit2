use std::fs;
use std::path::Path;

use crate::output::CoreCommandOutput;
use operit_model::SttCatalog::SttCatalog;
use operit_model::SttConfig::{SttConfig, SttHttpHeader, SttProviderType};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::preferences::SttConfigManager::SttConfigManager;
use operit_runtime::services::SttRecognitionService::SttRecognitionService;

/// Runs speech-to-text provider configuration and transcription commands.
pub fn run_stt_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("provider-list") if args.len() == 1 => print_provider_catalog(output),
        Some("provider-model-list") if args.len() == 2 => print_provider_models(&args[1], output),
        Some("config") => run_config_command(&args[1..], output),
        Some("transcribe") if args.len() == 2 => {
            transcribe_current(application, &args[1], None, output)
        }
        Some("transcribe") if args.len() == 4 && args[2] == "--language" => {
            transcribe_current(application, &args[1], Some(args[3].clone()), output)
        }
        Some("transcribe-config") if args.len() == 3 => {
            transcribe_config(application, &args[1], &args[2], None, output)
        }
        Some("transcribe-config") if args.len() == 5 && args[3] == "--language" => {
            transcribe_config(
                application,
                &args[1],
                &args[2],
                Some(args[4].clone()),
                output,
            )
        }
        _ => {
            print_stt_usage(output);
            Ok(())
        }
    }
}

/// Prints the built-in STT provider catalog.
fn print_provider_catalog(output: &mut CoreCommandOutput) -> Result<(), String> {
    for provider in SttConfigManager::getInstance().getProviderCatalogEntries()? {
        output.push_stdout_line(format!(
            "{}\t{}\t{}\t{}",
            provider.providerTypeId,
            provider.displayName,
            provider.defaultEndpoint,
            provider.defaultModel
        ));
    }
    Ok(())
}

/// Prints models exposed by one STT provider type.
fn print_provider_models(
    providerTypeId: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    for model in
        SttConfigManager::getInstance().getAvailableSttModels(providerTypeId.to_string())?
    {
        output.push_stdout_line(format!(
            "{}\t{}\t{}\t{}",
            model.model,
            model.displayName,
            model.description,
            model.languages.join(",")
        ));
    }
    Ok(())
}

/// Runs STT provider configuration commands.
fn run_config_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    let manager = SttConfigManager::getInstance();
    match args.first().map(String::as_str) {
        Some("list") if args.len() == 1 => {
            for config in manager.getAllSttConfigs()? {
                output.push_stdout_line(format!(
                    "{}\t{}\t{}\t{}\t{}",
                    config.id, config.name, config.providerType, config.model, config.endpoint
                ));
            }
            Ok(())
        }
        Some("show") if args.len() == 2 => {
            let config = manager.getSttConfig(&args[1])?;
            print_config(&config, output)?;
            Ok(())
        }
        Some("current") if args.len() == 1 => {
            let config = manager.getCurrentSttConfig()?;
            print_config(&config, output)?;
            Ok(())
        }
        Some("use") if args.len() == 2 => {
            let id = manager.setCurrentSttConfigId(&args[1])?;
            output.push_stdout_line(format!("currentSttConfigId={id}"));
            Ok(())
        }
        Some("create-local") if args.len() == 4 => {
            let config = create_local_config(&args[1], &args[2], &args[3])?;
            let created = manager.createSttConfig(config)?;
            output.push_stdout_line(format!("id={}", created.id));
            Ok(())
        }
        Some("create-remote") if args.len() == 6 => {
            let config = create_remote_config(&args[1], &args[2], &args[3], &args[4], &args[5])?;
            let created = manager.createSttConfig(config)?;
            output.push_stdout_line(format!("id={}", created.id));
            Ok(())
        }
        Some("update") if args.len() == 4 => {
            let mut config = manager.getSttConfig(&args[1])?;
            update_config_field(&mut config, &args[2], &args[3])?;
            let updated = manager.updateSttConfig(config)?;
            output.push_stdout_line(format!("id={}", updated.id));
            Ok(())
        }
        Some("delete") if args.len() == 2 => {
            manager.deleteSttConfig(&args[1])?;
            output.push_stdout_line(format!("deleted={}", args[1]));
            Ok(())
        }
        _ => {
            print_stt_usage(output);
            Ok(())
        }
    }
}

/// Prints one STT configuration without exposing its API key value.
fn print_config(config: &SttConfig, output: &mut CoreCommandOutput) -> Result<(), String> {
    output.push_stdout_line(format!("id={}", config.id));
    output.push_stdout_line(format!("name={}", config.name));
    output.push_stdout_line(format!("providerType={}", config.providerType));
    output.push_stdout_line(format!("endpoint={}", config.endpoint));
    output.push_stdout_line(format!("apiKeyLength={}", config.apiKey.len()));
    output.push_stdout_line(format!("model={}", config.model));
    output.push_stdout_line(format!("fileFieldName={}", config.fileFieldName));
    output.push_stdout_line(format!("modelFieldName={}", config.modelFieldName));
    output.push_stdout_line(format!("languageFieldName={}", config.languageFieldName));
    output.push_stdout_line(format!(
        "responseTextJsonPath={}",
        config.responseTextJsonPath
    ));
    output.push_stdout_line(format!(
        "headers={}",
        serde_json::to_string(&config.headers).map_err(|error| error.to_string())?
    ));
    output.push_stdout_line(format!("createdAt={}", config.createdAt));
    output.push_stdout_line(format!("updatedAt={}", config.updatedAt));
    Ok(())
}

/// Creates one LOCAL_MODEL STT configuration value.
fn create_local_config(name: &str, modelId: &str, version: &str) -> Result<SttConfig, String> {
    let catalog = SttCatalog::provider(SttProviderType::LOCAL_MODEL)?;
    Ok(SttConfig {
        id: String::new(),
        name: name.to_string(),
        providerType: catalog.providerTypeId,
        endpoint: String::new(),
        apiKey: String::new(),
        model: format!("{modelId}@{version}"),
        fileFieldName: catalog.defaultFileFieldName,
        modelFieldName: catalog.defaultModelFieldName,
        languageFieldName: catalog.defaultLanguageFieldName,
        responseTextJsonPath: catalog.defaultResponseTextJsonPath,
        headers: catalog.defaultHeaders,
        createdAt: 0,
        updatedAt: 0,
    })
}

/// Creates one remote STT configuration from a provider preset and explicit endpoint.
fn create_remote_config(
    name: &str,
    providerTypeId: &str,
    endpoint: &str,
    apiKey: &str,
    model: &str,
) -> Result<SttConfig, String> {
    let catalog = SttCatalog::provider(providerTypeId)?;
    if catalog.providerTypeId == SttProviderType::LOCAL_MODEL {
        return Err("create-remote does not accept LOCAL_MODEL".to_string());
    }
    Ok(SttConfig {
        id: String::new(),
        name: name.to_string(),
        providerType: catalog.providerTypeId,
        endpoint: endpoint.to_string(),
        apiKey: apiKey.to_string(),
        model: model.to_string(),
        fileFieldName: catalog.defaultFileFieldName,
        modelFieldName: catalog.defaultModelFieldName,
        languageFieldName: catalog.defaultLanguageFieldName,
        responseTextJsonPath: catalog.defaultResponseTextJsonPath,
        headers: catalog.defaultHeaders,
        createdAt: 0,
        updatedAt: 0,
    })
}

/// Updates one mutable STT configuration field.
fn update_config_field(config: &mut SttConfig, field: &str, value: &str) -> Result<(), String> {
    match field {
        "name" => config.name = value.to_string(),
        "endpoint" => config.endpoint = value.to_string(),
        "api-key" => config.apiKey = value.to_string(),
        "model" => config.model = value.to_string(),
        "file-field" => config.fileFieldName = value.to_string(),
        "model-field" => config.modelFieldName = value.to_string(),
        "language-field" => config.languageFieldName = value.to_string(),
        "response-text-json-path" => config.responseTextJsonPath = value.to_string(),
        "headers" => {
            config.headers = serde_json::from_str::<Vec<SttHttpHeader>>(value)
                .map_err(|error| error.to_string())?
        }
        other => return Err(format!("unknown STT config field: {other}")),
    }
    Ok(())
}

/// Transcribes one audio file with the current STT configuration.
fn transcribe_current(
    application: &OperitApplication,
    audioPath: &str,
    language: Option<String>,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (audioBytes, fileName, contentType) = readAudioInput(audioPath)?;
    let result = SttRecognitionService::getInstance(&application.hostManager).transcribeCurrent(
        audioBytes,
        fileName,
        contentType,
        language,
    )?;
    output.push_stdout_line(result.text);
    Ok(())
}

/// Transcribes one audio file with an explicit STT configuration id.
fn transcribe_config(
    application: &OperitApplication,
    configId: &str,
    audioPath: &str,
    language: Option<String>,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let (audioBytes, fileName, contentType) = readAudioInput(audioPath)?;
    let result = SttRecognitionService::getInstance(&application.hostManager)
        .transcribeWithConfigId(
            configId.to_string(),
            audioBytes,
            fileName,
            contentType,
            language,
        )?;
    output.push_stdout_line(result.text);
    Ok(())
}

/// Reads one CLI audio file and returns its explicit multipart metadata.
fn readAudioInput(audioPath: &str) -> Result<(Vec<u8>, String, String), String> {
    let path = Path::new(audioPath);
    if !path.is_file() {
        return Err(format!("STT audio file is missing: {}", path.display()));
    }
    let fileName = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "STT audio file name is invalid".to_string())?
        .to_string();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "STT audio file extension is missing".to_string())?
        .to_ascii_lowercase();
    let contentType = match extension.as_str() {
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "m4a" | "mp4" => "audio/mp4",
        "ogg" | "opus" => "audio/ogg",
        "webm" => "audio/webm",
        "flac" => "audio/flac",
        other => return Err(format!("unsupported STT audio extension: {other}")),
    };
    let audioBytes = fs::read(path).map_err(|error| error.to_string())?;
    if audioBytes.is_empty() {
        return Err("STT audio file is empty".to_string());
    }
    Ok((audioBytes, fileName, contentType.to_string()))
}

/// Prints speech-to-text provider command usage.
fn print_stt_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 stt provider-list");
    output.push_stdout_line("operit2 stt provider-model-list <provider-type-id>");
    output.push_stdout_line("operit2 stt config list");
    output.push_stdout_line("operit2 stt config show <id>");
    output.push_stdout_line("operit2 stt config current");
    output.push_stdout_line("operit2 stt config use <id>");
    output.push_stdout_line("operit2 stt config create-local <name> <model-id> <version>");
    output.push_stdout_line(
        "operit2 stt config create-remote <name> <provider-type-id> <endpoint> <api-key> <model>",
    );
    output.push_stdout_line("operit2 stt config update <id> <field> <value>");
    output.push_stdout_line("operit2 stt config delete <id>");
    output.push_stdout_line("operit2 stt transcribe <audio-path>");
    output.push_stdout_line("operit2 stt transcribe <audio-path> --language <language>");
    output.push_stdout_line("operit2 stt transcribe-config <config-id> <audio-path>");
    output.push_stdout_line(
        "operit2 stt transcribe-config <config-id> <audio-path> --language <language>",
    );
}
