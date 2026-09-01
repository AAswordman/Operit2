use crate::commands::util::{parse_bool_arg, parse_f32_arg, parse_i32_arg};
use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_model::ApiKeyInfo::ApiKeyInfo;
use operit_model::FunctionType::FunctionType;
use operit_model::ModelCatalog::ModelCatalog;
use operit_model::ModelConfigData::{
    ModelContextSpec, ModelPricing, ModelSummarySettings, ProviderProfile, ResolvedModelConfig,
};
use operit_model::ModelParameter::ModelParameter;
use operit_runtime::data::preferences::FunctionalConfigManager::{
    FunctionModelBinding, FunctionalConfigManager,
};
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use serde_json::{json, Value};

struct ModelCommand;

impl ModelCommand {
    /// Returns the model configuration manager.
    fn modelManager(&mut self) -> ModelConfigManager {
        ModelConfigManager::default()
    }

    /// Returns the functional model binding manager.
    fn functionalManager(&mut self) -> FunctionalConfigManager {
        FunctionalConfigManager::default()
    }
}

/// Runs model provider, model profile, and functional binding commands.
pub fn run_model_command(
    _context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let mut command = ModelCommand;
    if args.is_empty() {
        print_model_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "provider-type-list" => {
            let mut providers = ModelCatalog::providers().map_err(|error| error.to_string())?;
            providers.sort_by(|left, right| left.providerTypeId.cmp(&right.providerTypeId));
            output.push_stdout_line(format!("Provider types: {}", providers.len()));
            for provider in &providers {
                output.push_stdout_line(format!(
                    "- {} | {} | models: {} | {}",
                    provider.providerTypeId,
                    provider.displayName,
                    provider.models.len(),
                    provider.defaultEndpoint
                ));
            }
            output.setJsonStdout(
                serde_json::to_value(&providers).map_err(|error| error.to_string())?,
            );
        }
        "provider-list" => {
            let providers = command
                .modelManager()
                .getProviderProfiles()
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Providers: {}", providers.len()));
            for provider in &providers {
                output.push_stdout_line(format!(
                    "- {} | {} | {} | models: {} | {}",
                    provider.id,
                    provider.name,
                    provider.providerTypeId,
                    provider.models.len(),
                    provider.endpoint
                ));
            }
            output.setJsonStdout(json!({
                "providers": providers
                    .iter()
                    .map(provider_profile_json)
                    .collect::<Vec<_>>()
            }));
        }
        "provider-show" => {
            let providerId =
                requiredArg(args, 1, "usage: operit2 model provider-show <provider-id>")?;
            let provider = command
                .modelManager()
                .getProviderProfile(providerId)
                .map_err(|error| error.to_string())?;
            print_provider_profile(&provider, output);
            output.setJsonStdout(provider_profile_json(&provider));
        }
        "provider-create" => {
            let name = requiredArg(
                args,
                1,
                "usage: operit2 model provider-create <name> <provider-type-id> <endpoint>",
            )?
            .to_string();
            let providerTypeId = requiredArg(
                args,
                2,
                "usage: operit2 model provider-create <name> <provider-type-id> <endpoint>",
            )?
            .to_string();
            let endpoint = requiredArg(
                args,
                3,
                "usage: operit2 model provider-create <name> <provider-type-id> <endpoint>",
            )?
            .to_string();
            let providerId = command
                .modelManager()
                .createProvider(name.clone(), providerTypeId.clone(), endpoint.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Created provider {providerId}"));
            output.setJsonStdout(json!({
                "providerId": providerId,
                "name": name,
                "providerTypeId": providerTypeId,
                "endpoint": endpoint
            }));
        }
        "provider-set-key" => {
            let providerId = requiredArg(
                args,
                1,
                "usage: operit2 model provider-set-key <provider-id> <api-key>",
            )?;
            let apiKey = requiredArg(
                args,
                2,
                "usage: operit2 model provider-set-key <provider-id> <api-key>",
            )?
            .to_string();
            let apiKeyLength = apiKey.len();
            let manager = command.modelManager();
            let mut provider = manager
                .getProviderProfile(providerId)
                .map_err(|error| error.to_string())?;
            provider.apiKey = apiKey;
            manager
                .updateProviderProfile(provider)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Updated provider API key for {providerId}"));
            output.push_stdout_line(format!("API key length: {apiKeyLength}"));
            output.setJsonStdout(json!({
                "providerId": providerId,
                "apiKeyLength": apiKeyLength,
                "updated": true
            }));
        }
        "provider-set-endpoint" => {
            let providerId = requiredArg(
                args,
                1,
                "usage: operit2 model provider-set-endpoint <provider-id> <endpoint>",
            )?;
            let endpoint = requiredArg(
                args,
                2,
                "usage: operit2 model provider-set-endpoint <provider-id> <endpoint>",
            )?
            .to_string();
            let manager = command.modelManager();
            let mut provider = manager
                .getProviderProfile(providerId)
                .map_err(|error| error.to_string())?;
            provider.endpoint = endpoint.clone();
            manager
                .updateProviderProfile(provider)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Updated provider endpoint for {providerId}"));
            output.push_stdout_line(format!("Endpoint: {endpoint}"));
            output.setJsonStdout(json!({
                "providerId": providerId,
                "endpoint": endpoint,
                "updated": true
            }));
        }
        "provider-model-available-list" => {
            let providerId = requiredArg(
                args,
                1,
                "usage: operit2 model provider-model-available-list <provider-id>",
            )?;
            let mut models = command
                .modelManager()
                .getAvailableProviderModels(providerId)
                .map_err(|error| error.to_string())?;
            models.sort_by(|left, right| left.modelId.cmp(&right.modelId));
            output.push_stdout_line(format!("Available provider models: {}", models.len()));
            for model in &models {
                output.push_stdout_line(format!(
                    "- {} | source: {:?} | pricing: {} | context: {} | capabilities: {} | request: {}",
                    model.modelId,
                    model.source,
                    model.pricing.is_some(),
                    model.context.is_some(),
                    model.capabilities.is_some(),
                    model.request.is_some()
                ));
            }
            output.setJsonStdout(serde_json::to_value(&models).map_err(|error| error.to_string())?);
        }
        "provider-model-add" => {
            let providerId = requiredArg(
                args,
                1,
                "usage: operit2 model provider-model-add <provider-id> <model-id>",
            )?;
            let modelId = requiredArg(
                args,
                2,
                "usage: operit2 model provider-model-add <provider-id> <model-id>",
            )?
            .to_string();
            let modelId = command
                .modelManager()
                .addProviderModelFromAvailable(providerId, modelId)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Added provider model {modelId}"));
            output.setJsonStdout(json!({
                "providerId": providerId,
                "modelId": modelId,
                "added": true
            }));
        }
        "provider-model-create" => {
            let providerId = requiredArg(
                args,
                1,
                "usage: operit2 model provider-model-create <provider-id> <model-id>",
            )?;
            let modelId = requiredArg(
                args,
                2,
                "usage: operit2 model provider-model-create <provider-id> <model-id>",
            )?
            .to_string();
            let modelId = command
                .modelManager()
                .createProviderModel(providerId, modelId)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Created provider model {modelId}"));
            output.setJsonStdout(json!({
                "providerId": providerId,
                "modelId": modelId,
                "created": true
            }));
        }
        "list" => {
            let mut summaries = command
                .modelManager()
                .getAllModelSummaries()
                .map_err(|error| error.to_string())?;
            summaries.sort_by(|left, right| {
                left.providerName
                    .cmp(&right.providerName)
                    .then(left.modelId.cmp(&right.modelId))
            });
            output.push_stdout_line(format!("Configured models: {}", summaries.len()));
            for summary in &summaries {
                output.push_stdout_line(format!(
                    "- {}:{} | {} | {}",
                    summary.providerId,
                    summary.modelId,
                    summary.providerName,
                    summary.providerTypeId
                ));
            }
            output.setJsonStdout(
                serde_json::to_value(&summaries).map_err(|error| error.to_string())?,
            );
        }
        "show" => {
            let providerId = args
                .get(1)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_PROVIDER_ID);
            let modelId = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_MODEL_ID);
            let config = command
                .modelManager()
                .getResolvedModelConfig(providerId, modelId)
                .map_err(|error| error.to_string())?;
            print_resolved_model_config(&config, output);
            output.setJsonStdout(resolved_model_config_json(&config));
        }
        "use" => {
            let providerId =
                requiredArg(args, 1, "usage: operit2 model use <provider-id> <model-id>")?
                    .to_string();
            let modelId =
                requiredArg(args, 2, "usage: operit2 model use <provider-id> <model-id>")?
                    .to_string();
            command
                .functionalManager()
                .setModelForFunction(FunctionType::CHAT, providerId.clone(), modelId.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Updated chat model to {providerId}:{modelId}"));
            output.setJsonStdout(json!({
                "functionType": "CHAT",
                "providerId": providerId,
                "modelId": modelId,
                "updated": true
            }));
        }
        "params" => {
            let providerId = args
                .get(1)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_PROVIDER_ID);
            let modelId = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_MODEL_ID);
            let params = command
                .modelManager()
                .getModelParametersForModel(providerId, modelId)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Model parameters: {}", params.len()));
            for param in &params {
                output.push_stdout_line(format!(
                    "- {} | api: {} | enabled: {} | value: {}",
                    param.id, param.apiName, param.isEnabled, param.currentValue
                ));
            }
            output.setJsonStdout(serde_json::to_value(&params).map_err(|error| error.to_string())?);
        }
        "parameters" => {
            let providerId = requiredArg(
                args,
                1,
                "usage: operit2 model parameters <provider-id> <model-id> <parameters-json>",
            )?;
            let modelId = requiredArg(
                args,
                2,
                "usage: operit2 model parameters <provider-id> <model-id> <parameters-json>",
            )?;
            let parametersJson = requiredArg(
                args,
                3,
                "usage: operit2 model parameters <provider-id> <model-id> <parameters-json>",
            )?;
            let parameters =
                serde_json::from_str::<Vec<ModelParameter<serde_json::Value>>>(parametersJson)
                    .map_err(|error| error.to_string())?;
            command
                .modelManager()
                .updateParametersForModel(providerId, modelId, parameters.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Updated parameters for {providerId}:{modelId}"));
            output.setJsonStdout(json!({
                "providerId": providerId,
                "modelId": modelId,
                "parameters": parameters,
                "updated": true
            }));
        }
        "context-show" => {
            let providerId = args
                .get(1)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_PROVIDER_ID);
            let modelId = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_MODEL_ID);
            let config = command
                .modelManager()
                .getResolvedModelConfig(providerId, modelId)
                .map_err(|error| error.to_string())?;
            print_context_spec(&config.providerId, &config.modelId, &config.context, output);
            output.setJsonStdout(json!({
                "providerId": config.providerId,
                "modelId": config.modelId,
                "context": config.context
            }));
        }
        "context-set" => {
            let providerId = requiredArg(args, 1, "usage: operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>")?;
            let modelId = requiredArg(args, 2, "usage: operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>")?;
            let maxContextLength = parse_f32_arg(args.get(3), "usage: operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>")?;
            let enableMaxContextMode = parse_bool_arg(args.get(4), "usage: operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>")?;
            let context = ModelContextSpec {
                maxContextLength,
                enableMaxContextMode,
            };
            let model = command
                .modelManager()
                .updateContextForModel(providerId, modelId, context.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Updated context for {providerId}:{modelId}"));
            print_context_spec(providerId, modelId, &context, output);
            output.setJsonStdout(json!({
                "providerId": providerId,
                "modelId": modelId,
                "context": context,
                "model": model,
                "updated": true
            }));
        }
        "summary-show" => {
            let providerId = args
                .get(1)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_PROVIDER_ID);
            let modelId = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_MODEL_ID);
            let config = command
                .modelManager()
                .getResolvedModelConfig(providerId, modelId)
                .map_err(|error| error.to_string())?;
            print_summary_settings(&config.providerId, &config.modelId, &config.summary, output);
            output.setJsonStdout(json!({
                "providerId": config.providerId,
                "modelId": config.modelId,
                "summary": config.summary
            }));
        }
        "summary-set" => {
            let providerId = requiredArg(args, 1, "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let modelId = requiredArg(args, 2, "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let enableSummary = parse_bool_arg(args.get(3), "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let summaryTokenThreshold = parse_f32_arg(args.get(4), "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let enableSummaryByMessageCount = parse_bool_arg(args.get(5), "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let summaryMessageCountThreshold = parse_i32_arg(args.get(6), "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let summary = ModelSummarySettings {
                enableSummary,
                summaryTokenThreshold,
                enableSummaryByMessageCount,
                summaryMessageCountThreshold,
            };
            let model = command
                .modelManager()
                .updateSummaryForModel(providerId, modelId, summary.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Updated summary for {providerId}:{modelId}"));
            print_summary_settings(providerId, modelId, &summary, output);
            output.setJsonStdout(json!({
                "providerId": providerId,
                "modelId": modelId,
                "summary": summary,
                "model": model,
                "updated": true
            }));
        }
        "function-list" => {
            let mut rows = functionTypes()
                .into_iter()
                .map(|functionType| {
                    command
                        .functionalManager()
                        .getModelBindingForFunction(functionType.clone())
                        .map(|binding| (functionType, binding))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            rows.sort_by(|left, right| functionTypeName(&left.0).cmp(functionTypeName(&right.0)));
            output.push_stdout_line(format!("Function model bindings: {}", rows.len()));
            for (functionType, binding) in &rows {
                output.push_stdout_line(format!(
                    "- {} -> {}:{}",
                    functionTypeName(functionType),
                    binding.providerId,
                    binding.modelId
                ));
            }
            output.setJsonStdout(json!({
                "bindings": rows
                    .iter()
                    .map(|(functionType, binding)| function_binding_json(functionType, binding))
                    .collect::<Vec<_>>()
            }));
        }
        "function-show" => {
            let functionType = parseFunctionType(requiredArg(
                args,
                1,
                "usage: operit2 model function-show <function-type>",
            )?)?;
            let binding = command
                .functionalManager()
                .getModelBindingForFunction(functionType.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!(
                "{} -> {}:{}",
                functionTypeName(&functionType),
                binding.providerId,
                binding.modelId
            ));
            output.setJsonStdout(function_binding_json(&functionType, &binding));
        }
        "function-set" => {
            let functionType = parseFunctionType(requiredArg(
                args,
                1,
                "usage: operit2 model function-set <function-type> <provider-id> <model-id>",
            )?)?;
            let providerId = requiredArg(
                args,
                2,
                "usage: operit2 model function-set <function-type> <provider-id> <model-id>",
            )?
            .to_string();
            let modelId = requiredArg(
                args,
                3,
                "usage: operit2 model function-set <function-type> <provider-id> <model-id>",
            )?
            .to_string();
            command
                .functionalManager()
                .setModelForFunction(functionType.clone(), providerId.clone(), modelId.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!(
                "Updated {} -> {}:{}",
                functionTypeName(&functionType),
                providerId,
                modelId
            ));
            output.setJsonStdout(json!({
                "functionType": functionTypeName(&functionType),
                "providerId": providerId,
                "modelId": modelId,
                "updated": true
            }));
        }
        "function-reset" => {
            if let Some(functionTypeValue) = args.get(1) {
                let functionType = parseFunctionType(functionTypeValue)?;
                command
                    .functionalManager()
                    .resetFunctionConfig(functionType.clone())
                    .map_err(|error| error.to_string())?;
                output.push_stdout_line(format!("Reset {}", functionTypeName(&functionType)));
                output.setJsonStdout(json!({
                    "functionType": functionTypeName(&functionType),
                    "reset": true
                }));
            } else {
                command
                    .functionalManager()
                    .resetAllFunctionConfigs()
                    .map_err(|error| error.to_string())?;
                output.push_stdout_line("Reset all function model bindings");
                output.setJsonStdout(json!({ "resetAll": true }));
            }
        }
        _ => print_model_usage(output),
    }

    Ok(())
}

/// Returns one required command argument.
fn requiredArg<'a>(args: &'a [String], index: usize, usage: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| usage.to_string())
}

/// Parses a functional model role accepted by the model command.
fn parseFunctionType(value: &str) -> Result<FunctionType, String> {
    match value {
        "CHAT" => Ok(FunctionType::CHAT),
        "SUMMARY" => Ok(FunctionType::SUMMARY),
        "TITLE_GENERATION" => Ok(FunctionType::TITLE_GENERATION),
        "MEMORY" => Ok(FunctionType::MEMORY),
        "UI_CONTROLLER" => Ok(FunctionType::UI_CONTROLLER),
        "TRANSLATION" => Ok(FunctionType::TRANSLATION),
        "GREP" => Ok(FunctionType::GREP),
        "ROLE_RESPONSE_PLANNER" => Ok(FunctionType::ROLE_RESPONSE_PLANNER),
        "IMAGE_RECOGNITION" => Ok(FunctionType::IMAGE_RECOGNITION),
        "AUDIO_RECOGNITION" => Ok(FunctionType::AUDIO_RECOGNITION),
        "VIDEO_RECOGNITION" => Ok(FunctionType::VIDEO_RECOGNITION),
        other => Err(format!("invalid FunctionType: {other}")),
    }
}

/// Lists functional model roles supported by the model command.
fn functionTypes() -> Vec<FunctionType> {
    vec![
        FunctionType::CHAT,
        FunctionType::SUMMARY,
        FunctionType::TITLE_GENERATION,
        FunctionType::MEMORY,
        FunctionType::UI_CONTROLLER,
        FunctionType::TRANSLATION,
        FunctionType::GREP,
        FunctionType::ROLE_RESPONSE_PLANNER,
        FunctionType::IMAGE_RECOGNITION,
        FunctionType::AUDIO_RECOGNITION,
        FunctionType::VIDEO_RECOGNITION,
    ]
}

/// Formats a functional model role for command output.
fn functionTypeName(functionType: &FunctionType) -> &'static str {
    match functionType {
        FunctionType::CHAT => "CHAT",
        FunctionType::SUMMARY => "SUMMARY",
        FunctionType::TITLE_GENERATION => "TITLE_GENERATION",
        FunctionType::MEMORY => "MEMORY",
        FunctionType::UI_CONTROLLER => "UI_CONTROLLER",
        FunctionType::TRANSLATION => "TRANSLATION",
        FunctionType::GREP => "GREP",
        FunctionType::ROLE_RESPONSE_PLANNER => "ROLE_RESPONSE_PLANNER",
        FunctionType::IMAGE_RECOGNITION => "IMAGE_RECOGNITION",
        FunctionType::AUDIO_RECOGNITION => "AUDIO_RECOGNITION",
        FunctionType::VIDEO_RECOGNITION => "VIDEO_RECOGNITION",
    }
}

/// Prints a provider profile without API key material.
fn print_provider_profile(provider: &ProviderProfile, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("Provider {}", provider.id));
    output.push_stdout_line(format!("Name: {}", provider.name));
    output.push_stdout_line(format!("Type: {}", provider.providerType.name()));
    output.push_stdout_line(format!("Endpoint: {}", provider.endpoint));
    output.push_stdout_line(format!("API key length: {}", provider.apiKey.len()));
    output.push_stdout_line(format!("Multiple keys: {}", provider.useMultipleApiKeys));
    output.push_stdout_line(format!("API key pool size: {}", provider.apiKeyPool.len()));
    output.push_stdout_line(format!("Current key index: {}", provider.currentKeyIndex));
    output.push_stdout_line(format!("Key rotation mode: {}", provider.keyRotationMode));
    output.push_stdout_line(format!("Custom headers: {}", provider.customHeaders));
    output.push_stdout_line(format!(
        "Request limit per minute: {}",
        provider.requestLimitPerMinute
    ));
    output.push_stdout_line(format!(
        "Max concurrent requests: {}",
        provider.maxConcurrentRequests
    ));
    output.push_stdout_line(format!("Models: {}", provider.models.len()));
    for model in &provider.models {
        output.push_stdout_line(format!("- {}", model.id));
    }
}

