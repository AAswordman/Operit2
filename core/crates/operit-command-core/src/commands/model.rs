use std::cell::Cell;

use crate::commands::util::{parse_bool_arg, parse_f32_arg, parse_i32_arg};
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::data::model::FunctionType::FunctionType;
use operit_runtime::data::model::ModelCatalog::ModelCatalog;
use operit_runtime::data::model::ModelConfigData::{ModelContextSpec, ModelSummarySettings};
use operit_runtime::data::model::ModelParameter::ModelParameter;
use operit_runtime::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;

macro_rules! println {
    () => {
        model_stdout_line("")
    };
    ($($arg:tt)*) => {
        model_stdout_line(format!($($arg)*))
    };
}

thread_local! {
    static MODEL_OUTPUT: Cell<*mut CoreCommandOutput> = Cell::new(std::ptr::null_mut());
}

fn set_model_output(output: &mut CoreCommandOutput) {
    MODEL_OUTPUT.with(|slot| slot.set(output as *mut CoreCommandOutput));
}

fn model_stdout_line(line: impl AsRef<str>) {
    MODEL_OUTPUT.with(|slot| {
        let output = slot.get();
        assert!(!output.is_null(), "model command output is not set");
        unsafe { (&mut *output).push_stdout_line(line.as_ref()) };
    });
}

struct ModelCommand;

impl ModelCommand {
    fn modelManager(&mut self) -> ModelConfigManager {
        ModelConfigManager::default()
    }

    fn functionalManager(&mut self) -> FunctionalConfigManager {
        FunctionalConfigManager::default()
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.modelManager()
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        self.functionalManager()
            .initializeIfNeeded()
            .map_err(|error| error.to_string())
    }
}

