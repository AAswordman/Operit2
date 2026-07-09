use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use uuid::Uuid;

use operit_model::PromptTag::{PromptTag, TagType};

#[derive(Clone)]
pub struct PromptTagManager {
    dataStore: PreferencesDataStore,
}

impl PromptTagManager {
    #[allow(non_snake_case)]
    /// Opens the prompt tag manager with default runtime store paths.
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    /// Creates a prompt tag manager backed by runtime store paths.
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(paths.prompt_tags_preferences_path()),
        }
    }

    #[allow(non_snake_case)]
    fn PROMPT_TAG_LIST() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("prompt_tag_list")
    }

    #[allow(non_snake_case)]
    /// Observes the ordered list of prompt tag identifiers.
    pub fn tagListFlow(&self) -> Flow<Vec<String>> {
        let dataStore = self.dataStore.clone();
        dataStore
            .dataFlow()
            .map(|preferences| Self::readTagList(&preferences))
    }

    #[allow(non_snake_case)]
    /// Observes all prompt tags sorted by most recent update time.
    pub fn allTagsFlow(&self) -> Flow<Vec<PromptTag>> {
        let manager = self.clone();
        self.dataStore.dataFlow().map(move |preferences| {
            let mut tags = Self::readTagList(&preferences)
                .into_iter()
                .map(|id| manager.getPromptTagFromPreferences(&preferences, &id))
                .collect::<Vec<_>>();
            tags.sort_by(|left, right| right.updatedAt.cmp(&left.updatedAt));
            tags
        })
    }

    #[allow(non_snake_case)]
    /// Observes one prompt tag by identifier.
    pub fn getPromptTagFlow(&self, id: &str) -> Flow<PromptTag> {
        let manager = self.clone();
        let id = id.to_string();
        self.dataStore
            .dataFlow()
            .map(move |preferences| manager.getPromptTagFromPreferences(&preferences, &id))
    }

    #[allow(non_snake_case)]
    fn getPromptTagFromPreferences(&self, preferences: &Preferences, id: &str) -> PromptTag {
        let tagType = preferences
            .get(&stringPreferencesKey(&format!("prompt_tag_{id}_tag_type")))
            .and_then(|value| parseTagType(value))
            .unwrap_or(TagType::CUSTOM);
        PromptTag {
            id: id.to_string(),
            name: preferences
                .get(&stringPreferencesKey(&format!("prompt_tag_{id}_name")))
                .cloned()
                .unwrap_or_else(|| "Unnamed Tag".to_string()),
            description: preferences
                .get(&stringPreferencesKey(&format!(
                    "prompt_tag_{id}_description"
                )))
                .cloned()
                .unwrap_or_default(),
            promptContent: preferences
                .get(&stringPreferencesKey(&format!(
                    "prompt_tag_{id}_prompt_content"
                )))
                .cloned()
                .unwrap_or_default(),
            tagType,
            createdAt: preferences
                .get(&stringPreferencesKey(&format!(
                    "prompt_tag_{id}_created_at"
                )))
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or_else(currentTimeMillis),
            updatedAt: preferences
                .get(&stringPreferencesKey(&format!(
                    "prompt_tag_{id}_updated_at"
                )))
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or_else(currentTimeMillis),
        }
    }

    #[allow(non_snake_case)]
    /// Creates a prompt tag and returns its generated identifier.
    pub fn createPromptTag(
        &self,
        name: String,
        description: String,
        promptContent: String,
        tagType: TagType,
    ) -> Result<String, PreferencesDataStoreError> {
        let id = Uuid::new_v4().to_string();
        let now = currentTimeMillis();
        self.dataStore.try_edit_result(|preferences| {
            Self::assertTagNameUnique(preferences, &name, None)?;
            let mut currentList = Self::readTagList(preferences);
            currentList.push(id.clone());
            currentList.sort();
            currentList.dedup();
            Self::writeTagList(preferences, currentList);
            preferences.set(
                &stringPreferencesKey(&format!("prompt_tag_{id}_name")),
                name,
            );
            preferences.set(
                &stringPreferencesKey(&format!("prompt_tag_{id}_description")),
                description,
            );
            preferences.set(
                &stringPreferencesKey(&format!("prompt_tag_{id}_prompt_content")),
                promptContent,
            );
            preferences.set(
                &stringPreferencesKey(&format!("prompt_tag_{id}_tag_type")),
                tagTypeName(&tagType).to_string(),
            );
            preferences.set(
                &stringPreferencesKey(&format!("prompt_tag_{id}_created_at")),
                now.to_string(),
            );
            preferences.set(
                &stringPreferencesKey(&format!("prompt_tag_{id}_updated_at")),
                now.to_string(),
            );
            Ok::<(), PreferencesDataStoreError>(())
        })?;
        Ok(id)
    }

    #[allow(non_snake_case)]
    /// Updates selected prompt tag fields and refreshes the update timestamp.
    pub fn updatePromptTag(
        &self,
        id: &str,
        name: Option<String>,
        description: Option<String>,
        promptContent: Option<String>,
        tagType: Option<TagType>,
    ) -> Result<(), PreferencesDataStoreError> {
        let now = currentTimeMillis();
        self.dataStore.try_edit_result(|preferences| {
            if let Some(name) = name {
                Self::assertTagNameUnique(preferences, &name, Some(id))?;
                preferences.set(
                    &stringPreferencesKey(&format!("prompt_tag_{id}_name")),
                    name,
                );
            }
            if let Some(description) = description {
                preferences.set(
                    &stringPreferencesKey(&format!("prompt_tag_{id}_description")),
                    description,
                );
            }
            if let Some(promptContent) = promptContent {
                preferences.set(
                    &stringPreferencesKey(&format!("prompt_tag_{id}_prompt_content")),
                    promptContent,
                );
            }
            if let Some(tagType) = tagType {
                preferences.set(
                    &stringPreferencesKey(&format!("prompt_tag_{id}_tag_type")),
                    tagTypeName(&tagType).to_string(),
                );
            }
            preferences.set(
                &stringPreferencesKey(&format!("prompt_tag_{id}_updated_at")),
                now.to_string(),
            );
            Ok::<(), PreferencesDataStoreError>(())
        })
    }

    #[allow(non_snake_case)]
    /// Deletes a prompt tag and removes its stored fields.
    pub fn deletePromptTag(&self, id: &str) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.try_edit_result(|preferences| {
            let mut currentList = Self::readTagList(preferences);
            currentList.retain(|item| item != id);
            Self::writeTagList(preferences, currentList);
            self.removeTagPreferenceKeys(preferences, id);
            Ok::<(), PreferencesDataStoreError>(())
        })
    }

    #[allow(non_snake_case)]
    /// Reads all prompt tags sorted by most recent update time.
    pub fn getAllTags(&self) -> Result<Vec<PromptTag>, PreferencesDataStoreError> {
        self.allTagsFlow().first()
    }

    #[allow(non_snake_case)]
    /// Reads prompt tags that match the requested tag type.
    pub fn getTagsByType(
        &self,
        tagType: TagType,
    ) -> Result<Vec<PromptTag>, PreferencesDataStoreError> {
        Ok(self
            .getAllTags()?
            .into_iter()
            .filter(|tag| tag.tagType == tagType)
            .collect())
    }

    #[allow(non_snake_case)]
    /// Finds a prompt tag whose prompt content matches the provided content.
    pub fn findTagWithSameContent(
        &self,
        promptContent: &str,
    ) -> Result<Option<PromptTag>, PreferencesDataStoreError> {
        let trimmed = promptContent.trim();
        Ok(self
            .getAllTags()?
            .into_iter()
            .find(|tag| tag.promptContent.trim() == trimmed))
    }

    #[allow(non_snake_case)]
    /// Creates a prompt tag or returns an existing tag with identical prompt content.
    pub fn createOrReusePromptTag(
        &self,
        name: String,
        description: String,
        promptContent: String,
        tagType: TagType,
    ) -> Result<String, PreferencesDataStoreError> {
        if let Some(existingTag) = self.findTagWithSameContent(&promptContent)? {
            return Ok(existingTag.id);
        }
        self.createPromptTag(name, description, promptContent, tagType)
    }

    #[allow(non_snake_case)]
    /// Removes built-in prompt tag records created by older preference schemas.
    pub fn removeLegacyBuiltInTags(&self) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.try_edit_result(|preferences| {
            let mut currentList = Self::readTagList(preferences);
            let idsToRemove = currentList
                .iter()
                .filter(|id| {
                    preferences.get(&stringPreferencesKey(&format!(
                        "prompt_tag_{id}_is_system_tag"
                    ))) == Some(&"true".to_string())
                        || preferences
                            .get(&stringPreferencesKey(&format!("prompt_tag_{id}_tag_type")))
                            .map(|value| value.starts_with("SYSTEM_"))
                            .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>();
            currentList.retain(|id| !idsToRemove.contains(id));
            Self::writeTagList(preferences, currentList);
            for id in idsToRemove {
                self.removeTagPreferenceKeys(preferences, &id);
            }
            Ok::<(), PreferencesDataStoreError>(())
        })
    }

    #[allow(non_snake_case)]

    fn assertTagNameUnique(
        preferences: &Preferences,
        name: &str,
        currentTagId: Option<&str>,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedName = name.trim();
        let tagIds = Self::readTagList(preferences);
        for tagId in tagIds {
            if currentTagId == Some(tagId.as_str()) {
                continue;
            }
            let existingName = preferences
                .get(&stringPreferencesKey(&format!("prompt_tag_{tagId}_name")))
                .cloned()
                .unwrap_or_default();
            if existingName.trim() == normalizedName {
                return Err(PreferencesDataStoreError::Message(format!(
                    "prompt tag name already exists: {normalizedName}"
                )));
            }
        }
        Ok(())
    }
    fn readTagList(preferences: &Preferences) -> Vec<String> {
        preferences
            .get(&Self::PROMPT_TAG_LIST())
            .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    fn writeTagList(preferences: &mut Preferences, tagIds: Vec<String>) {
        let encoded = serde_json::to_string(&tagIds).expect("prompt tag list must serialize");
        preferences.set(&Self::PROMPT_TAG_LIST(), encoded);
    }

    #[allow(non_snake_case)]
    fn removeTagPreferenceKeys(&self, preferences: &mut Preferences, id: &str) {
        for suffix in [
            "name",
            "description",
            "prompt_content",
            "tag_type",
            "is_system_tag",
            "created_at",
            "updated_at",
        ] {
            preferences.remove(&stringPreferencesKey(&format!("prompt_tag_{id}_{suffix}")));
        }
    }
}

fn parseTagType(value: &str) -> Option<TagType> {
    match value {
        "TONE" => Some(TagType::TONE),
        "CHARACTER" => Some(TagType::CHARACTER),
        "FUNCTION" => Some(TagType::FUNCTION),
        "CUSTOM" => Some(TagType::CUSTOM),
        _ => None,
    }
}

fn tagTypeName(tagType: &TagType) -> &'static str {
    match tagType {
        TagType::TONE => "TONE",
        TagType::CHARACTER => "CHARACTER",
        TagType::FUNCTION => "FUNCTION",
        TagType::CUSTOM => "CUSTOM",
    }
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
