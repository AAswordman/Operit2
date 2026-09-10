fn buildOperit2PromptTags(parsed: &ParsedOperit1Snapshot) -> Result<Vec<PromptTag>, String> {
    let Some(preferences) = parsed.archive.datastorePreferences.get(ENTRY_PROMPT_TAGS) else {
        return Ok(Vec::new());
    };
    let ids = optionalPreferenceStringSet(preferences, "prompt_tag_list")?;
    let legacyTagIds = collectOperit1LegacyPromptTagIds(preferences)?;
    let mut tags = Vec::new();
    for id in ids {
        if legacyTagIds.contains(&id) {
            continue;
        }
        let name = requiredPreferenceString(
            preferences,
            &format!("prompt_tag_{id}_name"),
            &format!("Operit1 提示标签缺少名称：{id}"),
        )?;
        tags.push(PromptTag {
            id: id.clone(),
            name: name.to_string(),
            description: optionalPreferenceString(
                preferences,
                &format!("prompt_tag_{id}_description"),
            )?
            .unwrap_or_default(),
            promptContent: optionalPreferenceString(
                preferences,
                &format!("prompt_tag_{id}_prompt_content"),
            )?
            .unwrap_or_default(),
            tagType: parseOperit1PromptTagType(
                optionalPreferenceString(preferences, &format!("prompt_tag_{id}_tag_type"))?
                    .as_deref(),
            )?,
            createdAt: optionalPreferenceI64(preferences, &format!("prompt_tag_{id}_created_at"))?
                .unwrap_or_else(currentTimeMillis),
            updatedAt: optionalPreferenceI64(preferences, &format!("prompt_tag_{id}_updated_at"))?
                .unwrap_or_else(currentTimeMillis),
        });
    }
    Ok(tags)
}

/// Builds Operit2 character cards from an Operit1 character-card datastore.
fn buildOperit2CharacterCards(
    parsed: &ParsedOperit1Snapshot,
    fileImportPlan: &SnapshotFileImportPlan,
) -> Result<Vec<CharacterCard>, String> {
    let Some(preferences) = parsed
        .archive
        .datastorePreferences
        .get(ENTRY_CHARACTER_CARDS)
    else {
        return Ok(Vec::new());
    };
    let legacyPromptTagIds = parsed
        .archive
        .datastorePreferences
        .get(ENTRY_PROMPT_TAGS)
        .map(collectOperit1LegacyPromptTagIds)
        .transpose()?
        .unwrap_or_default();
    let ids = optionalPreferenceStringSet(preferences, "character_card_list")?;
    let userPreferences = parsed
        .archive
        .datastorePreferences
        .get(ENTRY_USER_PREFERENCES)
        .ok_or_else(|| "Operit1 快照缺少用户偏好，无法读取角色头像绑定".to_string())?;
    let avatarUris = buildOperit1CharacterCardAvatarUris(userPreferences, &ids, fileImportPlan)?;
    let mut cards = Vec::new();
    for id in ids {
        let name = requiredPreferenceString(
            preferences,
            &format!("character_card_{id}_name"),
            &format!("Operit1 角色卡缺少名称：{id}"),
        )?;
        let chatModelBindingMode = optionalPreferenceString(
            preferences,
            &format!("character_card_{id}_chat_model_binding_mode"),
        )?;
        let chatModelId = resolveOperit1CharacterChatModelId(parsed, preferences, &id)?;
        let memoryBinding = resolveOperit1CharacterMemoryBinding(parsed, preferences, &id)?;
        cards.push(CharacterCard {
            id: id.clone(),
            name: name.to_string(),
            description: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_description"),
            )?
            .unwrap_or_default(),
            characterSetting: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_character_setting"),
            )?
            .unwrap_or_default(),
            openingStatement: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_opening_statement"),
            )?
            .unwrap_or_default(),
            otherContentChat: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_other_content_chat"),
            )?
            .unwrap_or_default(),
            otherContentVoice: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_other_content_voice"),
            )?
            .unwrap_or_default(),
            avatarUri: avatarUris.get(&id).cloned(),
            attachedTagIds: optionalPreferenceStringSet(
                preferences,
                &format!("character_card_{id}_attached_tag_ids"),
            )?
            .into_iter()
            .filter(|tagId| !legacyPromptTagIds.contains(tagId))
            .collect(),
            advancedCustomPrompt: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_advanced_custom_prompt"),
            )?
            .unwrap_or_default(),
            marks: optionalPreferenceString(preferences, &format!("character_card_{id}_marks"))?
                .unwrap_or_default(),
            chatModelBindingMode: if chatModelBindingMode.as_deref() == Some("FIXED_CONFIG") {
                CharacterCardChatModelBindingMode::FIXED_MODEL.to_string()
            } else {
                CharacterCardChatModelBindingMode::FOLLOW_GLOBAL.to_string()
            },
            chatModelId,
            ttsConfigId: None,
            memoryBindingMode: memoryBinding.memoryBindingMode,
            sharedMemoryId: memoryBinding.sharedMemoryId,
            sharedMemoryMounts: Vec::new(),
            toolAccessConfig: optionalPreferenceString(
                preferences,
                &format!("character_card_{id}_tool_access_config_json"),
            )?
            .map(|raw| serde_json::from_str::<CharacterCardToolAccessConfig>(&raw))
            .transpose()
            .map_err(|error| format!("Operit1 角色卡工具权限格式不正确：{id}: {error}"))?
            .unwrap_or_default(),
            isDefault: optionalPreferenceBool(
                preferences,
                &format!("character_card_{id}_is_default"),
            )?
            .unwrap_or(id == CharacterCardManager::DEFAULT_CHARACTER_CARD_ID),
            createdAt: optionalPreferenceI64(
                preferences,
                &format!("character_card_{id}_created_at"),
            )?
            .unwrap_or_else(currentTimeMillis),
            updatedAt: optionalPreferenceI64(
                preferences,
                &format!("character_card_{id}_updated_at"),
            )?
            .unwrap_or_else(currentTimeMillis),
        });
    }
    Ok(cards)
}