pub fn run_model_command(
    _context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    set_model_output(output);
    let core = &mut ModelCommand;
    if args.is_empty() {
        print_model_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "init" => {
            core.initialize()?;
            println!("initialized");
        }
        "provider-type-list" => {
            let mut providers = ModelCatalog::providers().map_err(|error| error.to_string())?;
            providers.sort_by(|left, right| left.providerTypeId.cmp(&right.providerTypeId));
            for provider in providers {
                println!(
                    "{}\t{}\t{}\t{}",
                    provider.providerTypeId,
                    provider.displayName,
                    provider.defaultEndpoint,
                    provider.models.len()
                );
            }
        }
        "provider-list" => {
            core.initialize()?;
            for provider in core
                .modelManager()
                .getProviderProfiles()
                .map_err(|error| error.to_string())?
            {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    provider.id,
                    provider.name,
                    provider.providerTypeId,
                    provider.endpoint,
                    provider.models.len()
                );
            }
        }
        "provider-show" => {
            core.initialize()?;
            let providerId =
                requiredArg(args, 1, "usage: operit2 model provider-show <provider-id>")?;
            let provider = core
                .modelManager()
                .getProviderProfile(providerId)
                .map_err(|error| error.to_string())?;
            println!("id={}", provider.id);
            println!("name={}", provider.name);
            println!("providerTypeId={}", provider.providerTypeId);
            println!("providerType={}", provider.providerType.name());
            println!("endpoint={}", provider.endpoint);
            println!("apiKeyLength={}", provider.apiKey.len());
            println!("useMultipleApiKeys={}", provider.useMultipleApiKeys);
            println!(
                "apiKeyPool={}",
                serde_json::to_string(&provider.apiKeyPool).map_err(|error| error.to_string())?
            );
            println!("currentKeyIndex={}", provider.currentKeyIndex);
            println!("keyRotationMode={}", provider.keyRotationMode);
            println!("customHeaders={}", provider.customHeaders);
            println!("requestLimitPerMinute={}", provider.requestLimitPerMinute);
            println!("maxConcurrentRequests={}", provider.maxConcurrentRequests);
            for model in provider.models {
                println!("model\t{}", model.id);
            }
        }
        "provider-create" => {
            core.initialize()?;
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
            let providerId = core
                .modelManager()
                .createProvider(name, providerTypeId, endpoint)
                .map_err(|error| error.to_string())?;
            println!("provider created: {providerId}");
        }
        "provider-set-key" => {
            core.initialize()?;
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
            let manager = core.modelManager();
            let mut provider = manager
                .getProviderProfile(providerId)
                .map_err(|error| error.to_string())?;
            provider.apiKey = apiKey;
            manager
                .updateProviderProfile(provider)
                .map_err(|error| error.to_string())?;
            println!("provider api key updated: {providerId}");
        }
        "provider-set-endpoint" => {
            core.initialize()?;
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
            let manager = core.modelManager();
            let mut provider = manager
                .getProviderProfile(providerId)
                .map_err(|error| error.to_string())?;
            provider.endpoint = endpoint;
            manager
                .updateProviderProfile(provider)
                .map_err(|error| error.to_string())?;
            println!("provider endpoint updated: {providerId}");
        }
        "provider-model-available-list" => {
            core.initialize()?;
            let providerId = requiredArg(
                args,
                1,
                "usage: operit2 model provider-model-available-list <provider-id>",
            )?;
            let mut models = core
                .modelManager()
                .getAvailableProviderModels(providerId)
                .map_err(|error| error.to_string())?;
            models.sort_by(|left, right| left.modelId.cmp(&right.modelId));
            for model in models {
                println!(
                    "{}\t{:?}\t{}\t{}\t{}\t{}",
                    model.modelId,
                    model.source,
                    model.pricing.is_some(),
                    model.context.is_some(),
                    model.capabilities.is_some(),
                    model.request.is_some()
                );
            }
        }
        "provider-model-add" => {
            core.initialize()?;
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
            let modelId = core
                .modelManager()
                .addProviderModelFromAvailable(providerId, modelId)
                .map_err(|error| error.to_string())?;
            println!("provider model added: {modelId}");
        }
        "provider-model-create" => {
            core.initialize()?;
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
            let modelId = core
                .modelManager()
                .createProviderModel(providerId, modelId)
                .map_err(|error| error.to_string())?;
            println!("provider model created: {modelId}");
        }
        "list" => {
            core.initialize()?;
            let mut summaries = core
                .modelManager()
                .getAllModelSummaries()
                .map_err(|error| error.to_string())?;
            summaries.sort_by(|left, right| {
                left.providerName
                    .cmp(&right.providerName)
                    .then(left.modelId.cmp(&right.modelId))
            });
            for summary in summaries {
                println!(
                    "{}\t{}\t{}\t{}",
                    summary.providerId,
                    summary.providerName,
                    summary.providerTypeId,
                    summary.modelId
                );
            }
        }
        "show" => {
            core.initialize()?;
            let providerId = args
                .get(1)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_PROVIDER_ID);
            let modelId = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_MODEL_ID);
            let config = core
                .modelManager()
                .getResolvedModelConfig(providerId, modelId)
                .map_err(|error| error.to_string())?;
            println!("providerId={}", config.providerId);
            println!("modelId={}", config.modelId);
            println!("providerName={}", config.providerName);
            println!("providerTypeId={}", config.apiProviderTypeId);
            println!("providerType={}", config.apiProviderType.name());
            println!("endpoint={}", config.apiEndpoint);
            println!("apiKeyLength={}", config.apiKey.len());
            println!("customHeaders={}", config.customHeaders);
            println!("requestLimitPerMinute={}", config.requestLimitPerMinute);
            println!("maxConcurrentRequests={}", config.maxConcurrentRequests);
            println!(
                "supportsStructuredTools={}",
                config.request.supportsStructuredTools
            );
            println!("maxContextLength={}", config.context.maxContextLength);
            println!(
                "enableMaxContextMode={}",
                config.context.enableMaxContextMode
            );
            println!("directImage={}", config.capabilities.directImage);
            println!("directAudio={}", config.capabilities.directAudio);
            println!("directVideo={}", config.capabilities.directVideo);
            println!("toolCall={}", config.capabilities.toolCall);
            println!(
                "builtinTools={}",
                serde_json::to_string(&config.builtinTools).map_err(|error| error.to_string())?
            );
            println!("enableSummary={}", config.summary.enableSummary);
            println!(
                "summaryTokenThreshold={}",
                config.summary.summaryTokenThreshold
            );
            println!(
                "enableSummaryByMessageCount={}",
                config.summary.enableSummaryByMessageCount
            );
            println!(
                "summaryMessageCountThreshold={}",
                config.summary.summaryMessageCountThreshold
            );
            println!(
                "parameters={}",
                serde_json::to_string(&config.parameters).map_err(|error| error.to_string())?
            );
            if let Some(pricing) = config.pricing {
                println!("billingMode={:?}", pricing.billingMode);
                println!("inputPricePerMillion={}", pricing.inputPricePerMillion);
                println!(
                    "cachedInputPricePerMillion={}",
                    pricing
                        .cachedInputPricePerMillion
                        .map(|value| value.to_string())
                        .unwrap_or_default()
                );
                println!("outputPricePerMillion={}", pricing.outputPricePerMillion);
                println!("pricePerRequest={}", pricing.pricePerRequest);
                println!("currency={}", pricing.currency.code());
            }
        }
        "use" => {
            core.initialize()?;
            let providerId =
                requiredArg(args, 1, "usage: operit2 model use <provider-id> <model-id>")?
                    .to_string();
            let modelId =
                requiredArg(args, 2, "usage: operit2 model use <provider-id> <model-id>")?
                    .to_string();
            core.functionalManager()
                .setModelForFunction(FunctionType::CHAT, providerId.clone(), modelId.clone())
                .map_err(|error| error.to_string())?;
            println!("chat model updated: {providerId}\t{modelId}");
        }
        "params" => {
            core.initialize()?;
            let providerId = args
                .get(1)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_PROVIDER_ID);
            let modelId = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_MODEL_ID);
            let params = core
                .modelManager()
                .getModelParametersForModel(providerId, modelId)
                .map_err(|error| error.to_string())?;
            for param in params {
                println!(
                    "{}\t{}\t{}\t{}",
                    param.id, param.apiName, param.isEnabled, param.currentValue
                );
            }
        }
        "parameters" => {
            core.initialize()?;
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
            core.modelManager()
                .updateParametersForModel(providerId, modelId, parameters)
                .map_err(|error| error.to_string())?;
            println!("parameters updated: {providerId}\t{modelId}");
        }
        "context-show" => {
            core.initialize()?;
            let providerId = args
                .get(1)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_PROVIDER_ID);
            let modelId = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_MODEL_ID);
            let config = core
                .modelManager()
                .getResolvedModelConfig(providerId, modelId)
                .map_err(|error| error.to_string())?;
            println!("providerId={}", config.providerId);
            println!("modelId={}", config.modelId);
            println!("maxContextLength={}", config.context.maxContextLength);
            println!(
                "enableMaxContextMode={}",
                config.context.enableMaxContextMode
            );
        }
        "context-set" => {
            core.initialize()?;
            let providerId = requiredArg(args, 1, "usage: operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>")?;
            let modelId = requiredArg(args, 2, "usage: operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>")?;
            let maxContextLength = parse_f32_arg(args.get(3), "usage: operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>")?;
            let enableMaxContextMode = parse_bool_arg(args.get(4), "usage: operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>")?;
            core.modelManager()
                .updateContextForModel(
                    providerId,
                    modelId,
                    ModelContextSpec {
                        maxContextLength,
                        enableMaxContextMode,
                    },
                )
                .map_err(|error| error.to_string())?;
            println!("context settings updated: {providerId}\t{modelId}");
        }
        "summary-show" => {
            core.initialize()?;
            let providerId = args
                .get(1)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_PROVIDER_ID);
            let modelId = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(ModelConfigManager::DEFAULT_MODEL_ID);
            let config = core
                .modelManager()
                .getResolvedModelConfig(providerId, modelId)
                .map_err(|error| error.to_string())?;
            println!("providerId={}", config.providerId);
            println!("modelId={}", config.modelId);
            println!("enableSummary={}", config.summary.enableSummary);
            println!(
                "summaryTokenThreshold={}",
                config.summary.summaryTokenThreshold
            );
            println!(
                "enableSummaryByMessageCount={}",
                config.summary.enableSummaryByMessageCount
            );
            println!(
                "summaryMessageCountThreshold={}",
                config.summary.summaryMessageCountThreshold
            );
        }
        "summary-set" => {
            core.initialize()?;
            let providerId = requiredArg(args, 1, "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let modelId = requiredArg(args, 2, "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let enableSummary = parse_bool_arg(args.get(3), "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let summaryTokenThreshold = parse_f32_arg(args.get(4), "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let enableSummaryByMessageCount = parse_bool_arg(args.get(5), "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            let summaryMessageCountThreshold = parse_i32_arg(args.get(6), "usage: operit2 model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>")?;
            core.modelManager()
                .updateSummaryForModel(
                    providerId,
                    modelId,
                    ModelSummarySettings {
                        enableSummary,
                        summaryTokenThreshold,
                        enableSummaryByMessageCount,
                        summaryMessageCountThreshold,
                    },
                )
                .map_err(|error| error.to_string())?;
            println!("summary settings updated: {providerId}\t{modelId}");
        }
        "function-list" => {
            core.initialize()?;
            let mut rows = functionTypes()
                .into_iter()
                .map(|functionType| {
                    core.functionalManager()
                        .getModelBindingForFunction(functionType.clone())
                        .map(|binding| (functionType, binding))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            rows.sort_by(|left, right| functionTypeName(&left.0).cmp(functionTypeName(&right.0)));
            for (functionType, binding) in rows {
                println!(
                    "{}\t{}\t{}",
                    functionTypeName(&functionType),
                    binding.providerId,
                    binding.modelId
                );
            }
        }
        "function-show" => {
            core.initialize()?;
            let functionType = parseFunctionType(requiredArg(
                args,
                1,
                "usage: operit2 model function-show <function-type>",
            )?)?;
            let binding = core
                .functionalManager()
                .getModelBindingForFunction(functionType.clone())
                .map_err(|error| error.to_string())?;
            println!("functionType={}", functionTypeName(&functionType));
            println!("providerId={}", binding.providerId);
            println!("modelId={}", binding.modelId);
        }
        "function-set" => {
            core.initialize()?;
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
            core.functionalManager()
                .setModelForFunction(functionType.clone(), providerId.clone(), modelId.clone())
                .map_err(|error| error.to_string())?;
            println!(
                "function mapping updated: {}\t{}\t{}",
                functionTypeName(&functionType),
                providerId,
                modelId
            );
        }
        "function-reset" => {
            core.initialize()?;
            if let Some(functionTypeValue) = args.get(1) {
                let functionType = parseFunctionType(functionTypeValue)?;
                core.functionalManager()
                    .resetFunctionConfig(functionType.clone())
                    .map_err(|error| error.to_string())?;
                println!(
                    "function mapping reset: {}",
                    functionTypeName(&functionType)
                );
            } else {
                core.functionalManager()
                    .resetAllFunctionConfigs()
                    .map_err(|error| error.to_string())?;
                println!("all function mappings reset");
            }
        }
        _ => print_model_usage(),
    }

    Ok(())
}

fn requiredArg<'a>(args: &'a [String], index: usize, usage: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| usage.to_string())
}

fn parseFunctionType(value: &str) -> Result<FunctionType, String> {
    match value {
        "CHAT" => Ok(FunctionType::CHAT),
        "SUMMARY" => Ok(FunctionType::SUMMARY),
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

fn functionTypes() -> Vec<FunctionType> {
    vec![
        FunctionType::CHAT,
        FunctionType::SUMMARY,
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

fn functionTypeName(functionType: &FunctionType) -> &'static str {
    match functionType {
        FunctionType::CHAT => "CHAT",
        FunctionType::SUMMARY => "SUMMARY",
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

fn print_model_usage() {
    println!("operit2 model init");
    println!("operit2 model provider-type-list");
    println!("operit2 model provider-list");
    println!("operit2 model provider-show <provider-id>");
    println!("operit2 model provider-create <name> <provider-type-id> <endpoint>");
    println!("operit2 model provider-set-key <provider-id> <api-key>");
    println!("operit2 model provider-set-endpoint <provider-id> <endpoint>");
    println!("operit2 model provider-model-available-list <provider-id>");
    println!("operit2 model provider-model-add <provider-id> <provider-model-id>");
    println!("operit2 model provider-model-create <provider-id> <provider-model-id>");
    println!("operit2 model list");
    println!("operit2 model show [model-id]");
    println!("operit2 model use <model-id>");
    println!("operit2 model params [model-id]");
    println!("operit2 model parameters <model-id> <parameters-json>");
    println!("operit2 model context-show [model-id]");
    println!("operit2 model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>");
    println!("operit2 model summary-show [model-id]");
    println!("operit2 model summary-set <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>");
    println!("operit2 model function-list");
    println!("operit2 model function-show <function-type>");
    println!("operit2 model function-set <function-type> <model-id>");
    println!("operit2 model function-reset [function-type]");
}
