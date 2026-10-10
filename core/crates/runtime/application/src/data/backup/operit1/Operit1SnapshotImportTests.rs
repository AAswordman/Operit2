use super::*;
use std::io::Cursor;
use crate::data::backup::Operit1RoomSchemaMigration::tests::TestSqliteConnection;

// These fixtures preserve the source schema; Rust tests exercise actual readers and projections when run.
const ROOM_V10_SQL: &str = r#"
CREATE TABLE chats (
    id TEXT PRIMARY KEY, title TEXT NOT NULL, createdAt INTEGER NOT NULL, updatedAt INTEGER NOT NULL,
    inputTokens INTEGER NOT NULL, outputTokens INTEGER NOT NULL, currentWindowSize INTEGER NOT NULL,
    "group" TEXT, displayOrder INTEGER NOT NULL, workspace TEXT, parentChatId TEXT,
    characterCardName TEXT, locked INTEGER NOT NULL
);
INSERT INTO chats VALUES ('source-chat', 'Legacy conversation', 1720000000000, 1720000000001,
    9223372036854775806, 12, 13, 'legacy-folder', 14,
    '/data/user/0/com.ai.assistance.operit/files/workspace/demo', 'parent-chat', 'legacy-card', 1);
CREATE TABLE messages (
    messageId TEXT PRIMARY KEY, chatId TEXT, sender TEXT, content TEXT, timestamp INTEGER,
    orderIndex INTEGER, roleName TEXT, provider TEXT, modelName TEXT
);
INSERT INTO messages VALUES ('message-1', 'source-chat', 'user', 'Preserved user history',
    1720000000002, 0, 'Historical role text', 'provider', 'model');
PRAGMA user_version = 10;
"#;

const ROOM_V20_SQL: &str = r#"
ALTER TABLE chats ADD COLUMN characterGroupId TEXT;
ALTER TABLE chats ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
UPDATE chats SET characterGroupId = 'legacy-group', pinned = 1;
ALTER TABLE messages ADD COLUMN selectedVariantIndex INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN inputTokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN outputTokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN cachedInputTokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN sentAt INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN outputDurationMs INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN waitDurationMs INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN completedAt INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN displayMode TEXT NOT NULL DEFAULT 'NORMAL';
ALTER TABLE messages ADD COLUMN isFavorite INTEGER NOT NULL DEFAULT 0;
CREATE TABLE message_variants (
    chatId TEXT, messageTimestamp INTEGER, variantIndex INTEGER, content TEXT, roleName TEXT,
    provider TEXT, modelName TEXT, inputTokens INTEGER, outputTokens INTEGER,
    cachedInputTokens INTEGER, sentAt INTEGER, outputDurationMs INTEGER,
    waitDurationMs INTEGER, completedAt INTEGER
);
PRAGMA user_version = 20;
"#;

/// Reads immutable ZIP bytes while the fixture retains its actual legacy SQLite source file.
struct SnapshotFixture {
    bytes: Vec<u8>,
    databasePath: PathBuf,
}

impl ArchiveSource for SnapshotFixture {
    /// Returns the actual immutable ZIP byte length.
    fn len(&self) -> Result<u64, String> { Ok(self.bytes.len() as u64) }

    /// Reads a checked requested range without fabricating a missing archive entry.
    fn readAt(&self, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        let start = usize::try_from(offset).map_err(|error| error.to_string())?;
        let end = start.checked_add(length).ok_or_else(|| "Fixture range overflow".to_string())?;
        if start > self.bytes.len() { return Err("Fixture range is outside the archive".to_string()); }
        Ok(self.bytes[start..end.min(self.bytes.len())].to_vec())
    }
}

impl Drop for SnapshotFixture {
    /// Removes only this fixture's uniquely allocated SQLite file after all readers are dropped.
    fn drop(&mut self) {
        std::fs::remove_file(&self.databasePath).expect("fixture SQLite cleanup must succeed");
    }
}

/// Encodes one actual length-delimited AndroidX protobuf field.
fn protobufField(bytes: &mut Vec<u8>, tag: u8, value: &[u8]) {
    bytes.push(tag);
    let mut length = value.len();
    while length >= 128 {
        bytes.push((length as u8 & 127) | 128);
        length >>= 7;
    }
    bytes.push(length as u8);
    bytes.extend_from_slice(value);
}

