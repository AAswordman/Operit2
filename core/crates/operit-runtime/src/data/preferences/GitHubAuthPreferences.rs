use std::path::PathBuf;

use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
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
    dataStore: PreferencesDataStore,
}

impl GitHubAuthPreferences {
    const REQUIRED_AUTH_VERSION: i64 = 2;
    const GITHUB_SCOPE: &'static str = "notifications,public_repo,user:email,read:user";

    /// Returns the runtime directory that owns GitHub authentication preferences.
    pub fn data_dir() -> PathBuf {
        OperitPaths::operitRootDir().expect("Operit root dir must be available")
    }

    /// Opens GitHub authentication preferences from the default runtime directory.
    pub fn getInstance() -> Self {
        Self::new(Self::data_dir())
    }

    /// Creates a GitHub authentication preference manager rooted at a directory.
    pub fn new(root_dir: PathBuf) -> Self {
        let path = root_dir.join(OperitPaths::GITHUB_AUTH_PREFERENCES_PATH);
        Self {
            dataStore: PreferencesDataStore::newEncrypted(path),
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
    pub fn getCurrentAccessToken(&self) -> Option<String> {
        let preferences = self.dataStore.dataFlow().first().ok()?;
        if !self.isAuthSessionCurrent(&preferences) {
            return None;
        }
        preferences
            .get(&stringPreferencesKey("access_token"))
            .cloned()
    }

    #[allow(non_snake_case)]
    /// Returns the saved GitHub user profile when the stored session is current.
    pub fn getCurrentUserInfo(&self) -> Option<GitHubUser> {
        let preferences = self.dataStore.dataFlow().first().ok()?;
        if !self.isAuthSessionCurrent(&preferences) {
            return None;
        }
        preferences
            .get(&stringPreferencesKey("user_info"))
            .and_then(|value| serde_json::from_str::<GitHubUser>(value).ok())
    }

    #[allow(non_snake_case)]
    /// Reports whether the saved GitHub authentication session is usable.
    pub fn isLoggedIn(&self) -> bool {
        self.dataStore
            .dataFlow()
            .first()
            .map(|preferences| {
                preferences
                    .get(&stringPreferencesKey("is_logged_in"))
                    .and_then(|value| value.parse::<bool>().ok())
                    .unwrap_or(false)
                    && self.isAuthSessionCurrent(&preferences)
            })
            .unwrap_or(false)
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
    pub fn getAuthorizationHeader(&self) -> Option<String> {
        self.getCurrentAccessToken()
            .filter(|token| !token.trim().is_empty())
            .map(|token| format!("Bearer {token}"))
    }

    #[allow(non_snake_case)]
    fn isAuthSessionCurrent(
        &self,
        preferences: &operit_store::PreferencesDataStore::Preferences,
    ) -> bool {
        let authVersion = preferences
            .get(&stringPreferencesKey("auth_version"))
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(0);
        let grantedScopes = preferences
            .get(&stringPreferencesKey("granted_scope"))
            .map(|value| parseScopeSet(value))
            .unwrap_or_default();
        let requiredScopes = parseScopeSet(Self::GITHUB_SCOPE);
        authVersion >= Self::REQUIRED_AUTH_VERSION
            && requiredScopes
                .iter()
                .all(|scope| grantedScopes.iter().any(|item| item == scope))
    }
}

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
            serde_json::to_string(userInfo)?,
        );
    }
    Ok(preferences)
}

fn buildAccessTokenPreferences(
    accessToken: &str,
    tokenType: &str,
    grantedScope: Option<&str>,
) -> Preferences {
    let mut preferences = Preferences::default();
    preferences.set(&stringPreferencesKey("is_logged_in"), true.to_string());
    preferences.set(
        &stringPreferencesKey("access_token"),
        accessToken.to_string(),
    );
    preferences.set(&stringPreferencesKey("token_type"), tokenType.to_string());
    preferences.set(
        &stringPreferencesKey("last_login_time"),
        currentTimeMillis().to_string(),
    );
    preferences.set(
        &stringPreferencesKey("auth_version"),
        GitHubAuthPreferences::REQUIRED_AUTH_VERSION.to_string(),
    );
    preferences.set(
        &stringPreferencesKey("granted_scope"),
        grantedScope
            .unwrap_or(GitHubAuthPreferences::GITHUB_SCOPE)
            .to_string(),
    );
    preferences
}
