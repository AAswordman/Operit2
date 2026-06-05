use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::data::model::CharacterGroupCard::{CharacterGroupCard, GroupMemberConfig};
use crate::data::preferences::CharacterCardManager::CharacterCardManager;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CharacterGroupsBackupFile {
    #[serde(default, rename = "characterGroups")]
    characterGroups: Vec<CharacterGroupCard>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CharacterGroupImportResult {
    pub new: i32,
    pub updated: i32,
    pub skipped: i32,
    pub total: i32,
}

#[derive(Clone)]
pub struct CharacterGroupCardManager {
    dataStore: PreferencesDataStore,
    characterCardManager: CharacterCardManager,
}

impl CharacterGroupCardManager {
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(
                paths.root_dir().join("character_groups.preferences.json"),
            ),
            characterCardManager: CharacterCardManager::new(paths),
        }
    }

    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    #[allow(non_snake_case)]
    fn CHARACTER_GROUP_LIST() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("character_group_list")
    }

    #[allow(non_snake_case)]
    fn ACTIVE_CHARACTER_GROUP_ID() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("active_character_group_id")
    }

    #[allow(non_snake_case)]
    fn groupDataKey(groupId: &str) -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey(&format!("character_group_{groupId}_data"))
    }

    #[allow(non_snake_case)]
    pub fn characterGroupCardListFlow(&self) -> Flow<Vec<String>> {
        self.dataStore
            .dataFlow()
            .map(|preferences| Self::readGroupList(&preferences))
    }

    #[allow(non_snake_case)]
    pub fn observeActiveCharacterGroupId(&self) -> Flow<Option<String>> {
        self.dataStore.dataFlow().map(|preferences| {
            preferences
                .get(&Self::ACTIVE_CHARACTER_GROUP_ID())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
    }

    #[allow(non_snake_case)]
    pub fn allCharacterGroupCardsFlow(&self) -> Flow<Vec<CharacterGroupCard>> {
        let manager = self.clone();
        self.dataStore.dataFlow().map(move |preferences| {
            let mut groups = Self::readGroupList(&preferences)
                .into_iter()
                .filter_map(|id| {
                    preferences
                        .get(&Self::groupDataKey(&id))
                        .and_then(|raw| manager.decodeGroup(raw))
                })
                .collect::<Vec<_>>();
            groups.sort_by(|left, right| right.updatedAt.cmp(&left.updatedAt));
            groups
        })
    }

    #[allow(non_snake_case)]
    pub fn getCharacterGroupCardFlow(&self, id: &str) -> Flow<Option<CharacterGroupCard>> {
        let manager = self.clone();
        let id = id.to_string();
        self.dataStore.dataFlow().map(move |preferences| {
            preferences
                .get(&Self::groupDataKey(&id))
                .and_then(|raw| manager.decodeGroup(raw))
        })
    }

    #[allow(non_snake_case)]
    pub fn activeCharacterGroupCardFlow(&self) -> Flow<Option<CharacterGroupCard>> {
        let manager = self.clone();
        self.dataStore.dataFlow().map(move |preferences| {
            let activeId = preferences
                .get(&Self::ACTIVE_CHARACTER_GROUP_ID())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())?;
            preferences
                .get(&Self::groupDataKey(&activeId))
                .and_then(|raw| manager.decodeGroup(raw))
        })
    }

    #[allow(non_snake_case)]
    pub fn createCharacterGroupCard(
        &self,
        group: CharacterGroupCard,
    ) -> Result<String, PreferencesDataStoreError> {
        let now = currentTimeMillis();
        let id = if group.id.trim().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            group.id.clone()
        };
        let normalizedGroup = self.normalizeGroup(CharacterGroupCard {
            id: id.clone(),
            createdAt: if group.createdAt > 0 {
                group.createdAt
            } else {
                now
            },
            updatedAt: now,
            ..group
        });
        self.dataStore.edit(|preferences| {
            let mut currentList = Self::readGroupList(preferences);
            if !currentList.contains(&id) {
                currentList.push(id.clone());
            }
            currentList.sort();
            currentList.dedup();
            Self::writeGroupList(preferences, currentList);
            preferences.set(
                &Self::groupDataKey(&id),
                serde_json::to_string(&normalizedGroup).expect("character group must serialize"),
            );
            if preferences
                .get(&Self::ACTIVE_CHARACTER_GROUP_ID())
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                preferences.set(&Self::ACTIVE_CHARACTER_GROUP_ID(), id.clone());
            }
        })?;
        Ok(id)
    }

    #[allow(non_snake_case)]
    pub fn updateCharacterGroupCard(
        &self,
        group: CharacterGroupCard,
    ) -> Result<(), PreferencesDataStoreError> {
        if group.id.trim().is_empty() {
            return Ok(());
        }
        let normalizedGroup = self.normalizeGroup(CharacterGroupCard {
            updatedAt: currentTimeMillis(),
            ..group.clone()
        });
        self.dataStore.edit(|preferences| {
            let mut currentList = Self::readGroupList(preferences);
            if !currentList.contains(&group.id) {
                currentList.push(group.id.clone());
            }
            currentList.sort();
            currentList.dedup();
            Self::writeGroupList(preferences, currentList);
            preferences.set(
                &Self::groupDataKey(&group.id),
                serde_json::to_string(&normalizedGroup).expect("character group must serialize"),
            );
        })
    }

    #[allow(non_snake_case)]
    pub fn deleteCharacterGroupCard(&self, groupId: &str) -> Result<(), PreferencesDataStoreError> {
        if groupId.trim().is_empty() {
            return Ok(());
        }
        self.dataStore.edit(|preferences| {
            let mut currentList = Self::readGroupList(preferences);
            currentList.retain(|id| id != groupId);
            Self::writeGroupList(preferences, currentList);
            preferences.remove(&Self::groupDataKey(groupId));
            if preferences.get(&Self::ACTIVE_CHARACTER_GROUP_ID()) == Some(&groupId.to_string()) {
                preferences.remove(&Self::ACTIVE_CHARACTER_GROUP_ID());
            }
        })
    }

    #[allow(non_snake_case)]
    pub fn setActiveCharacterGroupCard(
        &self,
        groupId: Option<String>,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            match groupId
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
            {
                Some(groupId) => preferences.set(&Self::ACTIVE_CHARACTER_GROUP_ID(), groupId),
                None => preferences.remove(&Self::ACTIVE_CHARACTER_GROUP_ID()),
            }
        })
    }

    #[allow(non_snake_case)]
    pub fn getCharacterGroupCard(
        &self,
        groupId: &str,
    ) -> Result<Option<CharacterGroupCard>, PreferencesDataStoreError> {
        self.getCharacterGroupCardFlow(groupId).first()
    }

    #[allow(non_snake_case)]
    pub fn getAllCharacterGroupCards(
        &self,
    ) -> Result<Vec<CharacterGroupCard>, PreferencesDataStoreError> {
        self.allCharacterGroupCardsFlow().first()
    }

    #[allow(non_snake_case)]
    pub fn initializeIfNeeded(&self) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            if preferences.get(&Self::CHARACTER_GROUP_LIST()).is_none() {
                Self::writeGroupList(preferences, Vec::new());
            }
        })
    }

    #[allow(non_snake_case)]
    pub fn duplicateCharacterGroupCard(
        &self,
        sourceGroupId: &str,
        newName: Option<String>,
    ) -> Result<Option<String>, PreferencesDataStoreError> {
        let Some(source) = self.getCharacterGroupCard(sourceGroupId)? else {
            return Ok(None);
        };
        let now = currentTimeMillis();
        let duplicated = CharacterGroupCard {
            id: String::new(),
            name: newName
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or(source.name),
            description: source.description,
            members: source.members,
            createdAt: now,
            updatedAt: now,
        };
        let newId = self.createCharacterGroupCard(duplicated)?;
        self.cloneBindingsFromCharacterGroup(sourceGroupId, &newId);
        Ok(Some(newId))
    }

    #[allow(non_snake_case)]
    pub fn cloneBindingsFromCharacterGroup(&self, _sourceGroupId: &str, _targetGroupId: &str) {}

    #[allow(non_snake_case)]
    pub fn exportAllCharacterGroupsToBackupContent(&self) -> Result<String, String> {
        let groups = self
            .getAllCharacterGroupCards()
            .map_err(|error| error.to_string())?;
        let backup = CharacterGroupsBackupFile {
            characterGroups: groups,
        };
        serde_json::to_string_pretty(&backup)
            .map_err(|error| format!("导出角色组备份失败：{error}"))
    }

    #[allow(non_snake_case)]
    pub fn importAllCharacterGroupsFromBackupContent(
        &self,
        jsonContent: &str,
    ) -> Result<CharacterGroupImportResult, String> {
        if jsonContent.trim().is_empty() {
            return Err("角色组备份内容不能为空".to_string());
        }
        let backup = serde_json::from_str::<CharacterGroupsBackupFile>(jsonContent)
            .map_err(|error| format!("角色组备份 JSON 格式错误：{error}"))?;
        let existingIds = self
            .characterGroupCardListFlow()
            .first()
            .map_err(|error| error.to_string())?;
        let mut newCount = 0;
        let mut updatedCount = 0;
        let mut skippedCount = 0;

        for group in backup.characterGroups {
            if group.id.trim().is_empty() || group.name.trim().is_empty() {
                skippedCount += 1;
                continue;
            }
            if existingIds.contains(&group.id) {
                updatedCount += 1;
            } else {
                newCount += 1;
            }
            self.upsertCharacterGroupCardWithId(group)
                .map_err(|error| error.to_string())?;
        }

        Ok(CharacterGroupImportResult {
            new: newCount,
            updated: updatedCount,
            skipped: skippedCount,
            total: newCount + updatedCount,
        })
    }

    #[allow(non_snake_case)]
    fn decodeGroup(&self, json: &str) -> Option<CharacterGroupCard> {
        serde_json::from_str::<CharacterGroupCard>(json)
            .ok()
            .map(|group| self.normalizeGroup(group))
    }

    #[allow(non_snake_case)]
    fn normalizeGroup(&self, group: CharacterGroupCard) -> CharacterGroupCard {
        let mut normalizedMembers = group
            .members
            .into_iter()
            .filter(|member| !member.characterCardId.trim().is_empty())
            .collect::<Vec<_>>();
        normalizedMembers.sort_by_key(|member| member.orderIndex);
        normalizedMembers = normalizedMembers
            .into_iter()
            .enumerate()
            .map(|(index, member)| GroupMemberConfig {
                characterCardId: member.characterCardId,
                orderIndex: index as i32,
            })
            .collect();
        let now = currentTimeMillis();
        CharacterGroupCard {
            members: normalizedMembers,
            createdAt: if group.createdAt > 0 {
                group.createdAt
            } else {
                now
            },
            updatedAt: if group.updatedAt > 0 {
                group.updatedAt
            } else {
                now
            },
            ..group
        }
    }

    #[allow(non_snake_case)]
    fn upsertCharacterGroupCardWithId(
        &self,
        group: CharacterGroupCard,
    ) -> Result<(), PreferencesDataStoreError> {
        let id = group.id.clone();
        if id.trim().is_empty() {
            return Ok(());
        }
        let normalizedGroup = self.normalizeGroup(group);
        self.dataStore.edit(|preferences| {
            let mut currentList = Self::readGroupList(preferences);
            if !currentList.contains(&id) {
                currentList.push(id.clone());
            }
            currentList.sort();
            currentList.dedup();
            Self::writeGroupList(preferences, currentList);
            preferences.set(
                &Self::groupDataKey(&id),
                serde_json::to_string(&normalizedGroup).expect("character group must serialize"),
            );
            if preferences
                .get(&Self::ACTIVE_CHARACTER_GROUP_ID())
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                preferences.set(&Self::ACTIVE_CHARACTER_GROUP_ID(), id.clone());
            }
        })
    }

    #[allow(non_snake_case)]
    fn readGroupList(preferences: &Preferences) -> Vec<String> {
        preferences
            .get(&Self::CHARACTER_GROUP_LIST())
            .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    fn writeGroupList(preferences: &mut Preferences, groupIds: Vec<String>) {
        preferences.set(
            &Self::CHARACTER_GROUP_LIST(),
            serde_json::to_string(&groupIds).expect("group list must serialize"),
        );
    }
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
