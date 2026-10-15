//! Tests the removable migration boundary against the production SQLite and storage Hosts.
use super::*;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{
    CHARACTER_CARDS_PREFERENCES_PATH, DATA_MEMORY_CHARACTERS_DIR_PATH,
};

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

/// Builds valid source data using the same typed records as released Core preferences.
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
fn relational_owner_metadata_preserves_integer_counters_and_real_search_weights() {
    let fixture = Fixture::new();
    let value = ownerMetadata(
        "character:legacy",
        MemorySettings::default(),
        MemorySearchConfig::default(),
    );
    fixture
        .writer()
        .relational
        .put(&[(
            "owners".into(),
            "character:legacy".into(),
            value.clone(),
            None,
        )])
        .unwrap();
    let actual = fixture
        .writer()
        .get("owners", "character:legacy")
        .unwrap()
        .unwrap();
    assert_eq!(actual["value"], value);
}

/// Exercises actual bounded commits, exact identities, durable reopen, and once-only source adoption.
#[test]
fn v28_object_adoption_verifies_and_retires_business_records_once() {
    let fixture = Fixture::new();
    let writer = fixture.writer();
    let marker = writer
        .legacyGet("imports", "objects-to-relational-v1")
        .unwrap()
        .unwrap();
    let mut card = serde_json::to_value(&data().cards[0]).unwrap();
    card["themeConfigId"] = Value::Null;
    let mut source_memory = memory(12);
    let expected_card = card.clone();
    let expected_memory = source_memory.2.clone();
    // Older structured JS hosts persisted safe integer timestamps as JSON floats.
    card["createdAt"] = json!(100.0);
    card["updatedAt"] = json!(200.0);
    for field in ["createdAt", "updatedAt", "lastAccessedAt"] {
        source_memory.2["record"][field] =
            json!(source_memory.2["record"][field].as_i64().unwrap() as f64);
    }
    writer
        .request(StorageRequest::Commit {
            handle: writer.handle.clone(),
            mutations: vec![
                StorageMutation {
                    collection: "imports".into(),
                    key: "objects-to-relational-v1".into(),
                    value: Value::Null,
                    deleted: true,
                    check_version: true,
                    expected_version: Some(requiredString(&marker, "version").unwrap().into()),
                },
                StorageMutation {
                    collection: "cards".into(),
                    key: "legacy".into(),
                    value: card.clone(),
                    deleted: false,
                    check_version: true,
                    expected_version: None,
                },
                StorageMutation {
                    collection: source_memory.0.clone(),
                    key: source_memory.1.clone(),
                    value: source_memory.2.clone(),
                    deleted: false,
                    check_version: true,
                    expected_version: None,
                },
            ],
        })
        .unwrap();
    // Resume an interrupted attempt that already wrote the equivalent integer card.
    writer
        .relational
        .put(&[("cards".into(), "legacy".into(), expected_card.clone(), None)])
        .unwrap();
    let reopened = CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap();
    assert_eq!(
        reopened.get("cards", "legacy").unwrap().unwrap()["value"],
        expected_card
    );
    assert_eq!(
        reopened.get("memories", &source_memory.1).unwrap().unwrap()["value"],
        expected_memory
    );
    assert!(reopened.legacyList("cards").unwrap().is_empty());
    assert!(reopened.legacyList("memories").unwrap().is_empty());
    assert_eq!(
        reopened
            .legacyGet("imports", "objects-to-relational-v1")
            .unwrap()
            .unwrap()["value"]["complete"],
        true
    );
    card = expected_card;
    card["name"] = json!("Edited after adoption");
    reopened
        .commit(vec![reopened
            .mutation("cards", "legacy", card.clone())
            .unwrap()])
        .unwrap();
    let again = CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap();
    assert_eq!(
        again.get("cards", "legacy").unwrap().unwrap()["value"],
        card
    );
}

