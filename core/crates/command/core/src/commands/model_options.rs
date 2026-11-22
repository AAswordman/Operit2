use operit_host_api::HostManager::HostManager;
use operit_model::ModelConfigData::{ModelProfile, ProviderProfile};
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use serde_json::{json, Value};

use super::util::{parse_bool_arg, parse_i32_arg, read_content_arg};

/// Applies model settings through the same preference owner used by graphical clients.
pub(super) fn run_model_options_command(
    context: &HostManager,
    args: &[String],
) -> Result<Value, String> {
    let manager = ModelConfigManager::default();
    let command = args[0].as_str();
    if command == "provider-set" {
        if args.len() != 4 {
            return Err(
                "usage: operit2 model provider-set <provider-id> <field> <value|@vfs-file>".into(),
            );
        }
        let mut provider = manager
            .getProviderProfile(&args[1])
            .map_err(|error| error.to_string())?;
        let value = read_content_arg(context, &args[3])?;
        set_provider_field(&mut provider, &args[2], &value)?;
        let provider = manager
            .updateProviderProfile(provider)
            .map_err(|error| error.to_string())?;
        return Ok(super::model::provider_profile_json(&provider));
    }
    let expected = match command {
        "capabilities-show" | "request-show" | "profile-show" | "builtin-tools-show" => 3,
        "capabilities-set" | "request-set" | "profile-set" | "builtin-tool-set" | "param-set" => 5,
        _ => return Err(format!("unknown model options command: {command}")),
    };
    if args.len() != expected && !(command == "param-set" && args.len() == 6) {
        return Err(format!(
            "invalid arguments for model {command}; run operit2 model for usage"
        ));
    }
    let provider = &args[1];
    let model_id = &args[2];
    let mut model = manager
        .getModelProfile(provider, model_id)
        .map_err(|error| error.to_string())?;
    match command {
        "profile-show" => return Ok(json!(model)),
        "profile-set" => {
            let value = read_content_arg(context, &args[4])?;
            model = set_profile_section(
                model,
                &args[3],
                serde_json::from_str(&value).map_err(|error| error.to_string())?,
            )?;
        }
        "capabilities-set" | "request-set" => {
            let value = match args[4].as_str() {
                "auto" => None,
                _ => Some(parse_bool_arg(args.get(4), "expected true | false | auto")?),
            };
            set_model_switch(&mut model, command, &args[3], value)?;
        }
        "builtin-tool-set" => {
            let kind = match args[3].as_str() {
                "WebSearch" => operit_model::ModelConfigData::BuiltinToolType::WebSearch,
                "CodeExecution" => operit_model::ModelConfigData::BuiltinToolType::CodeExecution,
                "UrlContext" => operit_model::ModelConfigData::BuiltinToolType::UrlContext,
                other => return Err(format!("unknown built-in tool: {other}")),
            };
            let enabled = parse_bool_arg(args.get(4), "expected true | false")?;
            let mut tools = manager
                .getResolvedModelConfig(provider, model_id)
                .map_err(|error| error.to_string())?
                .builtinTools;
            let matches = tools
                .iter_mut()
                .filter(|tool| tool.toolType == kind)
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(format!("expected one configured {} tool, found {}; use builtin-tools to edit the full list", args[3], matches.len()));
            }
            for tool in matches {
                tool.enabled = enabled;
            }
            model.builtinToolsOverride = Some(tools);
        }
        "param-set" => {
            let enabled = parse_bool_arg(args.get(4), "expected true | false")?;
            let parameter = model
                .parameters
                .iter_mut()
                .find(|parameter| parameter.id == args[3])
                .ok_or_else(|| format!("unknown model parameter: {}", args[3]))?;
            if let Some(raw) = args.get(5) {
                parameter.currentValue = serde_json::from_str(&read_content_arg(context, raw)?)
                    .map_err(|error| error.to_string())?;
            }
            parameter.isEnabled = enabled;
        }
        _ => {}
    }
    let updated = matches!(
        command,
        "capabilities-set" | "request-set" | "profile-set" | "builtin-tool-set" | "param-set"
    );
    if updated {
        model = manager
            .updateModelProfile(provider, model)
            .map_err(|error| error.to_string())?;
    }
    if command == "profile-set" || command == "param-set" {
        return Ok(json!(model));
    }
    let config = manager
        .getResolvedModelConfig(provider, model_id)
        .map_err(|error| error.to_string())?;
    Ok(match command {
        "capabilities-show" | "capabilities-set" => json!({
            "providerId": provider, "modelId": model_id, "updated": updated,
            "capabilities": config.capabilities, "overrides": model.capabilitiesOverride,
        }),
        "request-show" | "request-set" => json!({
            "providerId": provider, "modelId": model_id, "updated": updated,
            "request": config.request, "overrides": model.requestOverride,
        }),
        "builtin-tools-show" | "builtin-tool-set" => json!({
            "providerId": provider, "modelId": model_id, "updated": updated,
            "builtinTools": config.builtinTools,
        }),
        _ => return Err(format!("unknown model options result: {command}")),
    })
}

