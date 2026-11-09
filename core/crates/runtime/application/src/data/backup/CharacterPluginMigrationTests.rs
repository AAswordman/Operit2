//! Tests direct Operit1 domain import against the production SQLite and storage Hosts.
use super::*;
use operit_host_native_storage::NativeRuntimeStorageHost;

struct Fixture {
    root: std::path::PathBuf,
    host: Arc<NativeRuntimeStorageHost>,
    writer: Option<CharacterPluginWriter>,
}
impl Fixture {
    /// Opens an isolated production Host and the actual plugin migration writer.
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "operit-character-migration-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        let writer = CharacterPluginWriter::open(host.clone(), host.clone()).unwrap();
        Self {
            root,
            host,
            writer: Some(writer),
        }
    }
    /// Returns the retained production writer without modifying its session.
    fn writer(&self) -> &CharacterPluginWriter {
        self.writer.as_ref().unwrap()
    }
}
impl Drop for Fixture {
    /// Closes SQLite before removing only the fixture's unique temporary directory.
    fn drop(&mut self) {
        self.writer.take();
        assert_eq!(self.root.parent().unwrap(), std::env::temp_dir());
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

/// Builds valid source data using the Operit1 projection records.
fn data() -> CharacterMigrationData {
    let card: CharacterCard = serde_json::from_value(json!({
        "id":"legacy","name":"Original","description":"description","characterSetting":"setting",
        "openingStatement":"opening","otherContentChat":"chat","otherContentVoice":"voice","avatarUri":null,
        "attachedTagIds":[],"advancedCustomPrompt":"advanced","marks":"marks","chatModelBindingMode":"FOLLOW_GLOBAL",
        "chatModelId":null,"ttsConfigId":null,"memoryBindingMode":"CHARACTER","sharedMemoryId":null,"sharedMemoryMounts":[],
        "toolAccessConfig":{"enabled":false,"allowedBuiltinTools":[],"allowedPackages":[],"allowedSkills":[],"allowedMcpServers":[]},
        "isDefault":true,"createdAt":100,"updatedAt":200
    })).unwrap();
    let mut source = CharacterMigrationData {
        cards: vec![card],
        active: Some(json!({"CharacterCard":{"id":"legacy"}})),
        memory_preferences: Some(json!({"userDescriptionEnabled":true,"autoUpdateEnabled":true})),
        ..CharacterMigrationData::default()
    };
    source.owners.push(ownerMetadata(
        "character:legacy",
        MemorySettings::default(),
        MemorySearchConfig::default(),
    ));
    source.documents.insert(
        "character:legacy".to_string(),
        b"# Actual source profile\n".to_vec(),
    );
    source
}

/// Creates one complete legacy memory while keeping int64 identities exact through conversion.
fn memory(id: i64) -> (String, String, Value) {
    ownerRecord("memories", "character:legacy", json!({
        "id":id,"uuid":format!("uuid-{id}"),"title":"kept","content":"content","contentType":"text","source":"source",
        "credibility":0.8,"importance":0.9,"documentPath":null,"isDocumentNode":false,"chunkIndexFilePath":null,"folderPath":"folder",
        "createdAt":100,"updatedAt":200,"lastAccessedAt":300,"tags":[{"id":id,"name":"tag"}],"properties":[{"id":id,"key":"key","value":"value"}]
    })).unwrap()
}

/// Creates one document memory so imported chunks can resolve their exact parent UUID.
fn document(id: i64) -> (String, String, Value) {
    let (collection, key, mut value) = memory(id);
    value["record"]["isDocumentNode"] = json!(true);
    (collection, key, value)
}

/// Exercises actual bounded commits, exact identities, durable reopen, and once-only source adoption.
#[test]
fn json_owner_metadata_preserves_integer_counters_and_real_search_weights() {
    let fixture = Fixture::new();
    let value = ownerMetadata(
        "character:legacy",
        MemorySettings::default(),
        MemorySearchConfig::default(),
    );
    fixture.writer().adoptRecord("owners", "character:legacy", value.clone()).unwrap();
    let actual = fixture
        .writer()
        .get("owners", "character:legacy")
        .unwrap()
        .unwrap();
    assert_eq!(actual["value"], value);
}

/// Requires explicit global flags before any fresh import can publish business data.
#[test]
fn import_without_memory_preferences_rejects_before_publication() {
    let fixture = Fixture::new();
    let mut source = data();
    source.memory_preferences = None;
    assert!(fixture.writer().import("operit1", source).is_err());
    assert!(fixture.writer().get("meta", "state").unwrap().is_none());
    assert!(fixture.writer().list("cards").unwrap().is_empty());
}

/// Exercises actual bounded commits, exact identities, durable reopen, and once-only source adoption.
#[test]
fn imports_pages_and_reopens_without_replacing_completed_data() {
    let fixture = Fixture::new();
    let mut source = data();
    for id in 1..=1005 {
        source.records.push(memory(id));
    }
    source.records.push(memory(9007199254740993));
    fixture.writer().import("operit1:fixture", source).unwrap();
    assert_eq!(fixture.writer().list("memories").unwrap().len(), 1006);
    let entry = fixture
        .writer()
        .get("memories", "character:legacy/9007199254740993")
        .unwrap()
        .unwrap();
    assert_eq!(entry["value"]["record"]["id"], "9007199254740993");
    assert_eq!(
        entry["value"]["record"]["tags"][0]["id"],
        "9007199254740993"
    );
    fixture
        .writer()
        .import("operit1:fixture", CharacterMigrationData::default())
        .unwrap();
    let reopened = CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap();
    assert_eq!(reopened.list("memories").unwrap().len(), 1006);
    assert_eq!(
        fixture
            .host
            .readBytes(&format!(
                "{}/{}",
                reopened.directory,
                ownerDocumentPath("character:legacy")
            ))
            .unwrap(),
        b"# Actual source profile\n"
    );
}

/// Builds a legacy group with explicit identities and unchanged source ordering metadata.
fn group(id: &str, members: &[(&str, i32)]) -> CharacterGroupCard {
    CharacterGroupCard {
        id: id.to_string(),
        name: "Original group".to_string(),
        description: "Retained description".to_string(),
        members: members
            .iter()
            .map(|(id, order)| GroupMemberConfig {
                characterCardId: id.to_string(),
                orderIndex: *order,
            })
            .collect(),
        createdAt: 100,
        updatedAt: 200,
    }
}

/// Repairs only missing member identities and leaves the same snapshot unchanged on a second pass.
#[test]
fn removes_missing_group_members_without_changing_surviving_records() {
    let mut source = data();
    source.groups = vec![
        group("mixed", &[("deleted", 0), ("legacy", 7), ("also-deleted", 9)]),
        group("valid", &[("legacy", 3)]),
    ];
    let valid = source.groups[1].clone();
    let mut expected = source.groups[0].clone();
    expected.members = vec![expected.members[1].clone()];
    let dropped = dropUnresolvableRecords(&mut source);
    assert_eq!(dropped.groupMembers, 2);
    assert_eq!(dropped.relationships, 0);
    assert_eq!(dropped.documentChunks, 0);
    assert_eq!(source.groups, vec![expected, valid]);
    assert_eq!(dropUnresolvableRecords(&mut source), DroppedLegacyRecords::default());
    validateMigrationData(&source).unwrap();
}

/// Imports Operit1 groups without rebinding a deleted role to a same-name replacement.
#[test]
fn imports_groups_with_deleted_characters_from_operit1() {
    let source_id = "operit1:test:100";
    let fixture = Fixture::new();
    let mut source = data();
    source.cards[0].name = "Viya".to_string();
    source.groups = vec![
        group("012f6aeb", &[("c6292a6b-b552-4d83-8491-e9f7381b24e8", 0), ("legacy", 7)]),
        group("fc7d15e6", &[("legacy", 3)]),
    ];
    source.active = Some(json!({"CharacterGroup":{"id":"012f6aeb"}}));
    let expected_active = source.active.clone().unwrap();
    let mut expected_groups = source.groups.clone();
    expected_groups[0].members.remove(0);
    fixture.writer().import(source_id, source).unwrap();
    let reopened = CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap();
    for expected in expected_groups {
        let mut value = serde_json::to_value(&expected).unwrap();
        value["themeConfigId"] = Value::Null;
        assert_eq!(reopened.get("groups", &expected.id).unwrap().unwrap()["value"], value);
    }
    assert_eq!(reopened.list("cards").unwrap().len(), 1);
    assert_eq!(reopened.get("cards", "legacy").unwrap().unwrap()["value"]["name"], "Viya");
    let metadata = reopened.get("meta", "state").unwrap().unwrap()["value"].clone();
    assert_eq!(metadata["active"], expected_active);
    assert_eq!(metadata["migrationStatus"], "complete");
    assert!(reopened.get("imports", source_id).unwrap().is_some());
    reopened.import(source_id, CharacterMigrationData::default()).unwrap();
    assert_eq!(reopened.list("groups").unwrap().len(), 2);
    assert_eq!(reopened.get("groups", "012f6aeb").unwrap().unwrap()["value"]["members"],
        json!([{"characterCardId":"legacy","orderIndex":7}]));
}

/// Retains a group's identity and active selection even when every former member was deleted.
#[test]
fn keeps_active_group_after_removing_all_deleted_members() {
    let fixture = Fixture::new();
    let mut source = data();
    source.groups.push(group("empty", &[("deleted", 0)]));
    source.active = Some(json!({"CharacterGroup":{"id":"empty"}}));
    let mut expected = serde_json::to_value(&source.groups[0]).unwrap();
    expected["members"] = json!([]);
    expected["themeConfigId"] = Value::Null;
    fixture.writer().import("operit1:fixture", source).unwrap();
    assert_eq!(fixture.writer().get("groups", "empty").unwrap().unwrap()["value"], expected);
    assert_eq!(fixture.writer().get("meta", "state").unwrap().unwrap()["value"]["active"],
        json!({"CharacterGroup":{"id":"empty"}}));
}

/// Cleans empty deleted references without preventing retained roles from being imported.
#[test]
fn empty_group_members_and_tag_references_do_not_block_import() {
    let fixture = Fixture::new();
    let mut source = data();
    source.groups.push(group("group", &[("", 0), ("deleted", 1), ("legacy", 2)]));
    source.cards[0].attachedTagIds.push("".to_string());
    fixture.writer().import("operit1:fixture", source).unwrap();
    let writer = fixture.writer();
    assert_eq!(writer.get("groups", "group").unwrap().unwrap()["value"]["members"],
        json!([{"characterCardId":"legacy","orderIndex":2}]));
    assert_eq!(writer.get("cards", "legacy").unwrap().unwrap()["value"]["attachedTagIds"], json!([]));
    assert!(writer.get("imports", "operit1:fixture").unwrap().is_some());
}

/// Keeps resolvable relationships and drops references to a legacy memory that was deleted without cleanup.
#[test]
fn drops_relationships_whose_legacy_target_was_deleted() {
    let fixture = Fixture::new();
    let mut source = data();
    source.records.push(memory(1));
    source.records.push(memory(2));
    let link = |id: i64, target: i64| {
        ownerRecord("links", "character:legacy", json!({
            "id":id,"sourceMemoryId":1,"targetMemoryId":target,"type_":"related","weight":0.5,"description":"link"
        })).unwrap()
    };
    source.records.push(link(1, 2));
    source.records.push(link(2, 99));
    fixture.writer().import("operit1:fixture", source).unwrap();
    let links = fixture.writer().list("links").unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["value"]["record"]["targetMemoryId"], "2");
    assert_eq!(fixture.writer().list("memories").unwrap().len(), 2);
}

/// Keeps chunks of imported documents and drops chunks whose legacy document was deleted.
#[test]
fn drops_chunks_whose_legacy_document_was_deleted() {
    let fixture = Fixture::new();
    let mut source = data();
    source.records.push(document(1));
    let chunk = |id: i64, uuid: &str| {
        ownerRecord(
            "chunks",
            "character:legacy",
            json!({
                "id":id,"memoryUuid":uuid,"chunkIndex":0,"content":"chunk"
            }),
        )
        .unwrap()
    };
    source.records.push(chunk(1, "uuid-1"));
    source.records.push(chunk(2, "uuid-99"));
    fixture.writer().import("operit1:fixture", source).unwrap();
    let chunks = fixture.writer().list("chunks").unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0]["value"]["record"]["memoryUuid"], "uuid-1");
}

