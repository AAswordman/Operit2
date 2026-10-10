/// Imports Operit1's parsed domain records directly into the same Host database used by the character plugin.
#[allow(non_snake_case)]
impl Operit1SnapshotImportManager {
    /// Reuses existing snapshot readers and the single migration writer without reviving Core domain managers.
    fn importCharactersAndMemory(
        &self,
        parsed: &ParsedOperit1Snapshot,
        plan: &SnapshotFileImportPlan,
        archive: &OperitChatArchive,
    ) -> Result<(i32, i32), String> {
        use crate::data::backup::CharacterPluginMigration::{
            ownerMetadata, CharacterMigrationData, CharacterPluginWriter,
        };
        use crate::data::backup::CharacterPluginLegacyData::SharedMemoryStore;
        let writer =
            CharacterPluginWriter::open(self.sqliteHost.clone(), self.storageHost.clone())?;
        let source = format!(
            "operit1:{}:{}",
            parsed.archive.manifest.packageName, parsed.archive.manifest.createdAt
        );
        if writer.get("imports", &source)?.is_some() {
            return Ok((0, 0));
        }
        validateOperit1MemorySpaces(parsed)?;
        let mut data = CharacterMigrationData {
            cards: buildOperit2CharacterCards(parsed, plan)?,
            groups: buildOperit2CharacterGroups(parsed)?,
            tags: buildOperit2PromptTags(parsed)?,
            ..CharacterMigrationData::default()
        };
        for card in &mut data.cards {
            if let Some(path) = &card.avatarUri {
                if path.starts_with("runtime/") || path.starts_with("workspaces/") { card.avatarUri = Some(crate::data::backup::CharacterPluginMigration::publicResourcePath(path)?); }
            }
        }
        let profiles = buildOperit1MemorySpaces(parsed)?;
        let bindings = collectOperit1CharacterMemoryProfileBindings(parsed)?;
        let now = parsed.archive.manifest.createdAt;
        let mut memory_count = 0;
        let mut link_count = 0;
        for profile in profiles.values() {
            let id = operit1SharedMemoryStoreId(&profile.id);
            let owner = format!("shared:{id}");
            data.stores.push(SharedMemoryStore {
                id,
                name: profile.name.clone(),
                createdAt: now,
                updatedAt: now,
            });
            let (settings, search) = readOperit1MemorySettings(parsed, profile)?;
            data.owners.push(ownerMetadata(&owner, settings, search));
            let mut document = Vec::new();
            parsed.copyEntryTo(&operit1UserMarkdownEntry(&profile.id), &mut document)?;
            data.documents.insert(owner.clone(), document);
            let entry = operit1ObjectBoxEntryForProfile(&profile.id);
            if let Some(metadata) = parsed.archive.entries.get(&entry) {
                let target =
                    operit_util::RuntimeStorageLayout::OPERIT1_SNAPSHOT_OBJECTBOX_IMPORT_PATH;
                writeArchiveEntryToStorage(self.storageWriteHost.as_ref(), parsed, &entry, target)?;
                let result = buildOperit1PluginMemoryRecords(
                    self.storageHost.as_ref(),
                    target,
                    metadata.uncompressedSize,
                    &owner,
                    plan,
                    archive,
                );
                self.storageHost
                    .delete(target, false)
                    .map_err(|error| error.to_string())?;
                let (mut records, memories, links) = result?;
                for (collection, _, envelope) in &mut records {
                    if collection != "memories" { continue; }
                    for field in ["documentPath", "chunkIndexFilePath"] {
                        if let Some(path) = envelope["record"][field].as_str() {
                            if path.starts_with("runtime/") || path.starts_with("workspaces/") {
                                self.storageHost.readBytes(path).map_err(|error| error.to_string())?;
                                envelope["record"][field] = serde_json::json!(crate::data::backup::CharacterPluginMigration::publicResourcePath(path)?);
                            }
                        }
                    }
                }
                data.records.extend(records);
                memory_count += memories;
                link_count += links;
            }
        }
        for card in &data.cards {
            let owner = format!("character:{}", card.id);
            let profile = bindings
                .get(&card.id)
                .ok_or_else(|| format!("Operit1 character has no profile binding: {}", card.id))?;
            let profile_metadata = profiles
                .get(profile)
                .ok_or("Operit1 character profile is missing")?;
            let (settings, search) = readOperit1MemorySettings(parsed, profile_metadata)?;
            data.owners.push(ownerMetadata(&owner, settings, search));
            let mut document = Vec::new();
            parsed.copyEntryTo(&operit1UserMarkdownEntry(profile), &mut document)?;
            data.documents.insert(owner, document);
        }
        let card_preferences = parsed
            .archive
            .datastorePreferences
            .get(ENTRY_CHARACTER_CARDS)
            .ok_or("Operit1 character preferences are missing")?;
        let group_preferences = parsed
            .archive
            .datastorePreferences
            .get(ENTRY_CHARACTER_GROUPS);
        let group_id = group_preferences
            .map(|preferences| optionalPreferenceString(preferences, "active_character_group_id"))
            .transpose()?
            .flatten()
            .filter(|id| !id.is_empty());
        data.active = Some(match group_id {
            Some(id) => serde_json::json!({"CharacterGroup":{"id":id}}),
            None => {
                serde_json::json!({"CharacterCard":{"id":requiredPreferenceString(card_preferences, "active_character_card_id", "Operit1 active character is missing")?}})
            }
        });
        writer.import(&source, data)?;
        Ok((memory_count, link_count))
    }
}

