use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use operit_runtime::data::model::ActivePrompt::ActivePrompt;
use operit_runtime::data::model::AttachmentInfo::AttachmentInfo;
use operit_runtime::data::model::CharacterCard::{
    CharacterCard, CharacterCardChatModelBindingMode, CharacterCardMemoryProfileBindingMode,
    CharacterCardToolAccessConfig,
};
use operit_runtime::data::model::CharacterGroupCard::{CharacterGroupCard, GroupMemberConfig};
use operit_runtime::data::model::ApiKeyInfo::ApiKeyInfo;
use operit_runtime::data::model::FunctionType::FunctionType;
use operit_runtime::data::model::ModelConfigData::ApiProviderType;
use operit_runtime::data::model::ModelParameter::ModelParameter;
use operit_runtime::data::model::PromptFunctionType::PromptFunctionType;
use operit_runtime::data::model::PromptTag::TagType;
use operit_runtime::data::preferences::ActivePromptManager::ActivePromptManager;
use operit_runtime::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_runtime::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;
use operit_runtime::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::data::preferences::PromptTagManager::PromptTagManager;
use operit_runtime::data::repository::ChatHistoryManager::ChatHistoryManager;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::model::ChatTurnOptions::ChatTurnOptions;
use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;
use operit_runtime::services::core::MessageCoordinationDelegate::MessageCoordinationDelegate;
use operit_runtime::util::stream::Stream::Stream;

mod bootstrap;
mod tui;

use bootstrap::create_cli_application;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return tui::run_tui_command(&[]).await;
    }

    match args[0].as_str() {
        "help" | "-h" | "--help" => {
            print_root_usage();
            Ok(())
        }
        "cli" => run_cli_root(&args[1..]).await,
        "tui" => tui::run_tui_command(&args[1..]).await,
        value if value.starts_with('-') => tui::run_tui_command(&args).await,
        _ => {
            print_root_usage();
            Ok(())
        }
    }
}

async fn run_cli_root(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_cli_usage();
        return Ok(());
    }

    let mut application = create_cli_application();
    application.onCreate()?;

    let result = match args[0].as_str() {
        "model" => run_model_command(&args[1..]),
        "chat" => run_chat_command(&args[1..]).await,
        "shell" => run_shell_command(&args[1..]).await,
        "tag" => run_tag_command(&args[1..]),
        "character" => run_character_command(&args[1..]),
        "group" => run_group_command(&args[1..]),
        "active-prompt" => run_active_prompt_command(&args[1..]),
        _ => {
            print_cli_usage();
            Ok(())
        }
    };
    result.map_err(rewrite_cli_usage_message)
}

fn rewrite_cli_usage_message(message: String) -> String {
    message.replace("usage: operit2 ", "usage: operit2 cli ")
}