/// Resolves unique user-defined names and group IDs without requiring a particular card or choosing another target.
#[test]
fn resolves_only_unique_legacy_binding_targets() {
    let cards = vec![json!({"id":"one","name":"User renamed card","isDefault":true})];
    assert_eq!(
        resolveBinding(
            &cards,
            &[],
            &json!({"characterCardName":"User renamed card","characterGroupId":null})
        )
        .unwrap(),
        Some(json!({"version":1,"selection":"card:one"}))
    );
    assert_eq!(
        resolveBinding(
            &cards,
            &[],
            &json!({"characterCardName":null,"characterGroupId":null})
        )
        .unwrap(),
        Some(json!({"version":1,"selection":"model:chat"}))
    );
    for name in ["Operit", "Deleted card"] {
        assert_eq!(
            resolveBinding(
                &cards,
                &[],
                &json!({"characterCardName":name,"characterGroupId":null})
            )
            .unwrap(),
            None
        );
    }
    assert_eq!(
        resolveBinding(&[], &[], &json!({"characterCardName":"Operit"})).unwrap(),
        None
    );
    let duplicate = json!({"id":"two","name":"User renamed card","isDefault":false});
    assert_eq!(
        resolveBinding(
            &[cards[0].clone(), duplicate],
            &[],
            &json!({"characterCardName":"User renamed card"})
        )
        .unwrap(),
        None
    );
    let groups = vec![json!({"id":"group","name":"User group"})];
    assert_eq!(
        resolveBinding(
            &cards,
            &groups,
            &json!({"characterCardName":"Operit","characterGroupId":"group"})
        )
        .unwrap(),
        Some(json!({"version":1,"selection":"group:group"}))
    );
    assert_eq!(
        resolveBinding(
            &cards,
            &groups,
            &json!({"characterCardName":"User renamed card","characterGroupId":"deleted-group"})
        )
        .unwrap(),
        None
    );
    assert!(resolveBinding(
        &[json!({"name":"Malformed imported card"})],
        &[],
        &json!({"characterCardName":"Malformed imported card"})
    )
    .is_err());
}

