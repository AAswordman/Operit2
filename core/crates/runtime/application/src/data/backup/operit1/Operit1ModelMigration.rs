struct ParsedOperit1Snapshot {
    archive: Operit1SnapshotArchive,
    configs: Vec<Operit1ModelConfig>,
    chatMapping: Operit1FunctionConfigMapping,
}

impl ParsedOperit1Snapshot {
    /// Parses importer metadata from an immutable range-readable snapshot source.
    fn fromSource(source: Arc<dyn ArchiveSource>) -> Result<Self, String> {
        let archive = Operit1SnapshotArchive::fromSource(source)?;
        let configs = archive
            .modelConfigJsons
            .iter()
            .map(|config| {
                serde_json::from_value(config.value.clone())
                    .map_err(|error| format!("模型配置「{}」格式不正确：{error}", config.id))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let chatMapping = decodeChatMapping(archive.chatMappingJson.clone())?;

        Ok(Self {
            archive,
            configs,
            chatMapping,
        })
    }

    /// Copies one archive entry to a caller-owned output stream.
    fn copyEntryTo<W: Write>(&self, name: &str, writer: &mut W) -> Result<(), String> {
        self.archive.copyEntryTo(name, writer)
    }

    /// Builds snapshot preview metadata using host-derived database counts.
    fn preview(&self, databaseCounts: (i32, i32)) -> Result<Operit1SnapshotPreview, String> {
        let modelConfig = self.modelConfigPreview()?;
        let datastoreFiles = self
            .archive
            .datastorePreferences
            .iter()
            .map(|(entryName, values)| Operit1DataStoreFilePreview {
                fileName: entryName
                    .strip_prefix(ENTRY_DATASTORE_PREFIX)
                    .unwrap_or(entryName)
                    .to_string(),
                keyCount: values.len() as i32,
            })
            .collect::<Vec<_>>();
        let (chatCount, messageCount) = databaseCounts;
        let importedFileCount = self.countImportableFiles(ENTRY_FILES_PREFIX);
        let importedExternalFileCount = self.countImportableFiles(ENTRY_EXTERNAL_FILES_PREFIX);
        let mut detectedDomains = Vec::new();
        if !self.configs.is_empty() {
            detectedDomains.push("model_configs".to_string());
        }
        if chatCount > 0 || messageCount > 0 {
            detectedDomains.push("chat_history".to_string());
        }
        for entry in self.archive.datastorePreferences.keys() {
            let name = entry
                .strip_prefix(ENTRY_DATASTORE_PREFIX)
                .unwrap_or(entry)
                .trim_end_matches(".preferences_pb")
                .to_string();
            if !detectedDomains.contains(&name) {
                detectedDomains.push(name);
            }
        }
        if importedFileCount > 0 || importedExternalFileCount > 0 {
            detectedDomains.push("user_files".to_string());
        }
        Ok(Operit1SnapshotPreview {
            formatVersion: self.archive.manifest.formatVersion,
            packageName: self.archive.manifest.packageName.clone(),
            createdAt: self.archive.manifest.createdAt,
            modelConfig,
            datastoreFiles,
            chatCount,
            messageCount,
            importedFileCount,
            importedExternalFileCount,
            detectedDomains,
        })
    }

    #[allow(non_snake_case)]
    fn modelConfigPreview(&self) -> Result<Operit1ModelConfigSnapshotPreview, String> {
        let configs = self
            .configs
            .iter()
            .map(|config| {
                let providerBinding = config.providerBinding()?;
                let modelIds = splitModelIds(&config.modelName);
                let selectedModelIndex =
                    (self.chatMapping.configId == config.id).then_some(self.chatMapping.modelIndex);
                let selectedModelId =
                    selectedModelIndex.and_then(|index| modelIds.get(index as usize).cloned());
                Ok(Operit1ModelConfigPreview {
                    configId: config.id.clone(),
                    name: config.name.clone(),
                    providerTypeId: providerBinding.providerTypeId,
                    providerDisplayName: providerBinding.displayName,
                    endpoint: config.apiEndpoint.clone(),
                    modelIds,
                    selectedModelId,
                    selectedModelIndex,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        let chatConfigId = Some(self.chatMapping.configId.clone());
        let chatModelIndex = Some(self.chatMapping.modelIndex);
        let chatModelId = self
            .configs
            .iter()
            .find(|config| config.id == self.chatMapping.configId)
            .and_then(|config| {
                splitModelIds(&config.modelName)
                    .get(self.chatMapping.modelIndex as usize)
                    .cloned()
            });

        Ok(Operit1ModelConfigSnapshotPreview {
            formatVersion: self.archive.manifest.formatVersion,
            packageName: self.archive.manifest.packageName.clone(),
            createdAt: self.archive.manifest.createdAt,
            configs,
            chatConfigId,
            chatModelId,
            chatModelIndex,
        })
    }

    #[allow(non_snake_case)]
    fn selectedChatConfig(&self) -> Result<Operit1ModelConfigPreview, String> {
        self.modelConfigPreview()?
            .configs
            .into_iter()
            .find(|config| config.configId == self.chatMapping.configId)
            .ok_or_else(|| format!("快照里没有聊天模型配置：{}", self.chatMapping.configId))
    }

    #[allow(non_snake_case)]
    fn configById(&self, configId: &str) -> Result<Operit1ModelConfig, String> {
        self.configs
            .iter()
            .find(|config| config.id == configId)
            .cloned()
            .ok_or_else(|| format!("快照里没有模型配置：{configId}"))
    }

    #[allow(non_snake_case)]
    fn countImportableFiles(&self, prefix: &str) -> i32 {
        self.archive
            .entries
            .keys()
            .filter(|entry| entry.starts_with(prefix))
            .filter(|entry| !isArchiveDataStoreEntry(entry))
            .count() as i32
    }
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1FunctionConfigMapping {
    #[serde(default = "defaultOperit1ConfigId")]
    configId: String,
    #[serde(default)]
    modelIndex: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1ModelConfig {
    id: String,
    name: String,
    #[serde(default)]
    apiKey: String,
    #[serde(default)]
    apiEndpoint: String,
    #[serde(default)]
    modelName: String,
    #[serde(default)]
    apiProviderType: String,
    #[serde(default)]
    apiProviderTypeId: String,
    #[serde(default)]
    useMultipleApiKeys: bool,
    #[serde(default)]
    apiKeyPool: Vec<Operit1ApiKeyInfo>,
    #[serde(default)]
    currentKeyIndex: i32,
    #[serde(default = "defaultKeyRotationMode")]
    keyRotationMode: String,
    #[serde(default)]
    hasCustomParameters: bool,
    #[serde(default)]
    maxTokensEnabled: bool,
    #[serde(default)]
    temperatureEnabled: bool,
    #[serde(default)]
    topPEnabled: bool,
    #[serde(default)]
    topKEnabled: bool,
    #[serde(default)]
    presencePenaltyEnabled: bool,
    #[serde(default)]
    frequencyPenaltyEnabled: bool,
    #[serde(default)]
    repetitionPenaltyEnabled: bool,
    #[serde(default = "defaultMaxTokens")]
    maxTokens: i32,
    #[serde(default = "defaultTemperature")]
    temperature: f32,
    #[serde(default = "defaultTopP")]
    topP: f32,
    #[serde(default)]
    topK: i32,
    #[serde(default)]
    presencePenalty: f32,
    #[serde(default)]
    frequencyPenalty: f32,
    #[serde(default = "defaultRepetitionPenalty")]
    repetitionPenalty: f32,
    #[serde(default = "defaultCustomParameters")]
    customParameters: String,
    #[serde(default)]
    thinkingConfigurations: String,
    #[serde(default)]
    thinkingOptionId: String,
    #[serde(default = "defaultCustomHeaders")]
    customHeaders: String,
    #[serde(default = "defaultMaxContextLength")]
    maxContextLength: f32,
    #[serde(default)]
    enableMaxContextMode: bool,
    #[serde(default = "defaultSummaryTokenThreshold")]
    summaryTokenThreshold: f32,
    #[serde(default = "defaultEnableSummary")]
    enableSummary: bool,
    #[serde(default = "defaultEnableSummaryByMessageCount")]
    enableSummaryByMessageCount: bool,
    #[serde(default = "defaultSummaryMessageCountThreshold")]
    summaryMessageCountThreshold: i32,
    #[serde(default)]
    mnnForwardType: i32,
    #[serde(default = "defaultThreadCount")]
    mnnThreadCount: i32,
    #[serde(default = "defaultThreadCount")]
    llamaThreadCount: i32,
    #[serde(default = "defaultLlamaContextSize")]
    llamaContextSize: i32,
    #[serde(default = "defaultLlamaBatchSize")]
    llamaBatchSize: i32,
    #[serde(default = "defaultLlamaBatchSize")]
    llamaUBatchSize: i32,
    #[serde(default)]
    llamaGpuLayers: i32,
    #[serde(default)]
    llamaUseMmap: bool,
    #[serde(default)]
    llamaFlashAttention: bool,
    #[serde(default = "defaultLlamaKvUnified")]
    llamaKvUnified: bool,
    #[serde(default)]
    llamaOffloadKqv: bool,
    #[serde(default)]
    enableDirectImageProcessing: bool,
    #[serde(default)]
    enableDirectAudioProcessing: bool,
    #[serde(default)]
    enableDirectVideoProcessing: bool,
    #[serde(default)]
    enableToolCall: bool,
    #[serde(default)]
    requestLimitPerMinute: i32,
    #[serde(default)]
    maxConcurrentRequests: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1ApiKeyInfo {
    id: String,
    key: String,
    #[serde(default)]
    name: String,
    #[serde(default = "defaultTrue")]
    isEnabled: bool,
    #[serde(default = "defaultApiKeyAvailabilityStatus")]
    availabilityStatus: ApiKeyAvailabilityStatus,
    #[serde(default)]
    usageCount: i64,
    #[serde(default)]
    lastUsed: i64,
    #[serde(default)]
    errorCount: i64,
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
struct Operit1ProviderBinding {
    providerType: ApiProviderType,
    providerTypeId: String,
    displayName: String,
}

impl Operit1ApiKeyInfo {
    #[allow(non_snake_case)]
    /// Converts the legacy API key record into the current key record.
    fn toApiKeyInfo(&self) -> ApiKeyInfo {
        ApiKeyInfo {
            id: self.id.clone(),
            key: self.key.clone(),
            name: self.name.clone(),
            isEnabled: self.isEnabled,
            availabilityStatus: self.availabilityStatus.clone(),
            usageCount: self.usageCount,
            lastUsed: self.lastUsed,
            errorCount: self.errorCount,
        }
    }
}

impl Operit1ModelConfig {
    #[allow(non_snake_case)]
    /// Resolves the provider protocol and persisted type id from one legacy model config.
    fn providerBinding(&self) -> Result<Operit1ProviderBinding, String> {
        let providerType = self.providerType()?;
        let providerTypeId = self.providerTypeIdForImport(&providerType);
        let displayName = if providerTypeId.eq_ignore_ascii_case(providerType.name()) {
            ModelCatalog::provider(providerType.name())?.displayName
        } else {
            self.name.clone()
        };
        Ok(Operit1ProviderBinding {
            providerType,
            providerTypeId,
            displayName,
        })
    }

    #[allow(non_snake_case)]
    /// Resolves the built-in protocol enum used by the imported provider.
    fn providerType(&self) -> Result<ApiProviderType, String> {
        let providerTypeId = self.apiProviderTypeId.trim();
        if let Some(providerType) = self.knownProviderType(providerTypeId) {
            return Ok(providerType);
        }

        let providerType = self.apiProviderType.trim();
        if !providerType.is_empty() {
            return self.knownProviderType(providerType).ok_or_else(|| {
                format!("无法识别 Operit1 供应商类型：{}", providerType)
            });
        }

        if providerTypeId.is_empty() {
            return Ok(ApiProviderType::DEEPSEEK);
        }

        Err(format!("无法识别 Operit1 供应商类型：{}", providerTypeId))
    }

    #[allow(non_snake_case)]
    /// Converts a known legacy provider id into its current provider protocol enum.
    fn knownProviderType(&self, providerTypeId: &str) -> Option<ApiProviderType> {
        if providerTypeId.trim().is_empty() {
            return None;
        }
        match providerTypeId.to_ascii_uppercase().as_str() {
            "MNN" | "LLAMA_CPP" => {
                AppLogger::i(
                    "Operit1SnapshotImport",
                    &format!(
                        "legacy local provider mapped to LOCAL_MODEL configId={} providerType={providerTypeId}",
                        self.id
                    ),
                );
                Some(ApiProviderType::LOCAL_MODEL)
            }
            _ => ApiProviderType::fromProviderTypeId(providerTypeId),
        }
    }

    #[allow(non_snake_case)]
    /// Computes the provider type id that should be written to the current profile.
    fn providerTypeIdForImport(&self, providerType: &ApiProviderType) -> String {
        let providerTypeId = self.apiProviderTypeId.trim();
        if providerTypeId.is_empty() {
            return providerType.name().to_string();
        }
        if self.knownProviderType(providerTypeId).is_some() {
            return providerType.name().to_string();
        }
        providerTypeId.to_string()
    }
}

#[allow(non_snake_case)]
fn buildModelProfile(
    modelId: &str,
    config: &Operit1ModelConfig,
) -> Result<ModelProfile, String> {
    let mut model = ModelProfile::new(modelId.to_string());
    model.contextOverride = Some(ModelContextSpec {
        maxContextLength: config.maxContextLength,
        enableMaxContextMode: config.enableMaxContextMode,
    });
    model.capabilitiesOverride = Some(ModelCapabilities {
        directImage: config.enableDirectImageProcessing,
        directAudio: config.enableDirectAudioProcessing,
        directVideo: config.enableDirectVideoProcessing,
        toolCall: config.enableToolCall,
    });
    model.requestOverride = Some(ModelRequestSpec {
        supportsStructuredTools: config.enableToolCall,
    });
    model.summary = ModelSummarySettings {
        enableSummary: config.enableSummary,
        summaryTokenThreshold: config.summaryTokenThreshold,
        enableSummaryByMessageCount: config.enableSummaryByMessageCount,
        summaryMessageCountThreshold: config.summaryMessageCountThreshold,
    };
    model.localRuntime.mnnForwardType = config.mnnForwardType;
    model.localRuntime.mnnThreadCount = config.mnnThreadCount;
    model.localRuntime.llamaThreadCount = config.llamaThreadCount;
    model.localRuntime.llamaContextSize = config.llamaContextSize;
    model.localRuntime.llamaBatchSize = config.llamaBatchSize;
    model.localRuntime.llamaUBatchSize = config.llamaUBatchSize;
    model.localRuntime.llamaGpuLayers = config.llamaGpuLayers;
    model.localRuntime.llamaUseMmap = config.llamaUseMmap;
    model.localRuntime.llamaFlashAttention = config.llamaFlashAttention;
    model.localRuntime.llamaKvUnified = config.llamaKvUnified;
    model.localRuntime.llamaOffloadKqv = config.llamaOffloadKqv;
    model.parameters = buildModelParameters(config)?;
    Ok(model)
}

#[allow(non_snake_case)]
fn buildModelParameters(config: &Operit1ModelConfig) -> Result<Vec<ModelParameter<Value>>, String> {
    let mut parameters = Vec::new();
    pushStandardParameter(
        &mut parameters,
        "max_tokens",
        config.maxTokensEnabled,
        serde_json::json!(config.maxTokens),
    )?;
    pushStandardParameter(
        &mut parameters,
        "temperature",
        config.temperatureEnabled,
        serde_json::json!(config.temperature),
    )?;
    pushStandardParameter(
        &mut parameters,
        "top_p",
        config.topPEnabled,
        serde_json::json!(config.topP),
    )?;
    pushStandardParameter(
        &mut parameters,
        "top_k",
        config.topKEnabled,
        serde_json::json!(config.topK),
    )?;
    pushStandardParameter(
        &mut parameters,
        "presence_penalty",
        config.presencePenaltyEnabled,
        serde_json::json!(config.presencePenalty),
    )?;
    pushStandardParameter(
        &mut parameters,
        "frequency_penalty",
        config.frequencyPenaltyEnabled,
        serde_json::json!(config.frequencyPenalty),
    )?;
    pushStandardParameter(
        &mut parameters,
        "repetition_penalty",
        config.repetitionPenaltyEnabled,
        serde_json::json!(config.repetitionPenalty),
    )?;
    if config.hasCustomParameters && config.customParameters.trim() != "[]" {
        let customParameters: Vec<CustomParameterData> =
            serde_json::from_str(&config.customParameters)
                .map_err(|error| format!("Operit1 自定义模型参数格式不正确：{error}"))?;
        for parameter in customParameters {
            parameters.push(ModelParameter {
                id: parameter.id,
                name: parameter.name,
                apiName: parameter.apiName,
                description: parameter.description,
                defaultValue: parseCustomParameterValue(&parameter.defaultValue)?,
                currentValue: parseCustomParameterValue(&parameter.currentValue)?,
                isEnabled: parameter.isEnabled,
                valueType: parseParameterValueType(&parameter.valueType)?,
                minValue: parameter
                    .minValue
                    .map(|value| parseCustomParameterValue(&value))
                    .transpose()?,
                maxValue: parameter
                    .maxValue
                    .map(|value| parseCustomParameterValue(&value))
                    .transpose()?,
                category: parseParameterCategory(&parameter.category)?,
                isCustom: true,
            });
        }
    }
    Ok(parameters)
}

#[allow(non_snake_case)]
fn pushStandardParameter(
    parameters: &mut Vec<ModelParameter<Value>>,
    id: &str,
    enabled: bool,
    value: Value,
) -> Result<(), String> {
    let definition = StandardModelParameters::DEFINITIONS()
        .into_iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| format!("标准模型参数不存在：{id}"))?;
    parameters.push(ModelParameter {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        apiName: definition.apiName.to_string(),
        description: definition.description.to_string(),
        defaultValue: definition.defaultValue,
        currentValue: value,
        isEnabled: enabled,
        valueType: definition.valueType,
        minValue: definition.minValue,
        maxValue: definition.maxValue,
        category: definition.category,
        isCustom: false,
    });
    Ok(())
}

#[allow(non_snake_case)]
fn decodeChatMapping(value: Value) -> Result<Operit1FunctionConfigMapping, String> {
    if let Some(configId) = value.as_str() {
        return Ok(Operit1FunctionConfigMapping {
            configId: configId.to_string(),
            modelIndex: 0,
        });
    }
    serde_json::from_value(value).map_err(|error| format!("CHAT 模型映射格式不正确：{error}"))
}

#[allow(non_snake_case)]
fn splitModelIds(modelName: &str) -> Vec<String> {
    modelName
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

impl Operit1PreferenceValue {
    /// Converts one legacy preference value into its target text representation.
    fn toTargetPreferenceStringForKey(
        &self,
        key: &str,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<String, String> {
        match self {
            Self::Boolean(value) => Ok(value.to_string()),
            Self::Float(value) => Ok(value.to_string()),
            Self::Double(value) => Ok(value.to_string()),
            Self::Int(value) => Ok(value.to_string()),
            Self::String(value) => fileImportPlan.rewritePreferenceValue(key, value),
            Self::StringSet(value) => {
                serde_json::to_string(value).map_err(|error| error.to_string())
            }
            Self::Long(value) => Ok(value.to_string()),
        }
    }

    /// Rewrites one legacy preference key and its text representation for the target runtime.
    fn toTargetPreferenceEntry(
        &self,
        key: &str,
        fileImportPlan: &SnapshotFileImportPlan,
    ) -> Result<(String, String), String> {
        let targetKey = fileImportPlan.rewritePreferenceKey(key)?;
        let targetValue = self.toTargetPreferenceStringForKey(key, fileImportPlan)?;
        Ok((targetKey, targetValue))
    }
}