fn run_model_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_model_usage();
        return Ok(());
    }

    let modelConfigManager = ModelConfigManager::default();
    let functionalConfigManager = FunctionalConfigManager::default();

    match args[0].as_str() {
        "init" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            functionalConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            println!("initialized");
        }
        "list" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            for summary in modelConfigManager
                .getAllConfigSummaries()
                .map_err(|error| error.to_string())?
            {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    summary.id,
                    summary.name,
                    summary.apiProviderType.name(),
                    summary.apiEndpoint,
                    summary.modelName
                );
            }
        }
        "show" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let config = modelConfigManager
                .getModelConfig(configId)
                .map_err(|error| error.to_string())?;
            println!("id={}", config.id);
            println!("name={}", config.name);
            println!("provider={}", config.apiProviderType.name());
            println!("providerTypeId={}", config.apiProviderTypeId);
            println!("endpoint={}", config.apiEndpoint);
            println!("modelName={}", config.modelName);
            println!("apiKeyLength={}", config.apiKey.len());
            println!("useMultipleApiKeys={}", config.useMultipleApiKeys);
            println!("apiKeyPool={}", serde_json::to_string(&config.apiKeyPool).map_err(|error| error.to_string())?);
            println!("currentKeyIndex={}", config.currentKeyIndex);
            println!("keyRotationMode={}", config.keyRotationMode);
            println!("hasCustomParameters={}", config.hasCustomParameters);
            println!("maxTokensEnabled={}", config.maxTokensEnabled);
            println!("temperatureEnabled={}", config.temperatureEnabled);
            println!("topPEnabled={}", config.topPEnabled);
            println!("topKEnabled={}", config.topKEnabled);
            println!("presencePenaltyEnabled={}", config.presencePenaltyEnabled);
            println!("frequencyPenaltyEnabled={}", config.frequencyPenaltyEnabled);
            println!("repetitionPenaltyEnabled={}", config.repetitionPenaltyEnabled);
            println!("maxTokens={}", config.maxTokens);
            println!("temperature={}", config.temperature);
            println!("topP={}", config.topP);
            println!("topK={}", config.topK);
            println!("presencePenalty={}", config.presencePenalty);
            println!("frequencyPenalty={}", config.frequencyPenalty);
            println!("repetitionPenalty={}", config.repetitionPenalty);
            println!("customParameters={}", config.customParameters);
            println!("customHeaders={}", config.customHeaders);
            println!("contextLength={}", config.contextLength);
            println!("maxContextLength={}", config.maxContextLength);
            println!("enableMaxContextMode={}", config.enableMaxContextMode);
            println!("summaryTokenThreshold={}", config.summaryTokenThreshold);
            println!("enableSummary={}", config.enableSummary);
            println!("enableSummaryByMessageCount={}", config.enableSummaryByMessageCount);
            println!("summaryMessageCountThreshold={}", config.summaryMessageCountThreshold);
            println!("mnnForwardType={}", config.mnnForwardType);
            println!("mnnThreadCount={}", config.mnnThreadCount);
            println!("llamaThreadCount={}", config.llamaThreadCount);
            println!("llamaContextSize={}", config.llamaContextSize);
            println!("llamaBatchSize={}", config.llamaBatchSize);
            println!("llamaUBatchSize={}", config.llamaUBatchSize);
            println!("llamaGpuLayers={}", config.llamaGpuLayers);
            println!("llamaUseMmap={}", config.llamaUseMmap);
            println!("llamaFlashAttention={}", config.llamaFlashAttention);
            println!("llamaKvUnified={}", config.llamaKvUnified);
            println!("llamaOffloadKqv={}", config.llamaOffloadKqv);
            println!("enableDirectImageProcessing={}", config.enableDirectImageProcessing);
            println!("enableDirectAudioProcessing={}", config.enableDirectAudioProcessing);
            println!("enableDirectVideoProcessing={}", config.enableDirectVideoProcessing);
            println!("enableGoogleSearch={}", config.enableGoogleSearch);
            println!("enableToolCall={}", config.enableToolCall);
            println!("requestLimitPerMinute={}", config.requestLimitPerMinute);
            println!("maxConcurrentRequests={}", config.maxConcurrentRequests);
        }
        "set-key" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let apiKey = args
                .get(1)
                .ok_or_else(|| "usage: operit2 model set-key <api-key> [config-id]".to_string())?
                .clone();
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateApiKey(configId, apiKey)
                .map_err(|error| error.to_string())?;
            println!("api key updated: {configId}");
        }
        "set" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let endpoint = args
                .get(1)
                .ok_or_else(|| "usage: operit2 model set <endpoint> <model-name> [config-id]".to_string())?
                .clone();
            let modelName = args
                .get(2)
                .ok_or_else(|| "usage: operit2 model set <endpoint> <model-name> [config-id]".to_string())?
                .clone();
            let configId = match args.get(3).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let current = modelConfigManager
                .getModelConfig(configId)
                .map_err(|error| error.to_string())?;
            modelConfigManager
                .updateModelConfig(configId, current.apiKey, endpoint, modelName)
                .map_err(|error| error.to_string())?;
            println!("model updated: {configId}");
        }
        "tool-call" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let enableToolCall = parse_bool_arg(
                args.get(1),
                "usage: operit2 model tool-call <enable-tool-call> [config-id]",
            )?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateToolCall(configId, enableToolCall)
                .map_err(|error| error.to_string())?;
            println!("tool call updated: {configId}\t{enableToolCall}");
        }
        "api-settings-full" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let apiKey = args
                .get(1)
                .ok_or_else(|| "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]".to_string())?
                .clone();
            let apiEndpoint = args
                .get(2)
                .ok_or_else(|| "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]".to_string())?
                .clone();
            let modelName = args
                .get(3)
                .ok_or_else(|| "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]".to_string())?
                .clone();
            let apiProviderType = parseApiProviderType(
                args.get(4)
                    .ok_or_else(|| "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]".to_string())?
                    .as_str(),
            )?;
            let apiProviderTypeId = args
                .get(5)
                .ok_or_else(|| "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]".to_string())?
                .clone();
            let mnnForwardType = parse_i32_arg(args.get(6), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let mnnThreadCount = parse_i32_arg(args.get(7), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let llamaThreadCount = parse_i32_arg(args.get(8), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let llamaContextSize = parse_i32_arg(args.get(9), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let llamaGpuLayers = parse_i32_arg(args.get(10), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let enableDirectImageProcessing = parse_bool_arg(args.get(11), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let enableDirectAudioProcessing = parse_bool_arg(args.get(12), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let enableDirectVideoProcessing = parse_bool_arg(args.get(13), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let enableGoogleSearch = parse_bool_arg(args.get(14), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let enableToolCall = parse_bool_arg(args.get(15), "usage: operit2 model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]")?;
            let configId = match args.get(16).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateApiSettingsFull(
                    configId,
                    apiKey,
                    apiEndpoint,
                    modelName,
                    apiProviderType,
                    apiProviderTypeId,
                    mnnForwardType,
                    mnnThreadCount,
                    llamaThreadCount,
                    llamaContextSize,
                    llamaGpuLayers,
                    enableDirectImageProcessing,
                    enableDirectAudioProcessing,
                    enableDirectVideoProcessing,
                    enableGoogleSearch,
                    enableToolCall,
                )
                .map_err(|error| error.to_string())?;
            println!("api settings full updated: {configId}");
        }
        "custom-headers" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let customHeaders = args
                .get(1)
                .ok_or_else(|| "usage: operit2 model custom-headers <custom-headers-json> [config-id]".to_string())?
                .clone();
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateCustomHeaders(configId, customHeaders)
                .map_err(|error| error.to_string())?;
            println!("custom headers updated: {configId}");
        }
        "request-queue" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let requestLimitPerMinute = parse_i32_arg(args.get(1), "usage: operit2 model request-queue <request-limit-per-minute> <max-concurrent-requests> [config-id]")?;
            let maxConcurrentRequests = parse_i32_arg(args.get(2), "usage: operit2 model request-queue <request-limit-per-minute> <max-concurrent-requests> [config-id]")?;
            let configId = match args.get(3).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateRequestQueueSettings(configId, requestLimitPerMinute, maxConcurrentRequests)
                .map_err(|error| error.to_string())?;
            println!("request queue updated: {configId}");
        }
        "api-key-pool" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let useMultipleApiKeys = parse_bool_arg(args.get(1), "usage: operit2 model api-key-pool <use-multiple-api-keys> <api-key-pool-json> [config-id]")?;
            let apiKeyPoolJson = args
                .get(2)
                .ok_or_else(|| "usage: operit2 model api-key-pool <use-multiple-api-keys> <api-key-pool-json> [config-id]".to_string())?;
            let apiKeyPool = serde_json::from_str::<Vec<ApiKeyInfo>>(apiKeyPoolJson)
                .map_err(|error| error.to_string())?;
            let configId = match args.get(3).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateApiKeyPoolSettings(configId, useMultipleApiKeys, apiKeyPool)
                .map_err(|error| error.to_string())?;
            println!("api key pool updated: {configId}");
        }
        "custom-parameters" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let parametersJson = args
                .get(1)
                .ok_or_else(|| "usage: operit2 model custom-parameters <parameters-json> [config-id]".to_string())?
                .clone();
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateCustomParameters(configId, parametersJson)
                .map_err(|error| error.to_string())?;
            println!("custom parameters updated: {configId}");
        }
        "parameters" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let parametersJson = args
                .get(1)
                .ok_or_else(|| "usage: operit2 model parameters <parameters-json> [config-id]".to_string())?;
            let parameters = serde_json::from_str::<Vec<ModelParameter<serde_json::Value>>>(parametersJson)
                .map_err(|error| error.to_string())?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateParameters(configId, parameters)
                .map_err(|error| error.to_string())?;
            println!("parameters updated: {configId}");
        }
        "direct-image" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let enableDirectImageProcessing = parse_bool_arg(args.get(1), "usage: operit2 model direct-image <enable-direct-image-processing> [config-id]")?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateDirectImageProcessing(configId, enableDirectImageProcessing)
                .map_err(|error| error.to_string())?;
            println!("direct image processing updated: {configId}\t{enableDirectImageProcessing}");
        }
        "direct-audio" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let enableDirectAudioProcessing = parse_bool_arg(args.get(1), "usage: operit2 model direct-audio <enable-direct-audio-processing> [config-id]")?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateDirectAudioProcessing(configId, enableDirectAudioProcessing)
                .map_err(|error| error.to_string())?;
            println!("direct audio processing updated: {configId}\t{enableDirectAudioProcessing}");
        }
        "direct-video" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let enableDirectVideoProcessing = parse_bool_arg(args.get(1), "usage: operit2 model direct-video <enable-direct-video-processing> [config-id]")?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateDirectVideoProcessing(configId, enableDirectVideoProcessing)
                .map_err(|error| error.to_string())?;
            println!("direct video processing updated: {configId}\t{enableDirectVideoProcessing}");
        }
        "google-search" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let enableGoogleSearch = parse_bool_arg(args.get(1), "usage: operit2 model google-search <enable-google-search> [config-id]")?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            modelConfigManager
                .updateGoogleSearch(configId, enableGoogleSearch)
                .map_err(|error| error.to_string())?;
            println!("google search updated: {configId}\t{enableGoogleSearch}");
        }
        "params" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let params = modelConfigManager
                .getModelParametersForConfig(configId)
                .map_err(|error| error.to_string())?;
            for param in params {
                println!(
                    "{}\t{}\t{}\t{}",
                    param.id,
                    param.apiName,
                    param.isEnabled,
                    param.currentValue
                );
            }
        }
        "context-show" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let config = modelConfigManager
                .getModelConfig(configId)
                .map_err(|error| error.to_string())?;
            println!("id={}", config.id);
            println!("contextLength={}", config.contextLength);
            println!("maxContextLength={}", config.maxContextLength);
            println!("enableMaxContextMode={}", config.enableMaxContextMode);
            println!(
                "effectiveContextLength={}",
                if config.enableMaxContextMode {
                    config.maxContextLength
                } else {
                    config.contextLength
                }
            );
        }
        "context-set" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let configId = match args.get(4).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let contextLength = parse_f32_arg(args.get(1), "usage: operit2 model context-set <context-length> <max-context-length> <enable-max-context-mode> [config-id]")?;
            let maxContextLength = parse_f32_arg(args.get(2), "usage: operit2 model context-set <context-length> <max-context-length> <enable-max-context-mode> [config-id]")?;
            let enableMaxContextMode = parse_bool_arg(args.get(3), "usage: operit2 model context-set <context-length> <max-context-length> <enable-max-context-mode> [config-id]")?;
            modelConfigManager
                .updateContextSettings(
                    configId,
                    contextLength,
                    maxContextLength,
                    enableMaxContextMode,
                )
                .map_err(|error| error.to_string())?;
            println!("context settings updated: {configId}");
        }
        "summary-show" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let config = modelConfigManager
                .getModelConfig(configId)
                .map_err(|error| error.to_string())?;
            println!("id={}", config.id);
            println!("enableSummary={}", config.enableSummary);
            println!("summaryTokenThreshold={}", config.summaryTokenThreshold);
            println!("enableSummaryByMessageCount={}", config.enableSummaryByMessageCount);
            println!("summaryMessageCountThreshold={}", config.summaryMessageCountThreshold);
        }
        "summary-set" => {
            modelConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let configId = match args.get(5).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let enableSummary = parse_bool_arg(args.get(1), "usage: operit2 model summary-set <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold> [config-id]")?;
            let summaryTokenThreshold = parse_f32_arg(args.get(2), "usage: operit2 model summary-set <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold> [config-id]")?;
            let enableSummaryByMessageCount = parse_bool_arg(args.get(3), "usage: operit2 model summary-set <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold> [config-id]")?;
            let summaryMessageCountThreshold = parse_i32_arg(args.get(4), "usage: operit2 model summary-set <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold> [config-id]")?;
            modelConfigManager
                .updateSummarySettings(
                    configId,
                    enableSummary,
                    summaryTokenThreshold,
                    enableSummaryByMessageCount,
                    summaryMessageCountThreshold,
                )
                .map_err(|error| error.to_string())?;
            println!("summary settings updated: {configId}");
        }
        "function-list" => {
            functionalConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let mut mappings = functionalConfigManager
                .functionConfigMappingWithIndexFlow()
                .map_err(|error| error.to_string())?
                .into_iter()
                .collect::<Vec<_>>();
            mappings.sort_by(|left, right| functionTypeName(&left.0).cmp(functionTypeName(&right.0)));
            for (functionType, mapping) in mappings {
                println!(
                    "{}\t{}\t{}",
                    functionTypeName(&functionType),
                    mapping.configId,
                    mapping.modelIndex
                );
            }
        }
        "function-show" => {
            functionalConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let functionType = parseFunctionType(
                args.get(1)
                    .ok_or_else(|| "usage: operit2 model function-show <function-type>".to_string())?
                    .as_str(),
            )?;
            let mapping = functionalConfigManager
                .getConfigMappingForFunction(functionType.clone())
                .map_err(|error| error.to_string())?;
            println!("functionType={}", functionTypeName(&functionType));
            println!("configId={}", mapping.configId);
            println!("modelIndex={}", mapping.modelIndex);
        }
        "function-set" => {
            functionalConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            let functionType = parseFunctionType(
                args.get(1)
                    .ok_or_else(|| "usage: operit2 model function-set <function-type> <config-id> [model-index]".to_string())?
                    .as_str(),
            )?;
            let configId = args
                .get(2)
                .ok_or_else(|| "usage: operit2 model function-set <function-type> <config-id> [model-index]".to_string())?
                .clone();
            let modelIndex = args
                .get(3)
                .map(|value| value.parse::<i32>())
                .transpose()
                .map_err(|error| error.to_string())?
                .unwrap_or(0);
            functionalConfigManager
                .setConfigForFunctionWithIndex(functionType.clone(), configId.clone(), modelIndex)
                .map_err(|error| error.to_string())?;
            println!("function mapping updated: {}\t{}\t{}", functionTypeName(&functionType), configId, modelIndex);
        }
        "function-reset" => {
            functionalConfigManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            if let Some(functionTypeValue) = args.get(1) {
                let functionType = parseFunctionType(functionTypeValue)?;
                functionalConfigManager
                    .resetFunctionConfig(functionType.clone())
                    .map_err(|error| error.to_string())?;
                println!("function mapping reset: {}", functionTypeName(&functionType));
            } else {
                functionalConfigManager
                    .resetAllFunctionConfigs()
                    .map_err(|error| error.to_string())?;
                println!("all function mappings reset");
            }
        }
        _ => print_model_usage(),
    }

    Ok(())
}

