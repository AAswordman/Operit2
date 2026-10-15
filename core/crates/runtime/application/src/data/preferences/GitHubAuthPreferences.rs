use std::path::PathBuf;

use operit_host_api::RuntimeStorageHost;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, CoreNodeSecretStore, Preferences, PreferencesDataStoreError,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::OperitPaths;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubUser {
    pub id: String,
    pub login: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(rename = "avatar_url", alias = "avatarUrl", default)]
    pub avatarUrl: String,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(rename = "public_repos", alias = "publicRepos", default)]
    pub publicRepos: Option<i32>,
    #[serde(default)]
    pub followers: Option<i32>,
    #[serde(default)]
    pub following: Option<i32>,
}

pub struct GitHubAuthPreferences {
    dataStore: CoreNodeSecretStore,
}

impl GitHubAuthPreferences {
    const REQUIRED_AUTH_VERSION: i64 = 2;
    const GITHUB_SCOPE: &'static str = "notifications,public_repo,user:email,read:user";

    /// Returns the runtime directory that owns GitHub authentication preferences.
    pub fn data_dir() -> PathBuf {
        defaultRuntimeStorageHost()
            .runtimeRootDir()
            .expect("runtime storage host must provide a GitHub preference root")
    }

    /// Opens GitHub authentication preferences from the default runtime directory.
    pub fn getInstance() -> Self {
        Self {
            dataStore: CoreNodeSecretStore::newWithStorage(
                defaultRuntimeStorageHost(),
                OperitPaths::GITHUB_AUTH_PREFERENCES_PATH,
            )
            .withSchema(2, Self::migratePreferences),
        }
    }

    /// Opens the GitHub-owned encrypted preference store for explicit runtime import.
    pub(crate) fn store(storage: std::sync::Arc<dyn RuntimeStorageHost>) -> CoreNodeSecretStore {
        CoreNodeSecretStore::newWithStorage(storage, OperitPaths::GITHUB_AUTH_PREFERENCES_PATH)
            .withSchema(2, Self::migratePreferences)
    }

    /// Creates a GitHub authentication preference manager rooted at a directory.
    pub fn new(root_dir: PathBuf) -> Self {
        let path =
            OperitPaths::runtimePathFromRoot(&root_dir, OperitPaths::GITHUB_AUTH_PREFERENCES_PATH)
                .expect(
                    "GitHub authentication preferences path must use the runtime storage prefix",
                );
        Self {
            dataStore: CoreNodeSecretStore::new(path).withSchema(2, Self::migratePreferences),
        }
    }

    /// Migrates GitHub-owned typed session fields at version one.
    pub(crate) fn migratePreferences(
        version: u32,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        match version {
            0 => Ok(()),
            1 => {
                let key = stringPreferencesKey("is_logged_in");
                if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                    preferences.set(&key, serde_json::from_str::<bool>(&encoded)?);
                }
                for key in ["auth_version", "last_login_time", "token_expires_at"] {
                    let key = stringPreferencesKey(key);
                    if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                        preferences.set(&key, serde_json::from_str::<i64>(&encoded)?);
                    }
                }
                let key = stringPreferencesKey("user_info");
                if let Some(encoded) = preferences.getTyped::<String>(&key)? {
                    let user: GitHubUser = serde_json::from_str(&encoded)?;
                    preferences.set(&key, serde_json::to_value(user)?);
                }
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        }
    }

    #[allow(non_snake_case)]
    /// Saves a GitHub login session including token metadata and user profile data.
    pub fn saveAuthInfo(
        &self,
        accessToken: &str,
        tokenType: &str,
        userInfo: Option<&GitHubUser>,
        grantedScope: Option<&str>,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.replace(buildAuthPreferences(
            accessToken,
            tokenType,
            userInfo,
            grantedScope,
        )?)
    }

    #[allow(non_snake_case)]
    /// Updates the saved GitHub access token without replacing the saved user profile.
    pub fn updateAccessToken(
        &self,
        accessToken: &str,
        tokenType: &str,
        grantedScope: Option<&str>,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.replace(buildAccessTokenPreferences(
            accessToken,
            tokenType,
            grantedScope,
        ))
    }

    #[allow(non_snake_case)]
    /// Returns the saved GitHub access token when the stored session is current.
    pub fn getCurrentAccessToken(&self) -> Result<Option<String>, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        if !self.isAuthSessionCurrent(&preferences)? {
            return Ok(None);
        }
        preferences.getTyped::<String>(&stringPreferencesKey("access_token"))
    }

    #[allow(non_snake_case)]
    /// Returns the saved GitHub profile while propagating malformed session data.
    pub fn getCurrentUserInfo(&self) -> Result<Option<GitHubUser>, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        if !self.isAuthSessionCurrent(&preferences)? {
            return Ok(None);
        }
        preferences.getTyped::<GitHubUser>(&stringPreferencesKey("user_info"))
    }

    #[allow(non_snake_case)]
    /// Reports whether the saved GitHub session is usable without concealing storage or type errors.
    pub fn isLoggedIn(&self) -> Result<bool, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        Ok(matches!(
            preferences.getTyped::<bool>(&stringPreferencesKey("is_logged_in"))?,
            Some(true)
        ) && self.isAuthSessionCurrent(&preferences)?)
    }

    /// Clears the saved GitHub authentication session.
    pub fn logout(&self) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            for key in [
                "is_logged_in",
                "access_token",
                "token_type",
                "token_expires_at",
                "refresh_token",
                "user_info",
                "last_login_time",
                "auth_version",
                "granted_scope",
                "pending_oauth_state",
            ] {
                preferences.remove(&stringPreferencesKey(key));
            }
        })
    }

    #[allow(non_snake_case)]
    /// Builds the HTTP Authorization header for the current GitHub token.
    pub fn getAuthorizationHeader(&self) -> Result<Option<String>, PreferencesDataStoreError> {
        Ok(self
            .getCurrentAccessToken()?
            .filter(|token| !token.trim().is_empty())
            .map(|token| format!("Bearer {token}")))
    }

    /// Validates the typed auth version and declared scopes for an existing session.
    #[allow(non_snake_case)]
    fn isAuthSessionCurrent(
        &self,
        preferences: &Preferences,
    ) -> Result<bool, PreferencesDataStoreError> {
        let Some(authVersion) =
            preferences.getTyped::<i64>(&stringPreferencesKey("auth_version"))?
        else {
            return Ok(false);
        };
        let Some(grantedScope) =
            preferences.getTyped::<String>(&stringPreferencesKey("granted_scope"))?
        else {
            return Ok(false);
        };
        let grantedScopes = parseScopeSet(&grantedScope);
        Ok(authVersion >= Self::REQUIRED_AUTH_VERSION
            && parseScopeSet(Self::GITHUB_SCOPE)
                .iter()
                .all(|scope| grantedScopes.iter().any(|item| item == scope)))
    }
}

