use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use uuid::Uuid;

use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_model::TtsCatalog::TtsCatalog;
use operit_model::TtsConfig::{
    AvailableTtsVoice, TtsConfig, TtsHttpHeader, TtsHttpResponsePipelineStep,
    TtsProviderCatalogEntry, TtsProviderType,
};
use operit_providers::voice::TtsVoiceListFetcher::TtsVoiceListFetcher;

const DEFAULT_SYSTEM_TTS_CONFIG_ID: &str = "system_tts_default";

#[derive(Clone)]
pub struct TtsConfigManager {
    paths: RuntimeStorePaths,
    dataStore: PreferencesDataStore,
}

impl TtsConfigManager {
    /// Creates a text-to-speech configuration manager backed by runtime store paths.
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::newEncryptedSynced(
                paths.tts_configs_preferences_path(),
            ),
            paths,
        }
    }

    #[allow(non_snake_case)]
    /// Creates a text-to-speech configuration manager using default runtime store paths.
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    #[allow(non_snake_case)]
    fn TTS_CONFIG_LIST() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("tts_config_list")
    }

    #[allow(non_snake_case)]
    fn CURRENT_TTS_CONFIG_ID() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("current_tts_config_id")
    }

    #[allow(non_snake_case)]
    /// Observes the ordered list of text-to-speech configuration identifiers.
    pub fn ttsConfigListFlow(&self) -> Flow<Vec<String>> {
        self.dataStore
            .dataFlow()
            .mapResult(|preferences| readConfigList(&preferences))
    }

    #[allow(non_snake_case)]
    /// Observes the identifier of the currently selected text-to-speech configuration.
    pub fn currentTtsConfigIdFlow(&self) -> Flow<String> {
        self.dataStore
            .dataFlow()
            .mapResult(|preferences| readCurrentTtsConfigId(&preferences))
    }

    #[allow(non_snake_case)]
    /// Reads the currently selected text-to-speech configuration identifier.
    pub fn getCurrentTtsConfigId(&self) -> Result<String, String> {
        self.bootstrapDefaultSystemTtsConfig()?;
        self.currentTtsConfigIdFlow()
            .first()
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    /// Reads the currently selected text-to-speech configuration.
    pub fn getCurrentTtsConfig(&self) -> Result<TtsConfig, String> {
        let id = self.getCurrentTtsConfigId()?;
        self.getTtsConfig(&id)
    }

    #[allow(non_snake_case)]
    /// Selects the active text-to-speech configuration by identifier.
    pub fn setCurrentTtsConfigId(&self, id: &str) -> Result<String, String> {
        self.bootstrapDefaultSystemTtsConfig()?;
        let id = id.trim().to_string();
        if id.is_empty() {
            return Err("current tts config id is empty".to_string());
        }
        self.getTtsConfig(&id)?;
        self.dataStore
            .edit(|preferences| {
                preferences.set(&TtsConfigManager::CURRENT_TTS_CONFIG_ID(), id.clone());
            })
            .map_err(|error| error.to_string())?;
        Ok(id)
    }

    #[allow(non_snake_case)]
    /// Reads every configured text-to-speech provider or voice profile.
    pub fn getAllTtsConfigs(&self) -> Result<Vec<TtsConfig>, String> {
        self.bootstrapDefaultSystemTtsConfig()?;
        let ids = self
            .ttsConfigListFlow()
            .first()
            .map_err(|error| error.to_string())?;
        let mut configs = Vec::new();
        for id in ids {
            configs.push(self.getTtsConfig(&id)?);
        }
        Ok(configs)
    }

    #[allow(non_snake_case)]
    /// Reads the built-in catalog of supported text-to-speech provider presets.
    pub fn getProviderCatalogEntries(&self) -> Result<Vec<TtsProviderCatalogEntry>, String> {
        TtsCatalog::providers()
    }

    #[allow(non_snake_case)]
    /// Reads one text-to-speech configuration by identifier.
    pub fn getTtsConfig(&self, id: &str) -> Result<TtsConfig, String> {
        self.bootstrapDefaultSystemTtsConfig()?;
        self.getTtsConfigFlow(id)
            .first()
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    /// Observes one text-to-speech configuration by identifier.
    pub fn getTtsConfigFlow(&self, id: &str) -> Flow<TtsConfig> {
        let id = id.trim().to_string();
        self.dataStore
            .dataFlow()
            .mapResult(move |preferences| readExistingTtsConfig(&preferences, &id))
    }

    #[allow(non_snake_case)]
    /// Creates a text-to-speech configuration and assigns store timestamps.
    pub fn createTtsConfig(&self, config: TtsConfig) -> Result<TtsConfig, String> {
        self.bootstrapDefaultSystemTtsConfig()?;
        let now = currentTimeMillis();
        let id = Uuid::new_v4().to_string();
        let config = normalizeConfig(TtsConfig {
            id: id.clone(),
            createdAt: now,
            updatedAt: now,
            ..config
        })?;
        self.dataStore
            .try_edit_result(|preferences| -> Result<(), PreferencesDataStoreError> {
                assertTtsConfigVoiceDoesNotExist(preferences, &config, None)?;
                let mut list = readConfigList(preferences)?;
                list.push(id.clone());
                list.sort();
                list.dedup();
                writeConfigList(preferences, &list);
                writeTtsConfig(preferences, &config);
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        Ok(config)
    }

    #[allow(non_snake_case)]
    /// Lists available voices reported by a provider configuration.
    pub fn getAvailableTtsVoices(
        &self,
        providerConfigId: &str,
    ) -> Result<Vec<AvailableTtsVoice>, String> {
        let providerConfig = self.getTtsConfig(providerConfigId)?;
        availableTtsVoicesForProvider(&providerConfig)
    }

    #[allow(non_snake_case)]
    /// Creates a voice configuration from one provider-reported voice entry.
    pub fn addTtsVoiceFromAvailable(
        &self,
        providerConfigId: &str,
        model: String,
        voice: String,
    ) -> Result<TtsConfig, String> {
        let providerConfig = self.getTtsConfig(providerConfigId)?;
        let voiceConfig = findAvailableTtsVoice(&providerConfig, &model, &voice)?;
        self.createTtsVoiceFromProvider(
            providerConfig,
            voiceConfig.model,
            voiceConfig.voice,
            voiceConfig.responseFormat,
            voiceConfig.speed,
        )
    }

    #[allow(non_snake_case)]
    /// Creates a voice configuration from custom model and voice values.
    pub fn createCustomTtsVoice(
        &self,
        providerConfigId: &str,
        model: String,
        voice: String,
    ) -> Result<TtsConfig, String> {
        let providerConfig = self.getTtsConfig(providerConfigId)?;
        let model = model.trim().to_string();
        let voice = voice.trim().to_string();
        if model.is_empty() && voice.is_empty() {
            return Err("tts custom voice model and voice are empty".to_string());
        }
        self.createTtsVoiceFromProvider(
            providerConfig.clone(),
            model,
            voice,
            providerConfig.responseFormat.clone(),
            providerConfig.speed,
        )
    }

    #[allow(non_snake_case)]
    fn createTtsVoiceFromProvider(
        &self,
        providerConfig: TtsConfig,
        model: String,
        voice: String,
        responseFormat: String,
        speed: f64,
    ) -> Result<TtsConfig, String> {
        self.createTtsConfig(TtsConfig {
            id: String::new(),
            name: providerConfig.name,
            providerType: providerConfig.providerType,
            endpoint: providerConfig.endpoint,
            apiKey: providerConfig.apiKey,
            model,
            voice,
            responseFormat,
            speed,
            httpMethod: providerConfig.httpMethod,
            requestBody: providerConfig.requestBody,
            contentType: providerConfig.contentType,
            headers: providerConfig.headers,
            responsePipeline: providerConfig.responsePipeline,
            createdAt: 0,
            updatedAt: 0,
        })
    }

    #[allow(non_snake_case)]
    /// Updates a text-to-speech configuration and preserves its creation timestamp.
    pub fn updateTtsConfig(&self, config: TtsConfig) -> Result<TtsConfig, String> {
        self.bootstrapDefaultSystemTtsConfig()?;
        let id = config.id.trim().to_string();
        if id.is_empty() {
            return Err("tts config id is empty".to_string());
        }
        let current = self.getTtsConfig(&id)?;
        let now = currentTimeMillis();
        let config = normalizeConfig(TtsConfig {
            id: id.clone(),
            updatedAt: now,
            ..config
        })?;
        self.dataStore
            .try_edit_result(|preferences| -> Result<(), PreferencesDataStoreError> {
                assertTtsConfigVoiceDoesNotExist(preferences, &config, Some(id.as_str()))?;
                writeTtsConfig(
                    preferences,
                    &TtsConfig {
                        createdAt: current.createdAt,
                        ..config.clone()
                    },
                );
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        self.getTtsConfig(&id)
    }

    #[allow(non_snake_case)]
    /// Deletes a text-to-speech configuration and reports whether it existed.
    pub fn deleteTtsConfig(&self, id: &str) -> Result<bool, String> {
        let id = id.trim().to_string();
        if id.is_empty() {
            return Err("tts config id is empty".to_string());
        }
        let currentConfigId = self
            .dataStore
            .dataFlow()
            .first()
            .map_err(|error| error.to_string())?
            .get(&TtsConfigManager::CURRENT_TTS_CONFIG_ID())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if currentConfigId.as_deref() == Some(id.as_str()) {
            return Err("current tts config cannot be deleted".to_string());
        }
        let cardManager = CharacterCardManager::new(self.paths.clone());
        let cards = cardManager
            .getAllCharacterCards()
            .map_err(|error| error.to_string())?;
        if let Some(card) = cards
            .iter()
            .find(|card| card.ttsConfigId.as_deref() == Some(id.as_str()))
        {
            return Err(format!("tts config is used by character card: {}", card.id));
        }
        let mut list = self
            .ttsConfigListFlow()
            .first()
            .map_err(|error| error.to_string())?;
        let originalLen = list.len();
        list.retain(|entry| entry != &id);
        let deleted = list.len() != originalLen;
        self.dataStore
            .edit(|preferences| {
                writeConfigList(preferences, &list);
                preferences.remove(&configJsonKey(&id));
            })
            .map_err(|error| error.to_string())?;
        Ok(deleted)
    }

    #[allow(non_snake_case)]
    fn bootstrapDefaultSystemTtsConfig(&self) -> Result<(), String> {
        let preferences = self
            .dataStore
            .dataFlow()
            .first()
            .map_err(|error| error.to_string())?;
        let currentConfigId = preferences
            .get(&TtsConfigManager::CURRENT_TTS_CONFIG_ID())
            .map(|value| value.trim().to_string());
        if currentConfigId
            .as_ref()
            .is_some_and(|value| !value.is_empty())
        {
            return Ok(());
        }
        let list = readConfigList(&preferences).map_err(|error| error.to_string())?;
        if !list.is_empty() {
            return Ok(());
        }
        let now = currentTimeMillis();
        let config = defaultSystemTtsConfig(now);
        self.dataStore
            .edit(|preferences| {
                writeConfigList(preferences, &[DEFAULT_SYSTEM_TTS_CONFIG_ID.to_string()]);
                writeTtsConfig(preferences, &config);
                preferences.set(
                    &TtsConfigManager::CURRENT_TTS_CONFIG_ID(),
                    DEFAULT_SYSTEM_TTS_CONFIG_ID.to_string(),
                );
            })
            .map_err(|error| error.to_string())
    }
}

fn readConfigList(preferences: &Preferences) -> Result<Vec<String>, PreferencesDataStoreError> {
    let Some(raw) = preferences.get(&TtsConfigManager::TTS_CONFIG_LIST()) else {
        return Ok(Vec::new());
    };
    serde_json::from_str::<Vec<String>>(raw).map_err(PreferencesDataStoreError::from)
}

fn writeConfigList(preferences: &mut Preferences, list: &[String]) {
    preferences.set(
        &TtsConfigManager::TTS_CONFIG_LIST(),
        serde_json::to_string(list).expect("tts config list must serialize"),
    );
}

fn readCurrentTtsConfigId(preferences: &Preferences) -> Result<String, PreferencesDataStoreError> {
    let raw = preferences
        .get(&TtsConfigManager::CURRENT_TTS_CONFIG_ID())
        .ok_or_else(|| {
            PreferencesDataStoreError::Message("current tts config id missing".to_string())
        })?;
    let id = raw.trim().to_string();
    if id.is_empty() {
        return Err(PreferencesDataStoreError::Message(
            "current tts config id is empty".to_string(),
        ));
    }
    let list = readConfigList(preferences)?;
    if !list.iter().any(|entry| entry == &id) {
        return Err(PreferencesDataStoreError::Message(format!(
            "current tts config not found: {id}"
        )));
    }
    Ok(id)
}

fn readExistingTtsConfig(
    preferences: &Preferences,
    id: &str,
) -> Result<TtsConfig, PreferencesDataStoreError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(PreferencesDataStoreError::Message(
            "tts config id is empty".to_string(),
        ));
    }
    let list = readConfigList(preferences)?;
    if !list.iter().any(|entry| entry == id) {
        return Err(PreferencesDataStoreError::Message(format!(
            "tts config not found: {id}"
        )));
    }
    let raw = preferences.get(&configJsonKey(id)).ok_or_else(|| {
        PreferencesDataStoreError::Message(format!("tts config payload missing: {id}"))
    })?;
    let config = serde_json::from_str::<TtsConfig>(raw).map_err(PreferencesDataStoreError::from)?;
    normalizeConfig(config).map_err(PreferencesDataStoreError::Message)
}

fn assertTtsConfigVoiceDoesNotExist(
    preferences: &Preferences,
    config: &TtsConfig,
    currentId: Option<&str>,
) -> Result<(), PreferencesDataStoreError> {
    for id in readConfigList(preferences)? {
        if currentId.is_some_and(|currentId| currentId == id) {
            continue;
        }
        let existing = readExistingTtsConfig(preferences, &id)?;
        if existing.providerType == config.providerType
            && existing.endpoint == config.endpoint
            && existing.model == config.model
            && existing.voice == config.voice
        {
            return Err(PreferencesDataStoreError::Message(format!(
                "tts voice already exists: providerType={} endpoint={} model={} voice={}",
                config.providerType, config.endpoint, config.model, config.voice
            )));
        }
    }
    Ok(())
}

fn writeTtsConfig(preferences: &mut Preferences, config: &TtsConfig) {
    preferences.set(
        &configJsonKey(&config.id),
        serde_json::to_string(config).expect("tts config must serialize"),
    );
}

#[allow(non_snake_case)]
fn availableTtsVoicesForProvider(
    providerConfig: &TtsConfig,
) -> Result<Vec<AvailableTtsVoice>, String> {
    let mut voices = TtsCatalog::voicesForProvider(
        &providerConfig.providerType,
        &providerConfig.model,
        &providerConfig.responseFormat,
        providerConfig.speed,
    )?;
    let remoteVoices = TtsVoiceListFetcher::fetch(providerConfig)?;
    for remoteVoice in remoteVoices {
        if voices
            .iter()
            .any(|voice| voice.model == remoteVoice.model && voice.voice == remoteVoice.voice)
        {
            continue;
        }
        voices.push(remoteVoice);
    }
    Ok(voices)
}

#[allow(non_snake_case)]
fn findAvailableTtsVoice(
    providerConfig: &TtsConfig,
    model: &str,
    voice: &str,
) -> Result<AvailableTtsVoice, String> {
    let model = model.trim();
    let voice = voice.trim();
    availableTtsVoicesForProvider(providerConfig)?
        .into_iter()
        .find(|entry| entry.model == model && entry.voice == voice)
        .ok_or_else(|| format!("available tts voice not found: {model}/{voice}"))
}

#[allow(non_snake_case)]
fn defaultSystemTtsConfig(now: i64) -> TtsConfig {
    TtsConfig {
        id: DEFAULT_SYSTEM_TTS_CONFIG_ID.to_string(),
        name: "系统 TTS".to_string(),
        providerType: TtsProviderType::SYSTEM_TTS.to_string(),
        endpoint: String::new(),
        apiKey: String::new(),
        model: String::new(),
        voice: String::new(),
        responseFormat: "wav".to_string(),
        speed: 1.0,
        httpMethod: "POST".to_string(),
        requestBody: String::new(),
        contentType: "application/json".to_string(),
        headers: Vec::new(),
        responsePipeline: Vec::new(),
        createdAt: now,
        updatedAt: now,
    }
}

fn configJsonKey(id: &str) -> operit_store::PreferencesDataStore::PreferencesKey {
    stringPreferencesKey(&format!("tts_config_{id}_json"))
}

#[allow(non_snake_case)]
fn normalizeConfig(config: TtsConfig) -> Result<TtsConfig, String> {
    let name = config.name.trim().to_string();
    if name.is_empty() {
        return Err("tts config name is empty".to_string());
    }
    let providerType = TtsProviderType::normalize(&config.providerType);
    let providerCatalog = TtsCatalog::provider(&providerType)?;
    let endpoint = catalogText(&config.endpoint, &providerCatalog.defaultEndpoint);
    let model = catalogText(&config.model, &providerCatalog.defaultModel);
    if providerType != TtsProviderType::SYSTEM_TTS {
        if endpoint.is_empty() {
            return Err("tts config endpoint is empty".to_string());
        }
        validateHttpUrl(&endpoint)?;
    }
    let voice = config.voice.trim().to_string();
    let responseFormat = catalogText(
        &config.responseFormat,
        &providerCatalog.defaultResponseFormat,
    );
    let httpMethod = catalogText(&config.httpMethod, &providerCatalog.defaultHttpMethod);
    let httpMethod = normalizeHttpMethod(&httpMethod)?;
    let contentType = catalogText(&config.contentType, &providerCatalog.defaultContentType);
    let requestBody = catalogText(&config.requestBody, &providerCatalog.defaultRequestBody);
    let headers = catalogHeaders(config.headers, &providerCatalog);
    let responsePipeline = catalogPipeline(config.responsePipeline, &providerCatalog);
    if providerType != TtsProviderType::SYSTEM_TTS
        && templateUsesModel(&endpoint, &requestBody, &headers)
    {
        if model.is_empty() {
            return Err("tts config model is empty".to_string());
        }
    }
    if providerType != TtsProviderType::SYSTEM_TTS
        && templateUsesVoice(&endpoint, &requestBody, &headers)
    {
        if voice.is_empty() {
            return Err("tts config voice is empty".to_string());
        }
    }
    if providerType != TtsProviderType::SYSTEM_TTS
        && templateUsesApiKey(&endpoint, &requestBody, &headers)
    {
        if config.apiKey.trim().is_empty() {
            return Err("tts config api key is empty".to_string());
        }
    }
    if responseFormat.is_empty() {
        return Err("tts config response format is empty".to_string());
    }
    if config.speed <= 0.0 {
        return Err("tts config speed must be positive".to_string());
    }
    if providerType == TtsProviderType::HTTP_TTS {
        if httpMethod == "POST" && requestBody.is_empty() {
            return Err("http tts request body is empty".to_string());
        }
        let template = if httpMethod == "GET" {
            endpoint.as_str()
        } else {
            requestBody.as_str()
        };
        if !templateHasTextPlaceholder(template) {
            return Err("http tts text placeholder is missing".to_string());
        }
    }
    if providerType != TtsProviderType::SYSTEM_TTS
        && httpMethod == "POST"
        && !requestBody.is_empty()
        && !templateHasTextPlaceholder(&requestBody)
    {
        return Err("tts request body text placeholder is missing".to_string());
    }
    let headers = normalizeHeaders(headers)?;
    let responsePipeline = normalizePipeline(responsePipeline)?;
    Ok(TtsConfig {
        id: config.id.trim().to_string(),
        name,
        providerType,
        endpoint,
        apiKey: config.apiKey.trim().to_string(),
        model,
        voice,
        responseFormat,
        speed: config.speed,
        httpMethod,
        requestBody,
        contentType,
        headers,
        responsePipeline,
        createdAt: config.createdAt,
        updatedAt: config.updatedAt,
    })
}

#[allow(non_snake_case)]
fn catalogText(value: &str, catalogValue: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        catalogValue.trim().to_string()
    } else {
        value.to_string()
    }
}

#[allow(non_snake_case)]
fn catalogHeaders(
    headers: Vec<TtsHttpHeader>,
    providerCatalog: &TtsProviderCatalogEntry,
) -> Vec<TtsHttpHeader> {
    if headers.is_empty() {
        providerCatalog.defaultHeaders.clone()
    } else {
        headers
    }
}

#[allow(non_snake_case)]
fn catalogPipeline(
    responsePipeline: Vec<TtsHttpResponsePipelineStep>,
    providerCatalog: &TtsProviderCatalogEntry,
) -> Vec<TtsHttpResponsePipelineStep> {
    if responsePipeline.is_empty() {
        providerCatalog.defaultResponsePipeline.clone()
    } else {
        responsePipeline
    }
}

#[allow(non_snake_case)]
fn templateUsesModel(endpoint: &str, requestBody: &str, headers: &[TtsHttpHeader]) -> bool {
    templateHasPlaceholder(endpoint, "model")
        || templateHasPlaceholder(requestBody, "model")
        || headers
            .iter()
            .any(|header| templateHasPlaceholder(&header.value, "model"))
}

#[allow(non_snake_case)]
fn templateUsesVoice(endpoint: &str, requestBody: &str, headers: &[TtsHttpHeader]) -> bool {
    templateHasPlaceholder(endpoint, "voice")
        || templateHasPlaceholder(requestBody, "voice")
        || headers
            .iter()
            .any(|header| templateHasPlaceholder(&header.value, "voice"))
}

#[allow(non_snake_case)]
fn templateUsesApiKey(endpoint: &str, requestBody: &str, headers: &[TtsHttpHeader]) -> bool {
    templateHasPlaceholder(endpoint, "apiKey")
        || templateHasPlaceholder(requestBody, "apiKey")
        || headers
            .iter()
            .any(|header| templateHasPlaceholder(&header.value, "apiKey"))
}

#[allow(non_snake_case)]
fn normalizeHttpMethod(method: &str) -> Result<String, String> {
    let trimmed = method.trim();
    if trimmed.eq_ignore_ascii_case("GET") {
        Ok("GET".to_string())
    } else if trimmed.eq_ignore_ascii_case("POST") {
        Ok("POST".to_string())
    } else {
        Err(format!("unsupported http tts method: {trimmed}"))
    }
}

#[allow(non_snake_case)]
fn validateHttpUrl(endpoint: &str) -> Result<(), String> {
    let url =
        url::Url::parse(endpoint).map_err(|error| format!("invalid http tts url: {error}"))?;
    match url.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("unsupported http tts url scheme: {scheme}")),
    }
}