fn run_tag_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_tag_usage();
        return Ok(());
    }

    let promptTagManager = PromptTagManager::getInstance();
    match args[0].as_str() {
        "list" => {
            for tag in promptTagManager.getAllTags().map_err(|error| error.to_string())? {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    tag.id,
                    tag.name,
                    tagTypeName(&tag.tagType),
                    tag.description,
                    tag.promptContent.replace('\n', "\\n")
                );
            }
        }
        "show" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag show <id>".to_string())?;
            let tag = promptTagManager
                .getPromptTagFlow(id)
                .first()
                .map_err(|error| error.to_string())?;
            print_tag(&tag);
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag create <name> [prompt-content] [description] [tag-type]".to_string())?
                .clone();
            let promptContent = args.get(2).cloned().unwrap_or_default();
            let description = args.get(3).cloned().unwrap_or_default();
            let tagType = parseTagType(args.get(4).map(String::as_str))?;
            let id = promptTagManager
                .createPromptTag(name, description, promptContent, tagType)
                .map_err(|error| error.to_string())?;
            println!("{id}");
        }
        "update" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag update <id> <field> <value>".to_string())?;
            let field = args
                .get(2)
                .ok_or_else(|| "usage: operit2 tag update <id> <field> <value>".to_string())?;
            let value = args
                .get(3)
                .ok_or_else(|| "usage: operit2 tag update <id> <field> <value>".to_string())?
                .clone();
            let (name, description, promptContent, tagType) = match field.as_str() {
                "name" => (Some(value), None, None, None),
                "description" => (None, Some(value), None, None),
                "promptContent" => (None, None, Some(value), None),
                "tagType" => (None, None, None, Some(parseTagType(Some(&value))?)),
                _ => return Err("tag fields: name | description | promptContent | tagType".to_string()),
            };
            promptTagManager
                .updatePromptTag(id, name, description, promptContent, tagType)
                .map_err(|error| error.to_string())?;
            println!("updated: {id}");
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag delete <id>".to_string())?;
            promptTagManager
                .deletePromptTag(id)
                .map_err(|error| error.to_string())?;
            println!("deleted: {id}");
        }
        _ => print_tag_usage(),
    }
    Ok(())
}

fn run_character_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_character_usage();
        return Ok(());
    }

    let characterCardManager = CharacterCardManager::getInstance();
    match args[0].as_str() {
        "init" => {
            characterCardManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            println!("initialized");
        }
        "list" => {
            characterCardManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            for card in characterCardManager
                .getAllCharacterCards()
                .map_err(|error| error.to_string())?
            {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    card.id,
                    card.name,
                    card.isDefault,
                    card.attachedTagIds.join(","),
                    card.description
                );
            }
        }
        "show" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character show <id>".to_string())?;
            let card = characterCardManager
                .getCharacterCard(id)
                .map_err(|error| error.to_string())?;
            print_character_card(&card);
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character create <name> [character-setting]".to_string())?
                .clone();
            let characterSetting = args.get(2).cloned().unwrap_or_default();
            let now = currentTimeMillis();
            let id = characterCardManager
                .createCharacterCard(CharacterCard {
                    id: String::new(),
                    name,
                    description: String::new(),
                    characterSetting,
                    openingStatement: String::new(),
                    otherContentChat: String::new(),
                    otherContentVoice: String::new(),
                    attachedTagIds: Vec::new(),
                    advancedCustomPrompt: String::new(),
                    marks: String::new(),
                    chatModelBindingMode: CharacterCardChatModelBindingMode::FOLLOW_GLOBAL.to_string(),
                    chatModelConfigId: None,
                    chatModelIndex: 0,
                    memoryProfileBindingMode: CharacterCardMemoryProfileBindingMode::FOLLOW_GLOBAL.to_string(),
                    memoryProfileId: None,
                    toolAccessConfig: CharacterCardToolAccessConfig::default(),
                    isDefault: false,
                    createdAt: now,
                    updatedAt: now,
                })
                .map_err(|error| error.to_string())?;
            println!("{id}");
        }
        "update" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character update <id> <field> <value>".to_string())?;
            let field = args
                .get(2)
                .ok_or_else(|| "usage: operit2 character update <id> <field> <value>".to_string())?;
            let value = args
                .get(3)
                .ok_or_else(|| "usage: operit2 character update <id> <field> <value>".to_string())?
                .clone();
            let mut card = characterCardManager
                .getCharacterCard(id)
                .map_err(|error| error.to_string())?;
            match field.as_str() {
                "name" => card.name = value,
                "description" => card.description = value,
                "characterSetting" => card.characterSetting = value,
                "openingStatement" => card.openingStatement = value,
                "otherContentChat" => card.otherContentChat = value,
                "otherContentVoice" => card.otherContentVoice = value,
                "advancedCustomPrompt" => card.advancedCustomPrompt = value,
                "marks" => card.marks = value,
                "attachedTagIds" => card.attachedTagIds = parseCsvList(&value),
                "chatModelBindingMode" => card.chatModelBindingMode = CharacterCardChatModelBindingMode::normalize(Some(&value)),
                "chatModelConfigId" => card.chatModelConfigId = nonBlankString(value),
                "chatModelIndex" => {
                    card.chatModelIndex = value
                        .parse::<i32>()
                        .map_err(|error| format!("invalid chatModelIndex: {error}"))?
                        .max(0)
                }
                "memoryProfileBindingMode" => {
                    card.memoryProfileBindingMode = CharacterCardMemoryProfileBindingMode::normalize(Some(&value))
                }
                "memoryProfileId" => card.memoryProfileId = nonBlankString(value),
                _ => {
                    return Err("character fields: name | description | characterSetting | openingStatement | otherContentChat | otherContentVoice | attachedTagIds | advancedCustomPrompt | marks | chatModelBindingMode | chatModelConfigId | chatModelIndex | memoryProfileBindingMode | memoryProfileId".to_string())
                }
            }
            characterCardManager
                .updateCharacterCard(card)
                .map_err(|error| error.to_string())?;
            println!("updated: {id}");
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character delete <id>".to_string())?;
            characterCardManager
                .deleteCharacterCard(id)
                .map_err(|error| error.to_string())?;
            println!("deleted: {id}");
        }
        "set-active" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character set-active <id>".to_string())?;
            ActivePromptManager::getInstance()
                .setActivePrompt(ActivePrompt::CharacterCard { id: id.clone() })
                .map_err(|error| error.to_string())?;
            println!("active character: {id}");
        }
        "combine" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character combine <id> [CHAT|VOICE] [tag-id-csv]".to_string())?;
            let promptFunctionType = parsePromptFunctionType(args.get(2).map(String::as_str))?;
            let additionalTagIds = args.get(3).map(|value| parseCsvList(value)).unwrap_or_default();
            let prompt = characterCardManager
                .combinePrompts(id, additionalTagIds, promptFunctionType)
                .map_err(|error| error.to_string())?;
            println!("{prompt}");
        }
        "reset-default" => {
            characterCardManager
                .resetDefaultCharacterCard()
                .map_err(|error| error.to_string())?;
            println!("default character reset");
        }
        _ => print_character_usage(),
    }
    Ok(())
}

