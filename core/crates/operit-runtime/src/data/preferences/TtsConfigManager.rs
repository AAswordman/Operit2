use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use uuid::Uuid;

use crate::data::model::TtsConfig::{TtsConfig, TtsProviderType};
use crate::data::preferences::CharacterCardManager::CharacterCardManager;

const DEFAULT_SYSTEM_TTS_CONFIG_ID: &str = "system_tts_default";

#[derive(Clone)]
pub struct TtsConfigManager {
    paths: RuntimeStorePaths,
    dataStore: PreferencesDataStore,
}

impl TtsConfigManager {
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(paths.tts_configs_preferences_path()),
            paths,
        }
    }

    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    #[allow(non_snake_case)]
    fn TTS_CONFIG_LIST() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("tts_config_list")
    }

    #[allow(non_snake_case)]
    pub fn ttsConfigListFlow(&self) -> Flow<Vec<String>> {
        self.dataStore
            .dataFlow()
            .mapResult(|preferences| readConfigList(&preferences))
    }

    #[allow(non_snake_case)]
    pub fn getAllTtsConfigs(&self) -> Result<Vec<TtsConfig>, String> {
        self.ensureDefaultSystemTtsConfig()?;
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
    pub fn getTtsConfig(&self, id: &str) -> Result<TtsConfig, String> {
        self.ensureDefaultSystemTtsConfig()?;
        self.getTtsConfigFlow(id)
            .first()
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    pub fn getTtsConfigFlow(&self, id: &str) -> Flow<TtsConfig> {
        let id = id.trim().to_string();
        self.dataStore.dataFlow().mapResult(move |preferences| {
            readExistingTtsConfig(&preferences, &id)
        })
    }

    #[allow(non_snake_case)]
    pub fn createTtsConfig(&self, config: TtsConfig) -> Result<TtsConfig, String> {
        self.ensureDefaultSystemTtsConfig()?;
        let now = currentTimeMillis();
        let id = Uuid::new_v4().to_string();
        let config = normalizeConfig(TtsConfig {
            id: id.clone(),
            createdAt: now,
            updatedAt: now,
            ..config
        })?;
        let mut list = self
            .ttsConfigListFlow()
            .first()
            .map_err(|error| error.to_string())?;
        list.push(id.clone());
        list.sort();
        list.dedup();
        self.dataStore
            .edit(|preferences| {
                writeConfigList(preferences, &list);
                writeTtsConfig(preferences, &config);
            })
            .map_err(|error| error.to_string())?;
        Ok(config)
    }

    #[allow(non_snake_case)]
    pub fn updateTtsConfig(&self, config: TtsConfig) -> Result<TtsConfig, String> {
        self.ensureDefaultSystemTtsConfig()?;
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
            .edit(|preferences| {
                writeTtsConfig(
                    preferences,
                    &TtsConfig {
                        createdAt: current.createdAt,
                        ..config.clone()
                    },
                );
            })
            .map_err(|error| error.to_string())?;
        self.getTtsConfig(&id)
    }

    #[allow(non_snake_case)]
    pub fn deleteTtsConfig(&self, id: &str) -> Result<bool, String> {
        self.ensureDefaultSystemTtsConfig()?;
        let id = id.trim().to_string();
        if id.is_empty() {
            return Err("tts config id is empty".to_string());
        }
        if id == DEFAULT_SYSTEM_TTS_CONFIG_ID {
            return Err("default system tts config cannot be deleted".to_string());
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
    fn ensureDefaultSystemTtsConfig(&self) -> Result<(), String> {
        let mut list = self
            .ttsConfigListFlow()
            .first()
            .map_err(|error| error.to_string())?;
        if list.iter().any(|entry| entry == DEFAULT_SYSTEM_TTS_CONFIG_ID) {
            return Ok(());
        }
        let now = currentTimeMillis();
        let config = defaultSystemTtsConfig(now);
        list.push(DEFAULT_SYSTEM_TTS_CONFIG_ID.to_string());
        list.sort();
        list.dedup();
        self.dataStore
            .edit(|preferences| {
                writeConfigList(preferences, &list);
                writeTtsConfig(preferences, &config);
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
    let raw = preferences
        .get(&configJsonKey(id))
        .ok_or_else(|| PreferencesDataStoreError::Message(format!("tts config payload missing: {id}")))?;
    serde_json::from_str::<TtsConfig>(raw).map_err(PreferencesDataStoreError::from)
}

fn writeTtsConfig(preferences: &mut Preferences, config: &TtsConfig) {
    preferences.set(
        &configJsonKey(&config.id),
        serde_json::to_string(config).expect("tts config must serialize"),
    );
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
        enabled: true,
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
    if providerType != TtsProviderType::SYSTEM_TTS
        && providerType != TtsProviderType::OPENAI_COMPATIBLE
    {
        return Err(format!("unsupported tts provider type: {providerType}"));
    }
    let endpoint = config.endpoint.trim().to_string();
    let model = config.model.trim().to_string();
    if providerType == TtsProviderType::OPENAI_COMPATIBLE {
        if endpoint.is_empty() {
            return Err("tts config endpoint is empty".to_string());
        }
        if model.is_empty() {
            return Err("tts config model is empty".to_string());
        }
    }
    let voice = config.voice.trim().to_string();
    let responseFormat = config.responseFormat.trim().to_string();
    if providerType == TtsProviderType::OPENAI_COMPATIBLE && voice.is_empty() {
        return Err("tts config voice is empty".to_string());
    }
    if responseFormat.is_empty() {
        return Err("tts config response format is empty".to_string());
    }
    if config.speed <= 0.0 {
        return Err("tts config speed must be positive".to_string());
    }
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
        enabled: config.enabled,
        createdAt: config.createdAt,
        updatedAt: config.updatedAt,
    })
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_millis() as i64
}