/// Requires explicit global flags before any fresh import can publish business data.
#[test]
fn core_rows_adopt_legacy_bindings_snapshots_and_only_committed_import_drafts() {
    let fixture = Fixture::new();
    let writer = fixture.writer();
    writer.import("operit2-v27", data()).unwrap();
    let database =
        SqliteStore::openWithHost(fixture.host.clone(), "runtime/test-core.sqlite").unwrap();
    operit_store::db::AppDatabase::createAllTables(&database).unwrap();
    for (id, extensions) in [
        (
            "bound",
            json!({PLUGIN_ID:{"version":1,"selection":"card:legacy"},"other.plugin":{"retained":true}}),
        ),
        (
            "v27",
            json!({LEGACY_BINDING_NAMESPACE:{"characterCardName":"Original","characterGroupId":null}}),
        ),
        (
            "imported",
            json!({PLUGIN_ID:{"recordId":"imported","schemaVersion":1}}),
        ),
        ("same-id", json!({})),
    ] {
        database.execute("INSERT INTO chats(id,title,createdAt,updatedAt,pluginExtensions) VALUES(?1,?1,1,1,?2)", vec![SqliteValue::Text(id.into()), SqliteValue::Text(extensions.to_string())]).unwrap();
    }
    for id in ["imported", "same-id", "uncommitted"] {
        writer
            .adoptRecord(
                "chat_drafts",
                id,
                json!({"version":1,"selection":"card:legacy"}),
            )
            .unwrap();
    }
    let profile = json!({
        "id":"legacy","name":"Historical role","avatarUri":null,"introPrompt":"Frozen prompt",
        "userPreferencesText":"Frozen USER","openingStatement":"Hello",
        "modelBinding":{"providerId":"provider","modelId":"model"},"ttsConfigId":"voice",
        "toolAccess":{"enabled":false,"allowedBuiltinTools":[],"allowedPackages":[],"allowedSkills":[],"allowedMcpServers":[]},
        "resources":[{"key":"character:legacy","readable":true,"writable":true}]
    });
    let snapshot = json!({"version":1,"selection":"card:legacy","promptFunctionType":"CHAT","primaryOwnerKey":"character:legacy","profile":profile,"participants":[profile]});
    database.execute("INSERT INTO messages(chatId,sender,timestamp,orderIndex,pluginExtensions) VALUES('bound','ai',10,0,?1)", vec![SqliteValue::Text(json!({PLUGIN_ID:snapshot,"other.plugin":{"retained":true}}).to_string())]).unwrap();
    adoptCoreConfigurationRows(&database, writer).unwrap();
    for id in ["bound", "v27", "imported"] {
        assert_eq!(
            writer.get("chat_bindings", id).unwrap().unwrap()["value"]["selection"],
            "card:legacy"
        );
    }
    assert!(writer.get("chat_bindings", "same-id").unwrap().is_none());
    assert!(writer
        .get("chat_bindings", "uncommitted")
        .unwrap()
        .is_none());
    assert!(writer.list("chat_drafts").unwrap().is_empty());
    let extensions: String = database
        .queryOne(
            "SELECT pluginExtensions FROM chats WHERE id='bound'",
            vec![],
        )
        .unwrap()
        .unwrap()
        .get(0)
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&extensions).unwrap(),
        json!({"other.plugin":{"retained":true}})
    );
    let extensions: String = database
        .queryOne(
            "SELECT pluginExtensions FROM messages WHERE chatId='bound'",
            vec![],
        )
        .unwrap()
        .unwrap()
        .get(0)
        .unwrap();
    let extensions: Value = serde_json::from_str(&extensions).unwrap();
    assert_eq!(extensions["other.plugin"]["retained"], true);
    assert_eq!(extensions[PLUGIN_ID]["schemaVersion"], 1);
    let key = requiredString(&extensions[PLUGIN_ID], "recordId").unwrap();
    assert_eq!(
        writer.get("message_snapshots", key).unwrap().unwrap()["value"],
        snapshot
    );
    adoptCoreConfigurationRows(&database, writer).unwrap();
    assert_eq!(writer.list("message_snapshots").unwrap().len(), 1);
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
    fixture.writer().import("operit2-v27", source).unwrap();
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
    assert_eq!(
        fixture.writer().get("meta", "state").unwrap().unwrap()["value"]["nextId"],
        "9007199254740994"
    );
    fixture
        .writer()
        .import("operit2-v27", CharacterMigrationData::default())
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
    fixture.writer().import("operit2-v27", source).unwrap();
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
    fixture.writer().import("operit2-v27", source).unwrap();
    let chunks = fixture.writer().list("chunks").unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0]["value"]["record"]["memoryUuid"], "uuid-1");
}

/// Confirms verified resources and records survive deletion of legacy owners and preference files.
#[test]
fn deletes_sources_only_after_verified_import_and_resumes_cleanup() {
    let fixture = Fixture::new();
    let root = format!("{DATA_MEMORY_CHARACTERS_DIR_PATH}/legacy");
    fixture
        .host
        .writeBytes(&format!("{root}/document.txt"), b"source document")
        .unwrap();
    fixture
        .host
        .writeBytes(CHARACTER_CARDS_PREFERENCES_PATH, b"source preferences")
        .unwrap();
    let mut source = data();
    source.cleanup_paths = vec![root.clone(), CHARACTER_CARDS_PREFERENCES_PATH.to_string()];
    let resource = "owners/character%3Alegacy/resources/document.txt";
    source
        .resources
        .insert(resource.to_string(), b"source document".to_vec());
    fixture.writer().import("operit2-v27", source).unwrap();
    assert!(fixture.host.exists(&root).unwrap());
    cleanupLegacyCharacters(fixture.writer()).unwrap();
    assert!(!fixture.host.exists(&root).unwrap());
    assert!(!fixture
        .host
        .exists(DATA_MEMORY_CHARACTERS_DIR_PATH)
        .unwrap());
    assert!(!fixture
        .host
        .exists(CHARACTER_CARDS_PREFERENCES_PATH)
        .unwrap());
    assert_eq!(
        fixture
            .host
            .readBytes(&format!("{}/{resource}", fixture.writer().directory))
            .unwrap(),
        b"source document"
    );
    let mut marker = fixture
        .writer()
        .get("imports", "operit2-v27")
        .unwrap()
        .unwrap()["value"]
        .clone();
    marker["cleanupComplete"] = json!(false);
    fixture
        .writer()
        .commit(vec![fixture
            .writer()
            .mutation("imports", "operit2-v27", marker)
            .unwrap()])
        .unwrap();
    cleanupLegacyCharacters(fixture.writer()).unwrap();
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27")
            .unwrap()
            .unwrap()["value"]["cleanupComplete"],
        true
    );
}