fn run_group_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_group_usage();
        return Ok(());
    }

    let groupManager = CharacterGroupCardManager::getInstance();
    match args[0].as_str() {
        "init" => {
            groupManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            println!("initialized");
        }
        "list" => {
            groupManager
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            for group in groupManager
                .getAllCharacterGroupCards()
                .map_err(|error| error.to_string())?
            {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    group.id,
                    group.name,
                    group.description,
                    group.members
                        .iter()
                        .map(|member| format!("{}:{}", member.characterCardId, member.orderIndex))
                        .collect::<Vec<_>>()
                        .join(","),
                    group.createdAt
                );
            }
        }
        "show" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group show <id>".to_string())?;
            let group = groupManager
                .getCharacterGroupCard(id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            print_character_group_card(&group);
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group create <name> [description]".to_string())?
                .clone();
            let description = args.get(2).cloned().unwrap_or_default();
            let id = groupManager
                .createCharacterGroupCard(CharacterGroupCard {
                    id: String::new(),
                    name,
                    description,
                    members: Vec::new(),
                    createdAt: currentTimeMillis(),
                    updatedAt: currentTimeMillis(),
                })
                .map_err(|error| error.to_string())?;
            println!("{id}");
        }
        "update" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group update <id> <field> <value>".to_string())?;
            let field = args
                .get(2)
                .ok_or_else(|| "usage: operit2 group update <id> <field> <value>".to_string())?;
            let value = args
                .get(3)
                .ok_or_else(|| "usage: operit2 group update <id> <field> <value>".to_string())?
                .clone();
            let mut group = groupManager
                .getCharacterGroupCard(id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            match field.as_str() {
                "name" => group.name = value,
                "description" => group.description = value,
                "members" => group.members = parse_group_members(&value),
                _ => return Err("group fields: name | description | members".to_string()),
            }
            group.updatedAt = currentTimeMillis();
            groupManager
                .updateCharacterGroupCard(group)
                .map_err(|error| error.to_string())?;
            println!("updated: {id}");
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group delete <id>".to_string())?;
            groupManager
                .deleteCharacterGroupCard(id)
                .map_err(|error| error.to_string())?;
            println!("deleted: {id}");
        }
        "set-active" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group set-active <id>".to_string())?;
            ActivePromptManager::getInstance()
                .setActivePrompt(ActivePrompt::CharacterGroup { id: id.clone() })
                .map_err(|error| error.to_string())?;
            println!("active group: {id}");
        }
        "duplicate" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group duplicate <source-id> [new-name]".to_string())?;
            let newName = args.get(2).cloned();
            let newId = groupManager
                .duplicateCharacterGroupCard(id, newName)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            println!("{newId}");
        }
        _ => print_group_usage(),
    }
    Ok(())
}

fn run_active_prompt_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_active_prompt_usage();
        return Ok(());
    }

    let activePromptManager = ActivePromptManager::getInstance();
    match args[0].as_str() {
        "show" => {
            match activePromptManager
                .getActivePrompt()
                .map_err(|error| error.to_string())?
            {
                ActivePrompt::CharacterCard { id } => println!("character_card\t{id}"),
                ActivePrompt::CharacterGroup { id } => println!("character_group\t{id}"),
            }
        }
        "set-card" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 active-prompt set-card <id>".to_string())?;
            activePromptManager
                .setActivePrompt(ActivePrompt::CharacterCard { id: id.clone() })
                .map_err(|error| error.to_string())?;
            println!("active character card: {id}");
        }
        "set-group" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 active-prompt set-group <id>".to_string())?;
            activePromptManager
                .setActivePrompt(ActivePrompt::CharacterGroup { id: id.clone() })
                .map_err(|error| error.to_string())?;
            println!("active character group: {id}");
        }
        "activate-for-chat" => {
            let characterCardName = args.get(1).cloned().and_then(nonBlankString);
            let characterGroupId = args.get(2).cloned().and_then(nonBlankString);
            activePromptManager
                .activateForChatBinding(characterCardName, characterGroupId)
                .map_err(|error| error.to_string())?;
            println!("active prompt updated");
        }
        "resolved-card" => {
            let id = activePromptManager
                .resolveActiveCardIdForSend()
                .map_err(|error| error.to_string())?;
            println!("{id}");
        }
        _ => print_active_prompt_usage(),
    }
    Ok(())
}

async fn run_chat_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_chat_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "new" => create_chat(&args[1..]).await,
        "list" => list_chats(),
        "show" => show_chat(&args[1..]),
        "current" => show_current_chat(),
        "switch" => switch_chat_command(&args[1..]),
        "stats" => show_chat_stats(),
        "bind-character" => bind_chat_character(&args[1..]),
        "bind-group" => bind_chat_group_card(&args[1..]),
        "set-group" => set_chat_group(&args[1..]),
        "shell" => run_shell_command(&args[1..]).await,
        "send" => {
            let sendArgs = parse_chat_send_args(&args[1..])?;
            send_chat_message(sendArgs).await
        }
        _ => {
            print_chat_usage();
            Ok(())
        }
    }
}

fn list_chats() -> Result<(), String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    let messageCounts = manager
        .getMessageCountsByChatId()
        .map_err(|error| error.to_string())?;
    for chat in manager.chatHistoriesFlow().map_err(|error| error.to_string())? {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            chat.id,
            chat.title,
            chat.createdAt,
            chat.updatedAt,
            messageCounts.get(&chat.id).copied().unwrap_or(0),
            chat.inputTokens,
            chat.outputTokens,
            chat.characterCardName.clone().unwrap_or_default(),
            chat.characterGroupId.clone().unwrap_or_default()
        );
    }
    Ok(())
}

fn show_chat(args: &[String]) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat show <chat-id> [--runtime]".to_string())?;
    let runtimeOnly = args.get(1).map(String::as_str) == Some("--runtime");
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    let chat = manager
        .chatHistoriesFlow()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|chat| chat.id == *chatId)
        .ok_or_else(|| format!("chat not found: {chatId}"))?;
    print_chat_history_header(&chat);
    let messages = if runtimeOnly {
        manager
            .loadRuntimeChatMessages(chatId.clone())
            .map_err(|error| error.to_string())?
    } else {
        manager
            .loadChatMessages(chatId)
            .map_err(|error| error.to_string())?
    };
    for message in messages {
        print_chat_message(&message);
    }
    Ok(())
}

fn show_current_chat() -> Result<(), String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    match manager.currentChatIdFlow().map_err(|error| error.to_string())? {
        Some(chatId) => println!("{chatId}"),
        None => println!(),
    }
    Ok(())
}

fn switch_chat_command(args: &[String]) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat switch <chat-id>".to_string())?
        .clone();
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    manager
        .setCurrentChatId(chatId.clone())
        .map_err(|error| error.to_string())?;
    println!("current chat: {chatId}");
    Ok(())
}