/// Builds a source queue record with an automatic cutoff distinct from the source message timestamp.
fn candidate(source_type: &str, cutoff: i64) -> Value {
    json!({
        "id":9007199254740993_i64,"chatId":"source-chat","triggerMessageTimestamp":cutoff,
        "createdAt":cutoff,"updatedAt":cutoff,"status":"processing","attemptCount":2,
        "lastError":"original error","sourceType":source_type
    })
}

/// Keeps selected-user candidates exact rather than interpreting their timestamps as automatic history cutoffs.
#[test]
fn selected_candidate_requires_its_exact_user_message_revision() {
    let messages = vec![(100, 4, "user".to_string()), (200, 7, "ai".to_string())];
    let mut record = candidate("selected_user_message", 100);
    assert_eq!(
        migrateCandidateTrigger(&mut record, &messages).unwrap(),
        CandidateSourceState::Migrated
    );
    assert_eq!(record["triggerMessageTimestamp"], 100);
    assert_eq!(record["triggerVariantIndex"], 4);
    let mut missing = candidate("selected_user_message", 150);
    let original = missing.clone();
    assert_eq!(
        migrateCandidateTrigger(&mut missing, &messages).unwrap(),
        CandidateSourceState::Unresolvable
    );
    assert_eq!(missing, original);
}