/// Maps Operit1 character-theme avatar bindings to imported virtual paths.
#[allow(non_snake_case)]
fn buildOperit1CharacterCardAvatarUris(
    userPreferences: &HashMap<String, Operit1PreferenceValue>,
    cardIds: &[String],
    fileImportPlan: &SnapshotFileImportPlan,
) -> Result<BTreeMap<String, String>, String> {
    let mut avatarUris = BTreeMap::new();
    for cardId in cardIds {
        let key = format!("character_card_theme_{cardId}_custom_ai_avatar_uri");
        let Some(value) = optionalPreferenceString(userPreferences, &key)? else {
            continue;
        };
        if value.trim().is_empty() {
            continue;
        }
        if cardId == CharacterCardManager::DEFAULT_CHARACTER_CARD_ID
            && value.trim() == OPERIT1_DEFAULT_AI_AVATAR_URI
        {
            continue;
        }
        avatarUris.insert(cardId.clone(), fileImportPlan.rewritePath(&value)?);
    }
    Ok(avatarUris)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Leaves cards without an explicit theme avatar URI unassigned.
    #[test]
    fn leaves_operit1_character_card_avatar_unassigned_without_explicit_uri() {
        let cardId = "666eee2e-ea60-4ff6-9c92-04d396386231";
        let cardIds = vec![cardId.to_string()];

        let avatarUris = buildOperit1CharacterCardAvatarUris(
            &HashMap::new(),
            &cardIds,
            &SnapshotFileImportPlan::new(),
        )
        .expect("Operit1 cards without avatar URIs should be accepted");

        assert!(avatarUris.is_empty());
    }

    /// Uses the character-theme URI when an Operit1 card has multiple avatar resources.
    #[test]
    fn uses_operit1_character_card_avatar_uri_with_multiple_avatar_files() {
        let cardId = "4a08d0b6-83d7-4b09-bbaa-e9cad156b198";
        let currentResourceId = "d96932ce-f70d-4a58-835f-c540e3563f2d";
        let cardIds = vec![cardId.to_string()];
        let mut preferences = HashMap::new();
        preferences.insert(
            format!("character_card_theme_{cardId}_custom_ai_avatar_uri"),
            Operit1PreferenceValue::String(format!(
                "file:///data/user/0/com.ai.assistance.operit/files/avatar_{cardId}_{currentResourceId}.png"
            )),
        );
        let avatarUris = buildOperit1CharacterCardAvatarUris(
            &preferences,
            &cardIds,
            &SnapshotFileImportPlan::new(),
        )
        .expect("Operit1 avatar URI should identify the active avatar resource");

        assert_eq!(
            avatarUris.get(cardId),
            Some(&format!(
                "{RUNTIME_IMPORTED_OPERIT1_FILES_DIR_PATH}/avatar_{cardId}_{currentResourceId}.png"
            )),
        );
    }

    /// Maps avatar URIs written by the Operit1 Android debug package into runtime storage.
    #[test]
    fn rewrites_operit1_debug_package_character_card_avatar_uri() {
        let cardId = "666eee2e-ea60-4ff6-9c92-04d396386231";
        let resourceId = "329c80ae-f6f0-4bf8-9bb7-2199150f6f15";
        let cardIds = vec![cardId.to_string()];
        let mut preferences = HashMap::new();
        preferences.insert(
            format!("character_card_theme_{cardId}_custom_ai_avatar_uri"),
            Operit1PreferenceValue::String(format!(
                "file:///data/user/0/com.ai.assistance.operit.debug/files/avatar_{cardId}_{resourceId}.png"
            )),
        );

        let avatarUris = buildOperit1CharacterCardAvatarUris(
            &preferences,
            &cardIds,
            &SnapshotFileImportPlan::new(),
        )
        .expect("Operit1 debug avatar URI should be rewritten");

        assert_eq!(
            avatarUris.get(cardId),
            Some(&format!(
                "{RUNTIME_IMPORTED_OPERIT1_FILES_DIR_PATH}/avatar_{cardId}_{resourceId}.png"
            )),
        );
    }

    /// Keeps one explicitly shared Operit1 avatar URI attached to both character cards.
    #[test]
    fn preserves_shared_operit1_character_card_avatar_uri() {
        let sourceCardId = "666eee2e-ea60-4ff6-9c92-04d396386231";
        let copyCardId = "cfd9ae03-3e3c-4833-8dbb-14c06ed51daf";
        let resourceId = "d96932ce-f70d-4a58-835f-c540e3563f2d";
        let sourceUri = format!(
            "file:///data/user/0/com.ai.assistance.operit/files/avatar_{sourceCardId}_{resourceId}.png"
        );
        let cardIds = vec![sourceCardId.to_string(), copyCardId.to_string()];
        let mut preferences = HashMap::new();
        for cardId in &cardIds {
            preferences.insert(
                format!("character_card_theme_{cardId}_custom_ai_avatar_uri"),
                Operit1PreferenceValue::String(sourceUri.clone()),
            );
        }

        let avatarUris = buildOperit1CharacterCardAvatarUris(
            &preferences,
            &cardIds,
            &SnapshotFileImportPlan::new(),
        )
        .expect("shared Operit1 avatar URIs should be imported for every bound card");
        let expectedUri = format!(
            "{RUNTIME_IMPORTED_OPERIT1_FILES_DIR_PATH}/avatar_{sourceCardId}_{resourceId}.png"
        );

        assert_eq!(avatarUris.get(sourceCardId), Some(&expectedUri));
        assert_eq!(avatarUris.get(copyCardId), Some(&expectedUri));
    }

    /// Leaves Operit1's built-in default avatar unassigned for the Operit2 default avatar component.
    #[test]
    fn leaves_operit1_builtin_default_avatar_unassigned() {
        let cardId = CharacterCardManager::DEFAULT_CHARACTER_CARD_ID;
        let cardIds = vec![cardId.to_string()];
        let mut preferences = HashMap::new();
        preferences.insert(
            format!("character_card_theme_{cardId}_custom_ai_avatar_uri"),
            Operit1PreferenceValue::String(OPERIT1_DEFAULT_AI_AVATAR_URI.to_string()),
        );

        let avatarUris = buildOperit1CharacterCardAvatarUris(
            &preferences,
            &cardIds,
            &SnapshotFileImportPlan::new(),
        )
        .expect("Operit1's built-in avatar should map to Operit2's default avatar component");

        assert!(avatarUris.is_empty());
    }
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
struct Operit1CharacterMemoryBinding {
    memoryBindingMode: String,
    sharedMemoryId: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1UserPreferenceProfile {
    id: String,
    name: String,
    #[serde(default)]
    birthDate: i64,
    #[serde(default)]
    gender: String,
    #[serde(default)]
    personality: String,
    #[serde(default)]
    identity: String,
    #[serde(default)]
    occupation: String,
    #[serde(default)]
    aiStyle: String,
    #[serde(default)]
    isInitialized: bool,
}

#[allow(non_snake_case)]
fn resolveOperit1CharacterMemoryBinding(
    parsed: &ParsedOperit1Snapshot,
    preferences: &HashMap<String, Operit1PreferenceValue>,
    cardId: &str,
) -> Result<Operit1CharacterMemoryBinding, String> {
    let bindingMode = optionalPreferenceString(
        preferences,
        &format!("character_card_{cardId}_memory_profile_binding_mode"),
    )?;
    if bindingMode.as_deref() == Some("FIXED_PROFILE") {
        let profileId = requiredPreferenceString(
            preferences,
            &format!("character_card_{cardId}_memory_profile_id"),
            &format!("Operit1 角色卡固定记忆库缺少配置 ID：{cardId}"),
        )?;
        return Ok(Operit1CharacterMemoryBinding {
            memoryBindingMode: CharacterCardMemoryBindingMode::SHARED.to_string(),
            sharedMemoryId: Some(operit1SharedMemoryStoreId(profileId)),
        });
    }
    let profileId = operit1ActiveProfileId(parsed)?;
    Ok(Operit1CharacterMemoryBinding {
        memoryBindingMode: CharacterCardMemoryBindingMode::SHARED.to_string(),
        sharedMemoryId: Some(operit1SharedMemoryStoreId(&profileId)),
    })
}

#[allow(non_snake_case)]
fn collectOperit1CharacterMemoryProfileBindings(
    parsed: &ParsedOperit1Snapshot,
) -> Result<BTreeMap<String, String>, String> {
    let Some(preferences) = parsed
        .archive
        .datastorePreferences
        .get(ENTRY_CHARACTER_CARDS)
    else {
        return Ok(BTreeMap::new());
    };
    let ids = optionalPreferenceStringSet(preferences, "character_card_list")?;
    let mut bindings = BTreeMap::new();
    for id in ids {
        let bindingMode = optionalPreferenceString(
            preferences,
            &format!("character_card_{id}_memory_profile_binding_mode"),
        )?;
        let profileId = if bindingMode.as_deref() == Some("FIXED_PROFILE") {
            requiredPreferenceString(
                preferences,
                &format!("character_card_{id}_memory_profile_id"),
                &format!("Operit1 角色卡固定用户偏好缺少配置 ID：{id}"),
            )?
            .to_string()
        } else {
            operit1ActiveProfileId(parsed)?
        };
        bindings.insert(id, profileId);
    }
    Ok(bindings)
}

#[allow(non_snake_case)]
fn collectOperit1MemoryProfileIds(
    parsed: &ParsedOperit1Snapshot,
) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    ids.insert(OPERIT1_DEFAULT_PROFILE_ID.to_string());
    ids.extend(collectOperit1ObjectBoxProfileIds(parsed)?);
    for profileId in collectOperit1CharacterMemoryProfileBindings(parsed)?.values() {
        ids.insert(profileId.clone());
    }
    Ok(ids)
}

#[allow(non_snake_case)]
fn collectOperit1ObjectBoxProfileIds(
    parsed: &ParsedOperit1Snapshot,
) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for entry in parsed.archive.entries.keys() {
        if entry == ENTRY_OBJECTBOX_DEFAULT_DATA {
            ids.insert(OPERIT1_DEFAULT_PROFILE_ID.to_string());
            continue;
        }
        let Some(rest) = entry.strip_prefix("payload/files/objectbox_") else {
            continue;
        };
        let Some(profileId) = rest.strip_suffix("/data.mdb") else {
            continue;
        };
        validateOperit1ProfileId(profileId)?;
        ids.insert(profileId.to_string());
    }
    Ok(ids)
}