fn show_chat_stats() -> Result<(), String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    println!("totalChats={}", manager.getTotalChatCount().map_err(|error| error.to_string())?);
    println!("totalMessages={}", manager.getTotalMessageCount().map_err(|error| error.to_string())?);
    for stats in manager
        .characterCardStatsFlow()
        .map_err(|error| error.to_string())?
    {
        println!(
            "characterCard\t{}\t{}\t{}",
            stats.characterCardName.clone().unwrap_or_default(),
            stats.chatCount,
            stats.messageCount
        );
    }
    for stats in manager
        .characterGroupStatsFlow()
        .map_err(|error| error.to_string())?
    {
        println!(
            "characterGroup\t{}\t{}\t{}",
            stats.characterGroupId.clone().unwrap_or_default(),
            stats.chatCount,
            stats.messageCount
        );
    }
    Ok(())
}

fn bind_chat_character(args: &[String]) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat bind-character <chat-id> <character-card-name>".to_string())?
        .clone();
    let characterCardName = args
        .get(1)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| "usage: operit2 chat bind-character <chat-id> <character-card-name>".to_string())?;
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    manager
        .updateChatCharacterBinding(chatId.clone(), Some(characterCardName), None)
        .map_err(|error| error.to_string())?;
    println!("chat character binding updated: {chatId}");
    Ok(())
}

fn bind_chat_group_card(args: &[String]) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat bind-group <chat-id> <character-group-id>".to_string())?
        .clone();
    let characterGroupId = args
        .get(1)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| "usage: operit2 chat bind-group <chat-id> <character-group-id>".to_string())?;
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    manager
        .updateChatCharacterBinding(chatId.clone(), None, Some(characterGroupId))
        .map_err(|error| error.to_string())?;
    println!("chat group binding updated: {chatId}");
    Ok(())
}

fn set_chat_group(args: &[String]) -> Result<(), String> {
    let chatId = args
        .get(0)
        .ok_or_else(|| "usage: operit2 chat set-group <chat-id> <group-name>".to_string())?
        .clone();
    let groupName = args
        .get(1)
        .cloned()
        .and_then(nonBlankString)
        .ok_or_else(|| "usage: operit2 chat set-group <chat-id> <group-name>".to_string())?;
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    manager
        .updateChatGroup(chatId.clone(), Some(groupName))
        .map_err(|error| error.to_string())?;
    println!("chat group updated: {chatId}");
    Ok(())
}

async fn create_chat(args: &[String]) -> Result<(), String> {
    let (characterCardName, characterGroupId, group) = parse_chat_new_args(args)?;
    let mut application = create_cli_application();
    application.onCreate()?;
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.createNewChat(characterCardName, group, true, true, characterGroupId);
    let chatId = core
        .currentChatIdFlow()
        .value()
        .ok_or_else(|| "core did not create chat".to_string())?;
    println!("{chatId}");
    Ok(())
}

fn parse_chat_new_args(args: &[String]) -> Result<(Option<String>, Option<String>, Option<String>), String> {
    let mut characterCardName = None;
    let mut characterGroupId = None;
    let mut group = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--character" => {
                index += 1;
                characterCardName = args.get(index).cloned().and_then(nonBlankString);
            }
            "--group-card" => {
                index += 1;
                characterGroupId = args.get(index).cloned().and_then(nonBlankString);
            }
            "--group" => {
                index += 1;
                group = args.get(index).cloned().and_then(nonBlankString);
            }
            _ => return Err("usage: operit2 chat new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]".to_string()),
        }
        index += 1;
    }
    Ok((characterCardName, characterGroupId, group))
}

#[derive(Clone, Debug)]
pub(crate) struct ChatSendArgs {
    chatId: Option<String>,
    message: String,
    attachmentPaths: Vec<String>,
    replyToTimestamp: Option<i64>,
}

#[derive(Clone, Debug)]
pub(crate) struct ShellArgs {
    chatId: Option<String>,
    characterCardName: Option<String>,
    characterGroupId: Option<String>,
    group: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ChatSendResult {
    chatId: String,
    aiMessage: ChatMessage,
}

pub(crate) enum ShellLoopControl {
    Continue,
    Exit,
}

fn parse_chat_send_args(args: &[String]) -> Result<ChatSendArgs, String> {
    if args.is_empty() {
        return Err("usage: operit2 chat send [--chat <chat-id>] [--attachment <path>] [--reply-to <timestamp>] <message>".to_string());
    }
    let usage = "usage: operit2 chat send [--chat <chat-id>] [--attachment <path>] [--reply-to <timestamp>] <message>";
    let mut chatId = None;
    let mut attachmentPaths = Vec::new();
    let mut replyToTimestamp = None;
    let mut messageParts = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--chat" => {
                index += 1;
                chatId = Some(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            "--attachment" | "--attach" => {
                index += 1;
                attachmentPaths.push(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            "--reply-to" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| usage.to_string())?;
                replyToTimestamp = Some(
                    value
                        .parse::<i64>()
                        .map_err(|_| "reply-to must be a message timestamp".to_string())?,
                );
            }
            value => messageParts.push(value.to_string()),
        }
        index += 1;
    }
    if messageParts.is_empty() {
        return Err(usage.to_string());
    }
    Ok(ChatSendArgs {
        chatId,
        message: messageParts.join(" "),
        attachmentPaths,
        replyToTimestamp,
    })
}

pub(crate) fn parse_shell_args(args: &[String]) -> Result<ShellArgs, String> {
    let usage = "usage: operit2 [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]";
    let mut shellArgs = ShellArgs {
        chatId: None,
        characterCardName: None,
        characterGroupId: None,
        group: None,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--chat" => {
                index += 1;
                shellArgs.chatId = Some(args.get(index).ok_or_else(|| usage.to_string())?.clone());
            }
            "--character" => {
                index += 1;
                shellArgs.characterCardName = args.get(index).cloned().and_then(nonBlankString);
            }
            "--group-card" => {
                index += 1;
                shellArgs.characterGroupId = args.get(index).cloned().and_then(nonBlankString);
            }
            "--group" => {
                index += 1;
                shellArgs.group = args.get(index).cloned().and_then(nonBlankString);
            }
            _ => return Err(usage.to_string()),
        }
        index += 1;
    }
    if shellArgs.chatId.is_some()
        && (shellArgs.characterCardName.is_some()
            || shellArgs.characterGroupId.is_some()
            || shellArgs.group.is_some())
    {
        return Err(usage.to_string());
    }
    Ok(shellArgs)
}

pub(crate) async fn run_shell_command(args: &[String]) -> Result<(), String> {
    let shellArgs = parse_shell_args(args)?;
    let mut application = create_cli_application();
    application.onCreate()?;
    let mut queuedAttachmentPaths = Vec::<String>::new();
    let initialChatId = initialize_shell_chat(&mut application, &shellArgs)?;
    println!("interactive shell ready");
    println!("chat={initialChatId}");
    println!("type /help for commands");
    loop {
        let currentChatId = current_shell_chat_id(&mut application)?;
        print!("operit2[{}]> ", short_chat_label(&currentChatId));
        io::stdout().flush().map_err(|error| error.to_string())?;
        let mut line = String::new();
        let readBytes = io::stdin()
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if readBytes == 0 {
            println!();
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input.starts_with('/') {
            match handle_shell_command(
                input,
                &mut application,
                &mut queuedAttachmentPaths,
            )
            .await?
            {
                ShellLoopControl::Continue => continue,
                ShellLoopControl::Exit => break,
            }
        } else {
            let sendArgs = ChatSendArgs {
                chatId: Some(currentChatId),
                message: input.to_string(),
                attachmentPaths: queuedAttachmentPaths.clone(),
                replyToTimestamp: None,
            };
            match send_chat_message_with_application(&mut application, sendArgs).await {
                Ok(result) => {
                    print_chat_send_result(&result);
                    queuedAttachmentPaths.clear();
                }
                Err(error) => eprintln!("{error}"),
            }
        }
    }
    Ok(())
}

pub(crate) fn initialize_shell_chat(
    application: &mut OperitApplication,
    shellArgs: &ShellArgs,
) -> Result<String, String> {
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    if let Some(chatId) = shellArgs.chatId.clone() {
        ensure_chat_exists(&chatId)?;
        core.switchChat(chatId.clone());
        Ok(chatId)
    } else {
        core.createNewChat(
            shellArgs.characterCardName.clone(),
            shellArgs.group.clone(),
            true,
            true,
            shellArgs.characterGroupId.clone(),
        );
        core.currentChatIdFlow()
            .value()
            .ok_or_else(|| "core did not create chat".to_string())
    }
}

pub(crate) fn ensure_chat_exists(chatId: &str) -> Result<(), String> {
    let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
    let exists = manager
        .chatHistoriesFlow()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|chat| chat.id == chatId);
    if exists {
        Ok(())
    } else {
        Err(format!("chat not found: {chatId}"))
    }
}