/// Rejects invalid references and incomplete migrations while retaining every source path.
#[test]
fn invalid_or_incomplete_import_never_authorizes_cleanup() {
    let fixture = Fixture::new();
    fixture
        .host
        .writeBytes(CHARACTER_CARDS_PREFERENCES_PATH, b"source preferences")
        .unwrap();
    let mut source = data();
    source.active = Some(json!({"CharacterCard":{"id":"absent"}}));
    assert!(fixture
        .writer()
        .import("operit2-v27", source)
        .unwrap_err()
        .contains("active character"));
    assert!(fixture
        .writer()
        .get("imports", "operit2-v27")
        .unwrap()
        .is_none());
    assert!(cleanupLegacyCharacters(fixture.writer()).is_err());
    assert_eq!(
        fixture
            .host
            .readBytes(CHARACTER_CARDS_PREFERENCES_PATH)
            .unwrap(),
        b"source preferences"
    );
    let mutation = fixture.writer().mutation("meta", "state", json!({"version":3,"nextId":"1","active":{"CharacterCard":{"id":"legacy"}},"migrationStatus":"writing","migrationSource":"operit2-v27"})).unwrap();
    fixture.writer().commit(vec![mutation]).unwrap();
    fixture.writer().import("operit2-v27", data()).unwrap();
    assert_eq!(
        fixture.writer().get("meta", "state").unwrap().unwrap()["value"]["migrationStatus"],
        "complete"
    );
}

/// Validates all cleanup targets before touching even an earlier allowed target.
#[test]
fn cleanup_rejects_outside_paths_before_any_deletion() {
    let fixture = Fixture::new();
    fixture
        .host
        .writeBytes(CHARACTER_CARDS_PREFERENCES_PATH, b"keep")
        .unwrap();
    let mut source = data();
    source.cleanup_paths = vec![
        CHARACTER_CARDS_PREFERENCES_PATH.to_string(),
        "/app/data/unrelated".to_string(),
    ];
    fixture.writer().import("operit2-v27", source).unwrap();
    assert!(cleanupLegacyCharacters(fixture.writer())
        .unwrap_err()
        .contains("outside"));
    assert_eq!(
        fixture
            .host
            .readBytes(CHARACTER_CARDS_PREFERENCES_PATH)
            .unwrap(),
        b"keep"
    );
}

/// Resolves only exact original names and group identities, rejecting ambiguous old names.
#[test]
fn resolves_binding_and_checks_cross_owner_resources() {
    let cards = vec![json!({"id":"one","name":"Original","isDefault":true})];
    assert_eq!(
        resolveBinding(
            &cards,
            &[],
            &json!({"characterCardName":"Original","characterGroupId":null})
        )
        .unwrap(),
        json!({"version":1,"selection":"card:one"})
    );
    assert_eq!(
        resolveBinding(
            &cards,
            &[],
            &json!({"characterCardName":null,"characterGroupId":null})
        )
        .unwrap()["selection"],
        "card:one"
    );
    assert!(resolveBinding(
        &[cards[0].clone(), cards[0].clone()],
        &[],
        &json!({"characterCardName":"Original"})
    )
    .is_err());
    assert!(validateResourcePath("../outside").is_err());
    assert_eq!(
        legacyResourceStoragePath("/app/data/data/memory/characters/legacy/doc.txt").unwrap(),
        "runtime/data/memory/characters/legacy/doc.txt"
    );
    assert_eq!(
        publicResourcePath("runtime/plugin_data/space/example/doc.txt").unwrap(),
        "/app/data/plugin_data/space/example/doc.txt"
    );
    assert_eq!(
        legacyResourceRelativePath(&format!(
            "{DATA_MEMORY_CHARACTERS_DIR_PATH}/other/document.txt"
        )),
        Some("other/document.txt")
    );
}