#[allow(non_snake_case)]
fn operit1ObjectBoxEntryForProfile(profileId: &str) -> String {
    if profileId == OPERIT1_DEFAULT_PROFILE_ID {
        ENTRY_OBJECTBOX_DEFAULT_DATA.to_string()
    } else {
        format!("payload/files/objectbox_{profileId}/data.mdb")
    }
}

#[allow(non_snake_case)]
fn validateOperit1ProfileId(profileId: &str) -> Result<(), String> {
    if profileId.trim().is_empty()
        || profileId.contains('/')
        || profileId.contains('\\')
        || profileId.contains(':')
    {
        Err(format!("Operit1 用户偏好 ID 无效：{profileId}"))
    } else {
        Ok(())
    }
}

#[allow(non_snake_case)]
fn operit1SharedMemoryStoreId(profileId: &str) -> String {
    format!(
        "{OPERIT1_SHARED_MEMORY_STORE_ID_PREFIX}{}",
        sanitizeMemoryOwnerId(profileId)
    )
}

#[allow(non_snake_case)]
fn operit1SharedMemoryStoreName(
    parsed: &ParsedOperit1Snapshot,
    profileId: &str,
) -> Result<String, String> {
    let profiles = buildOperit1UserPreferenceProfiles(parsed)?;
    let profile = profiles
        .get(profileId)
        .ok_or_else(|| format!("Operit1 用户偏好缺少记忆库名称来源：{profileId}"))?;
    let profileName = profile.name.trim();
    if profileName.is_empty() {
        return Err(format!("Operit1 用户偏好名称为空：{profileId}"));
    }
    Ok(format!("Operit1 记忆库 - {profileName}"))
}