/// Reports queue records whose released history no longer resolves while malformed source types stay errors.
#[test]
fn unresolvable_candidate_sources_are_reported_while_unknown_types_stay_errors() {
    for (source_type, messages) in [
        ("reply_finalized_auto", vec![]),
        ("reply_finalized_auto", vec![(200, 0, "ai".to_string())]),
        (
            "reply_finalized_auto",
            vec![(100, 0, "ai".to_string()), (100, 1, "ai".to_string())],
        ),
        ("reply_finalized_auto", vec![(100, 0, "user".to_string())]),
        ("reply_finalized_auto", vec![(100, -1, "ai".to_string())]),
        ("selected_user_message", vec![(150, 0, "ai".to_string())]),
    ] {
        let mut record = candidate(source_type, 150);
        let original = record.clone();
        assert_eq!(
            migrateCandidateTrigger(&mut record, &messages).unwrap(),
            CandidateSourceState::Unresolvable
        );
        assert_eq!(record, original);
    }
    let mut unknown = candidate("unknown", 150);
    let original = unknown.clone();
    assert!(migrateCandidateTrigger(&mut unknown, &vec![(100, 0, "ai".to_string())]).is_err());
    assert_eq!(unknown, original);
}

/// Transfers both global flags into the sole plugin metadata and retains them across reopen/idempotent import.
#[test]
fn imports_global_memory_policy_without_a_second_runtime_preference_owner() {
    let fixture = Fixture::new();
    let mut source = data();
    source.memory_preferences =
        Some(json!({"userDescriptionEnabled":false,"autoUpdateEnabled":false}));
    fixture
        .writer()
        .import("global-memory-policy", source)
        .unwrap();
    let reopened = CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap();
    let saved =
        reopened.get("meta", "state").unwrap().unwrap()["value"]["memoryPreferences"].clone();
    assert_eq!(
        saved,
        json!({"userDescriptionEnabled":false,"autoUpdateEnabled":false})
    );
    reopened
        .import("global-memory-policy", CharacterMigrationData::default())
        .unwrap();
    assert_eq!(
        reopened.get("meta", "state").unwrap().unwrap()["value"]["memoryPreferences"],
        saved
    );
}

/// Malformed imported flags fail before the migration marker or any domain records are published.
#[test]
fn rejects_invalid_global_memory_policy_before_publication() {
    let fixture = Fixture::new();
    let mut source = data();
    source.memory_preferences =
        Some(json!({"userDescriptionEnabled":false,"autoUpdateEnabled":"false"}));
    assert!(fixture
        .writer()
        .import("invalid-global-memory-policy", source)
        .is_err());
    assert!(fixture.writer().get("meta", "state").unwrap().is_none());
    assert!(fixture.writer().list("cards").unwrap().is_empty());
    assert!(fixture
        .writer()
        .get("imports", "invalid-global-memory-policy")
        .unwrap()
        .is_none());
}

/// Opens the canonical user preference file on the fixture's production storage host.
fn themePreferences(fixture: &Fixture) -> PreferencesDataStore {
    PreferencesDataStore::newWithStorage(
        fixture.host.clone(),
        operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH,
    )
}

/// Skips missing and repeated targets without creating duplicate themes or mutating source values.
#[test]
fn theme_migration_discards_missing_and_duplicate_targets() {
    let fixture = Fixture::new();
    fixture.writer().import("operit1:fixture", data()).unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("character_card_theme_missing_theme_mode"),
                "dark".to_string(),
            );
            saved.set(
                &stringPreferencesKey("character_group_theme_missing_unknown"),
                "obsolete".to_string(),
            );
        })
        .unwrap();
    fixture
        .writer()
        .migrateThemes(
            "operit1:fixture",
            &preferences.data().unwrap(),
            &[
                ("cards".to_string(), "missing".to_string()),
                ("cards".to_string(), "legacy".to_string()),
                ("cards".to_string(), "legacy".to_string()),
                ("groups".to_string(), "missing".to_string()),
            ],
        )
        .unwrap();
    assert_eq!(
        ThemeConfigManager::new(fixture.host.clone())
            .list()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(preferences.data().unwrap().get(&stringPreferencesKey("character_card_theme_missing_theme_mode")), Some(&json!("dark")));
}

