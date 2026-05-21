use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use sha2::{Digest, Sha256};

pub struct SkillVisibilityPreferences {
    dataStore: PreferencesDataStore,
}

impl SkillVisibilityPreferences {
    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(
                paths
                    .root_dir()
                    .join("com.ai.assistance.operit.data.preferences.SkillVisibilityPreferences.preferences.json"),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn isSkillVisibleToAi(&self, skillName: &str) -> bool {
        if skillName.trim().is_empty() {
            return true;
        }
        match self.readVisibility(skillName) {
            Ok(value) => value,
            Err(_) => true,
        }
    }

    #[allow(non_snake_case)]
    pub fn setSkillVisibleToAi(
        &self,
        skillName: &str,
        visible: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        if skillName.trim().is_empty() {
            return Ok(());
        }
        self.dataStore.edit(|preferences| {
            preferences.set(&keyForSkillName(skillName), visible.to_string());
        })
    }

    #[allow(non_snake_case)]
    fn readVisibility(&self, skillName: &str) -> Result<bool, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        let newKey = keyForSkillName(skillName);
        if let Some(value) = preferences.get(&newKey) {
            return Ok(value == "true");
        }
        let legacyKey = legacyKeyForSkillName(skillName);
        if let Some(value) = preferences.get(&legacyKey) {
            let legacyValue = value == "true";
            self.dataStore.edit(|editable| {
                editable.remove(&legacyKey);
                editable.set(&newKey, legacyValue.to_string());
            })?;
            return Ok(legacyValue);
        }
        Ok(true)
    }
}

#[allow(non_snake_case)]
fn keyForSkillName(skillName: &str) -> operit_store::PreferencesDataStore::PreferencesKey {
    let normalized = skillName.trim();
    let digest = Sha256::digest(normalized.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    stringPreferencesKey(&format!("skill_visible_{}", &hex[..16]))
}

#[allow(non_snake_case)]
fn legacyKeyForSkillName(skillName: &str) -> operit_store::PreferencesDataStore::PreferencesKey {
    let safe = skillName
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    stringPreferencesKey(&format!("skill_visible_{safe}"))
}