/// Prints a resolved model config without API key material.
fn print_resolved_model_config(config: &ResolvedModelConfig, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("Model {}:{}", config.providerId, config.modelId));
    output.push_stdout_line(format!("Provider: {}", config.providerName));
    output.push_stdout_line(format!("Provider type: {}", config.apiProviderType.name()));
    output.push_stdout_line(format!("Endpoint: {}", config.apiEndpoint));
    output.push_stdout_line(format!("API key length: {}", config.apiKey.len()));
    output.push_stdout_line(format!("Custom headers: {}", config.customHeaders));
    output.push_stdout_line(format!(
        "Request limit per minute: {}",
        config.requestLimitPerMinute
    ));
    output.push_stdout_line(format!(
        "Max concurrent requests: {}",
        config.maxConcurrentRequests
    ));
    output.push_stdout_line(format!(
        "Structured tools: {}",
        config.request.supportsStructuredTools
    ));
    output.push_stdout_line(format!(
        "Context tokens: {}",
        config.context.maxContextLength
    ));
    output.push_stdout_line(format!(
        "Max context mode: {}",
        config.context.enableMaxContextMode
    ));
    output.push_stdout_line(format!("Direct image: {}", config.capabilities.directImage));
    output.push_stdout_line(format!("Direct audio: {}", config.capabilities.directAudio));
    output.push_stdout_line(format!("Direct video: {}", config.capabilities.directVideo));
    output.push_stdout_line(format!("Tool calls: {}", config.capabilities.toolCall));
    output.push_stdout_line(format!("Builtin tools: {}", config.builtinTools.len()));
    output.push_stdout_line(format!("Parameters: {}", config.parameters.len()));
    print_summary_settings(&config.providerId, &config.modelId, &config.summary, output);
    print_pricing(config.pricing.as_ref(), output);
}