/// Resolves the legacy archive's staged name bindings through the newly stored plugin records before Core import.
fn resolveOperit1ArchiveBindings(
    archive: &mut OperitChatArchive,
    sqlite: Arc<dyn RuntimeSqliteHost>,
    storage: Arc<dyn RuntimeStorageHost>,
) -> Result<(), String> {
    use crate::data::backup::CharacterPluginMigration::{
        resolveBinding, CharacterPluginWriter, LEGACY_BINDING_NAMESPACE, PLUGIN_ID,
    };
    let writer = CharacterPluginWriter::open(sqlite, storage)?;
    let cards = writer
        .list("cards")?
        .into_iter()
        .map(|entry| entry["value"].clone())
        .collect::<Vec<_>>();
    let groups = writer
        .list("groups")?
        .into_iter()
        .map(|entry| entry["value"].clone())
        .collect::<Vec<_>>();
    for chat in &mut archive.chats {
        let old = chat
            .pluginExtensions
            .remove(LEGACY_BINDING_NAMESPACE)
            .ok_or("Operit1 archive is missing its staged character binding")?;
        chat.pluginExtensions.insert(
            PLUGIN_ID.to_string(),
            resolveBinding(&cards, &groups, &old)?,
        );
    }
    Ok(())
}

/// Reads Android's declared SharedPreferences XML values and overlays only persisted fields on initial schema settings.
fn readOperit1MemorySettings(
    parsed: &ParsedOperit1Snapshot,
    profile: &Operit1MemorySpace,
) -> Result<
    (
        crate::data::backup::CharacterPluginLegacyData::MemorySettings,
        crate::data::backup::CharacterPluginLegacyData::MemorySearchConfig,
    ),
    String,