pub(crate) fn current_shell_chat_id(application: &mut OperitApplication) -> Result<String, String> {
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.currentChatIdFlow()
        .value()
        .ok_or_else(|| "no active chat in shell".to_string())
}

async fn handle_shell_command(
    input: &str,
    application: &mut OperitApplication,
    queuedAttachmentPaths: &mut Vec<String>,
) -> Result<ShellLoopControl, String> {
    let parts = split_shell_command_line(input)?;
    if parts.is_empty() {
        return Ok(ShellLoopControl::Continue);
    }
    let command = parts[0].trim_start_matches('/');
    let args = &parts[1..];
    match command {
        "help" => {
            print_shell_usage();
        }
        "exit" | "quit" => {
            return Ok(ShellLoopControl::Exit);
        }
        "chat" | "current" => {
            println!("{}", current_shell_chat_id(application)?);
        }
        "new" => {
            let shellArgs = parse_shell_args(args)?;
            if shellArgs.chatId.is_some() {
                return Err("shell /new does not accept --chat".to_string());
            }
            let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
            core.createNewChat(
                shellArgs.characterCardName,
                shellArgs.group,
                true,
                true,
                shellArgs.characterGroupId,
            );
            let chatId = core
                .currentChatIdFlow()
                .value()
                .ok_or_else(|| "core did not create chat".to_string())?;
            println!("chat={chatId}");
        }
        "switch" => {
            let chatId = args
                .get(0)
                .ok_or_else(|| "usage: /switch <chat-id>".to_string())?
                .clone();
            ensure_chat_exists(&chatId)?;
            let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
            core.switchChat(chatId.clone());
            println!("chat={chatId}");
        }
        "show" => {
            let chatId = current_shell_chat_id(application)?;
            let manager = ChatHistoryManager::default().map_err(|error| error.to_string())?;
            let chat = manager
                .chatHistoriesFlow()
                .map_err(|error| error.to_string())?
                .into_iter()
                .find(|chat| chat.id == chatId)
                .ok_or_else(|| format!("chat not found: {chatId}"))?;
            print_chat_history_header(&chat);
            let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
            for message in core.chatHistoryFlow().value() {
                print_chat_message(&message);
            }
        }
        "attach" => {
            let path = args
                .get(0)
                .ok_or_else(|| "usage: /attach <path>".to_string())?
                .clone();
            queuedAttachmentPaths.push(path.clone());
            println!("queued attachment: {path}");
        }
        "attachments" => {
            if queuedAttachmentPaths.is_empty() {
                println!("attachments=none");
            } else {
                for path in queuedAttachmentPaths.iter() {
                    println!("{path}");
                }
            }
        }
        "clear-attachments" => {
            queuedAttachmentPaths.clear();
            println!("attachments cleared");
        }
        "send" => {
            let message = args.join(" ");
            if message.trim().is_empty() {
                return Err("usage: /send <message>".to_string());
            }
            let chatId = current_shell_chat_id(application)?;
            let sendArgs = ChatSendArgs {
                chatId: Some(chatId),
                message,
                attachmentPaths: queuedAttachmentPaths.clone(),
                replyToTimestamp: None,
            };
            match send_chat_message_with_application(application, sendArgs).await {
                Ok(result) => {
                    print_chat_send_result(&result);
                    queuedAttachmentPaths.clear();
                }
                Err(error) => eprintln!("{error}"),
            }
        }
        _ => {
            return Err(format!("unknown shell command: /{command}"));
        }
    }
    Ok(ShellLoopControl::Continue)
}