/// Prints context settings for one provider model.
fn print_context_spec(
    providerId: &str,
    modelId: &str,
    context: &ModelContextSpec,
    output: &mut CoreCommandOutput,
) {
    output.push_stdout_line(format!("Context for {providerId}:{modelId}"));
    output.push_stdout_line(format!("Max context length: {}", context.maxContextLength));
    output.push_stdout_line(format!(
        "Max context mode: {}",
        context.enableMaxContextMode
    ));
}

/// Prints summary settings for one provider model.
fn print_summary_settings(
    providerId: &str,
    modelId: &str,
    summary: &ModelSummarySettings,
    output: &mut CoreCommandOutput,
) {
    output.push_stdout_line(format!("Summary for {providerId}:{modelId}"));
    output.push_stdout_line(format!("Enabled: {}", summary.enableSummary));
    output.push_stdout_line(format!(
        "Token threshold: {}",
        summary.summaryTokenThreshold
    ));
    output.push_stdout_line(format!(
        "Message-count summary: {}",
        summary.enableSummaryByMessageCount
    ));
    output.push_stdout_line(format!(
        "Message-count threshold: {}",
        summary.summaryMessageCountThreshold
    ));
}

/// Prints pricing settings for a resolved model.
fn print_pricing(pricing: Option<&ModelPricing>, output: &mut CoreCommandOutput) {
    match pricing {
        Some(pricing) => {
            output.push_stdout_line("Pricing");
            output.push_stdout_line(format!("Billing mode: {:?}", pricing.billingMode));
            output.push_stdout_line(format!(
                "Input price per million: {}",
                pricing.inputPricePerMillion
            ));
            output.push_stdout_line(format!(
                "Cached input price per million: {}",
                optional_number(pricing.cachedInputPricePerMillion)
            ));
            output.push_stdout_line(format!(
                "Cache write price per million: {}",
                optional_number(pricing.cacheWritePricePerMillion)
            ));
            output.push_stdout_line(format!(
                "Output price per million: {}",
                pricing.outputPricePerMillion
            ));
            output.push_stdout_line(format!("Price per request: {}", pricing.pricePerRequest));
            output.push_stdout_line(format!("Currency: {}", pricing.currency.code()));
        }
        None => output.push_stdout_line("Pricing: not configured"),
    }
}

