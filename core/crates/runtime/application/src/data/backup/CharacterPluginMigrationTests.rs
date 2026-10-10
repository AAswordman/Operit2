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
    let database = SqliteStore::openWithHost(fixture.host.clone(), "runtime/candidate-source.sqlite").unwrap();
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
    source.records.push(ownerRecord("candidates", "character:legacy", record).unwrap());
    fixture.writer().import("operit2-v27", source).unwrap();
    let stored = fixture.writer().get("candidates", "character:legacy/9007199254740993").unwrap().unwrap();
    assert_eq!(stored["value"]["record"]["triggerMessageTimestamp"], 100);
    assert_eq!(stored["value"]["record"]["triggerVariantIndex"], 3);
    assert_eq!(stored["value"]["record"]["id"], "9007199254740993");
}

/// Keeps selected-user candidates exact rather than interpreting their timestamps as automatic history cutoffs.
#[test]
fn selected_candidate_requires_its_exact_user_message_revision() {
    let messages = vec![(100,4,"user".to_string()),(200,7,"ai".to_string())];
    let mut record = candidate("selected_user_message", 100);
    migrateCandidateTrigger(&mut record, &messages).unwrap();
    assert_eq!(record["triggerMessageTimestamp"], 100);
    assert_eq!(record["triggerVariantIndex"], 4);
    let mut missing = candidate("selected_user_message", 150);
    let original = missing.clone();
    assert!(migrateCandidateTrigger(&mut missing, &messages).is_err());
    assert_eq!(missing, original);
}

/// Rejects invalid histories without creating fictional messages or versions during migration.
#[test]
fn invalid_candidate_sources_remain_errors() {
    for (source_type, messages) in [
        ("reply_finalized_auto", vec![]),
        ("reply_finalized_auto", vec![(200,0,"ai".to_string())]),
        ("reply_finalized_auto", vec![(100,0,"ai".to_string()),(100,1,"ai".to_string())]),
        ("reply_finalized_auto", vec![(100,0,"user".to_string())]),
        ("reply_finalized_auto", vec![(100,-1,"ai".to_string())]),
        ("selected_user_message", vec![(150,0,"ai".to_string())]),
        ("unknown", vec![(100,0,"ai".to_string())]),
    ] {
        let mut record = candidate(source_type, 150);
        let original = record.clone();
        assert!(migrateCandidateTrigger(&mut record, &messages).is_err());
        assert_eq!(record, original);
    }
}

/// Reads released optional fields and profiles without materializing or changing the source files.
#[test]
fn legacy_readers_preserve_source_defaults_without_writing_source_files() {
    let fixture = Fixture::new();
    let root = legacyMemoryRoot("character:legacy").unwrap();
    assert!(!fixture.host.exists(&root).unwrap());
    let (settings, search) = readLegacyMemorySettings(fixture.host.clone(), "character:legacy").unwrap();
    assert_eq!(settings, MemorySettings::default());
    assert_eq!(search, MemorySearchConfig::default());
    assert_eq!(readLegacyUserMarkdown(fixture.host.as_ref(), "character:legacy").unwrap(), b"# USER\n\n");
    assert!(!fixture.host.exists(&root).unwrap());
    let user_path = format!("{root}/USER.md");
    let original = b"# Source profile\r\n  exact whitespace  \r\n";
    fixture.host.writeBytes(&user_path, original).unwrap();
    assert_eq!(readLegacyUserMarkdown(fixture.host.as_ref(), "character:legacy").unwrap(), original);
    assert_eq!(fixture.host.readBytes(&user_path).unwrap(), original);
}

/// Leaves malformed source settings visible while retaining all original preferences bytes.
#[test]
fn legacy_settings_reader_propagates_malformed_json() {
    let fixture = Fixture::new();
    let path = format!("{}/settings/memory_search_settings.preferences.json", legacyMemoryRoot("character:legacy").unwrap());
    let preferences = PreferencesDataStore::newWithStorage(fixture.host.clone(), path.clone());
    preferences.edit(|fields| fields.set(&stringPreferencesKey("memory_settings"), "{invalid".to_string())).unwrap();
    let original = fixture.host.readBytes(&path).unwrap();
    assert!(readLegacyMemorySettings(fixture.host.clone(), "character:legacy").is_err());
    assert_eq!(fixture.host.readBytes(&path).unwrap(), original);
    assert!(legacyMemoryRoot("unknown:legacy").is_err());
    assert!(legacyMemoryRoot("character:").is_err());
}