/// Adopts rewritten archive resource paths while preserving the destination's ordinary user preferences.
#[test]
fn archive_theme_adoption_uses_source_snapshot_and_keeps_destination_preferences() {
    let fixture = Fixture::new();
    fixture
        .writer()
        .import("operit1:archive:1", data())
        .unwrap();
    let destination = themePreferences(&fixture);
    destination
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("active_theme_config_id"),
                "null".to_string(),
            );
            saved.set(&stringPreferencesKey("theme_mode"), "light".to_string());
            saved.set(
                &stringPreferencesKey("custom_user_avatar_uri"),
                "/operitapp/data/current-user.png".to_string(),
            );
        })
        .unwrap();
    let before = fixture
        .host
        .readBytes(operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH)
        .unwrap();
    let mut source = operit_store::PreferencesDataStore::emptyPreferences();
    source.set(
        &stringPreferencesKey("character_card_theme_legacy_background_image_uri"),
        "/operitapp/data/imported/archive-background.png".to_string(),
    );
    source.set(
        &stringPreferencesKey("character_card_theme_legacy_use_background_image"),
        "true".to_string(),
    );
    source.set(
        &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
        "dark".to_string(),
    );
    source.set(
        &stringPreferencesKey("custom_user_avatar_uri"),
        "/operitapp/data/imported/archive-user.png".to_string(),
    );
    let targets = vec![("cards".to_string(), "legacy".to_string())];
    fixture
        .writer()
        .migrateThemes("operit1:archive:1", &source, &targets)
        .unwrap();
    let manager =
        crate::data::preferences::ThemeConfigManager::ThemeConfigManager::new(fixture.host.clone());
    let configs = manager.list().unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(
        configs[0].snapshot["backgroundImageUri"],
        "/operitapp/data/imported/archive-background.png"
    );
    assert_eq!(
        configs[0].snapshot["customUserAvatarUri"],
        "/operitapp/data/imported/archive-user.png"
    );
    let saved = destination.data().unwrap();
    assert_eq!(
        saved.get(&stringPreferencesKey("theme_mode")).unwrap(),
        "light"
    );
    assert_eq!(
        saved
            .get(&stringPreferencesKey("custom_user_avatar_uri"))
            .unwrap(),
        "/operitapp/data/current-user.png"
    );
    assert!(source.contains(&stringPreferencesKey(
        "character_card_theme_legacy_theme_mode"
    )));
    let committed = fixture
        .host
        .readBytes(operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH)
        .unwrap();
    assert_ne!(committed, before);
    fixture
        .writer()
        .migrateThemes("operit1:archive:1", &source, &targets)
        .unwrap();
    assert_eq!(
        fixture
            .host
            .readBytes(operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH)
            .unwrap(),
        committed
    );
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit1:archive:1:themes")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
    );
}

/// Publishes current plugin records without deferring identity conversion to plugin startup.
#[test]
fn current_plugin_import_is_ready_without_typescript_identity_migration() {
    let fixture = Fixture::new();
    let writer = fixture.writer();
    let metadata = json!({"version":1,"active":{"Model":{"id":"chat"}},
        "memoryPreferences":{"userDescriptionEnabled":true,"autoUpdateEnabled":true}});
    writer.commit(vec![writer.mutation("meta", "state", metadata).unwrap()]).unwrap();
    writer.import("operit1-new-source", data()).unwrap();
    let current = writer.get("meta", "state").unwrap().unwrap()["value"].clone();
    assert_eq!(current["version"], 1);
    assert!(current.get("legacyImportPending").is_none());
    assert_eq!(current["migrationStatus"], "complete");
    assert_eq!(writer.get("cards", "legacy").unwrap().unwrap()["value"]["id"], "legacy");
    assert!(writer.get("owners", "character:legacy").unwrap().is_some());
}

/// Converts the documented fixed-mode alias and preserves explicit provider/model identities in persisted records.
#[test]
fn migrates_fixed_model_modes_without_erasing_provider_bindings() {
    for mode in ["FIXED_CONFIG", "FIXED_MODEL"] {
        let fixture = Fixture::new();
        let mut source = data();
        source.cards[0].chatModelBindingMode = mode.to_string();
        source.cards[0].chatProviderId = Some("original-provider".to_string());
        source.cards[0].chatModelId = Some("original-model".to_string());
        fixture.writer().import("fixed-model-binding", source).unwrap();
        let entry = fixture.writer().get("cards", "legacy").unwrap().unwrap();
        assert_eq!(entry["value"]["chatModelBindingMode"], "FIXED_MODEL");
        assert_eq!(entry["value"]["chatProviderId"], "original-provider");
        assert_eq!(entry["value"]["chatModelId"], "original-model");
        assert_eq!(entry["value"]["characterSetting"], "setting");
    }
}

/// Retains incomplete and unknown source selections so the installed plugin can offer explicit rebinding.
#[test]
fn retains_unresolved_model_bindings_without_selecting_a_replacement() {
    for mode in ["UNRECOGNIZED_MODE", "FIXED_CONFIG", "FIXED_MODEL"] {
        let fixture = Fixture::new();
        let mut source = data();
        source.cards[0].chatModelBindingMode = mode.to_string();
        fixture.writer().import("unresolved-model-binding", source).unwrap();
        let entry = fixture.writer().get("cards", "legacy").unwrap().unwrap();
        let expected = if mode == "FIXED_CONFIG" { "FIXED_MODEL" } else { mode };
        assert_eq!(entry["value"]["chatModelBindingMode"], expected);
        assert_eq!(entry["value"]["chatProviderId"], Value::Null);
        assert_eq!(entry["value"]["chatModelId"], Value::Null);
    }
}