/// Changes one explicit override while retaining every unrelated capability and request setting.
fn set_model_switch(
    model: &mut ModelProfile,
    command: &str,
    field: &str,
    value: Option<bool>,
) -> Result<(), String> {
    match (command, field) {
        ("capabilities-set", "tool-call") => model.capabilitiesOverride.toolCall = value,
        ("capabilities-set", "direct-image") => model.capabilitiesOverride.directImage = value,
        ("capabilities-set", "direct-audio") => model.capabilitiesOverride.directAudio = value,
        ("capabilities-set", "direct-video") => model.capabilitiesOverride.directVideo = value,
        ("request-set", "structured-tools") => {
            model.requestOverride.supportsStructuredTools = value
        }
        ("request-set", "claude-1h-prompt-cache") => {
            model.requestOverride.enableClaude1hPromptCache = value
        }
        _ => return Err(format!("unknown field for {command}: {field}")),
    }
    Ok(())
}

/// Replaces one typed profile section without changing the model identity or neighboring sections.
fn set_profile_section(
    model: ModelProfile,
    field: &str,
    value: Value,
) -> Result<ModelProfile, String> {
    let key = match field {
        "pricing" => "pricingOverride",
        "context" => "contextOverride",
        "capabilities" => "capabilitiesOverride",
        "builtin-tools" => "builtinToolsOverride",
        "request" => "requestOverride",
        "thinking-spec" => "thinkingOverride",
        "thinking-level" => "thinkingLevel",
        "parameters" => "parameters",
        "summary" => "summary",
        "local-runtime" => "localRuntime",
        _ => return Err(format!("unknown model profile section: {field}")),
    };
    let mut document = serde_json::to_value(model).map_err(|error| error.to_string())?;
    document[key] = value;
    serde_json::from_value(document).map_err(|error| error.to_string())
}

/// Edits provider connection controls without replacing models, credentials or key health unintentionally.
fn set_provider_field(
    provider: &mut ProviderProfile,
    field: &str,
    value: &str,
) -> Result<(), String> {
    let argument = value.to_string();
    match field {
        "name" if !value.trim().is_empty() => provider.name = argument,
        "endpoint" => provider.endpoint = argument,
        "api-key" => provider.apiKey = argument,
        "multiple-api-keys" => {
            provider.useMultipleApiKeys = parse_bool_arg(Some(&argument), "expected true | false")?
        }
        "api-key-pool" => {
            provider.apiKeyPool = serde_json::from_str(value).map_err(|error| error.to_string())?
        }
        "custom-headers" => {
            let headers: std::collections::BTreeMap<String, String> =
                serde_json::from_str(value).map_err(|error| error.to_string())?;
            provider.customHeaders =
                serde_json::to_string(&headers).map_err(|error| error.to_string())?;
        }
        "request-limit" | "max-concurrent" => {
            let limit = parse_i32_arg(Some(&argument), "expected a nonnegative integer")?;
            if limit < 0 {
                return Err("request limits must be nonnegative; zero means unlimited".into());
            }
            if field == "request-limit" {
                provider.requestLimitPerMinute = limit;
            } else {
                provider.maxConcurrentRequests = limit;
            }
        }
        _ => return Err(format!("invalid provider field or value: {field}")),
    }
    Ok(())
}