#[allow(non_snake_case)]
/// Reads the selected Operit1 memory-space identifier from the user preferences payload.
fn operit1ActiveProfileId(parsed: &ParsedOperit1Snapshot) -> Result<String, String> {
    let preferences = parsed
        .archive
        .datastorePreferences
        .get(ENTRY_USER_PREFERENCES)
        .ok_or_else(|| format!("快照里没有 Operit1 用户偏好文件：{ENTRY_USER_PREFERENCES}"))?;
    let value = requiredPreferenceString(
        preferences,
        KEY_ACTIVE_MEMORY_SPACE_ID,
        "Operit1 用户偏好缺少当前记忆库 ID",
    )?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err("Operit1 当前记忆库 ID 为空".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

#[allow(non_snake_case)]
/// Builds Operit1 user preference profiles from the released memory-space schema.
fn buildOperit1UserPreferenceProfiles(
    parsed: &ParsedOperit1Snapshot,
) -> Result<BTreeMap<String, Operit1UserPreferenceProfile>, String> {
    let Some(preferences) = parsed
        .archive
        .datastorePreferences
        .get(ENTRY_USER_PREFERENCES)
    else {
        return Ok(BTreeMap::new());
    };
    let profileIds = requiredPreferenceStringList(
        preferences,
        KEY_MEMORY_SPACE_LIST,
        "Operit1 用户偏好缺少记忆库列表",
    )?;
    let mut profiles = BTreeMap::new();
    for profileId in profileIds {
        validateOperit1ProfileId(&profileId)?;
        let key = format!("{KEY_MEMORY_SPACE_PREFIX}{profileId}");
        let raw = requiredPreferenceString(
            preferences,
            &key,
            &format!("Operit1 用户偏好缺少记忆库配置：{profileId}"),
        )?;
        let profile: Operit1UserPreferenceProfile = serde_json::from_str(raw)
            .map_err(|error| format!("Operit1 记忆库配置格式不正确：{profileId}: {error}"))?;
        if profile.id != profileId {
            return Err(format!(
                "Operit1 记忆库配置 ID 不匹配：{profileId}/{}",
                profile.id
            ));
        }
        profiles.insert(profile.id.clone(), profile);
    }
    Ok(profiles)
}

#[allow(non_snake_case)]
/// Builds a Markdown memory document from one Operit1 profile record.
fn buildOperit2UserMarkdown(profile: &Operit1UserPreferenceProfile) -> Result<String, String> {
    let mut lines = Vec::new();
    lines.push(format!("## Operit1 用户偏好 - {}", profile.name.trim()));
    lines.push(String::new());
    pushMarkdownField(&mut lines, "配置 ID", &profile.id);
    if profile.birthDate > 0 {
        lines.push(format!(
            "- 出生日期：{}",
            epochMillisToLocalDateString(profile.birthDate)?
        ));
    }
    pushMarkdownField(&mut lines, "性别", &profile.gender);
    pushMarkdownField(&mut lines, "性格特点", &profile.personality);
    pushMarkdownField(&mut lines, "身份认同", &profile.identity);
    pushMarkdownField(&mut lines, "职业", &profile.occupation);
    pushMarkdownField(&mut lines, "期待的 AI 风格", &profile.aiStyle);
    Ok(lines.join("\n"))
}

#[allow(non_snake_case)]
fn pushMarkdownField(lines: &mut Vec<String>, label: &str, value: &str) {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
        lines.push(format!("- {label}：{trimmed}"));
    }
}

#[allow(non_snake_case)]
fn appendSharedUserMarkdown(
    storageHost: &dyn RuntimeStorageHost,
    profileId: &str,
    importedMarkdown: &str,
) -> Result<(), String> {
    let storeId = operit1SharedMemoryStoreId(profileId);
    let path = format!(
        "{}/{}/USER.md",
        DATA_MEMORY_SHARED_DIR_PATH.trim_end_matches('/'),
        sanitizeMemoryOwnerId(&storeId)
    );
    let current = if storageHost
        .exists(&path)
        .map_err(|error| error.to_string())?
    {
        String::from_utf8(
            storageHost
                .readBytes(&path)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?
    } else {
        "# USER\n\n".to_string()
    };
    let mut next = current.trim().to_string();
    let imported = importedMarkdown.trim();
    if !next.is_empty() {
        next.push_str("\n\n");
    }
    next.push_str(imported);
    next.push('\n');
    storageHost
        .writeBytes(&path, next.as_bytes())
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn buildOperit2CharacterGroups(
    parsed: &ParsedOperit1Snapshot,
) -> Result<Vec<CharacterGroupCard>, String> {
    let Some(preferences) = parsed
        .archive
        .datastorePreferences
        .get(ENTRY_CHARACTER_GROUPS)
    else {
        return Ok(Vec::new());
    };
    let ids = optionalPreferenceStringSet(preferences, "character_group_list")?;
    let mut groups = Vec::new();
    for id in ids {
        let raw = requiredPreferenceString(
            preferences,
            &format!("character_group_{id}_data"),
            &format!("Operit1 角色组缺少数据：{id}"),
        )?;
        let group: CharacterGroupCard = serde_json::from_str(raw)
            .map_err(|error| format!("Operit1 角色组格式不正确：{id}: {error}"))?;
        groups.push(CharacterGroupCard {
            id: group.id,
            name: group.name,
            description: group.description,
            members: group
                .members
                .into_iter()
                .map(|member| GroupMemberConfig {
                    characterCardId: member.characterCardId,
                    orderIndex: member.orderIndex,
                })
                .collect(),
            createdAt: group.createdAt,
            updatedAt: group.updatedAt,
        });
    }
    Ok(groups)
}

#[allow(non_snake_case)]
fn buildOperit2TtsConfig(parsed: &ParsedOperit1Snapshot) -> Result<Option<TtsConfig>, String> {
    let Some(preferences) = parsed
        .archive
        .datastorePreferences
        .get(ENTRY_SPEECH_SERVICES)
    else {
        return Ok(None);
    };
    let serviceType = optionalPreferenceString(preferences, "tts_service_type")?;
    let Some(serviceType) = serviceType else {
        return Ok(None);
    };
    let serviceType = serviceType.trim();
    let now = currentTimeMillis();
    if serviceType == "SIMPLE_TTS" {
        return Ok(Some(TtsConfig {
            id: String::new(),
            name: "Operit1 系统 TTS".to_string(),
            providerType: TtsProviderType::SYSTEM_TTS.to_string(),
            endpoint: String::new(),
            apiKey: String::new(),
            model: String::new(),
            voice: String::new(),
            responseFormat: "wav".to_string(),
            speed: optionalPreferenceF64(preferences, "tts_speech_rate")?.unwrap_or(1.0),
            httpMethod: "POST".to_string(),
            requestBody: String::new(),
            contentType: "application/json".to_string(),
            headers: Vec::new(),
            responsePipeline: Vec::new(),
            createdAt: now,
            updatedAt: now,
        }));
    }
    let httpConfigRaw = requiredPreferenceString(
        preferences,
        "tts_http_config",
        &format!("Operit1 TTS 服务缺少 HTTP 配置：{serviceType}"),
    )?;
    let httpConfig: Operit1TtsHttpConfig = serde_json::from_str(httpConfigRaw)
        .map_err(|error| format!("Operit1 TTS HTTP 配置格式不正确：{error}"))?;
    Ok(Some(TtsConfig {
        id: String::new(),
        name: format!("Operit1 {serviceType}"),
        providerType: TtsProviderType::HTTP_TTS.to_string(),
        endpoint: httpConfig.urlTemplate,
        apiKey: httpConfig.apiKey,
        model: httpConfig.modelName,
        voice: httpConfig.voiceId,
        responseFormat: "wav".to_string(),
        speed: optionalPreferenceF64(preferences, "tts_speech_rate")?.unwrap_or(1.0),
        httpMethod: httpConfig.httpMethod,
        requestBody: httpConfig.requestBody,
        contentType: httpConfig.contentType,
        headers: httpConfig
            .headers
            .into_iter()
            .map(|(name, value)| TtsHttpHeader { name, value })
            .collect(),
        responsePipeline: httpConfig
            .responsePipeline
            .into_iter()
            .map(|step| TtsHttpResponsePipelineStep {
                stepType: step.stepType,
                path: step.path,
                headers: step
                    .headers
                    .into_iter()
                    .map(|(name, value)| TtsHttpHeader { name, value })
                    .collect(),
            })
            .collect(),
        createdAt: now,
        updatedAt: now,
    }))
}

#[allow(non_snake_case)]
fn importOperit1TtsConfig(manager: &TtsConfigManager, config: TtsConfig) -> Result<(), String> {
    let existing = manager
        .getAllTtsConfigs()
        .map_err(|error| format!("读取 Operit2 TTS 配置失败：{error}"))?
        .into_iter()
        .find(|existing| {
            existing.providerType == config.providerType
                && existing.endpoint == config.endpoint
                && existing.model == config.model
                && existing.voice == config.voice
        });
    let imported = match existing {
        Some(existing) => manager.updateTtsConfig(TtsConfig {
            id: existing.id,
            ..config
        }),
        None => manager.createTtsConfig(config),
    }
    .map_err(|error| format!("导入 Operit1 TTS 配置失败：{error}"))?;
    manager
        .setCurrentTtsConfigId(&imported.id)
        .map_err(|error| format!("设置 Operit1 TTS 配置失败：{error}"))?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1TtsHttpConfig {
    urlTemplate: String,
    apiKey: String,
    headers: HashMap<String, String>,
    #[serde(default = "defaultOperit1TtsHttpMethod")]
    httpMethod: String,
    #[serde(default)]
    requestBody: String,
    #[serde(default = "defaultOperit1TtsContentType")]
    contentType: String,
    #[serde(default)]
    voiceId: String,
    #[serde(default)]
    modelName: String,
    #[serde(default)]
    responsePipeline: Vec<Operit1TtsHttpResponsePipelineStep>,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
struct Operit1TtsHttpResponsePipelineStep {
    #[serde(alias = "type")]
    stepType: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    headers: HashMap<String, String>,
}

#[allow(non_snake_case)]
/// Resolves one character card fixed chat-model binding into a current model id.
fn resolveOperit1CharacterChatModelId(
    parsed: &ParsedOperit1Snapshot,
    preferences: &HashMap<String, Operit1PreferenceValue>,
    cardId: &str,
) -> Result<Option<String>, String> {
    let bindingMode = optionalPreferenceString(
        preferences,
        &format!("character_card_{cardId}_chat_model_binding_mode"),
    )?;
    if bindingMode.as_deref() != Some("FIXED_CONFIG") {
        return Ok(None);
    }
    let configId = requiredPreferenceString(
        preferences,
        &format!("character_card_{cardId}_chat_model_config_id"),
        &format!("Operit1 角色卡固定模型缺少配置 ID：{cardId}"),
    )?;
    let modelIndex = optionalPreferenceI32(
        preferences,
        &format!("character_card_{cardId}_chat_model_index"),
    )?
    .unwrap_or(0)
    .max(0);
    let config = parsed.configById(configId)?;
    let modelIds = splitModelIds(&config.modelName);
    modelIds
        .get(modelIndex as usize)
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("Operit1 角色卡模型索引越界：{cardId}/{modelIndex}"))
}

#[allow(non_snake_case)]
fn optionalPreferenceString(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<String>, String> {
    preferences
        .get(key)
        .map(|value| {
            value
                .asString()
                .map(ToString::to_string)
                .ok_or_else(|| format!("Operit1 DataStore 键不是字符串：{key}"))
        })
        .transpose()
}

#[allow(non_snake_case)]
fn optionalPreferenceStringSet(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Vec<String>, String> {
    match preferences.get(key) {
        Some(value) => value
            .asStringSet()
            .map(|values| values.to_vec())
            .ok_or_else(|| format!("Operit1 DataStore 键不是字符串集合：{key}")),
        None => Ok(Vec::new()),
    }
}

#[allow(non_snake_case)]
fn optionalPreferenceStringList(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<Vec<String>>, String> {
    let Some(raw) = optionalPreferenceString(preferences, key)? else {
        return Ok(None);
    };
    serde_json::from_str::<Vec<String>>(&raw)
        .map(Some)
        .map_err(|error| format!("Operit1 DataStore 键不是字符串列表 JSON：{key}: {error}"))
}

#[allow(non_snake_case)]
/// Reads a required JSON-encoded string list from one Operit1 DataStore payload.
fn requiredPreferenceStringList(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
    missingMessage: &str,
) -> Result<Vec<String>, String> {
    let raw = requiredPreferenceString(preferences, key, missingMessage)?;
    serde_json::from_str::<Vec<String>>(raw)
        .map_err(|error| format!("Operit1 DataStore 键不是字符串列表 JSON：{key}: {error}"))
}

#[allow(non_snake_case)]
fn optionalPreferenceBool(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<bool>, String> {
    match preferences.get(key) {
        Some(Operit1PreferenceValue::Boolean(value)) => Ok(Some(*value)),
        Some(value) => Err(format!("Operit1 DataStore 键不是布尔值：{key}={value:?}")),
        None => Ok(None),
    }
}

#[allow(non_snake_case)]
fn optionalPreferenceI32(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<i32>, String> {
    match preferences.get(key) {
        Some(Operit1PreferenceValue::Int(value)) => Ok(Some(*value)),
        Some(value) => Err(format!("Operit1 DataStore 键不是 i32：{key}={value:?}")),
        None => Ok(None),
    }
}

#[allow(non_snake_case)]
fn optionalPreferenceI64(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<i64>, String> {
    match preferences.get(key) {
        Some(Operit1PreferenceValue::Long(value)) => Ok(Some(*value)),
        Some(Operit1PreferenceValue::Int(value)) => Ok(Some(i64::from(*value))),
        Some(value) => Err(format!("Operit1 DataStore 键不是整数：{key}={value:?}")),
        None => Ok(None),
    }
}

#[allow(non_snake_case)]
fn optionalPreferenceF64(
    preferences: &HashMap<String, Operit1PreferenceValue>,
    key: &str,
) -> Result<Option<f64>, String> {
    match preferences.get(key) {
        Some(Operit1PreferenceValue::Float(value)) => Ok(Some(f64::from(*value))),
        Some(Operit1PreferenceValue::Double(value)) => Ok(Some(*value)),
        Some(value) => Err(format!("Operit1 DataStore 键不是浮点数：{key}={value:?}")),
        None => Ok(None),
    }
}

#[allow(non_snake_case)]
fn collectOperit1LegacyPromptTagIds(
    preferences: &HashMap<String, Operit1PreferenceValue>,
) -> Result<HashSet<String>, String> {
    let ids = optionalPreferenceStringSet(preferences, "prompt_tag_list")?;
    let mut legacyTagIds = HashSet::new();
    for id in ids {
        let isSystemTag =
            optionalPreferenceBool(preferences, &format!("prompt_tag_{id}_is_system_tag"))?
                .unwrap_or(false);
        let tagType = optionalPreferenceString(preferences, &format!("prompt_tag_{id}_tag_type"))?;
        if isSystemTag
            || tagType
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| value.starts_with("SYSTEM_"))
        {
            legacyTagIds.insert(id);
        }
    }
    Ok(legacyTagIds)
}

#[allow(non_snake_case)]
fn parseOperit1PromptTagType(value: Option<&str>) -> Result<TagType, String> {
    match value {
        Some("TONE") => Ok(TagType::TONE),
        Some("CHARACTER") => Ok(TagType::CHARACTER),
        Some("FUNCTION") => Ok(TagType::FUNCTION),
        Some("CUSTOM") | None => Ok(TagType::CUSTOM),
        Some(other) => Err(format!("Operit1 提示标签类型未知：{other}")),
    }
}
