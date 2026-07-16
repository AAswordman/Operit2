#![allow(non_snake_case)]

use operit_host_api::TimeUtils::currentTimeMillis;
use operit_local_models::LocalEngineManifest::LocalPlatformTarget;
use operit_local_models::LocalModelManifest::LocalModelKind;
use operit_local_models::LocalModelRegistryStore::LocalModelRegistryStore;
use operit_model::SttCatalog::SttCatalog;
use operit_model::SttConfig::{
    AvailableSttModel, SttConfig, SttHttpHeader, SttProviderCatalogEntry, SttProviderType,
};
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use uuid::Uuid;

#[derive(Clone)]
/// Stores speech-to-text provider profiles and the current provider selection.
pub struct SttConfigManager {
    paths: RuntimeStorePaths,
    dataStore: PreferencesDataStore,
}

impl SttConfigManager {
    /// Creates an STT configuration manager backed by explicit runtime paths.
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::newEncryptedSynced(
                paths.stt_configs_preferences_path(),
            ),
            paths,
        }
    }

    /// Creates an STT configuration manager using default runtime paths.
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    /// Returns the preference key for ordered STT configuration ids.
    fn configListKey() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("stt_config_list")
    }

    /// Returns the preference key for the current STT configuration id.
    fn currentConfigIdKey() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("current_stt_config_id")
    }

    /// Observes the ordered list of STT configuration ids.
    pub fn sttConfigListFlow(&self) -> Flow<Vec<String>> {
        self.dataStore
            .dataFlow()
            .mapResult(|preferences| readConfigList(&preferences))
    }

    /// Reads every configured STT provider profile.
    pub fn getAllSttConfigs(&self) -> Result<Vec<SttConfig>, String> {
        let ids = self
            .sttConfigListFlow()
            .first()
            .map_err(|error| error.to_string())?;
        ids.into_iter().map(|id| self.getSttConfig(&id)).collect()
    }

    /// Reads one STT configuration by id.
    pub fn getSttConfig(&self, id: &str) -> Result<SttConfig, String> {
        let preferences = self.dataStore.data().map_err(|error| error.to_string())?;
        readExistingConfig(&preferences, id).map_err(|error| error.to_string())
    }

    /// Reads the current STT configuration id.
    pub fn getCurrentSttConfigId(&self) -> Result<String, String> {
        self.getSelectedSttConfigId()?
            .ok_or_else(|| "current STT config is not selected".to_string())
    }

    /// Reads the selected STT configuration id when a selection exists.
    pub fn getSelectedSttConfigId(&self) -> Result<Option<String>, String> {
        let preferences = self.dataStore.data().map_err(|error| error.to_string())?;
        let Some(id) = preferences.get(&Self::currentConfigIdKey()) else {
            return Ok(None);
        };
        if id.trim().is_empty() {
            return Err("selected STT config id is empty".to_string());
        }
        self.getSttConfig(id)?;
        Ok(Some(id.to_string()))
    }

    /// Reads the current STT configuration.
    pub fn getCurrentSttConfig(&self) -> Result<SttConfig, String> {
        let id = self.getCurrentSttConfigId()?;
        self.getSttConfig(&id)
    }

    /// Selects the current STT configuration by id.
    pub fn setCurrentSttConfigId(&self, id: &str) -> Result<String, String> {
        let id = id.trim();
        if id.is_empty() {
            return Err("current STT config id is empty".to_string());
        }
        self.getSttConfig(id)?;
        self.dataStore
            .edit(|preferences| {
                preferences.set(&Self::currentConfigIdKey(), id.to_string());
            })
            .map_err(|error| error.to_string())?;
        Ok(id.to_string())
    }

    /// Creates and persists one STT provider configuration.
    pub fn createSttConfig(&self, config: SttConfig) -> Result<SttConfig, String> {
        let now = currentTimeMillis();
        let config = normalizeConfig(SttConfig {
            id: Uuid::new_v4().to_string(),
            createdAt: now,
            updatedAt: now,
            ..config
        })?;
        self.validateLocalModelSelection(&config)?;
        self.dataStore
            .try_edit_result(|preferences| -> Result<(), PreferencesDataStoreError> {
                let mut ids = readConfigList(preferences)?;
                ids.push(config.id.clone());
                writeConfigList(preferences, &ids)?;
                writeConfig(preferences, &config)?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        Ok(config)
    }

    /// Updates one existing STT provider configuration.
    pub fn updateSttConfig(&self, config: SttConfig) -> Result<SttConfig, String> {
        let existing = self.getSttConfig(&config.id)?;
        let config = normalizeConfig(SttConfig {
            createdAt: existing.createdAt,
            updatedAt: currentTimeMillis(),
            ..config
        })?;
        self.validateLocalModelSelection(&config)?;
        self.dataStore
            .try_edit_result(|preferences| -> Result<(), PreferencesDataStoreError> {
                writeConfig(preferences, &config)
            })
            .map_err(|error| error.to_string())?;
        Ok(config)
    }

    /// Deletes one STT provider configuration.
    pub fn deleteSttConfig(&self, id: &str) -> Result<bool, String> {
        let id = id.trim();
        self.getSttConfig(id)?;
        self.dataStore
            .try_edit_result(|preferences| -> Result<(), PreferencesDataStoreError> {
                let mut ids = readConfigList(preferences)?;
                ids.retain(|current| current != id);
                writeConfigList(preferences, &ids)?;
                preferences.remove(&configKey(id));
                if preferences
                    .get(&Self::currentConfigIdKey())
                    .map(String::as_str)
                    == Some(id)
                {
                    preferences.remove(&Self::currentConfigIdKey());
                }
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        Ok(true)
    }

    /// Returns every built-in STT provider catalog entry.
    pub fn getProviderCatalogEntries(&self) -> Result<Vec<SttProviderCatalogEntry>, String> {
        SttCatalog::providers()
    }

    /// Returns models exposed by one STT provider type.
    pub fn getAvailableSttModels(
        &self,
        providerTypeId: String,
    ) -> Result<Vec<AvailableSttModel>, String> {
        let providerTypeId = SttProviderType::normalize(&providerTypeId);
        SttCatalog::provider(&providerTypeId)?;
        if providerTypeId == SttProviderType::LOCAL_MODEL {
            return self.getInstalledLocalSttModels();
        }
        SttCatalog::modelsForProvider(&providerTypeId)
    }

    /// Returns installed local speech-to-text models as provider model entries.
    fn getInstalledLocalSttModels(&self) -> Result<Vec<AvailableSttModel>, String> {
        let platform = LocalPlatformTarget::current()?.platform;
        let registry = LocalModelRegistryStore::forRuntimeStorage(
            operit_store::RuntimeStorageHost::defaultRuntimeStorageHost(),
        )
        .read()
        .map_err(|error| error.to_string())?;
        let mut models = registry
            .installedModels
            .into_iter()
            .filter(|model| {
                model.manifest.kind == LocalModelKind::SpeechToText
                    && model.manifest.supportsPlatform(&platform)
            })
            .map(|model| AvailableSttModel {
                model: model.manifest.registryKey(),
                displayName: model.manifest.displayName,
                description: model.manifest.description,
                languages: model.manifest.languages,
            })
            .collect::<Vec<_>>();
        models.sort_by(|left, right| left.model.cmp(&right.model));
        Ok(models)
    }

    /// Validates that one LOCAL_MODEL config references an installed STT model.
    fn validateLocalModelSelection(&self, config: &SttConfig) -> Result<(), String> {
        if config.providerType != SttProviderType::LOCAL_MODEL {
            return Ok(());
        }
        let models = self.getInstalledLocalSttModels()?;
        if models.iter().any(|model| model.model == config.model) {
            return Ok(());
        }
        Err(format!(
            "LOCAL_MODEL STT model is not installed or platform-compatible: {}",
            config.model
        ))
    }
}

/// Validates and normalizes one STT provider configuration.
fn normalizeConfig(config: SttConfig) -> Result<SttConfig, String> {
    let providerType = SttProviderType::normalize(&config.providerType);
    SttCatalog::provider(&providerType)?;
    let name = requiredText(&config.name, "STT config name")?;
    let model = requiredText(&config.model, "STT model")?;
    if providerType == SttProviderType::LOCAL_MODEL {
        if !config.endpoint.trim().is_empty() {
            return Err("LOCAL_MODEL STT endpoint must be empty".to_string());
        }
        if !config.apiKey.trim().is_empty() {
            return Err("LOCAL_MODEL STT api key must be empty".to_string());
        }
        let (modelId, version) = model
            .split_once('@')
            .ok_or_else(|| "LOCAL_MODEL STT model must use modelId@version".to_string())?;
        if modelId.is_empty() || version.is_empty() {
            return Err("LOCAL_MODEL STT model must use non-empty modelId@version".to_string());
        }
    } else {
        validateRemoteConfig(&config)?;
    }
    Ok(SttConfig {
        id: config.id.trim().to_string(),
        name,
        providerType,
        endpoint: config.endpoint.trim().to_string(),
        apiKey: config.apiKey.trim().to_string(),
        model,
        fileFieldName: config.fileFieldName.trim().to_string(),
        modelFieldName: config.modelFieldName.trim().to_string(),
        languageFieldName: config.languageFieldName.trim().to_string(),
        responseTextJsonPath: config.responseTextJsonPath.trim().to_string(),
        headers: normalizeHeaders(config.headers)?,
        createdAt: config.createdAt,
        updatedAt: config.updatedAt,
    })
}

/// Validates one remote multipart STT provider configuration.
fn validateRemoteConfig(config: &SttConfig) -> Result<(), String> {
    let endpoint = requiredText(&config.endpoint, "STT endpoint")?;
    let url =
        url::Url::parse(&endpoint).map_err(|error| format!("invalid STT endpoint: {error}"))?;
    match url.scheme() {
        "http" | "https" => {}
        scheme => return Err(format!("unsupported STT endpoint scheme: {scheme}")),
    }
    requiredText(&config.fileFieldName, "STT file field name")?;
    requiredText(&config.modelFieldName, "STT model field name")?;
    requiredText(&config.responseTextJsonPath, "STT response text JSON path")?;
    Ok(())
}

/// Normalizes and validates custom STT request headers.
fn normalizeHeaders(headers: Vec<SttHttpHeader>) -> Result<Vec<SttHttpHeader>, String> {
    headers
        .into_iter()
        .map(|header| {
            let name = requiredText(&header.name, "STT header name")?;
            Ok(SttHttpHeader {
                name,
                value: header.value.trim().to_string(),
            })
        })
        .collect()
}

/// Returns one required trimmed text value.
fn requiredText(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} is empty"));
    }
    Ok(value.to_string())
}

/// Builds the preference key for one STT configuration.
fn configKey(id: &str) -> operit_store::PreferencesDataStore::PreferencesKey {
    stringPreferencesKey(&format!("stt_config_{}_json", id.trim()))
}

/// Reads the ordered STT configuration id list.
fn readConfigList(preferences: &Preferences) -> Result<Vec<String>, PreferencesDataStoreError> {
    match preferences.get(&SttConfigManager::configListKey()) {
        Some(encoded) => serde_json::from_str(encoded).map_err(Into::into),
        None => Ok(Vec::new()),
    }
}

/// Writes the ordered STT configuration id list.
fn writeConfigList(
    preferences: &mut Preferences,
    ids: &[String],
) -> Result<(), PreferencesDataStoreError> {
    preferences.set(
        &SttConfigManager::configListKey(),
        serde_json::to_string(ids)?,
    );
    Ok(())
}

/// Reads one existing STT configuration from preferences.
fn readExistingConfig(
    preferences: &Preferences,
    id: &str,
) -> Result<SttConfig, PreferencesDataStoreError> {
    let id = id.trim();
    let encoded = preferences
        .get(&configKey(id))
        .ok_or_else(|| PreferencesDataStoreError::Message(format!("STT config not found: {id}")))?;
    serde_json::from_str(encoded).map_err(Into::into)
}

/// Writes one STT configuration to preferences.
fn writeConfig(
    preferences: &mut Preferences,
    config: &SttConfig,
) -> Result<(), PreferencesDataStoreError> {
    preferences.set(&configKey(&config.id), serde_json::to_string(config)?);
    Ok(())
}