fn split_shell_command_line(input: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None::<char>;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match quote {
            Some(activeQuote) => {
                if ch == activeQuote {
                    quote = None;
                } else if ch == '\\' && activeQuote == '"' {
                    match chars.next() {
                        Some(next) => current.push(next),
                        None => current.push('\\'),
                    }
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '"' | '\'' => quote = Some(ch),
                '\\' => match chars.next() {
                    Some(next) => current.push(next),
                    None => current.push('\\'),
                },
                ch if ch.is_whitespace() => {
                    if !current.is_empty() {
                        parts.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            },
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".to_string());
    }
    if !current.is_empty() {
        parts.push(current);
    }
    Ok(parts)
}

fn short_chat_label(chatId: &str) -> String {
    chatId.chars().take(8).collect()
}

fn print_shell_usage() {
    println!("/help");
    println!("/exit");
    println!("/quit");
    println!("/chat");
    println!("/new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("/switch <chat-id>");
    println!("/show");
    println!("/attach <path>");
    println!("/attachments");
    println!("/clear-attachments");
    println!("/send <message>");
}

pub(crate) async fn begin_chat_message_with_application(
    application: &mut OperitApplication,
    sendArgs: ChatSendArgs,
) -> Result<ChatSendResult, String> {
    let beforeLastAiTimestamp = dispatch_chat_message_with_application(application, sendArgs).await?;
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    let currentChatId = core
        .currentChatIdFlow()
        .value()
        .ok_or_else(|| "core has no active chat after send".to_string())?;
    let aiMessage = core
        .chatHistoryFlow()
        .value()
        .iter()
        .rev()
        .find(|message| message.sender == "ai" && message.timestamp > beforeLastAiTimestamp)
        .ok_or_else(|| "core did not produce ai message for current turn".to_string())?
        .clone();
    Ok(ChatSendResult {
        chatId: currentChatId,
        aiMessage,
    })
}

pub(crate) async fn dispatch_chat_message_with_application(
    application: &mut OperitApplication,
    sendArgs: ChatSendArgs,
) -> Result<i64, String> {
    let modelConfigManager = ModelConfigManager::default();
    let functionalConfigManager = FunctionalConfigManager::default();
    modelConfigManager
        .initializeIfNeeded()
        .map_err(|error| error.to_string())?;
    functionalConfigManager
        .initializeIfNeeded()
        .map_err(|error| error.to_string())?;
    let chatMapping = functionalConfigManager
        .getConfigMappingForFunction(FunctionType::CHAT)
        .map_err(|error| error.to_string())?;
    let turnOptions = ChatTurnOptions::default();
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    if let Some(chatId) = sendArgs.chatId.as_ref() {
        core.switchChat(chatId.clone());
    }
    let attachments = sendArgs
        .attachmentPaths
        .iter()
        .map(|path| build_attachment_info(path))
        .collect::<Result<Vec<_>, _>>()?;
    let replyToMessage = match sendArgs.replyToTimestamp {
        Some(timestamp) => core
            .chatHistoryFlow()
            .value()
            .iter()
            .find(|message| message.timestamp == timestamp)
            .cloned()
            .ok_or_else(|| format!("reply-to message not found: {timestamp}"))?,
        None => ChatMessage::new(String::new()),
    };
    let replyToMessage = if replyToMessage.sender.is_empty() {
        None
    } else {
        Some(replyToMessage)
    };
    core.updateUserMessage(sendArgs.message);
    let beforeLastAiTimestamp = core
        .chatHistoryFlow()
        .value()
        .iter()
        .filter(|message| message.sender == "ai")
        .map(|message| message.timestamp)
        .max()
        .unwrap_or(0);
    core.sendUserMessage(
        PromptFunctionType::CHAT,
        None,
        None,
        None,
        None,
        Some(chatMapping.configId),
        Some(chatMapping.modelIndex),
        attachments,
        replyToMessage,
        turnOptions,
    )
    .await;
    let currentChatId = core
        .currentChatIdFlow()
        .value()
        .ok_or_else(|| "core has no active chat after send".to_string())?;
    let inputProcessingStateByChatId = core.inputProcessingStateByChatIdFlow().value();
    match inputProcessingStateByChatId
        .get(&currentChatId)
        .or_else(|| inputProcessingStateByChatId.get("__DEFAULT_CHAT__"))
    {
        Some(InputProcessingState::Error { message }) => return Err(message.clone()),
        _ => {}
    }
    Ok(beforeLastAiTimestamp)
}

pub(crate) fn launch_chat_message_with_application(
    application: &mut OperitApplication,
    sendArgs: ChatSendArgs,
) -> Result<String, String> {
    let modelConfigManager = ModelConfigManager::default();
    let functionalConfigManager = FunctionalConfigManager::default();
    modelConfigManager
        .initializeIfNeeded()
        .map_err(|error| error.to_string())?;
    functionalConfigManager
        .initializeIfNeeded()
        .map_err(|error| error.to_string())?;
    let chatMapping = functionalConfigManager
        .getConfigMappingForFunction(FunctionType::CHAT)
        .map_err(|error| error.to_string())?;
    let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
    core.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
    if let Some(chatId) = sendArgs.chatId.as_ref() {
        core.switchChat(chatId.clone());
    }
    let chatId = core
        .currentChatIdFlow()
        .value()
        .ok_or_else(|| "core has no active chat before send".to_string())?;
    let attachments = sendArgs
        .attachmentPaths
        .iter()
        .map(|path| build_attachment_info(path))
        .collect::<Result<Vec<_>, _>>()?;
    let replyToMessage = match sendArgs.replyToTimestamp {
        Some(timestamp) => core
            .chatHistoryFlow()
            .value()
            .iter()
            .find(|message| message.timestamp == timestamp)
            .cloned()
            .ok_or_else(|| format!("reply-to message not found: {timestamp}"))?,
        None => ChatMessage::new(String::new()),
    };
    let replyToMessage = if replyToMessage.sender.is_empty() {
        None
    } else {
        Some(replyToMessage)
    };
    core.updateUserMessage(sendArgs.message);

    let mut service = core
        .enhancedAiService
        .clone()
        .ok_or_else(|| "ai service is not initialized".to_string())?;
    let chatHistoryDelegate = core.chatHistoryDelegate.clone_for_core();
    let messageProcessingDelegate = core.messageProcessingDelegate.clone_for_core();
    let mut delegate = MessageCoordinationDelegate::new(chatHistoryDelegate, messageProcessingDelegate);
    let threadChatId = chatId.clone();
    std::thread::spawn(move || {
        let runtimeResult = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let runtime = match runtimeResult {
            Ok(runtime) => runtime,
            Err(error) => {
                delegate.messageProcessingDelegate.setInputProcessingStateForChat(
                    threadChatId,
                    InputProcessingState::Error {
                        message: error.to_string(),
                    },
                );
                return;
            }
        };
        runtime.block_on(async move {
            delegate
                .sendUserMessage(
                    &mut service,
                    PromptFunctionType::CHAT,
                    None,
                    Some(threadChatId),
                    None,
                    None,
                    Some(chatMapping.configId),
                    Some(chatMapping.modelIndex),
                    attachments,
                    replyToMessage,
                    ChatTurnOptions::default(),
                )
                .await;
        });
    });
    Ok(chatId)
}

pub(crate) async fn send_chat_message_with_application(
    application: &mut OperitApplication,
    sendArgs: ChatSendArgs,
) -> Result<ChatSendResult, String> {
    let mut result = begin_chat_message_with_application(application, sendArgs).await?;
    if let Some(mut stream) = result.aiMessage.contentStream.clone() {
        let mut content = String::new();
        stream.collect(&mut |chunk| {
            content.push_str(&chunk);
        });
        result.aiMessage.content = content;
        result.aiMessage.contentStream = None;
    }
    Ok(result)
}

fn print_chat_send_result(result: &ChatSendResult) {
    print!("{}", result.aiMessage.content);
    println!();
    eprintln!(
        "chat={} provider={} modelName={} inputTokens={} cachedInputTokens={} outputTokens={}",
        result.chatId,
        result.aiMessage.provider,
        result.aiMessage.modelName,
        result.aiMessage.inputTokens,
        result.aiMessage.cachedInputTokens,
        result.aiMessage.outputTokens
    );
}

async fn send_chat_message(sendArgs: ChatSendArgs) -> Result<(), String> {
    let mut application = create_cli_application();
    application.onCreate()?;
    let result = send_chat_message_with_application(&mut application, sendArgs).await?;
    print_chat_send_result(&result);
    Ok(())
}

pub(crate) fn build_attachment_info(path: &str) -> Result<AttachmentInfo, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("attachment metadata failed: {path}: {error}"))?;
    let fileName = Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("attachment file name invalid: {path}"))?
        .to_string();
    let content = fs::read_to_string(path).unwrap_or_default();
    Ok(AttachmentInfo {
        filePath: path.to_string(),
        fileName,
        mimeType: guess_mime_type(path).to_string(),
        fileSize: metadata.len() as i64,
        content,
    })
}

pub(crate) fn guess_mime_type(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("txt") | Some("md") | Some("rs") | Some("kt") | Some("json") | Some("toml") => "text/plain",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("mp4") => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn print_root_usage() {
    println!("operit2");
    println!("operit2 [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 tui [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli <model|chat|tag|character|group|active-prompt|shell>");
    println!();
    print_cli_usage();
}

fn print_cli_usage() {
    println!("operit2 cli model <init|list|show|set|set-key|api-settings-full|custom-headers|request-queue|api-key-pool|custom-parameters|parameters|tool-call|direct-image|direct-audio|direct-video|google-search|params|context-show|context-set|summary-show|summary-set|function-list|function-show|function-set|function-reset>");
    println!("operit2 cli tag <list|show|create|update|delete>");
    println!("operit2 cli character <init|list|show|create|update|delete|set-active|combine|reset-default>");
    println!("operit2 cli group <init|list|show|create|update|delete|set-active|duplicate>");
    println!("operit2 cli active-prompt <show|set-card|set-group|activate-for-chat|resolved-card>");
    println!("operit2 cli shell [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat <new|list|show|current|switch|stats|bind-character|bind-group|set-group|shell|send>");
    println!("operit2 cli chat new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat list");
    println!("operit2 cli chat show <chat-id> [--runtime]");
    println!("operit2 cli chat current");
    println!("operit2 cli chat switch <chat-id>");
    println!("operit2 cli chat stats");
    println!("operit2 cli chat bind-character <chat-id> <character-card-name>");
    println!("operit2 cli chat bind-group <chat-id> <character-group-id>");
    println!("operit2 cli chat set-group <chat-id> <group-name>");
    println!("operit2 cli chat shell [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat send [--chat <chat-id>] <message>");
}

fn print_model_usage() {
    println!("operit2 cli model init");
    println!("operit2 cli model list");
    println!("operit2 cli model show [config-id]");
    println!("operit2 cli model set <endpoint> <model-name> [config-id]");
    println!("operit2 cli model set-key <api-key> [config-id]");
    println!("operit2 cli model api-settings-full <api-key> <endpoint> <model-name> <provider-type> <provider-type-id> <mnn-forward-type> <mnn-thread-count> <llama-thread-count> <llama-context-size> <llama-gpu-layers> <enable-direct-image-processing> <enable-direct-audio-processing> <enable-direct-video-processing> <enable-google-search> <enable-tool-call> [config-id]");
    println!("operit2 cli model custom-headers <custom-headers-json> [config-id]");
    println!("operit2 cli model request-queue <request-limit-per-minute> <max-concurrent-requests> [config-id]");
    println!("operit2 cli model api-key-pool <use-multiple-api-keys> <api-key-pool-json> [config-id]");
    println!("operit2 cli model custom-parameters <parameters-json> [config-id]");
    println!("operit2 cli model parameters <parameters-json> [config-id]");
    println!("operit2 cli model tool-call <enable-tool-call> [config-id]");
    println!("operit2 cli model direct-image <enable-direct-image-processing> [config-id]");
    println!("operit2 cli model direct-audio <enable-direct-audio-processing> [config-id]");
    println!("operit2 cli model direct-video <enable-direct-video-processing> [config-id]");
    println!("operit2 cli model google-search <enable-google-search> [config-id]");
    println!("operit2 cli model params [config-id]");
    println!("operit2 cli model context-show [config-id]");
    println!("operit2 cli model context-set <context-length> <max-context-length> <enable-max-context-mode> [config-id]");
    println!("operit2 cli model summary-show [config-id]");
    println!("operit2 cli model summary-set <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold> [config-id]");
    println!("operit2 cli model function-list");
    println!("operit2 cli model function-show <function-type>");
    println!("operit2 cli model function-set <function-type> <config-id> [model-index]");
    println!("operit2 cli model function-reset [function-type]");
}

fn print_chat_usage() {
    println!("operit2 cli chat new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat list");
    println!("operit2 cli chat show <chat-id> [--runtime]");
    println!("operit2 cli chat current");
    println!("operit2 cli chat switch <chat-id>");
    println!("operit2 cli chat stats");
    println!("operit2 cli chat bind-character <chat-id> <character-card-name>");
    println!("operit2 cli chat bind-group <chat-id> <character-group-id>");
    println!("operit2 cli chat set-group <chat-id> <group-name>");
    println!("operit2 cli chat shell [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat send [--chat <chat-id>] <message>");
}

fn print_tag_usage() {
    println!("operit2 cli tag list");
    println!("operit2 cli tag show <id>");
    println!("operit2 cli tag create <name> [prompt-content] [description] [tag-type]");
    println!("operit2 cli tag update <id> <field> <value>");
    println!("operit2 cli tag delete <id>");
}

fn print_character_usage() {
    println!("operit2 cli character init");
    println!("operit2 cli character list");
    println!("operit2 cli character show <id>");
    println!("operit2 cli character create <name> [character-setting]");
    println!("operit2 cli character update <id> <field> <value>");
    println!("operit2 cli character delete <id>");
    println!("operit2 cli character set-active <id>");
    println!("operit2 cli character combine <id> [CHAT|VOICE] [tag-id-csv]");
    println!("operit2 cli character reset-default");
}

fn print_group_usage() {
    println!("operit2 cli group init");
    println!("operit2 cli group list");
    println!("operit2 cli group show <id>");
    println!("operit2 cli group create <name> [description]");
    println!("operit2 cli group update <id> <field> <value>");
    println!("operit2 cli group delete <id>");
    println!("operit2 cli group set-active <id>");
    println!("operit2 cli group duplicate <source-id> [new-name]");
}

fn print_active_prompt_usage() {
    println!("operit2 cli active-prompt show");
    println!("operit2 cli active-prompt set-card <id>");
    println!("operit2 cli active-prompt set-group <id>");
    println!("operit2 cli active-prompt activate-for-chat [character-card-name] [character-group-id]");
    println!("operit2 cli active-prompt resolved-card");
}

fn print_chat_history_header(chat: &operit_runtime::data::model::ChatHistory::ChatHistory) {
    println!("id={}", chat.id);
    println!("title={}", chat.title);
    println!("createdAt={}", chat.createdAt);
    println!("updatedAt={}", chat.updatedAt);
    println!("inputTokens={}", chat.inputTokens);
    println!("outputTokens={}", chat.outputTokens);
    println!("currentWindowSize={}", chat.currentWindowSize);
    println!("group={}", chat.group.clone().unwrap_or_default());
    println!("displayOrder={}", chat.displayOrder);
    println!("workspace={}", chat.workspace.clone().unwrap_or_default());
    println!("workspaceEnv={}", chat.workspaceEnv.clone().unwrap_or_default());
    println!("parentChatId={}", chat.parentChatId.clone().unwrap_or_default());
    println!("characterCardName={}", chat.characterCardName.clone().unwrap_or_default());
    println!("characterGroupId={}", chat.characterGroupId.clone().unwrap_or_default());
    println!("locked={}", chat.locked);
}

fn print_chat_message(message: &operit_runtime::data::model::ChatMessage::ChatMessage) {
    println!("--- message ---");
    println!("sender={}", message.sender);
    println!("timestamp={}", message.timestamp);
    println!("roleName={}", message.roleName);
    println!("selectedVariantIndex={}", message.selectedVariantIndex);
    println!("variantCount={}", message.variantCount);
    println!("provider={}", message.provider);
    println!("modelName={}", message.modelName);
    println!("inputTokens={}", message.inputTokens);
    println!("cachedInputTokens={}", message.cachedInputTokens);
    println!("outputTokens={}", message.outputTokens);
    println!("sentAt={}", message.sentAt);
    println!("waitDurationMs={}", message.waitDurationMs);
    println!("outputDurationMs={}", message.outputDurationMs);
    println!("completedAt={}", message.completedAt);
    println!("displayMode={:?}", message.displayMode);
    println!("isFavorite={}", message.isFavorite);
    println!("content={}", message.content);
}

fn print_tag(tag: &operit_runtime::data::model::PromptTag::PromptTag) {
    println!("id={}", tag.id);
    println!("name={}", tag.name);
    println!("description={}", tag.description);
    println!("promptContent={}", tag.promptContent);
    println!("tagType={}", tagTypeName(&tag.tagType));
    println!("createdAt={}", tag.createdAt);
    println!("updatedAt={}", tag.updatedAt);
}

fn print_character_card(card: &CharacterCard) {
    println!("id={}", card.id);
    println!("name={}", card.name);
    println!("description={}", card.description);
    println!("characterSetting={}", card.characterSetting);
    println!("openingStatement={}", card.openingStatement);
    println!("otherContentChat={}", card.otherContentChat);
    println!("otherContentVoice={}", card.otherContentVoice);
    println!("attachedTagIds={}", card.attachedTagIds.join(","));
    println!("advancedCustomPrompt={}", card.advancedCustomPrompt);
    println!("marks={}", card.marks);
    println!("chatModelBindingMode={}", card.chatModelBindingMode);
    println!("chatModelConfigId={}", card.chatModelConfigId.clone().unwrap_or_default());
    println!("chatModelIndex={}", card.chatModelIndex);
    println!("memoryProfileBindingMode={}", card.memoryProfileBindingMode);
    println!("memoryProfileId={}", card.memoryProfileId.clone().unwrap_or_default());
    println!("toolAccessConfig={}", serde_json::to_string(&card.toolAccessConfig).expect("toolAccessConfig must serialize"));
    println!("isDefault={}", card.isDefault);
    println!("createdAt={}", card.createdAt);
    println!("updatedAt={}", card.updatedAt);
}

fn print_character_group_card(group: &CharacterGroupCard) {
    println!("id={}", group.id);
    println!("name={}", group.name);
    println!("description={}", group.description);
    println!(
        "members={}",
        group
            .members
            .iter()
            .map(|member| format!("{}:{}", member.characterCardId, member.orderIndex))
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("createdAt={}", group.createdAt);
    println!("updatedAt={}", group.updatedAt);
}

fn parse_group_members(value: &str) -> Vec<GroupMemberConfig> {
    let mut result = Vec::new();
    for (index, item) in value.split(',').enumerate() {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        result.push(GroupMemberConfig {
            characterCardId: trimmed.to_string(),
            orderIndex: index as i32,
        });
    }
    result
}

fn parseTagType(value: Option<&str>) -> Result<TagType, String> {
    match value.unwrap_or("CUSTOM") {
        "TONE" => Ok(TagType::TONE),
        "CHARACTER" => Ok(TagType::CHARACTER),
        "FUNCTION" => Ok(TagType::FUNCTION),
        "CUSTOM" => Ok(TagType::CUSTOM),
        other => Err(format!("invalid tagType: {other}; expected TONE | CHARACTER | FUNCTION | CUSTOM")),
    }
}

fn tagTypeName(tagType: &TagType) -> &'static str {
    match tagType {
        TagType::TONE => "TONE",
        TagType::CHARACTER => "CHARACTER",
        TagType::FUNCTION => "FUNCTION",
        TagType::CUSTOM => "CUSTOM",
    }
}

fn parsePromptFunctionType(value: Option<&str>) -> Result<PromptFunctionType, String> {
    match value.unwrap_or("CHAT") {
        "CHAT" => Ok(PromptFunctionType::CHAT),
        "VOICE" => Ok(PromptFunctionType::VOICE),
        other => Err(format!("invalid promptFunctionType: {other}; expected CHAT | VOICE")),
    }
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

fn parse_f32_arg(value: Option<&String>, usage: &str) -> Result<f32, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<f32>()
        .map_err(|error| error.to_string())
}

fn parse_i32_arg(value: Option<&String>, usage: &str) -> Result<i32, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<i32>()
        .map_err(|error| error.to_string())
}

fn parse_bool_arg(value: Option<&String>, usage: &str) -> Result<bool, String> {
    match value.ok_or_else(|| usage.to_string())?.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("invalid bool: {other}; expected true | false")),
    }
}

fn parseApiProviderType(value: &str) -> Result<ApiProviderType, String> {
    ApiProviderType::fromProviderTypeId(value)
        .ok_or_else(|| format!("invalid ApiProviderType: {value}"))
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

fn parseCsvList(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if !trimmed.is_empty() && !result.iter().any(|entry| entry == trimmed) {
            result.push(trimmed.to_string());
        }
    }
    result
}

fn nonBlankString(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after unix epoch")
        .as_millis() as i64
}