/// Prevents old synchronization records from recreating deleted directories while preserving ordinary preferences.
#[test]
fn synchronization_rejects_retired_sources_with_exact_boundaries() {
    let mut operation = operit_store::SyncOperationStore::SyncOperation {
        opId: "test:1".to_string(),
        originDeviceId: "test".to_string(),
        sequence: 1,
        createdAt: 100,
        schemaVersion: 1,
        semantics: operit_store::SyncOperationStore::SyncOperationSemantics::EntityState,
        domain: "runtime_file".to_string(),
        entityType: "file".to_string(),
        entityId: format!("{DATA_MEMORY_CHARACTERS_DIR_PATH}/legacy/USER.md"),
        operation: "upsert".to_string(),
        payload: json!({}),
    };
    assert!(isRetiredCharacterOperation(&operation));
    operation.entityId = format!("{DATA_MEMORY_CHARACTERS_DIR_PATH}-other/USER.md");
    assert!(!isRetiredCharacterOperation(&operation));
    operation.domain = "preferences".to_string();
    operation.payload = json!({"storagePath":CHARACTER_CARDS_PREFERENCES_PATH});
    assert!(isRetiredCharacterOperation(&operation));
    operation.payload =
        json!({"storagePath":"runtime/config/preferences/user_preferences.preferences.json"});
    assert!(!isRetiredCharacterOperation(&operation));
}

/// Builds a source queue record with an automatic cutoff distinct from the source message timestamp.
fn candidate(source_type: &str, cutoff: i64) -> Value {
    json!({
        "id":9007199254740993_i64,"chatId":"source-chat","triggerMessageTimestamp":cutoff,
        "createdAt":cutoff,"updatedAt":cutoff,"status":"processing","attemptCount":2,
        "lastError":"original error","sourceType":source_type
    })
}