> {
    use crate::data::backup::CharacterPluginLegacyData::{MemoryScoreMode, MemorySearchConfig};
    use crate::data::backup::CharacterPluginLegacyData::MemorySettings;
    let mut settings =
        serde_json::to_value(MemorySettings::default()).map_err(|error| error.to_string())?;
    settings["profileAutoUpdateEnabled"] = serde_json::json!(profile.profileAutoUpdateEnabled);
    settings["profileAutoUpdateLocked"] = serde_json::json!(profile.profileAutoUpdateLocked);
    let mut search =
        serde_json::to_value(MemorySearchConfig::default()).map_err(|error| error.to_string())?;
    for (name, cloud) in [
        ("memory_search_settings", false),
        ("cloud_embedding_settings", true),
    ] {
        let entry = format!("payload/shared_prefs/{name}_{}.xml", profile.id);
        let Some(metadata) = parsed.archive.entries.get(&entry) else {
            continue;
        };
        if metadata.uncompressedSize > 1024 * 1024 {
            return Err("Operit1 memory settings XML exceeds its size limit".to_string());
        }
        let mut bytes = Vec::new();
        parsed.copyEntryTo(&entry, &mut bytes)?;
        let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
        let document = roxmltree::Document::parse(text).map_err(|error| error.to_string())?;
        if document.root_element().tag_name().name() != "map" {
            return Err("Invalid Operit1 SharedPreferences root".to_string());
        }
        let mut keys = std::collections::BTreeSet::new();
        for node in document
            .root_element()
            .children()
            .filter(|node| node.is_element())
        {
            let key = node
                .attribute("name")
                .ok_or("Operit1 preference name is missing")?;
            if !keys.insert(key) {
                return Err("Duplicate Operit1 memory setting".to_string());
            }
            let value = match node.tag_name().name() {
                "string" => serde_json::json!(node
                    .children()
                    .filter(|child| child.is_text())
                    .map(|child| child.text().expect("text node"))
                    .collect::<String>()),
                "boolean" => serde_json::json!(node
                    .attribute("value")
                    .ok_or("Missing boolean setting")?
                    .parse::<bool>()
                    .map_err(|error| error.to_string())?),
                "int" | "long" => serde_json::json!(node
                    .attribute("value")
                    .ok_or("Missing integer setting")?
                    .parse::<i64>()
                    .map_err(|error| error.to_string())?),
                "float" => serde_json::json!(node
                    .attribute("value")
                    .ok_or("Missing float setting")?
                    .parse::<f32>()
                    .map_err(|error| error.to_string())?),
                _ => return Err("Unsupported Operit1 memory setting value".to_string()),
            };
            if cloud {
                let field = match key {
                    "enabled" => "cloudEmbeddingEnabled",
                    "endpoint" => "cloudEmbeddingEndpoint",
                    "api_key" => "cloudEmbeddingApiKey",
                    "model" => "cloudEmbeddingModel",
                    _ => return Err(format!("Unknown Operit1 cloud setting: {key}")),
                };
                settings[field] = value;
            } else {
                match key {
                    "score_mode" => {
                        let mode = match value.as_i64() {
                            Some(0) => MemoryScoreMode::BALANCED,
                            Some(1) => MemoryScoreMode::KEYWORD_FIRST,
                            Some(2) => MemoryScoreMode::SEMANTIC_FIRST,
                            _ => return Err("Invalid Operit1 score mode".to_string()),
                        };
                        search["scoreMode"] =
                            serde_json::to_value(mode).map_err(|error| error.to_string())?;
                    }
                    "keyword_weight" => search["keywordWeight"] = value,
                    "tag_weight" => search["tagWeight"] = value,
                    "vector_weight" => search["vectorWeight"] = value,
                    "edge_weight" => search["edgeWeight"] = value,
                    "auto_save_interval_minutes" => settings["autoSaveIntervalMinutes"] = value,
                    "next_auto_save_run_at_ms" => settings["nextAutoSaveRunAtMs"] = value,
                    "memory_extraction_custom_rules" => {
                        settings["memoryExtractionCustomRules"] = value
                    }
                    _ => return Err(format!("Unknown Operit1 memory search setting: {key}")),
                }
            }
        }
    }
    Ok((
        serde_json::from_value(settings).map_err(|error| error.to_string())?,
        serde_json::from_value(search).map_err(|error| error.to_string())?,
    ))
}
