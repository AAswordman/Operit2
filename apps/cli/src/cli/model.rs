use super::core::CliCore;
use super::*;

pub(super) async fn run_model_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_model_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "init" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            core.preferences_functional_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            println!("initialized");
        }
        "list" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            for summary in core
                .preferences_model_config_manager()
                .getAllConfigSummaries()
                .await
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
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let config = core
                .preferences_model_config_manager()
                .getModelConfig(configId)
                .await
                .map_err(|error| error.to_string())?;
            println!("id={}", config.id);
            println!("name={}", config.name);
            println!("provider={}", config.apiProviderType.name());
            println!("providerTypeId={}", config.apiProviderTypeId);
            println!("endpoint={}", config.apiEndpoint);
            println!("modelName={}", config.modelName);
            println!("apiKeyLength={}", config.apiKey.len());
            println!("useMultipleApiKeys={}", config.useMultipleApiKeys);
            println!(
                "apiKeyPool={}",
                serde_json::to_string(&config.apiKeyPool).map_err(|error| error.to_string())?
            );
            println!("currentKeyIndex={}", config.currentKeyIndex);
            println!("keyRotationMode={}", config.keyRotationMode);
            println!("hasCustomParameters={}", config.hasCustomParameters);
            println!("maxTokensEnabled={}", config.maxTokensEnabled);
            println!("temperatureEnabled={}", config.temperatureEnabled);
            println!("topPEnabled={}", config.topPEnabled);
            println!("topKEnabled={}", config.topKEnabled);
            println!("presencePenaltyEnabled={}", config.presencePenaltyEnabled);
            println!("frequencyPenaltyEnabled={}", config.frequencyPenaltyEnabled);
            println!(
                "repetitionPenaltyEnabled={}",
                config.repetitionPenaltyEnabled
            );
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
            println!(
                "enableSummaryByMessageCount={}",
                config.enableSummaryByMessageCount
            );
            println!(
                "summaryMessageCountThreshold={}",
                config.summaryMessageCountThreshold
            );
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
            println!(
                "enableDirectImageProcessing={}",
                config.enableDirectImageProcessing
            );
            println!(
                "enableDirectAudioProcessing={}",
                config.enableDirectAudioProcessing
            );
            println!(
                "enableDirectVideoProcessing={}",
                config.enableDirectVideoProcessing
            );
            println!("enableGoogleSearch={}", config.enableGoogleSearch);
            println!("enableToolCall={}", config.enableToolCall);
            println!("requestLimitPerMinute={}", config.requestLimitPerMinute);
            println!("maxConcurrentRequests={}", config.maxConcurrentRequests);
        }
        "set-key" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let apiKey = args
                .get(1)
                .ok_or_else(|| "usage: operit2 model set-key <api-key> [config-id]".to_string())?
                .clone();
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateApiKey(configId, apiKey)
                .await
                .map_err(|error| error.to_string())?;
            println!("api key updated: {configId}");
        }
        "set" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let endpoint = args
                .get(1)
                .ok_or_else(|| {
                    "usage: operit2 model set <endpoint> <model-name> [config-id]".to_string()
                })?
                .clone();
            let modelName = args
                .get(2)
                .ok_or_else(|| {
                    "usage: operit2 model set <endpoint> <model-name> [config-id]".to_string()
                })?
                .clone();
            let configId = match args.get(3).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let current = core
                .preferences_model_config_manager()
                .getModelConfig(configId)
                .await
                .map_err(|error| error.to_string())?;
            core.preferences_model_config_manager()
                .updateModelConfig(configId, current.apiKey, endpoint, modelName)
                .await
                .map_err(|error| error.to_string())?;
            println!("model updated: {configId}");
        }
        "tool-call" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let enableToolCall = parse_bool_arg(
                args.get(1),
                "usage: operit2 model tool-call <enable-tool-call> [config-id]",
            )?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateToolCall(configId, enableToolCall)
                .await
                .map_err(|error| error.to_string())?;
            println!("tool call updated: {configId}\t{enableToolCall}");
        }
        "api-settings-full" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
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
            core.preferences_model_config_manager()
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
                .await
                .map_err(|error| error.to_string())?;
            println!("api settings full updated: {configId}");
        }
        "custom-headers" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let customHeaders = args
                .get(1)
                .ok_or_else(|| {
                    "usage: operit2 model custom-headers <custom-headers-json> [config-id]"
                        .to_string()
                })?
                .clone();
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateCustomHeaders(configId, customHeaders)
                .await
                .map_err(|error| error.to_string())?;
            println!("custom headers updated: {configId}");
        }
        "request-queue" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let requestLimitPerMinute = parse_i32_arg(args.get(1), "usage: operit2 model request-queue <request-limit-per-minute> <max-concurrent-requests> [config-id]")?;
            let maxConcurrentRequests = parse_i32_arg(args.get(2), "usage: operit2 model request-queue <request-limit-per-minute> <max-concurrent-requests> [config-id]")?;
            let configId = match args.get(3).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateRequestQueueSettings(configId, requestLimitPerMinute, maxConcurrentRequests)
                .await
                .map_err(|error| error.to_string())?;
            println!("request queue updated: {configId}");
        }
        "api-key-pool" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
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
            core.preferences_model_config_manager()
                .updateApiKeyPoolSettings(configId, useMultipleApiKeys, apiKeyPool)
                .await
                .map_err(|error| error.to_string())?;
            println!("api key pool updated: {configId}");
        }
        "custom-parameters" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let parametersJson = args
                .get(1)
                .ok_or_else(|| {
                    "usage: operit2 model custom-parameters <parameters-json> [config-id]"
                        .to_string()
                })?
                .clone();
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateCustomParameters(configId, parametersJson)
                .await
                .map_err(|error| error.to_string())?;
            println!("custom parameters updated: {configId}");
        }
        "parameters" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let parametersJson = args.get(1).ok_or_else(|| {
                "usage: operit2 model parameters <parameters-json> [config-id]".to_string()
            })?;
            let parameters =
                serde_json::from_str::<Vec<ModelParameter<serde_json::Value>>>(parametersJson)
                    .map_err(|error| error.to_string())?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateParameters(configId, parameters)
                .await
                .map_err(|error| error.to_string())?;
            println!("parameters updated: {configId}");
        }
        "direct-image" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let enableDirectImageProcessing = parse_bool_arg(
                args.get(1),
                "usage: operit2 model direct-image <enable-direct-image-processing> [config-id]",
            )?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateDirectImageProcessing(configId, enableDirectImageProcessing)
                .await
                .map_err(|error| error.to_string())?;
            println!("direct image processing updated: {configId}\t{enableDirectImageProcessing}");
        }
        "direct-audio" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let enableDirectAudioProcessing = parse_bool_arg(
                args.get(1),
                "usage: operit2 model direct-audio <enable-direct-audio-processing> [config-id]",
            )?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateDirectAudioProcessing(configId, enableDirectAudioProcessing)
                .await
                .map_err(|error| error.to_string())?;
            println!("direct audio processing updated: {configId}\t{enableDirectAudioProcessing}");
        }
        "direct-video" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let enableDirectVideoProcessing = parse_bool_arg(
                args.get(1),
                "usage: operit2 model direct-video <enable-direct-video-processing> [config-id]",
            )?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateDirectVideoProcessing(configId, enableDirectVideoProcessing)
                .await
                .map_err(|error| error.to_string())?;
            println!("direct video processing updated: {configId}\t{enableDirectVideoProcessing}");
        }
        "google-search" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let enableGoogleSearch = parse_bool_arg(
                args.get(1),
                "usage: operit2 model google-search <enable-google-search> [config-id]",
            )?;
            let configId = match args.get(2).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            core.preferences_model_config_manager()
                .updateGoogleSearch(configId, enableGoogleSearch)
                .await
                .map_err(|error| error.to_string())?;
            println!("google search updated: {configId}\t{enableGoogleSearch}");
        }
        "params" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let params = core
                .preferences_model_config_manager()
                .getModelParametersForConfig(configId)
                .await
                .map_err(|error| error.to_string())?;
            for param in params {
                println!(
                    "{}\t{}\t{}\t{}",
                    param.id, param.apiName, param.isEnabled, param.currentValue
                );
            }
        }
        "context-show" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let config = core
                .preferences_model_config_manager()
                .getModelConfig(configId)
                .await
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
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let configId = match args.get(4).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let contextLength = parse_f32_arg(args.get(1), "usage: operit2 model context-set <context-length> <max-context-length> <enable-max-context-mode> [config-id]")?;
            let maxContextLength = parse_f32_arg(args.get(2), "usage: operit2 model context-set <context-length> <max-context-length> <enable-max-context-mode> [config-id]")?;
            let enableMaxContextMode = parse_bool_arg(args.get(3), "usage: operit2 model context-set <context-length> <max-context-length> <enable-max-context-mode> [config-id]")?;
            core.preferences_model_config_manager()
                .updateContextSettings(
                    configId,
                    contextLength,
                    maxContextLength,
                    enableMaxContextMode,
                )
                .await
                .map_err(|error| error.to_string())?;
            println!("context settings updated: {configId}");
        }
        "summary-show" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let configId = match args.get(1).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let config = core
                .preferences_model_config_manager()
                .getModelConfig(configId)
                .await
                .map_err(|error| error.to_string())?;
            println!("id={}", config.id);
            println!("enableSummary={}", config.enableSummary);
            println!("summaryTokenThreshold={}", config.summaryTokenThreshold);
            println!(
                "enableSummaryByMessageCount={}",
                config.enableSummaryByMessageCount
            );
            println!(
                "summaryMessageCountThreshold={}",
                config.summaryMessageCountThreshold
            );
        }
        "summary-set" => {
            core.preferences_model_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let configId = match args.get(5).map(String::as_str) {
                Some(value) => value,
                None => ModelConfigManager::DEFAULT_CONFIG_ID,
            };
            let enableSummary = parse_bool_arg(args.get(1), "usage: operit2 model summary-set <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold> [config-id]")?;
            let summaryTokenThreshold = parse_f32_arg(args.get(2), "usage: operit2 model summary-set <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold> [config-id]")?;
            let enableSummaryByMessageCount = parse_bool_arg(args.get(3), "usage: operit2 model summary-set <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold> [config-id]")?;
            let summaryMessageCountThreshold = parse_i32_arg(args.get(4), "usage: operit2 model summary-set <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold> [config-id]")?;
            core.preferences_model_config_manager()
                .updateSummarySettings(
                    configId,
                    enableSummary,
                    summaryTokenThreshold,
                    enableSummaryByMessageCount,
                    summaryMessageCountThreshold,
                )
                .await
                .map_err(|error| error.to_string())?;
            println!("summary settings updated: {configId}");
        }
        "function-list" => {
            core.preferences_functional_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let mut mappings = core
                .preferences_functional_config_manager()
                .functionConfigMappingWithIndex()
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .collect::<Vec<_>>();
            mappings
                .sort_by(|left, right| functionTypeName(&left.0).cmp(functionTypeName(&right.0)));
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
            core.preferences_functional_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let functionType = parseFunctionType(
                args.get(1)
                    .ok_or_else(|| {
                        "usage: operit2 model function-show <function-type>".to_string()
                    })?
                    .as_str(),
            )?;
            let mapping = core
                .preferences_functional_config_manager()
                .getConfigMappingForFunction(functionType.clone())
                .await
                .map_err(|error| error.to_string())?;
            println!("functionType={}", functionTypeName(&functionType));
            println!("configId={}", mapping.configId);
            println!("modelIndex={}", mapping.modelIndex);
        }
        "function-set" => {
            core.preferences_functional_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            let functionType = parseFunctionType(
                args.get(1)
                    .ok_or_else(|| "usage: operit2 model function-set <function-type> <config-id> [model-index]".to_string())?
                    .as_str(),
            )?;
            let configId = args
                .get(2)
                .ok_or_else(|| {
                    "usage: operit2 model function-set <function-type> <config-id> [model-index]"
                        .to_string()
                })?
                .clone();
            let modelIndex = args
                .get(3)
                .map(|value| value.parse::<i32>())
                .transpose()
                .map_err(|error| error.to_string())?
                .unwrap_or(0);
            core.preferences_functional_config_manager()
                .setConfigForFunctionWithIndex(functionType.clone(), configId.clone(), modelIndex)
                .await
                .map_err(|error| error.to_string())?;
            println!(
                "function mapping updated: {}\t{}\t{}",
                functionTypeName(&functionType),
                configId,
                modelIndex
            );
        }
        "function-reset" => {
            core.preferences_functional_config_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            if let Some(functionTypeValue) = args.get(1) {
                let functionType = parseFunctionType(functionTypeValue)?;
                core.preferences_functional_config_manager()
                    .resetFunctionConfig(functionType.clone())
                    .await
                    .map_err(|error| error.to_string())?;
                println!(
                    "function mapping reset: {}",
                    functionTypeName(&functionType)
                );
            } else {
                core.preferences_functional_config_manager()
                    .resetAllFunctionConfigs()
                    .await
                    .map_err(|error| error.to_string())?;
                println!("all function mappings reset");
            }
        }
        _ => print_model_usage(),
    }

    Ok(())
}
