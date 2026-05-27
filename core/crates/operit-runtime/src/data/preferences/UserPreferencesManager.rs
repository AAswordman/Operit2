use std::time::{SystemTime, UNIX_EPOCH};

use serde_json;

use crate::data::model::PreferenceProfile::PreferenceProfile;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::default_data_dir;

#[derive(Clone)]
pub struct UserPreferencesManager {
    dataStore: PreferencesDataStore,
}

#[derive(Clone)]
pub struct PreferencesManager {
    inner: UserPreferencesManager,
}

impl UserPreferencesManager {
    pub const DEFAULT_PROFILE_ID: &'static str = "default";

    pub fn getInstance() -> Self {
        Self {
            dataStore: PreferencesDataStore::new(
                default_data_dir().join("user_preferences.preferences.json"),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn initializeIfNeeded(
        &self,
        defaultProfileName: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        let profiles = self.profileListFlow().first()?;
        if profiles.is_empty()
            || !profiles
                .iter()
                .any(|profile| profile == Self::DEFAULT_PROFILE_ID)
        {
            self.createProfile(defaultProfileName.to_string(), true)?;
        }
        Ok(())
    }

    pub fn activeProfileIdFlow(&self) -> Flow<String> {
        self.dataStore.dataFlow().map(|preferences| {
            preferences
                .get(&stringPreferencesKey("active_profile_id"))
                .cloned()
                .unwrap_or_else(|| Self::DEFAULT_PROFILE_ID.to_string())
        })
    }

    pub fn profileListFlow(&self) -> Flow<Vec<String>> {
        self.dataStore.dataFlow().map(|preferences| {
            let mut profiles = preferences
                .get(&stringPreferencesKey("profile_list"))
                .and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
                .unwrap_or_default();
            if !profiles
                .iter()
                .any(|profile| profile == Self::DEFAULT_PROFILE_ID)
            {
                profiles.insert(0, Self::DEFAULT_PROFILE_ID.to_string());
            }
            profiles
        })
    }

    pub fn activeProfileId(&self) -> Result<String, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        Ok(preferences
            .get(&stringPreferencesKey("active_profile_id"))
            .cloned()
            .unwrap_or_else(|| Self::DEFAULT_PROFILE_ID.to_string()))
    }

    pub fn createProfile(
        &self,
        name: String,
        isDefault: bool,
    ) -> Result<PreferenceProfile, PreferencesDataStoreError> {
        let profileId = if isDefault {
            Self::DEFAULT_PROFILE_ID.to_string()
        } else {
            format!("profile_{}", currentTimeMillis())
        };
        let profile = PreferenceProfile::new(profileId.clone(), name);
        self.saveProfile(&profile)?;
        self.dataStore.edit(|preferences| {
            let mut list = preferences
                .get(&stringPreferencesKey("profile_list"))
                .and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
                .unwrap_or_default();
            if !list.iter().any(|id| id == &profileId) {
                list.push(profileId.clone());
            }
            if let Ok(encoded) = serde_json::to_string(&list) {
                preferences.set(&stringPreferencesKey("profile_list"), encoded);
            }
            if isDefault
                || preferences
                    .get(&stringPreferencesKey("active_profile_id"))
                    .is_none()
            {
                preferences.set(
                    &stringPreferencesKey("active_profile_id"),
                    profileId.clone(),
                );
            }
            preferences.set(&stringPreferencesKey("birth_date_locked"), true.to_string());
        })?;
        Ok(profile)
    }

    #[allow(non_snake_case)]
    pub fn getUserPreferencesFlow(&self, profileId: String) -> Flow<PreferenceProfile> {
        let store = self.clone();
        self.dataStore.dataFlow().mapResult(move |preferences| {
            let targetProfileId = if profileId.is_empty() {
                preferences
                    .get(&stringPreferencesKey("active_profile_id"))
                    .cloned()
                    .unwrap_or_else(|| Self::DEFAULT_PROFILE_ID.to_string())
            } else {
                profileId.clone()
            };
            store.getProfile(&targetProfileId)
        })
    }

    #[allow(non_snake_case)]
    pub fn setActiveProfile(&self, profileId: String) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey("active_profile_id"), profileId);
        })
    }

    #[allow(non_snake_case)]
    pub fn categoryLockStatusFlow(&self) -> Flow<std::collections::BTreeMap<String, bool>> {
        self.dataStore.dataFlow().map(|preferences| {
            [
                ("birthDate", "birth_date_locked"),
                ("gender", "gender_locked"),
                ("personality", "personality_locked"),
                ("identity", "identity_locked"),
                ("occupation", "occupation_locked"),
                ("aiStyle", "ai_style_locked"),
            ]
            .into_iter()
            .map(|(category, key)| {
                (
                    category.to_string(),
                    preferences
                        .get(&stringPreferencesKey(key))
                        .map(|value| value == "true")
                        .unwrap_or(false),
                )
            })
            .collect()
        })
    }

    #[allow(non_snake_case)]
    pub fn isCategoryLocked(&self, category: &str) -> Result<bool, PreferencesDataStoreError> {
        Ok(self
            .categoryLockStatusFlow()
            .first()?
            .get(category)
            .copied()
            .unwrap_or(false))
    }

    #[allow(non_snake_case)]
    pub fn setCategoryLocked(
        &self,
        category: &str,
        locked: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        let key = match category {
            "birthDate" => "birth_date_locked",
            "gender" => "gender_locked",
            "personality" => "personality_locked",
            "identity" => "identity_locked",
            "occupation" => "occupation_locked",
            "aiStyle" => "ai_style_locked",
            _ => {
                return Err(PreferencesDataStoreError::Message(format!(
                    "Unknown preference category: {category}"
                )))
            }
        };
        self.dataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey(key), locked.to_string());
        })
    }

    pub fn getProfile(
        &self,
        profileId: &str,
    ) -> Result<PreferenceProfile, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        let key = profileKey(profileId);
        match preferences.get(&key) {
            Some(value) => serde_json::from_str(value).map_err(PreferencesDataStoreError::from),
            None => Ok(PreferenceProfile::new(
                profileId.to_string(),
                if profileId == Self::DEFAULT_PROFILE_ID {
                    "Default".to_string()
                } else {
                    profileId.to_string()
                },
            )),
        }
    }

    pub fn saveProfile(
        &self,
        profile: &PreferenceProfile,
    ) -> Result<(), PreferencesDataStoreError> {
        let encoded = serde_json::to_string(profile)?;
        let key = profileKey(&profile.id);
        self.dataStore.edit(|preferences| {
            preferences.set(&key, encoded.clone());
        })
    }

    pub fn updateProfileCategory(
        &self,
        profileId: String,
        birthDate: Option<i64>,
        gender: Option<String>,
        personality: Option<String>,
        identity: Option<String>,
        occupation: Option<String>,
        aiStyle: Option<String>,
    ) -> Result<PreferenceProfile, PreferencesDataStoreError> {
        let mut profile = self.getProfile(&profileId)?;
        let birthDateLocked = self.isCategoryLocked("birthDate")?;
        let genderLocked = self.isCategoryLocked("gender")?;
        let personalityLocked = self.isCategoryLocked("personality")?;
        let identityLocked = self.isCategoryLocked("identity")?;
        let occupationLocked = self.isCategoryLocked("occupation")?;
        let aiStyleLocked = self.isCategoryLocked("aiStyle")?;
        if let Some(value) = birthDate {
            if !birthDateLocked {
                profile.birthDate = value;
            }
        }
        if let Some(value) = gender {
            if !genderLocked {
                profile.gender = value;
            }
        }
        if let Some(value) = personality {
            if !personalityLocked {
                profile.personality = value;
            }
        }
        if let Some(value) = identity {
            if !identityLocked {
                profile.identity = value;
            }
        }
        if let Some(value) = occupation {
            if !occupationLocked {
                profile.occupation = value;
            }
        }
        if let Some(value) = aiStyle {
            if !aiStyleLocked {
                profile.aiStyle = value;
            }
        }
        profile.isInitialized = true;
        self.saveProfile(&profile)?;
        Ok(profile)
    }
}

impl PreferencesManager {
    pub fn getInstance() -> Self {
        Self {
            inner: UserPreferencesManager::getInstance(),
        }
    }

    pub fn activeProfileIdFlow(&self) -> Flow<String> {
        self.inner.activeProfileIdFlow()
    }

    pub fn activeProfileId(&self) -> Result<String, PreferencesDataStoreError> {
        self.inner.activeProfileId()
    }

    #[allow(non_snake_case)]
    pub fn initializeIfNeeded(
        &self,
        defaultProfileName: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        self.inner.initializeIfNeeded(defaultProfileName)
    }

    pub fn updateProfileCategory(
        &self,
        profileId: String,
        birthDate: Option<i64>,
        gender: Option<String>,
        personality: Option<String>,
        identity: Option<String>,
        occupation: Option<String>,
        aiStyle: Option<String>,
    ) -> Result<PreferenceProfile, PreferencesDataStoreError> {
        self.inner.updateProfileCategory(
            profileId,
            birthDate,
            gender,
            personality,
            identity,
            occupation,
            aiStyle,
        )
    }
}

fn profileKey(profileId: &str) -> operit_store::PreferencesDataStore::PreferencesKey {
    stringPreferencesKey(&format!("profile_{profileId}"))
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}
