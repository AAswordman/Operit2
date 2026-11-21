use crate::commands::util::read_content_arg;
use crate::output::CoreCommandOutput;
use operit_model::TtsConfig::{TtsConfig, TtsProviderType};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::preferences::TtsConfigManager::TtsConfigManager;
use operit_runtime::services::TtsSynthesisService::TtsSynthesisService;
use serde_json::json;

/// Shares speech configuration, provider voices and synthesis across local and remote commands.
pub fn run_tts_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let manager = TtsConfigManager::getInstance();
    let value = match args {
        [command] if command == "provider-list" => json!(manager.getProviderCatalogEntries()?),
        [command, id] if command == "voices" => json!(manager.getAvailableTtsVoices(id)?),
        [command, id, model, voice] if command == "voice-add" => {
            json!(manager.addTtsVoiceFromAvailable(id, model.clone(), voice.clone())?)
        }
        [command, id, model, voice] if command == "voice-custom" => {
            json!(manager.createCustomTtsVoice(id, model.clone(), voice.clone())?)
        }
        [command, rest @ ..] if command == "config" => config(application, &manager, rest)?,
        [command, rest @ ..] if command == "synthesize" => {
            let mut id = None;
            let mut text = None;
            for option in rest.chunks(2) {
                match option {
                    [flag, value] if flag == "--config" && id.is_none() => id = Some(value),
                    [flag, value] if flag == "--text" && text.is_none() => text = Some(value),
                    _ => {
                        return Err(
                            "usage: operit2 tts synthesize --config <id> --text <text>".into()
                        )
                    }
                }
            }
            let result = TtsSynthesisService::getInstance(&application.hostManager)
                .synthesizeWithConfig(
                    id.ok_or("--config is required")?,
                    text.ok_or("--text is required")?,
                )?;
            json!({"audioPaths": result.audioPaths})
        }
        [] => {
            let usage = [
                "operit2 tts provider-list",
                "operit2 tts voices <provider-config-id>",
                "operit2 tts voice-add <provider-config-id> <model> <voice>",
                "operit2 tts voice-custom <provider-config-id> <model> <voice>",
                "operit2 tts config <list|show <id>|current|use <id>|delete <id>>",
                "operit2 tts config create <name> <endpoint> <api-key> <model> <voice> <response-format> <speed>",
                "operit2 tts config create-local <name> <model-id> <version> <voice> <speed>",
                "operit2 tts config update <id> <name|endpoint|api-key|model|voice|response-format|speed|http-method|request-body|content-type> <value>",
                "operit2 tts config <create-json|save> <config-json|@vfs-file>",
                "operit2 tts synthesize --config <id> --text <text>",
            ];
            for line in usage {
                output.push_stdout_line(line);
            }
            output.setJsonStdout(json!({"usage": usage}));
            return Ok(());
        }
        _ => return Err("Invalid TTS arguments; use 'operit2 tts'".into()),
    };
    let mut display = value.clone();
    redact_keys(&mut display);
    output.push_stdout_line(
        serde_json::to_string_pretty(&display).map_err(|error| error.to_string())?,
    );
    output.setJsonStdout(value);
    Ok(())
}

/// Retains the existing human-output key privacy while preserving explicit JSON configuration export.
fn redact_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(key) = fields.get_mut("apiKey") {
                *key = json!("<redacted>");
            }
            for value in fields.values_mut() {
                redact_keys(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                redact_keys(value);
            }
        }
        _ => {}
    }
}

/// Retains existing CLI spellings while adding complete provider and pipeline editing.
fn config(
    application: &OperitApplication,
    manager: &TtsConfigManager,
    args: &[String],
) -> Result<serde_json::Value, String> {
    Ok(match args {
        [command] if command == "list" => {
            json!({"configs": manager.getAllTtsConfigs()?, "currentConfigId": manager.getCurrentTtsConfigId()?})
        }
        [command, id] if command == "show" => json!(manager.getTtsConfig(id)?),
        [command] if command == "current" => json!(manager.getCurrentTtsConfig()?),
        [command, id] if command == "use" => {
            json!({"currentConfigId": manager.setCurrentTtsConfigId(id)?})
        }
        [command, id] if command == "delete" => {
            json!({"deleted": manager.deleteTtsConfig(id)?, "id": id})
        }
        [command, input] if command == "create-json" || command == "save" => {
            let config = serde_json::from_str(&read_content_arg(&application.hostManager, input)?)
                .map_err(|error| error.to_string())?;
            json!(if command == "create-json" {
                manager.createTtsConfig(config)?
            } else {
                manager.updateTtsConfig(config)?
            })
        }
        [command, id, field, value] if command == "update" => {
            let mut config = manager.getTtsConfig(id)?;
            match field.as_str() {
                "name" => config.name = value.clone(),
                "endpoint" => config.endpoint = value.clone(),
                "api-key" => config.apiKey = value.clone(),
                "model" => config.model = value.clone(),
                "voice" => config.voice = value.clone(),
                "response-format" => config.responseFormat = value.clone(),
                "speed" => {
                    config.speed = value.parse::<f64>().map_err(|error| error.to_string())?
                }
                "http-method" => config.httpMethod = value.clone(),
                "request-body" => config.requestBody = value.clone(),
                "content-type" => config.contentType = value.clone(),
                _ => return Err(format!("Unknown TTS config field: {field}")),
            }
            json!(manager.updateTtsConfig(config)?)
        }
        [command, name, endpoint, key, model, voice, format, speed] if command == "create" => {
            json!(manager.createTtsConfig(new_config(
                name,
                "OPENAI_COMPATIBLE",
                endpoint,
                key,
                model,
                voice,
                format,
                speed
            )?)?)
        }
        [command, name, model, version, voice, speed] if command == "create-local" => {
            json!(manager.createTtsConfig(new_config(
                name,
                &TtsProviderType::LOCAL_MODEL.to_string(),
                "",
                "",
                &format!("{model}@{version}"),
                voice,
                "wav",
                speed
            )?)?)
        }
        _ => return Err("Invalid TTS config arguments; use 'operit2 tts'".into()),
    })
}

/// Constructs the existing CLI profile with provider-specific defaults and validated speed syntax.
fn new_config(
    name: &str,
    provider: &str,
    endpoint: &str,
    key: &str,
    model: &str,
    voice: &str,
    format: &str,
    speed: &str,
) -> Result<TtsConfig, String> {
    Ok(TtsConfig {
        id: String::new(),
        name: name.into(),
        providerType: provider.into(),
        endpoint: endpoint.into(),
        apiKey: key.into(),
        model: model.into(),
        voice: voice.into(),
        responseFormat: format.into(),
        speed: speed.parse::<f64>().map_err(|error| error.to_string())?,
        httpMethod: "POST".into(),
        requestBody: String::new(),
        contentType: "application/json".into(),
        headers: vec![],
        responsePipeline: vec![],
        createdAt: 0,
        updatedAt: 0,
    })
}