/// Builds a real source prompt tag whose identity is independent of its display name.
fn migrationTag(id: &str, name: &str) -> PromptTag {
    PromptTag {
        id: id.to_string(),
        name: name.to_string(),
        description: "Original tag description".to_string(),
        promptContent: "Original prompt content".to_string(),
        tagType: TagType::CUSTOM,
        createdAt: 100,
        updatedAt: 200,
    }
}

/// Removes only missing tag attachments while retaining valid order, full card content and tag records.
#[test]
fn removes_missing_tag_attachments_without_changing_surviving_records() {
    let mut source = data();
    source.tags = vec![migrationTag("tag-one", "Same name"), migrationTag("tag-two", "Same name")];
    source.cards[0].attachedTagIds = vec![
        "deleted-tag".to_string(), "tag-two".to_string(),
        "d5846b23-05bd-4f4f-984a-b097917454ce".to_string(), "tag-one".to_string(),
    ];
    let tags = source.tags.clone();
    let mut expected = source.cards[0].clone();
    expected.attachedTagIds = vec!["tag-two".to_string(), "tag-one".to_string()];
    let dropped = dropUnresolvableRecords(&mut source);
    assert_eq!(dropped.tagAttachments, 2);
    assert_eq!(dropped.sharedMemoryMounts, 0);
    assert_eq!(dropped.groupMembers, 0);
    assert_eq!(dropped.relationships, 0);
    assert_eq!(dropped.documentChunks, 0);
    assert_eq!(source.cards, vec![expected]);
    assert_eq!(source.tags, tags);
    validateMigrationData(&source).unwrap();
    assert_eq!(dropUnresolvableRecords(&mut source), DroppedLegacyRecords::default());
}

/// Imports dangling Operit1 tag references and confirms durable completion without invented tags.
#[test]
fn imports_cards_with_deleted_tags_from_operit1() {
    let source_id = "operit1:test:100";
    for retain_tag in [false, true] {
        let fixture = Fixture::new();
        let mut source = data();
        source.cards[0].attachedTagIds = vec!["d5846b23-05bd-4f4f-984a-b097917454ce".to_string()];
        if retain_tag {
            source.tags.push(migrationTag("real-tag", "Original"));
            source.cards[0].attachedTagIds.push("real-tag".to_string());
        }
        source.records.push(memory(1));
        let mut expected = serde_json::to_value(&source.cards[0]).unwrap();
        expected.as_object_mut().unwrap().remove("isDefault");
        expected["themeConfigId"] = Value::Null;
        expected["attachedTagIds"] = if retain_tag { json!(["real-tag"]) } else { json!([]) };
        fixture.writer().import(source_id, source).unwrap();
        let reopened = CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap();
        assert_eq!(reopened.get("cards", "legacy").unwrap().unwrap()["value"], expected);
        assert_eq!(reopened.list("tags").unwrap().len(), usize::from(retain_tag));
        assert_eq!(reopened.list("memories").unwrap().len(), 1);
        let metadata = reopened.get("meta", "state").unwrap().unwrap()["value"].clone();
        assert_eq!(metadata["migrationStatus"], "complete");
        assert_eq!(metadata["active"], json!({"CharacterCard":{"id":"legacy"}}));
        assert!(reopened.get("imports", source_id).unwrap().is_some());
        reopened.import(source_id, CharacterMigrationData::default()).unwrap();
        assert_eq!(reopened.get("cards", "legacy").unwrap().unwrap()["value"], expected);
    }
}

/// Prunes deleted mount targets while retaining permissions and the explicit primary library selection.
#[test]
fn removes_deleted_shared_memory_mounts_without_rebinding_primary_memory() {
    for source_id in ["operit1", "operit1:fixture"] {
        let fixture = Fixture::new();
        let mut source = data();
        source.stores.push(SharedMemoryStore {
            id: "real-store".to_string(), name: "Original store".to_string(), createdAt: 100, updatedAt: 200,
        });
        source.owners.push(ownerMetadata("shared:real-store", MemorySettings::default(), MemorySearchConfig::default()));
        source.documents.insert("shared:real-store".to_string(), b"# Original shared profile".to_vec());
        source.cards[0].memoryBindingMode = "SHARED".to_string();
        source.cards[0].sharedMemoryId = Some("real-store".to_string());
        source.cards[0].sharedMemoryMounts = vec![
            CharacterSharedMemoryMount { sharedMemoryId: "deleted-store".to_string(), readable: true, writable: true },
            CharacterSharedMemoryMount { sharedMemoryId: "real-store".to_string(), readable: true, writable: false },
        ];
        let mut expected_card = source.cards[0].clone();
        expected_card.sharedMemoryMounts.remove(0);
        let expected_store = source.stores[0].clone();
        let dropped = dropUnresolvableRecords(&mut source);
        assert_eq!(dropped.sharedMemoryMounts, 1);
        assert_eq!(dropped.tagAttachments, 0);
        assert_eq!(source.cards[0], expected_card);
        assert_eq!(source.stores, vec![expected_store]);
        validateMigrationData(&source).unwrap();
        assert_eq!(dropUnresolvableRecords(&mut source), DroppedLegacyRecords::default());
        // Exercise the import entry with the stale mount present so pruning must precede validation.
        source.cards[0].sharedMemoryMounts.insert(0, CharacterSharedMemoryMount {
            sharedMemoryId: "deleted-store".to_string(), readable: false, writable: true,
        });
        fixture.writer().import(source_id, source).unwrap();
        let entry = fixture.writer().get("cards", "legacy").unwrap().unwrap();
        assert_eq!(entry["value"]["sharedMemoryMounts"], serde_json::to_value(&expected_card.sharedMemoryMounts).unwrap());
        assert_eq!(entry["value"]["memoryBindingMode"], "SHARED");
        assert_eq!(entry["value"]["sharedMemoryId"], "real-store");
        assert_eq!(fixture.writer().list("stores").unwrap().len(), 1);
        assert!(fixture.writer().get("imports", source_id).unwrap().is_some());
    }
}