/// Formats an optional number for command output.
fn optional_number(value: Option<f64>) -> String {
    match value {
        Some(number) => number.to_string(),
        None => "-".to_string(),
    }
}

/// Builds a provider profile JSON document without API key material.
fn provider_profile_json(provider: &ProviderProfile) -> Value {
    json!({
        "id": &provider.id,
        "name": &provider.name,
        "providerTypeId": &provider.providerTypeId,
        "providerType": provider.providerType.name(),
        "endpoint": &provider.endpoint,
        "apiKeyLength": provider.apiKey.len(),
        "useMultipleApiKeys": provider.useMultipleApiKeys,
        "apiKeyPool": api_key_pool_json(&provider.apiKeyPool),
        "currentKeyIndex": provider.currentKeyIndex,
        "keyRotationMode": &provider.keyRotationMode,
        "customHeaders": &provider.customHeaders,
        "requestLimitPerMinute": provider.requestLimitPerMinute,
        "maxConcurrentRequests": provider.maxConcurrentRequests,
        "models": &provider.models
    })
}

/// Builds a resolved model config JSON document without API key material.
fn resolved_model_config_json(config: &ResolvedModelConfig) -> Value {
    json!({
        "providerId": &config.providerId,
        "providerName": &config.providerName,
        "modelId": &config.modelId,
        "apiKeyLength": config.apiKey.len(),
        "apiEndpoint": &config.apiEndpoint,
        "apiProviderType": config.apiProviderType.name(),
        "apiProviderTypeId": &config.apiProviderTypeId,
        "useMultipleApiKeys": config.useMultipleApiKeys,
        "apiKeyPool": api_key_pool_json(&config.apiKeyPool),
        "currentKeyIndex": config.currentKeyIndex,
        "keyRotationMode": &config.keyRotationMode,
        "customHeaders": &config.customHeaders,
        "requestLimitPerMinute": config.requestLimitPerMinute,
        "maxConcurrentRequests": config.maxConcurrentRequests,
        "pricing": &config.pricing,
        "context": &config.context,
        "capabilities": &config.capabilities,
        "builtinTools": &config.builtinTools,
        "request": &config.request,
        "parameters": &config.parameters,
        "summary": &config.summary,
        "localRuntime": &config.localRuntime
    })
}

