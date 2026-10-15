use operit_host_api::RuntimeStorageHost;
use operit_providers::chat::llmprovider::CodexOAuth::CodexOAuthTokens;
use operit_store::PreferencesDataStore::{stringPreferencesKey, CoreNodeSecretStore, Preferences, PreferencesDataStoreError};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::OperitPaths;

/// Stores the ChatGPT Codex OAuth session for one Core node.
pub struct CodexAuthPreferences {
    dataStore: CoreNodeSecretStore,
}

impl CodexAuthPreferences {
    /// Opens Codex credentials from the default runtime directory.
    pub fn getInstance() -> Self {
        Self {
            dataStore: CoreNodeSecretStore::newWithStorage(
                defaultRuntimeStorageHost(),
                OperitPaths::CODEX_AUTH_PREFERENCES_PATH,
            ).withSchema(2, |version, preferences| match version {
                0 => Ok(()),
                1 => {
                    let key = stringPreferencesKey("tokens");
                    if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                        let tokens: CodexOAuthTokens = serde_json::from_str(&encoded)?;
                        preferences.set(&key, serde_json::to_value(tokens)?);
                    }
                    Ok(())
                }
                from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            }),
        }
    }

    /// Replaces the saved Codex OAuth session.
    #[operit_route_macros::operit_core_internal]
    pub fn save(&self, tokens: &CodexOAuthTokens) -> Result<(), String> {
        let encoded = serde_json::to_value(tokens).map_err(|error| error.to_string())?;
        self.dataStore
            .edit(|preferences| {
                preferences.set(&stringPreferencesKey("tokens"), encoded.clone());
            })
            .map_err(|error| error.to_string())
    }

    /// Returns the saved Codex OAuth session.
    #[operit_route_macros::operit_core_internal]
    pub fn load(&self) -> Result<Option<CodexOAuthTokens>, String> {
        let preferences = self
            .dataStore
            .dataFlow()
            .first()
            .map_err(|error| error.to_string())?;
        read_tokens(&preferences)
    }

    /// Removes the saved Codex OAuth session.
    pub fn clear(&self) -> Result<(), String> {
        self.dataStore
            .edit(|preferences| {
                preferences.remove(&stringPreferencesKey("tokens"));
            })
            .map_err(|error| error.to_string())
    }
}

fn read_tokens(preferences: &Preferences) -> Result<Option<CodexOAuthTokens>, String> {
    let Some(encoded) = preferences.get(&stringPreferencesKey("tokens")) else {
        return Ok(None);
    };
    serde_json::from_value(encoded.clone())
        .map(Some)
        .map_err(|error| error.to_string())
}