/// Parses the declared comma-separated OAuth scopes.
#[allow(non_snake_case)]
fn parseScopeSet(scope: &str) -> Vec<String> {
    scope
        .split(',')
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}

/// Builds a complete native login record including its optional user profile.
fn buildAuthPreferences(
    accessToken: &str,
    tokenType: &str,
    userInfo: Option<&GitHubUser>,
    grantedScope: Option<&str>,
) -> Result<Preferences, PreferencesDataStoreError> {
    let mut preferences = buildAccessTokenPreferences(accessToken, tokenType, grantedScope);
    if let Some(userInfo) = userInfo {
        preferences.set(
            &stringPreferencesKey("user_info"),
            serde_json::to_value(userInfo)?,
        );
    }
    Ok(preferences)
}

/// Builds a complete native access-token record for one acknowledged login.
fn buildAccessTokenPreferences(
    accessToken: &str,
    tokenType: &str,
    grantedScope: Option<&str>,
) -> Preferences {
    let mut preferences = Preferences::default();
    preferences.set(&stringPreferencesKey("is_logged_in"), true);
    preferences.set(
        &stringPreferencesKey("access_token"),
        accessToken.to_string(),
    );
    preferences.set(&stringPreferencesKey("token_type"), tokenType.to_string());
    preferences.set(
        &stringPreferencesKey("last_login_time"),
        currentTimeMillis(),
    );
    preferences.set(
        &stringPreferencesKey("auth_version"),
        GitHubAuthPreferences::REQUIRED_AUTH_VERSION,
    );
    preferences.set(
        &stringPreferencesKey("granted_scope"),
        grantedScope
            .unwrap_or(GitHubAuthPreferences::GITHUB_SCOPE)
            .to_string(),
    );
    preferences
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a private encrypted storage owner without modifying process-wide host state.
    fn manager() -> GitHubAuthPreferences {
        let root = std::env::temp_dir().join(format!(
            "operit-github-preferences-{}",
            uuid::Uuid::new_v4()
        ));
        let host = std::sync::Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        GitHubAuthPreferences {
            dataStore: GitHubAuthPreferences::store(host),
        }
    }

    /// Saves and clears native session values through the owning schema.
    #[test]
    fn native_login_round_trip_retains_typed_fields() {
        let auth = manager();
        assert!(!auth.isLoggedIn().unwrap());
        let user = GitHubUser {
            id: "test-id".into(),
            login: "test-user".into(),
            ..Default::default()
        };
        auth.saveAuthInfo(
            "token",
            "bearer",
            Some(&user),
            Some(GitHubAuthPreferences::GITHUB_SCOPE),
        )
        .unwrap();
        assert!(auth.isLoggedIn().unwrap());
        assert_eq!(auth.getCurrentAccessToken().unwrap(), Some("token".into()));
        assert_eq!(auth.getCurrentUserInfo().unwrap(), Some(user));
        auth.logout().unwrap();
        assert!(!auth.isLoggedIn().unwrap());
        assert_eq!(auth.getCurrentAccessToken().unwrap(), None);
    }

    /// Propagates malformed native auth values instead of reporting an unauthenticated session.
    #[test]
    fn malformed_native_login_fields_are_reported() {
        let auth = manager();
        auth.updateAccessToken("token", "bearer", Some(GitHubAuthPreferences::GITHUB_SCOPE))
            .unwrap();
        auth.dataStore
            .edit(|p| {
                p.set(&stringPreferencesKey("access_token"), 17);
                p.set(&stringPreferencesKey("is_logged_in"), "true");
                p.set(&stringPreferencesKey("user_info"), true);
            })
            .unwrap();
        assert!(auth.isLoggedIn().is_err());
        assert!(auth.getCurrentAccessToken().is_err());
        assert!(auth.getCurrentUserInfo().is_err());
    }
}