/// Cleans empty tags and mounts while preserving the card for user rebinding.
#[test]
fn empty_tag_and_mount_targets_do_not_block_import() {
    let fixture = Fixture::new();
    let mut source = data();
    source.cards[0].attachedTagIds = vec!["deleted-tag".to_string(), "".to_string()];
    source.cards[0].sharedMemoryMounts.push(CharacterSharedMemoryMount {
        sharedMemoryId: "".to_string(), readable: true, writable: false,
    });
    fixture.writer().import("operit1:fixture", source).unwrap();
    let card = fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"].clone();
    assert_eq!(card["attachedTagIds"], json!([]));
    assert_eq!(card["sharedMemoryMounts"], json!([]));
    assert!(fixture.writer().get("imports", "operit1:fixture").unwrap().is_some());
}

/// Converts supported settings and collects every obsolete key for cleanup without a retired-field allowlist.
#[test]
fn legacy_role_theme_projects_supported_fields_and_collects_arbitrary_cleanup_inputs() {
    for prefix in [
        "character_card_theme_c9cd4098-e3f9-4979-9675-fb8b486b3f9e_",
        "character_group_theme_legacy-group_",
    ] {
        let mut source = operit_store::PreferencesDataStore::emptyPreferences();
        let entries = vec![
            (
                format!("{prefix}bubble_ai_bubble_liquid_glass"),
                json!("not-a-boolean"),
            ),
            (
                format!("{prefix}unknown_string"),
                json!("{unparsed historical bytes}"),
            ),
            (format!("{prefix}unknown_boolean"), json!(true)),
            (
                format!("{prefix}unknown_object"),
                json!({"version":99,"values":[null,17,"历史"]}),
            ),
            (format!("{prefix}unknown_null"), Value::Null),
            (
                format!("{prefix}unknown_integer"),
                json!(9007199254740993_i64),
            ),
            (format!("{prefix}theme_mode"), json!("dark")),
            (
                format!("{prefix}bubble_ai_bubble_color"),
                json!("4278190335"),
            ),
            (
                format!("{prefix}input_rounded_corners_enabled"),
                json!("false"),
            ),
            (format!("{prefix}input_image_scale"), json!("1.75")),
            (
                format!("{prefix}input_image_uri"),
                json!("vfs:/original-input.png"),
            ),
        ];
        for (key, value) in &entries {
            source.set(&stringPreferencesKey(key), value.clone());
        }
        source.set(&stringPreferencesKey("unrelated"), "retained".to_string());
        let (mut snapshot, retained_entries) =
            readLegacyCharacterThemeSnapshot(&source, prefix).unwrap();
        assert_eq!(snapshot["themeMode"], "dark");
        assert_eq!(snapshot["bubbleAiBubbleColor"], json!(4278190335_u64));
        assert_eq!(snapshot["inputRoundedCornersEnabled"], false);
        assert_eq!(snapshot["inputImageScale"], json!(1.75));
        assert_eq!(snapshot["inputImageUri"], "vfs:/original-input.png");
        assert_eq!(
            retained_entries.into_iter().collect::<BTreeMap<_, _>>(),
            entries.into_iter().collect::<BTreeMap<_, _>>()
        );
        crate::data::preferences::ThemePreferenceSnapshot::validateThemePreferenceSnapshot(
            &snapshot,
        )
        .unwrap();
        // New snapshots still reject foreign fields; migration does not extend their schema.
        snapshot.insert("unknownHistoricalSetting".to_string(), json!(true));
        assert!(
            crate::data::preferences::ThemePreferenceSnapshot::validateThemePreferenceSnapshot(
                &snapshot
            )
            .is_err()
        );
        assert_eq!(
            source.get(&stringPreferencesKey("unrelated")).unwrap(),
            "retained"
        );
    }
}