/// Converts automatic history cutoffs using real SQLite revisions while preserving their extraction window.
#[test]
fn migrates_automatic_candidate_cutoff_to_the_actual_source_revision() {
    let fixture = Fixture::new();
    let database =
        SqliteStore::openWithHost(fixture.host.clone(), "runtime/candidate-source.sqlite").unwrap();
    database.executeBatch(
        "CREATE TABLE messages (chatId TEXT, timestamp INTEGER, selectedVariantIndex INTEGER, sender TEXT);
         INSERT INTO messages VALUES ('source-chat',90,0,'user'),('source-chat',100,3,'ai'),
         ('source-chat',200,4,'ai'),('other-chat',140,9,'ai');"
    ).unwrap();
    let mut record = candidate("reply_finalized_auto", 150);
    migrateStoredCandidateTrigger(&database, &mut record).unwrap();
    assert_eq!(record["triggerMessageTimestamp"], 100);
    assert_eq!(record["triggerVariantIndex"], 3);
    assert_eq!(record["status"], "processing");
    assert_eq!(record["attemptCount"], 2);
    assert_eq!(record["lastError"], "original error");
    let original_history = database.queryRows(
        "SELECT timestamp FROM messages WHERE chatId='source-chat' AND timestamp<=150 ORDER BY timestamp",
        vec![],
    ).unwrap();
    let adopted_history = database.queryRows(
        "SELECT timestamp FROM messages WHERE chatId='source-chat' AND timestamp<=100 ORDER BY timestamp",
        vec![],
    ).unwrap();
    assert_eq!(original_history, adopted_history);
    let mut source = data();
    source
        .records
        .push(ownerRecord("candidates", "character:legacy", record).unwrap());
    fixture.writer().import("operit2-v27", source).unwrap();
    let stored = fixture
        .writer()
        .get("candidates", "character:legacy/9007199254740993")
        .unwrap()
        .unwrap();
    assert_eq!(stored["value"]["record"]["triggerMessageTimestamp"], 100);
    assert_eq!(stored["value"]["record"]["triggerVariantIndex"], 3);
    assert_eq!(stored["value"]["record"]["id"], "9007199254740993");
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

/// Reads released optional fields and profiles without materializing or changing the source files.
#[test]
fn legacy_readers_preserve_source_defaults_without_writing_source_files() {
    let fixture = Fixture::new();
    let root = legacyMemoryRoot("character:legacy").unwrap();
    assert!(!fixture.host.exists(&root).unwrap());
    let (settings, search) =
        readLegacyMemorySettings(fixture.host.clone(), "character:legacy").unwrap();
    assert_eq!(settings, MemorySettings::default());
    assert_eq!(search, MemorySearchConfig::default());
    assert_eq!(
        readLegacyUserMarkdown(fixture.host.as_ref(), "character:legacy").unwrap(),
        b"# USER\n\n"
    );
    assert!(!fixture.host.exists(&root).unwrap());
    let user_path = format!("{root}/USER.md");
    let original = b"# Source profile\r\n  exact whitespace  \r\n";
    fixture.host.writeBytes(&user_path, original).unwrap();
    assert_eq!(
        readLegacyUserMarkdown(fixture.host.as_ref(), "character:legacy").unwrap(),
        original
    );
    assert_eq!(fixture.host.readBytes(&user_path).unwrap(), original);
}

/// Leaves malformed source settings visible while retaining all original preferences bytes.
#[test]
fn legacy_settings_reader_propagates_malformed_json() {
    let fixture = Fixture::new();
    let path = format!(
        "{}/settings/memory_search_settings.preferences.json",
        legacyMemoryRoot("character:legacy").unwrap()
    );
    let preferences = PreferencesDataStore::newWithStorage(fixture.host.clone(), path.clone());
    preferences
        .edit(|fields| {
            fields.set(
                &stringPreferencesKey("memory_settings"),
                "{invalid".to_string(),
            )
        })
        .unwrap();
    let original = fixture.host.readBytes(&path).unwrap();
    assert!(readLegacyMemorySettings(fixture.host.clone(), "character:legacy").is_err());
    assert_eq!(fixture.host.readBytes(&path).unwrap(), original);
    assert!(legacyMemoryRoot("unknown:legacy").is_err());
    assert!(legacyMemoryRoot("character:").is_err());
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

/// Exercises role, tag, speech, model, shared-memory and group preservation alongside one-to-one theme adoption.
#[test]
fn migrates_card_and_group_themes_without_changing_other_bindings() {
    let fixture = Fixture::new();
    let mut source = data();
    source.cards[0].attachedTagIds = vec!["tag".to_string()];
    source.cards[0].chatModelBindingMode = "FIXED_MODEL".to_string();
    source.cards[0].chatModelId = Some("model".to_string());
    source.cards[0].ttsConfigId = Some("voice".to_string());
    source.cards[0].memoryBindingMode = "SHARED".to_string();
    source.cards[0].sharedMemoryId = Some("store".to_string());
    source.cards[0].sharedMemoryMounts = vec![CharacterSharedMemoryMount {
        sharedMemoryId: "store".to_string(),
        readable: true,
        writable: false,
    }];
    source.tags.push(PromptTag {
        id: "tag".to_string(),
        name: "Tag".to_string(),
        description: "kept".to_string(),
        promptContent: "content".to_string(),
        tagType: TagType::CUSTOM,
        createdAt: 100,
        updatedAt: 200,
    });
    source.stores.push(SharedMemoryStore {
        id: "store".to_string(),
        name: "Store".to_string(),
        createdAt: 100,
        updatedAt: 200,
    });
    source.owners.push(ownerMetadata(
        "shared:store",
        MemorySettings::default(),
        MemorySearchConfig::default(),
    ));
    source.documents.insert(
        "shared:store".to_string(),
        b"# Shared source profile\n".to_vec(),
    );
    source.groups.push(CharacterGroupCard {
        id: "group".to_string(),
        name: "Group".to_string(),
        description: "kept".to_string(),
        members: vec![GroupMemberConfig {
            characterCardId: "legacy".to_string(),
            orderIndex: 7,
        }],
        createdAt: 100,
        updatedAt: 200,
    });
    source.active = Some(json!({"CharacterGroup":{"id":"group"}}));
    let mut expected_card = serde_json::to_value(&source.cards[0]).unwrap();
    let mut expected_group = serde_json::to_value(&source.groups[0]).unwrap();
    let expected_tag = serde_json::to_value(&source.tags[0]).unwrap();
    fixture.writer().import("operit2-v27", source).unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "dark".to_string(),
            );
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_font_scale"),
                "1.3".to_string(),
            );
            saved.set(
                &stringPreferencesKey("character_group_theme_group_chat_style"),
                "bubble".to_string(),
            );
            saved.set(
                &stringPreferencesKey("custom_user_avatar_uri"),
                "/app/data/global-avatar.png".to_string(),
            );
            saved.set(&stringPreferencesKey("unrelated"), "keep".to_string());
        })
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    let manager =
        crate::data::preferences::ThemeConfigManager::ThemeConfigManager::new(fixture.host.clone());
    let configs = manager.list().unwrap();
    assert_eq!(configs.len(), 2);
    let card_id = "legacy-theme:operit2-v27:cards:legacy";
    let group_id = "legacy-theme:operit2-v27:groups:group";
    expected_card["themeConfigId"] = json!(card_id);
    expected_group["themeConfigId"] = json!(group_id);
    assert_eq!(
        fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"],
        expected_card
    );
    assert_eq!(
        fixture.writer().get("groups", "group").unwrap().unwrap()["value"],
        expected_group
    );
    assert_eq!(
        fixture.writer().get("tags", "tag").unwrap().unwrap()["value"],
        expected_tag
    );
    assert_eq!(
        manager.get(card_id.to_string()).unwrap().snapshot["themeMode"],
        "dark"
    );
    assert_eq!(
        manager.get(card_id.to_string()).unwrap().snapshot["fontScale"],
        1.3
    );
    assert_eq!(
        manager.get(group_id.to_string()).unwrap().snapshot["chatStyle"],
        "bubble"
    );
    assert_eq!(
        manager.get(group_id.to_string()).unwrap().snapshot["customUserAvatarUri"],
        "/app/data/global-avatar.png"
    );
    let saved = preferences.data().unwrap();
    assert_eq!(
        saved
            .get(&stringPreferencesKey("active_theme_config_id"))
            .unwrap(),
        &json!(group_id)
    );
    assert_eq!(
        saved.get(&stringPreferencesKey("unrelated")).unwrap(),
        "keep"
    );
    assert!(saved
        .iterEntries()
        .all(|(key, _)| !key.starts_with("character_card_theme_")
            && !key.starts_with("character_group_theme_")));
}