#[allow(non_snake_case)]
fn normalizeHeaders(headers: Vec<TtsHttpHeader>) -> Result<Vec<TtsHttpHeader>, String> {
    let mut result = Vec::new();
    for header in headers {
        let name = header.name.trim().to_string();
        let value = header.value.trim().to_string();
        if name.is_empty() {
            return Err("http tts header name is empty".to_string());
        }
        result.push(TtsHttpHeader { name, value });
    }
    Ok(result)
}

#[allow(non_snake_case)]
fn normalizePipeline(
    steps: Vec<TtsHttpResponsePipelineStep>,
) -> Result<Vec<TtsHttpResponsePipelineStep>, String> {
    let mut result = Vec::new();
    for step in steps {
        let stepType = normalizePipelineStepType(&step.stepType)?;
        let headers = normalizeHeaders(step.headers)?;
        result.push(TtsHttpResponsePipelineStep {
            stepType,
            path: step.path.trim().to_string(),
            headers,
        });
    }
    Ok(result)
}

#[allow(non_snake_case)]
fn normalizePipelineStepType(stepType: &str) -> Result<String, String> {
    let trimmed = stepType.trim();
    let normalized = trimmed.to_ascii_lowercase();
    match normalized.as_str() {
        "parse_json"
        | "pick"
        | "parse_json_string"
        | "http_get"
        | "http_request_from_object"
        | "base64_decode"
        | "hex_decode" => Ok(normalized),
        _ => Err(format!(
            "unsupported http tts response pipeline step: {trimmed}"
        )),
    }
}

#[allow(non_snake_case)]
fn templateHasPlaceholder(template: &str, name: &str) -> bool {
    let needle = format!("{{{name}}}");
    template.match_indices(&needle).next().is_some()
}

#[allow(non_snake_case)]
fn templateHasTextPlaceholder(template: &str) -> bool {
    templateHasPlaceholder(template, "text") || templateHasPlaceholder(template, "textXml")
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_millis() as i64
}