/// Converts every declared color field using the same legacy scalar decoder for cards and groups.
#[test]
fn legacy_role_theme_colors_use_one_schema_conversion() {
    use crate::data::preferences::ThemePreferenceSnapshot::{snapshotFields, FieldKind};
    for prefix in [
        "character_card_theme_legacy_",
        "character_group_theme_legacy-group_",
    ] {
        for field in snapshotFields().filter(|field| matches!(field.kind, FieldKind::OptionalColor))
        {
            for (raw, expected) in [
                (json!("-16711936"), json!(4278255360_u32)),
                (json!(-1), json!(u32::MAX)),
                (json!("2147483648"), json!(2147483648_u32)),
                (json!("null"), Value::Null),
                (Value::Null, Value::Null),
            ] {
                let mut source = operit_store::PreferencesDataStore::emptyPreferences();
                source.set(
                    &stringPreferencesKey(&format!("{prefix}{}", field.preference_key)),
                    raw,
                );
                let snapshot = readLegacyCharacterThemeSnapshot(&source, prefix).unwrap().0;
                assert_eq!(snapshot[field.json_key], expected, "{}", field.json_key);
            }
        }
    }
}

/// Preserves missing primary libraries through both import sources without selecting another owner.
#[test]
fn retains_unresolved_primary_memory_bindings_without_blocking_import() {
    for source_id in ["operit1", "operit1:fixture"] {
        for mode in ["SHARED", "CHARACTER"] {
            for shared_id in [None, Some(""), Some("  "), Some("deleted-store")] {
                let fixture = Fixture::new();
                let mut source = data();
                source.cards[0].memoryBindingMode = mode.to_string();
                source.cards[0].sharedMemoryId = shared_id.map(str::to_string);
                source.records.push(memory(1));
                let mut expected = serde_json::to_value(&source.cards[0]).unwrap();
                expected.as_object_mut().unwrap().remove("isDefault");
                expected["themeConfigId"] = Value::Null;
                assert_eq!(
                    dropUnresolvableRecords(&mut source),
                    DroppedLegacyRecords::default()
                );
                validateMigrationData(&source).unwrap();
                fixture.writer().import(source_id, source).unwrap();
                let writer =
                    CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone())
                        .unwrap();
                assert_eq!(
                    writer.get("cards", "legacy").unwrap().unwrap()["value"],
                    expected
                );
                assert!(writer.list("stores").unwrap().is_empty());
                assert_eq!(writer.list("owners").unwrap().len(), 1);
                assert_eq!(writer.list("memories").unwrap().len(), 1);
                assert_eq!(
                    writer.get("meta", "state").unwrap().unwrap()["value"]["migrationStatus"],
                    "complete"
                );
                writer
                    .import(source_id, CharacterMigrationData::default())
                    .unwrap();
                assert_eq!(
                    writer.get("cards", "legacy").unwrap().unwrap()["value"],
                    expected
                );
            }
        }
    }
}

/// Retains a deleted active role without altering imported records or selecting a replacement role.
#[test]
fn retains_deleted_active_roles_from_operit1() {
    let source_id = "operit1:test:100";
    for active in [
        json!({"CharacterCard":{"id":"deleted-character"}}),
        json!({"CharacterGroup":{"id":"deleted-group"}}),
    ] {
        let fixture = Fixture::new();
        let mut source = data();
        source.active = Some(active.clone());
        source.records.push(memory(1));
        validateMigrationData(&source).unwrap();
        fixture.writer().import(source_id, source).unwrap();
        let writer =
            CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap();
        let metadata = writer.get("meta", "state").unwrap().unwrap()["value"].clone();
        assert_eq!(metadata["active"], active);
        assert_eq!(metadata["migrationStatus"], "complete");
        assert_eq!(writer.list("cards").unwrap().len(), 1);
        assert_eq!(writer.list("memories").unwrap().len(), 1);
        assert!(writer.list("groups").unwrap().is_empty());
        writer
            .import(source_id, CharacterMigrationData::default())
            .unwrap();
        assert_eq!(
            writer.get("meta", "state").unwrap().unwrap()["value"],
            metadata
        );
    }
}

/// Rejects malformed active metadata before any record is published.
#[test]
fn invalid_active_shapes_do_not_publish_a_legacy_import() {
    for source_id in ["operit1", "operit1:fixture"] {
        for active in [
            None,
            Some(Value::Null),
            Some(json!({})),
            Some(json!({"CharacterCard":{"id":""}})),
            Some(json!({"CharacterGroup":{"id":""}})),
            Some(json!({"CharacterGroup":{"id":null}})),
            Some(json!({"Unknown":{"id":"legacy"}})),
            Some(json!({"CharacterCard":{"id":"legacy"},"CharacterGroup":{"id":"group"}})),
        ] {
            let fixture = Fixture::new();
            let mut source = data();
            source.active = active.clone();
            assert!(fixture.writer().import(source_id, source).is_err(), "source={source_id}, active={active:?}");
            assert!(fixture.writer().get("meta", "state").unwrap().is_none());
            assert!(fixture.writer().get("imports", source_id).unwrap().is_none());
            assert!(fixture.writer().list("cards").unwrap().is_empty());
        }
    }
}