/// Repairs the previously completed schema and leaves later role/configuration edits intact on reopen.
#[test]
fn upgrades_completed_core_theme_migration_once() {
    let fixture = Fixture::new();
    let mut source = data();
    source
        .cleanup_paths
        .push(legacyMemoryRoot("character:legacy").unwrap());
    fixture.writer().import("operit2-v27", source).unwrap();
    let mut marker = fixture
        .writer()
        .get("imports", "operit2-v27")
        .unwrap()
        .unwrap()["value"]
        .clone();
    marker.as_object_mut().unwrap().remove("themeTargets");
    marker["cleanupComplete"] = json!(true);
    fixture
        .writer()
        .commit(vec![fixture
            .writer()
            .mutation("imports", "operit2-v27", marker)
            .unwrap()])
        .unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "dark".to_string(),
            )
        })
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    let id = "legacy-theme:operit2-v27:cards:legacy";
    let manager =
        crate::data::preferences::ThemeConfigManager::ThemeConfigManager::new(fixture.host.clone());
    manager
        .rename(id.to_string(), "User renamed".to_string())
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(manager.list().unwrap().len(), 1);
    assert_eq!(manager.get(id.to_string()).unwrap().name, "User renamed");
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes-v1")
            .unwrap()
            .unwrap()["value"]["complete"],
        true
    );
}

/// Stops on invalid source data without publishing a theme plan, changing a binding or deleting source bytes.
#[test]
fn invalid_legacy_theme_stops_before_publication_and_cleanup() {
    for (key, value) in [
        ("character_card_theme_legacy_theme_mode", "invalid"),
        ("character_card_theme_legacy_font_scale", "NaN"),
        ("character_card_theme_legacy_unknown", "true"),
        ("character_card_theme_missing_theme_mode", "dark"),
    ] {
        let fixture = Fixture::new();
        fixture.writer().import("operit2-v27", data()).unwrap();
        let preferences = themePreferences(&fixture);
        preferences
            .edit(|saved| saved.set(&stringPreferencesKey(key), value.to_string()))
            .unwrap();
        crate::data::preferences::UserPreferencesManager::UserPreferencesManager::store(
            fixture.host.clone(),
        )
        .data()
        .unwrap();
        let bytes = fixture
            .host
            .readBytes(operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH)
            .unwrap();
        assert!(fixture.writer().migrateCoreThemes().is_err());
        assert_eq!(
            fixture
                .host
                .readBytes(operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH)
                .unwrap(),
            bytes
        );
        assert!(fixture
            .writer()
            .get("imports", "operit2-v27:themes-v1")
            .unwrap()
            .is_none());
        assert!(
            fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"]["themeConfigId"]
                .is_null()
        );
    }
}

/// Preserves an explicit custom appearance while creating the independent role themes.
#[test]
fn completed_migration_upgrade_preserves_explicit_custom_appearance() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("active_theme_config_id"),
                "null".to_string(),
            );
            saved.set(&stringPreferencesKey("theme_mode"), "light".to_string());
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "dark".to_string(),
            );
        })
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    let saved = preferences.data().unwrap();
    assert_eq!(
        saved.get(&stringPreferencesKey("theme_mode")).unwrap(),
        "light"
    );
    assert_eq!(
        saved
            .get(&stringPreferencesKey("active_theme_config_id"))
            .unwrap(),
        &Value::Null
    );
    let manager =
        crate::data::preferences::ThemeConfigManager::ThemeConfigManager::new(fixture.host.clone());
    assert_eq!(manager.list().unwrap()[0].snapshot["themeMode"], "dark");
}

/// Rejects conflicting independent bindings without altering source themes or user-owned configuration references.
#[test]
fn theme_migration_never_replaces_a_saved_role_binding() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let mut card = fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"].clone();
    card["themeConfigId"] = json!("user-choice");
    fixture
        .writer()
        .commit(vec![fixture
            .writer()
            .mutation("cards", "legacy", card.clone())
            .unwrap()])
        .unwrap();
    assert!(fixture
        .writer()
        .migrateCoreThemes()
        .unwrap_err()
        .contains("existing theme binding"));
    assert_eq!(
        fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"],
        card
    );
    assert!(fixture
        .writer()
        .get("imports", "operit2-v27:themes-v1")
        .unwrap()
        .is_none());
}