/// Builds API key pool metadata without API key material.
fn api_key_pool_json(apiKeyPool: &[ApiKeyInfo]) -> Vec<Value> {
    apiKeyPool
        .iter()
        .map(|apiKey| {
            json!({
                "id": &apiKey.id,
                "name": &apiKey.name,
                "isEnabled": apiKey.isEnabled,
                "availabilityStatus": &apiKey.availabilityStatus,
                "usageCount": apiKey.usageCount,
                "lastUsed": apiKey.lastUsed,
                "errorCount": apiKey.errorCount,
                "keyLength": apiKey.key.len()
            })
        })
        .collect()
}

/// Builds one functional binding JSON row.
fn function_binding_json(functionType: &FunctionType, binding: &FunctionModelBinding) -> Value {
    json!({
        "functionType": functionTypeName(functionType),
        "providerId": &binding.providerId,
        "modelId": &binding.modelId
    })
}

/// Prints model command usage.
fn print_model_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 model provider-type-list",
        "operit2 model provider-list",
        "operit2 model provider-show <provider-id>",
        "operit2 model provider-create <name> <provider-type-id> <endpoint>",
        "operit2 model provider-set-key <provider-id> <api-key>",
        "operit2 model provider-set-endpoint <provider-id> <endpoint>",
        "operit2 model provider-model-available-list <provider-id>",
        "operit2 model provider-model-add <provider-id> <provider-model-id>",
        "operit2 model provider-model-create <provider-id> <provider-model-id>",
        "operit2 model list",
        "operit2 model show [provider-id] [model-id]",
        "operit2 model use <provider-id> <model-id>",
        "operit2 model params [provider-id] [model-id]",
        "operit2 model parameters <provider-id> <model-id> <parameters-json>",
        "operit2 model context-show [provider-id] [model-id]",
        "operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>",
        "operit2 model summary-show [provider-id] [model-id]",
        "operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>",
        "operit2 model function-list",
        "operit2 model function-show <function-type>",
        "operit2 model function-set <function-type> <provider-id> <model-id>",
        "operit2 model function-reset [function-type]",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}