/// Encodes only explicitly supplied string and string-set preferences using their real source field numbers.
fn datastoreFixture(strings: &[(&str, &str)], sets: &[(&str, &[&str])]) -> Vec<u8> {
    let mut values = Vec::new();
    for (key, value) in strings {
        let mut encoded = Vec::new();
        protobufField(&mut encoded, 0x2a, value.as_bytes());
        values.push((*key, encoded));
    }
    for (key, members) in sets {
        let mut encodedSet = Vec::new();
        for value in *members { protobufField(&mut encodedSet, 0x0a, value.as_bytes()); }
        let mut encoded = Vec::new();
        protobufField(&mut encoded, 0x32, &encodedSet);
        values.push((*key, encoded));
    }
    let mut bytes = Vec::new();
    for (key, value) in values {
        let mut entry = Vec::new();
        protobufField(&mut entry, 0x0a, key.as_bytes());
        protobufField(&mut entry, 0x12, &value);
        protobufField(&mut bytes, 0x0a, &entry);
    }
    bytes
}

/// Builds a real ZIP with a real SQLite database and intact legacy DataStore, memory and user-resource entries.
fn completeSnapshotFixture(version: i32) -> Arc<SnapshotFixture> {
    assert!(matches!(version, 10 | 20 | 21));
    let databasePath = std::env::temp_dir().join(format!("operit1-snapshot-{}.db", Uuid::new_v4()));
    {
        let mut connection = TestSqliteConnection::open(&databasePath);
        connection.executeBatch(ROOM_V10_SQL);
        if matches!(version, 20 | 21) { connection.executeBatch(ROOM_V20_SQL); }
        if version == 21 { connection.executeBatch("PRAGMA user_version = 21;"); }
    }
    let entries = [
        (ENTRY_MANIFEST, br#"{"formatVersion":1,"packageName":"test.operit1","createdAt":1720000000000,"sourceOnlyField":{"retained":true}}"#.to_vec()),
        (ENTRY_MODEL_CONFIGS, datastoreFixture(&[
            ("config_list", "[\"test-model\"]"),
            ("config_test-model", "{\"id\":\"test-model\",\"name\":\"Saved model\",\"modelName\":\"model\",\"apiProviderType\":\"OPENAI\",\"apiEndpoint\":\"https://fixture.invalid/v1\",\"sourceOnlyField\":true}")
        ], &[])),
        (ENTRY_FUNCTIONAL_CONFIGS, datastoreFixture(&[("function_config_mapping", "{\"CHAT\":{\"configId\":\"test-model\",\"modelIndex\":0}}")], &[])),
        (ENTRY_CHARACTER_CARDS, datastoreFixture(&[("character_card_card-1_name", "Legacy card"), ("character_card_card-1_unknown_source_field", "preserved")], &[("character_card_list", &["card-1"])])),
        (ENTRY_CHARACTER_GROUPS, datastoreFixture(&[("character_group_group-1_data", "{\"id\":\"group-1\",\"name\":\"Legacy group\",\"members\":[{\"characterCardId\":\"card-1\",\"orderIndex\":0}],\"sourceOnlyField\":true}")], &[("character_group_list", &["group-1"])])),
        (ENTRY_PROMPT_TAGS, datastoreFixture(&[("prompt_tag_tag-1_name", "Legacy tag")], &[("prompt_tag_list", &["tag-1"])])),
        (ENTRY_SPEECH_SERVICES, datastoreFixture(&[("tts_service_type", "SIMPLE_TTS")], &[])),
        (ENTRY_USER_PREFERENCES, datastoreFixture(&[
            ("language", "zh-CN"), ("active_memory_space_id", "default"), ("memory_space_list", "[\"default\"]"),
            ("memory_space_default", "{\"id\":\"default\",\"name\":\"Legacy memories\",\"sourceOnlyField\":true}"),
            ("character_card_theme_card-1_custom_ai_avatar_uri", "file:///data/user/0/com.ai.assistance.operit/files/avatar.png")
        ], &[])),
        ("payload/shared_prefs/memory_search_settings_default.xml", br#"<map><int name="auto_save_interval_minutes" value="17"/><long name="next_auto_save_run_at_ms" value="1720000000001"/><string name="memory_extraction_custom_rules">A &amp; B</string><int name="score_mode" value="2"/><float name="vector_weight" value="3.5"/></map>"#.to_vec()),
        ("payload/shared_prefs/cloud_embedding_settings_default.xml", br#"<map><boolean name="enabled" value="true"/><string name="endpoint">https://embedding.invalid</string><string name="api_key">actual-source-key</string><string name="model">source-model</string></map>"#.to_vec()),
        (ENTRY_DATABASE, std::fs::read(&databasePath).expect("real SQLite bytes must be readable")),
        (ENTRY_OBJECTBOX_DEFAULT_DATA, b"opaque source memory database: not decoded or adopted".to_vec()),
        ("payload/files/memory-space-profiles/default/user.md", b"Legacy user document".to_vec()),
        ("payload/files/workspace/demo/USER.md", b"Workspace-owned USER.md remains a normal file".to_vec()),
        ("payload/files/attachment.txt", b"Preserved attachment".to_vec()),
        ("payload/external_files/document.txt", b"Preserved external document".to_vec()),
    ];
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    Arc::new(SnapshotFixture { bytes: writer.finish().unwrap().into_inner(), databasePath })
}

/// Executes the production source bridge and target projection using the same database bytes retained in the ZIP.
fn verifySourceProjection(version: i32, expectedPinned: bool) {
    let fixture = completeSnapshotFixture(version);
    let parsed = ParsedOperit1Snapshot::fromSource(fixture.clone()).unwrap();
    let mut stagedBytes = Vec::new();
    parsed.copyEntryTo(ENTRY_DATABASE, &mut stagedBytes).unwrap();
    assert_eq!(stagedBytes, std::fs::read(&fixture.databasePath).unwrap());
    let mut connection = TestSqliteConnection::open(&fixture.databasePath);
    let bridge = prepareOperit1RoomImport(&mut connection).unwrap();
    let mut messageCount = 0;
    let archive = buildChatArchiveFromOperit1ToOperit2DatabaseBridge(bridge, &mut connection,
        &SnapshotFileImportPlan::new(), &mut |_| messageCount += 1).unwrap();
    assert_eq!(operit1ChatDatabaseCounts(&mut connection).unwrap(), (1, 1));
    assert_eq!(messageCount, 1);
    assert_eq!(archive.chats.len(), 1);
    assert_eq!(archive.workspaces.len(), 1);
    let chat = &archive.chats[0];
    assert_eq!(chat.id, "source-chat");
    assert_eq!(chat.inputTokens, i64::MAX - 1);
    assert_eq!(chat.displayOrder, 14);
    assert_eq!(chat.parentChatId.as_deref(), Some("parent-chat"));
    assert!(chat.locked);
    assert_eq!(chat.pinned, expectedPinned);
    assert_eq!(chat.pluginExtensions[crate::data::backup::CharacterPluginMigration::LEGACY_BINDING_NAMESPACE], serde_json::json!({"characterCardName":"legacy-card","characterGroupId":if version == 10 { None } else { Some("legacy-group") }}));
    assert_eq!(chat.messages.len(), 1);
    assert!(chat.messages[0].baseMessage.pluginExtensions.is_empty());
    assert_eq!(chat.messages[0].baseMessage.roleName, "Historical role text");
    assert_eq!(chat.messages[0].baseMessage.parts[0].content, "Preserved user history");
    let target = serde_json::to_value(chat).unwrap();
    for key in ["characterCardName", "characterGroupId", "activePrompt"] {
        assert!(target.get(key).is_none(), "legacy source binding must not enter target: {key}");
    }
    assert_eq!(target["group"], "legacy-folder");
    assert!(target["pluginExtensions"].get(crate::data::backup::CharacterPluginMigration::LEGACY_BINDING_NAMESPACE).is_some());
    let preview = parsed.preview(operit1ChatDatabaseCounts(&mut connection).unwrap()).unwrap();
    assert_eq!((preview.chatCount, preview.messageCount), (1, 1));
    assert_eq!(preview.modelConfig.chatModelId.as_deref(), Some("model"));
    let speech = buildOperit2TtsConfig(&parsed).unwrap().unwrap();
    assert_eq!(speech.providerType, TtsProviderType::SYSTEM_TTS.to_string());
}

/// Preserves non-null v10 source bindings and exact column indexes without adopting those bindings into Core.
#[test]
fn room_v10_complete_snapshot_projects_chats_without_legacy_associations() { verifySourceProjection(10, false); }

/// Preserves non-null v20 source group/card columns, locked and pinned while target namespaces remain empty.
#[test]
fn room_v20_complete_snapshot_projects_chats_without_legacy_associations() { verifySourceProjection(20, true); }

/// Accepts the original v21 source version against the explicit new target schema without changing the source version.
#[test]
fn room_v21_complete_snapshot_projects_chats_without_legacy_associations() { verifySourceProjection(21, true); }

/// Keeps legacy data and unknown source JSON/preferences available to parsing and truthful preview without importing it.
#[test]
fn complete_snapshot_keeps_legacy_source_entries_and_preview_domains() {
    let fixture = completeSnapshotFixture(20);
    let parsed = ParsedOperit1Snapshot::fromSource(fixture.clone()).unwrap();
    let mut connection = TestSqliteConnection::open(&fixture.databasePath);
    let preview = parsed.preview(operit1ChatDatabaseCounts(&mut connection).unwrap()).unwrap();
    for domain in ["character_cards", "character_groups", "prompt_tags", "memory"] {
        assert!(preview.detectedDomains.iter().any(|value| value == domain));
    }
    assert_eq!(parsed.archive.datastorePreferences[ENTRY_CHARACTER_CARDS]["character_card_card-1_unknown_source_field"], Operit1PreferenceValue::String("preserved".to_string()));
    assert!(parsed.archive.entries.contains_key(ENTRY_OBJECTBOX_DEFAULT_DATA));
    assert!(parsed.archive.datastorePreferences[ENTRY_CHARACTER_GROUPS]["character_group_group-1_data"].asString().unwrap().ends_with("\"sourceOnlyField\":true}"));
    assert_eq!((preview.importedFileCount, preview.importedExternalFileCount), (2, 1));
}

/// Applies the actual resource plan and ZIP streaming copy, retaining workspace USER.md while excluding legacy memory storage.
#[test]
fn resource_copy_plan_keeps_normal_files_and_does_not_copy_legacy_domain_storage() {
    let fixture = completeSnapshotFixture(20);
    let parsed = ParsedOperit1Snapshot::fromSource(fixture).unwrap();
    let plan = buildSnapshotFileCopyPlan(&parsed).unwrap();
    assert_eq!((plan.importedFiles, plan.importedExternalFiles, plan.importedWorkspaceFiles), (1, 1, 1));
    let sources = plan.items.iter().map(|item| item.sourceEntry.clone()).collect::<Vec<_>>();
    assert_eq!(sources, vec!["payload/files/workspace/demo/USER.md", "payload/files/attachment.txt", "payload/external_files/document.txt"]);
    let mut copied = Vec::new();
    parsed.archive.copyEntriesTo(&sources, |_, name, reader| {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        copied.push((name.to_string(), bytes));
        Ok(())
    }).unwrap();
    assert_eq!(copied.len(), 3);
    assert_eq!(copied[0].1, b"Workspace-owned USER.md remains a normal file");
    assert_eq!(copied[1].1, b"Preserved attachment");
    assert_eq!(copied[2].1, b"Preserved external document");
}

/// Filters only explicitly declared legacy preference keys, never unrelated keys that happen to mention memory or characters.
#[test]
fn mixed_preferences_skip_domain_adoption_without_rejecting_or_guessing_source_keys() {
    for key in ["active_prompt_id", "active_character_card_id", "character_card_list", "character_group_group-1_data", "prompt_tag_list", "active_memory_space_id", "memory_space_list", "memory_space_default", "enable_memory_auto_update", "memory_settings"] {
        assert!(isOperit1UnadoptedPreferenceKey(key), "{key}");
    }
    for key in ["language", "model_name", "tts_http_config", "workspace_state_demo", "memory_usage_display", "custom_character_count", "new_unknown_source_key"] {
        assert!(!isOperit1UnadoptedPreferenceKey(key), "{key}");
    }
    let paths = RuntimeStorePaths::new(PathBuf::from("test-runtime"), PathBuf::from("test-workspaces"));
    let mappings = datastorePreferenceMappings(&paths);
    for entry in [ENTRY_CHARACTER_CARDS, ENTRY_CHARACTER_GROUPS, ENTRY_PROMPT_TAGS, "payload/files/datastore/persona_card_chat_history.preferences_pb"] {
        assert!(!mappings.contains_key(entry));
    }
    assert!(mappings.contains_key(ENTRY_USER_PREFERENCES));
    let fixture = completeSnapshotFixture(20);
    let parsed = ParsedOperit1Snapshot::fromSource(fixture).unwrap();
    let source = &parsed.archive.datastorePreferences[ENTRY_USER_PREFERENCES];
    let before = source.clone();
    let adopted = adoptedOperit1PreferenceEntries(source, &SnapshotFileImportPlan::new()).unwrap();
    assert_eq!(adopted, vec![("language".to_string(), "zh-CN".to_string())]);
    assert_eq!(source, &before);
    let onlyLegacy = HashMap::from([("active_memory_space_id".to_string(), Operit1PreferenceValue::String("default".to_string()))]);
    assert!(adoptedOperit1PreferenceEntries(&onlyLegacy, &SnapshotFileImportPlan::new()).unwrap().is_empty());
}

/// Propagates errors from adopted preference conversion instead of converting a failed restore to an empty-success result.
#[test]
fn adopted_preference_projection_propagates_invalid_workspace_path() {
    let source = HashMap::from([
        ("open_files_/data/user/0/com.ai.assistance.operit/files/workspace/../escape".to_string(), Operit1PreferenceValue::String("{}".to_string())),
        ("active_memory_space_id".to_string(), Operit1PreferenceValue::String("default".to_string())),
    ]);
    let result = adoptedOperit1PreferenceEntries(&source, &SnapshotFileImportPlan::new());
    assert!(result.is_err());
}

/// Uses exact source memory directories rather than deleting normal workspace and externally supplied USER.md files.
#[test]
fn memory_copy_exclusion_is_scoped_to_declared_legacy_storage_directories() {
    for entry in [ENTRY_OBJECTBOX_DEFAULT_DATA, "payload/files/objectbox_profile/data.mdb", "payload/files/memory-space-profiles/default/user.md"] {
        assert!(isOperit1LegacyMemoryEntry(entry));
        assert!(!entryMatchesCopyPrefix(entry, ENTRY_FILES_PREFIX));
    }
    for entry in ["payload/files/workspace/demo/USER.md", "payload/external_files/USER.md", "payload/files/objectbox_notes.txt", "payload/files/normal/memory-notes.md"] {
        assert!(!isOperit1LegacyMemoryEntry(entry));
    }
}

/// Parses actual source XML rather than resetting custom memory configuration during adoption.
#[test]
fn memory_settings_xml_retains_declared_preferences() {
    let fixture = completeSnapshotFixture(20);
    let parsed = ParsedOperit1Snapshot::fromSource(fixture).unwrap();
    let profiles = buildOperit1MemorySpaces(&parsed).unwrap();
    let (settings, search) = readOperit1MemorySettings(&parsed, &profiles["default"]).unwrap();
    assert_eq!(settings.autoSaveIntervalMinutes, 17);
    assert_eq!(settings.nextAutoSaveRunAtMs, 1720000000001);
    assert_eq!(settings.memoryExtractionCustomRules, "A & B");
    assert!(settings.cloudEmbeddingEnabled);
    assert_eq!(settings.cloudEmbeddingEndpoint, "https://embedding.invalid");
    assert_eq!(settings.cloudEmbeddingApiKey, "actual-source-key");
    assert_eq!(settings.cloudEmbeddingModel, "source-model");
    assert_eq!(search.scoreMode, crate::data::backup::CharacterPluginLegacyData::MemoryScoreMode::SEMANTIC_FIRST);
    assert_eq!(search.vectorWeight, 3.5);
}

/// Tests the real binary table reader's int64 fields, encoded scalar zeros and corrupt offsets.
#[test]
fn objectbox_table_preserves_long_ids_and_explicit_scalar_defaults() {
    let mut bytes = vec![0u8; 32];
    bytes[0..4].copy_from_slice(&16u32.to_le_bytes());
    bytes[4..6].copy_from_slice(&6u16.to_le_bytes());
    bytes[6..8].copy_from_slice(&16u16.to_le_bytes());
    bytes[8..10].copy_from_slice(&8u16.to_le_bytes());
    bytes[16..20].copy_from_slice(&12i32.to_le_bytes());
    bytes[24..32].copy_from_slice(&9007199254740993i64.to_le_bytes());
    let table = FlatObjectBoxTable::new(&bytes).unwrap();
    assert_eq!(table.requiredI64(0, "id").unwrap(), 9007199254740993);
    assert_eq!(table.scalarI32(2, "chunkIndex").unwrap(), 0);
    assert!(!table.scalarBool(13).unwrap());
    bytes[8..10].copy_from_slice(&40u16.to_le_bytes());
    let broken = FlatObjectBoxTable::new(&bytes).unwrap();
    assert!(broken.scalarI32(0, "chunkIndex").is_err());
}