/// Prevents synchronization from recreating retired role theme keys while preserving ordinary theme preferences.
#[test]
fn retires_only_scoped_legacy_theme_synchronization_entries() {
    let mut operation = operit_store::SyncOperationStore::SyncOperation {
        opId: "id".to_string(),
        originDeviceId: "origin".to_string(),
        sequence: 1,
        createdAt: 1,
        domain: "preferences".to_string(),
        entityType: "entry".to_string(),
        entityId: "entry".to_string(),
        operation: "set".to_string(),
        semantics: operit_store::SyncOperationStore::SyncOperationSemantics::EntityState,
        schemaVersion: 1,
        payload: json!({"storagePath":operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH,
            "key":"character_card_theme_legacy_theme_mode","value":"dark"}),
    };
    assert!(isRetiredCharacterOperation(&operation));
    operation.payload["key"] = json!("character_group_theme_group_chat_style");
    assert!(isRetiredCharacterOperation(&operation));
    for key in [
        "theme_mode",
        "theme_configs",
        "custom_user_avatar_uri",
        "active_theme_config_id",
    ] {
        operation.payload["key"] = json!(key);
        assert!(!isRetiredCharacterOperation(&operation));
    }
}

/// Reopens a durable theme plan after a configuration-owner failure without duplicating configurations.
#[test]
fn resumes_theme_adoption_after_configuration_conflict() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("active_theme_config_id"),
                "null".to_string(),
            );
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "dark".to_string(),
            );
        })
        .unwrap();
    let manager =
        crate::data::preferences::ThemeConfigManager::ThemeConfigManager::new(fixture.host.clone());
    let conflict = crate::data::preferences::ThemeConfigManager::ThemeConfig {
        id: "legacy-theme:operit2-v27:cards:legacy".to_string(),
        name: "Conflicting source".to_string(),
        snapshot: readLegacyCharacterThemeSnapshot(
            &operit_store::PreferencesDataStore::emptyPreferences(),
            "character_card_theme_legacy_",
        )
        .unwrap()
        .0,
        createdAt: 100,
        updatedAt: 200,
    };
    manager
        .importLegacyConfigurations(&[conflict.clone()], None)
        .unwrap();
    assert!(fixture
        .writer()
        .migrateCoreThemes()
        .unwrap_err()
        .contains("different data"));
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes-v1")
            .unwrap()
            .unwrap()["value"]["complete"],
        false
    );
    assert_eq!(
        preferences
            .data()
            .unwrap()
            .get(&stringPreferencesKey(
                "character_card_theme_legacy_theme_mode"
            ))
            .unwrap(),
        "dark"
    );
    manager.delete(conflict.id).unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(manager.list().unwrap().len(), 1);
    assert_eq!(manager.list().unwrap()[0].snapshot["themeMode"], "dark");
    assert!(!preferences.data().unwrap().contains(&stringPreferencesKey(
        "character_card_theme_legacy_theme_mode"
    )));
}

/// Keeps a concurrently changed legacy preference visible instead of deleting unverified source data.
#[test]
fn theme_cleanup_rejects_source_changed_during_interruption() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("active_theme_config_id"),
                "null".to_string(),
            );
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "dark".to_string(),
            );
        })
        .unwrap();
    let manager =
        crate::data::preferences::ThemeConfigManager::ThemeConfigManager::new(fixture.host.clone());
    let conflict = crate::data::preferences::ThemeConfigManager::ThemeConfig {
        id: "legacy-theme:operit2-v27:cards:legacy".to_string(),
        name: "Conflicting source".to_string(),
        snapshot: readLegacyCharacterThemeSnapshot(
            &operit_store::PreferencesDataStore::emptyPreferences(),
            "character_card_theme_legacy_",
        )
        .unwrap()
        .0,
        createdAt: 100,
        updatedAt: 200,
    };
    manager
        .importLegacyConfigurations(&[conflict.clone()], None)
        .unwrap();
    assert!(fixture.writer().migrateCoreThemes().is_err());
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "light".to_string(),
            )
        })
        .unwrap();
    manager.delete(conflict.id).unwrap();
    assert!(fixture
        .writer()
        .migrateCoreThemes()
        .unwrap_err()
        .contains("changed before cleanup"));
    assert_eq!(
        preferences
            .data()
            .unwrap()
            .get(&stringPreferencesKey(
                "character_card_theme_legacy_theme_mode"
            ))
            .unwrap(),
        "light"
    );
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes-v1")
            .unwrap()
            .unwrap()["value"]["complete"],
        false
    );
}

/// Removes the obsolete Core memory-auto-update key only after the plugin owns a verified policy.
#[test]
fn cleanup_removes_retired_memory_policy_and_preserves_live_api_preferences() {
    let fixture = Fixture::new();
    let mut source = data();
    source.memory_preferences =
        Some(json!({"userDescriptionEnabled":true,"autoUpdateEnabled":false}));
    fixture.writer().import("operit2-v27", source).unwrap();
    let api = PreferencesDataStore::newWithStorage(fixture.host.clone(), API_PREFERENCES_PATH);
    api.edit(|saved| {
        saved.set(
            &stringPreferencesKey("enable_memory_auto_update"),
            "false".to_string(),
        );
        saved.set(
            &stringPreferencesKey("disable_user_preference_description"),
            "false".to_string(),
        );
        saved.set(&stringPreferencesKey("unrelated"), "keep".to_string());
    })
    .unwrap();
    cleanupLegacyCharacters(fixture.writer()).unwrap();
    cleanupLegacyCharacters(fixture.writer()).unwrap();
    let saved = api.data().unwrap();
    assert!(!saved.contains(&stringPreferencesKey("enable_memory_auto_update")));
    assert_eq!(
        saved
            .get(&stringPreferencesKey("disable_user_preference_description"))
            .unwrap(),
        &json!(false)
    );
    assert_eq!(
        saved.get(&stringPreferencesKey("unrelated")).unwrap(),
        "keep"
    );
    assert_eq!(
        fixture.writer().get("meta", "state").unwrap().unwrap()["value"]["memoryPreferences"]
            ["autoUpdateEnabled"],
        false
    );
}

/// Retains obsolete settings when the destination memory policy has not been verified.
#[test]
fn cleanup_keeps_unverified_memory_policy_source() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let api = PreferencesDataStore::newWithStorage(fixture.host.clone(), API_PREFERENCES_PATH);
    api.edit(|saved| {
        saved.set(
            &stringPreferencesKey("enable_memory_auto_update"),
            "false".to_string(),
        )
    })
    .unwrap();
    assert!(cleanupLegacyCharacters(fixture.writer()).is_err());
    assert_eq!(
        api.data()
            .unwrap()
            .get(&stringPreferencesKey("enable_memory_auto_update"))
            .unwrap(),
        &json!(false)
    );
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27")
            .unwrap()
            .unwrap()["value"]["cleanupComplete"],
        false
    );
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
                "/app/data/current-user.png".to_string(),
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
        "/app/data/imported/archive-background.png".to_string(),
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
        "/app/data/imported/archive-user.png".to_string(),
    );
    let targets = vec![("cards".to_string(), "legacy".to_string())];
    fixture
        .writer()
        .migrateThemes("operit1:archive:1", &source, &targets, false)
        .unwrap();
    let manager =
        crate::data::preferences::ThemeConfigManager::ThemeConfigManager::new(fixture.host.clone());
    let configs = manager.list().unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(
        configs[0].snapshot["backgroundImageUri"],
        "/app/data/imported/archive-background.png"
    );
    assert_eq!(
        configs[0].snapshot["customUserAvatarUri"],
        "/app/data/imported/archive-user.png"
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
        "/app/data/current-user.png"
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
        .migrateThemes("operit1:archive:1", &source, &targets, false)
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
            .get("imports", "operit1:archive:1:themes-v1")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
    );
}

/// A mixed transaction keeps its live preferences while the individual retired keys remain identifiable.
#[test]
fn retired_character_keys_do_not_discard_a_mixed_native_transaction() {
    let path = operit_util::RuntimeStorageLayout::USER_PREFERENCES_PATH;
    let mut operation = operit_store::SyncOperationStore::SyncOperation {
        opId: "test:1".into(),
        originDeviceId: "test".into(),
        sequence: 1,
        domain: "preferences".into(),
        entityType: "user".into(),
        entityId: serde_json::to_string(&(
            path,
            vec!["character_card_theme_old_theme_mode", "theme_mode"],
        ))
        .unwrap(),
        operation: "transaction".into(),
        semantics: operit_store::SyncOperationStore::SyncOperationSemantics::Transaction,
        payload: json!({"storagePath":path,"encrypted":false,"formatVersion":2,"entries":[
            {"key":"character_card_theme_old_theme_mode","operation":"set","value":"dark"},
            {"key":"theme_mode","operation":"set","value":"light"}]}),
        createdAt: 1,
        schemaVersion: 1,
    };
    assert!(!isRetiredCharacterOperation(&operation));
    assert!(isRetiredCharacterPreferenceKey(
        path,
        "character_card_theme_old_theme_mode"
    ));
    assert!(!isRetiredCharacterPreferenceKey(path, "theme_mode"));
    operation.payload["entries"][1]["key"] = json!("character_group_theme_old_theme_mode");
    assert!(isRetiredCharacterOperation(&operation));
}
