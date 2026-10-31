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

/// Converts the exact released namespace while retaining already current plugin data.
#[test]
fn released_core_bindings_keep_authors_and_current_namespaces() {
    let fixture = Fixture::new();
    let writer = fixture.writer();
    writer.import("operit2-v27", data()).unwrap();
    let database = SqliteStore::openWithHost(fixture.host.clone(), "runtime/test-core.sqlite").unwrap();
    operit_store::db::AppDatabase::createAllTables(&database).unwrap();
    database.execute("INSERT INTO chats(id,title,createdAt,updatedAt,pluginExtensions) VALUES('legacy','Legacy',1,1,?1)", vec![SqliteValue::Text(json!({
        LEGACY_BINDING_NAMESPACE:{"characterCardName":"Original","characterGroupId":null},
        "other.plugin":{"keep":true}
    }).to_string())]).unwrap();
    adoptCoreConfigurationRows(&database, writer).unwrap();
    assert_eq!(writer.get("chat_bindings", "legacy").unwrap().unwrap()["value"]["selection"], "card:legacy");
    let value: String = database.queryOne("SELECT pluginExtensions FROM chats WHERE id='legacy'",vec![]).unwrap().unwrap().get(0).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&value).unwrap(), json!({"other.plugin":{"keep":true}}));
    adoptCoreConfigurationRows(&database, writer).unwrap();
}

/// Cleans stale name references across live and synchronized history while leaving chats available for explicit rebinding.
#[test]
fn stale_legacy_bindings_do_not_abort_core_history_migration() {
    for (legacy, duplicate, resolved) in [
        (
            json!({"characterCardName":"Operit","characterGroupId":null}),
            false,
            false,
        ),
        (
            json!({"characterCardName":"User renamed card","characterGroupId":null}),
            true,
            false,
        ),
        (
            json!({"characterCardName":"User renamed card","characterGroupId":"deleted-group"}),
            false,
            false,
        ),
        (
            json!({"characterCardName":"User renamed card","characterGroupId":null}),
            false,
            true,
        ),
    ] {
        let fixture = Fixture::new();
        let writer = fixture.writer();
        let mut source = data();
        source.cards[0].name = "User renamed card".to_string();
        if duplicate {
            let mut second = source.cards[0].clone();
            second.id = "second-card".to_string();
            second.isDefault = false;
            source.cards.push(second);
            source.owners.push(ownerMetadata(
                "character:second-card",
                MemorySettings::default(),
                MemorySearchConfig::default(),
            ));
            source.documents.insert("character:second-card".into(), b"# Second card\n".to_vec());
        }
        writer.import("operit2-v27", source).unwrap();
        let cards = writer.list("cards").unwrap();
        let metadata = writer.get("meta", "state").unwrap().unwrap();
        let database =
            SqliteStore::openWithHost(fixture.host.clone(), "runtime/stale-bindings.sqlite")
                .unwrap();
        operit_store::db::AppDatabase::createAllTables(&database).unwrap();
        let extensions = json!({
            LEGACY_BINDING_NAMESPACE:legacy,
            "other.plugin":{"keep":true},
            "host.chat.identity":{"title":"Historical role name","avatarUri":null}
        });
        let encoded = extensions.to_string();
        database.execute("INSERT INTO chats(id,title,createdAt,updatedAt,pluginExtensions) VALUES('legacy','Kept conversation',1,1,?1)", vec![SqliteValue::Text(encoded.clone())]).unwrap();
        database.execute("INSERT INTO messages(chatId,sender,timestamp,orderIndex,roleName,pluginExtensions) VALUES('legacy','ai',10,0,'Historical role name',?1)", vec![SqliteValue::Text(encoded.clone())]).unwrap();
        database.execute("INSERT INTO message_variants(chatId,messageTimestamp,variantIndex,roleName,pluginExtensions) VALUES('legacy',10,0,'Historical role name',?1)", vec![SqliteValue::Text(encoded)]).unwrap();
        database.execute("INSERT INTO sync_sql_operations VALUES('legacy-op','test-device',1,'chat','chat','legacy','upsert','snapshot',1,7)", Vec::new()).unwrap();
        for (synchronized, live) in [
            ("sync_sql_chat_rows", "chats"),
            ("sync_sql_message_rows", "messages"),
            ("sync_sql_message_variant_rows", "message_variants"),
        ] {
            let columns = database
                .queryRows(&format!("PRAGMA table_info({synchronized})"), Vec::new())
                .unwrap()
                .into_iter()
                .map(|row| row.get::<_, String>(1).unwrap())
                .filter(|name| name != "opId")
                .map(|name| format!("\"{name}\""))
                .collect::<Vec<_>>()
                .join(",");
            database.execute(&format!("INSERT INTO {synchronized}(opId,{columns}) SELECT 'legacy-op',{columns} FROM {live}"), Vec::new()).unwrap();
        }
        adoptCoreConfigurationRows(&database, writer).unwrap();
        let binding = writer.get("chat_bindings", "legacy").unwrap();
        if resolved {
            assert_eq!(
                binding.unwrap()["value"],
                json!({"version":1,"selection":"card:legacy"})
            );
        } else {
            assert!(binding.is_none());
        }
        for table in [
            "chats",
            "messages",
            "message_variants",
            "sync_sql_chat_rows",
            "sync_sql_message_rows",
            "sync_sql_message_variant_rows",
        ] {
            let rows = database
                .queryRows(&format!("SELECT pluginExtensions FROM {table}"), Vec::new())
                .unwrap();
            assert_eq!(rows.len(), 1);
            let actual: Value =
                serde_json::from_str(&rows[0].get::<_, String>(0).unwrap()).unwrap();
            let mut expected = extensions.clone();
            expected
                .as_object_mut()
                .unwrap()
                .remove(LEGACY_BINDING_NAMESPACE);
            if resolved && table != "chats" && table != "sync_sql_chat_rows" {
                expected[PLUGIN_ID] = json!({"participantId":"legacy"});
            }
            assert_eq!(actual, expected, "wrong migration result for {table}");
        }
        assert_eq!(writer.list("cards").unwrap(), cards);
        assert_eq!(writer.get("meta", "state").unwrap().unwrap(), metadata);
        // An explicit rebind must survive subsequent startup migration passes.
        let rebound = json!({"version":1,"selection":"card:legacy"});
        writer
            .adoptRecord("chat_bindings", "legacy", rebound.clone())
            .unwrap();
        adoptCoreConfigurationRows(&database, writer).unwrap();
        assert_eq!(
            writer.get("chat_bindings", "legacy").unwrap().unwrap()["value"],
            rebound
        );
    }
}

/// Keeps a current explicit binding when an obsolete human-name reference remains in Core chat metadata.
#[test]
fn stale_legacy_names_do_not_replace_current_chat_bindings() {
    let fixture = Fixture::new();
    let writer = fixture.writer();
    writer.import("operit2-v27", data()).unwrap();
    let current = json!({"version":1,"selection":"card:legacy"});
    writer
        .adoptRecord("chat_bindings", "legacy", current.clone())
        .unwrap();
    let database =
        SqliteStore::openWithHost(fixture.host.clone(), "runtime/current-binding.sqlite").unwrap();
    operit_store::db::AppDatabase::createAllTables(&database).unwrap();
    database.execute("INSERT INTO chats(id,title,createdAt,updatedAt,pluginExtensions) VALUES('legacy','Kept',1,1,?1)", vec![SqliteValue::Text(json!({LEGACY_BINDING_NAMESPACE:{"characterCardName":"Operit","characterGroupId":null}}).to_string())]).unwrap();
    adoptCoreConfigurationRows(&database, writer).unwrap();
    assert_eq!(
        writer.get("chat_bindings", "legacy").unwrap().unwrap()["value"],
        current
    );
    let extensions: String = database
        .queryOne(
            "SELECT pluginExtensions FROM chats WHERE id='legacy'",
            Vec::new(),
        )
        .unwrap()
        .unwrap()
        .get(0)
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&extensions).unwrap(),
        json!({})
    );
}

/// Repairs both released import encodings across live and synchronized history without changing saved identities.
#[test]
fn historical_message_avatars_are_adopted_after_completed_imports() {
    let fixture = Fixture::new();
    let writer = fixture.writer();
    let mut source = data();
    source.cards[0].avatarUri = Some("/app/character/avatar.png".into());
    writer.import("operit1", source).unwrap();
    let database = SqliteStore::openWithHost(fixture.host.clone(), "runtime/avatar-history.sqlite").unwrap();
    operit_store::db::AppDatabase::createAllTables(&database).unwrap();
    database.execute("INSERT INTO chats(id,title,createdAt,updatedAt) VALUES('history','History',1,1)", vec![]).unwrap();
    let cases = [
        ("ai", "Original", json!({}), true),
        ("ai", "Historical name", json!({PLUGIN_ID:{"participantId":"legacy"},"other.plugin":{"keep":true}}), true),
        ("ai", "Original", json!({"host.chat.identity":{"title":"Saved name","avatarUri":null}}), false),
        ("user", "Original", json!({}), false),
        ("ai", "Deleted role", json!({}), false),
        ("ai", "Original", json!({PLUGIN_ID:{"participantId":"deleted-card"}}), false),
        ("ai", "Historical name", json!({LEGACY_BINDING_NAMESPACE:{"characterCardName":"Original","characterGroupId":null}}), true),
    ];
    for (index, (sender, name, extensions, _)) in cases.iter().enumerate() {
        database.execute("INSERT INTO messages(chatId,sender,timestamp,orderIndex,roleName,pluginExtensions) VALUES('history',?1,?2,?2,?3,?4)", vec![SqliteValue::Text((*sender).into()),SqliteValue::Integer(index as i64),SqliteValue::Text((*name).into()),SqliteValue::Text(extensions.to_string())]).unwrap();
        database.execute("INSERT INTO message_variants(chatId,messageTimestamp,variantIndex,roleName,pluginExtensions) VALUES('history',?1,0,?2,?3)", vec![SqliteValue::Integer(index as i64),SqliteValue::Text((*name).into()),SqliteValue::Text(extensions.to_string())]).unwrap();
    }
    database.execute("INSERT INTO sync_sql_operations VALUES('history-op','test-device',1,'chat','chat','history','upsert','snapshot',1,8)", vec![]).unwrap();
    for (synchronized, live) in [("sync_sql_message_rows","messages"),("sync_sql_message_variant_rows","message_variants")] {
        let columns = database.queryRows(&format!("PRAGMA table_info({synchronized})"), vec![]).unwrap().into_iter()
            .map(|row| row.get::<_,String>(1).unwrap()).filter(|name| name != "opId")
            .map(|name| format!("\"{name}\"")).collect::<Vec<_>>().join(",");
        database.execute(&format!("INSERT INTO {synchronized}(opId,{columns}) SELECT 'history-op',{columns} FROM {live}"), vec![]).unwrap();
    }
    for _ in 0..2 {
        adoptCoreConfigurationRows(&database, writer).unwrap();
        for (table, timestamp) in [("messages","timestamp"),("message_variants","messageTimestamp"),("sync_sql_message_rows","timestamp"),("sync_sql_message_variant_rows","messageTimestamp")] {
            let rows = database.queryRows(&format!("SELECT pluginExtensions FROM {table} ORDER BY {timestamp}"), vec![]).unwrap();
            assert_eq!(rows.len(), cases.len());
            for (row, (_, name, before, repaired)) in rows.iter().zip(&cases) {
                let actual: Value = serde_json::from_str(&row.get::<_,String>(0).unwrap()).unwrap();
                let mut expected = before.clone();
                expected.as_object_mut().unwrap().remove(LEGACY_BINDING_NAMESPACE);
                if *repaired {
                    expected[PLUGIN_ID] = json!({"participantId":"legacy"});
                    expected["host.chat.identity"] = json!({"title":name,"avatarUri":"/app/character/avatar.png"});
                }
                assert_eq!(actual, expected, "wrong historical avatar in {table}");
            }
        }
    }
}

/// Keeps ambiguous old names unassigned while exact participant IDs retain their own avatar.
#[test]
fn historical_avatar_adoption_requires_an_exact_author() {
    let cards = vec![
        json!({"id":"first","name":"Same name","avatarUri":"/app/first.png"}),
        json!({"id":"second","name":"Same name","avatarUri":"/app/second.png"}),
    ];
    let mut unnamed = BTreeMap::new();
    adoptMessageIdentity(&mut unnamed, &cards, Some("Same name")).unwrap();
    assert!(unnamed.is_empty());
    let mut identified = BTreeMap::from([(PLUGIN_ID.into(), json!({"participantId":"second"}))]);
    adoptMessageIdentity(&mut identified, &cards, Some("Historical name")).unwrap();
    assert_eq!(identified["host.chat.identity"], json!({"title":"Historical name","avatarUri":"/app/second.png"}));
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

/// Imports both legacy formats without rebinding a deleted role to a same-name replacement.
#[test]
fn imports_groups_with_deleted_characters_from_both_legacy_sources() {
    for source_id in ["operit2-v27", "operit1:test:100"] {
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
    fixture.writer().import("operit2-v27", source).unwrap();
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
    fixture.writer().import("operit2-v27", source).unwrap();
    let writer = fixture.writer();
    assert_eq!(writer.get("groups", "group").unwrap().unwrap()["value"]["members"],
        json!([{"characterCardId":"legacy","orderIndex":2}]));
    assert_eq!(writer.get("cards", "legacy").unwrap().unwrap()["value"]["attachedTagIds"], json!([]));
    assert!(writer.get("imports", "operit2-v27").unwrap().is_some());
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

/// Rejects malformed active metadata and incomplete migrations while retaining every source path.
#[test]
fn invalid_or_incomplete_import_never_authorizes_cleanup() {
    let fixture = Fixture::new();
    fixture
        .host
        .writeBytes(CHARACTER_CARDS_PREFERENCES_PATH, b"source preferences")
        .unwrap();
    let mut source = data();
    source.active = Some(json!({"CharacterCard":{"id":"legacy"},"CharacterGroup":{"id":"absent"}}));
    assert!(fixture
        .writer()
        .import("operit2-v27", source)
        .unwrap_err()
        .contains("Invalid imported active selection"));
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
    let mutation = fixture.writer().mutation("meta", "state", json!({"version":1,"active":{"CharacterCard":{"id":"legacy"}},"migrationStatus":"writing","migrationSource":"operit2-v27"})).unwrap();
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

/// Rejects invalid paths while retaining exact cross-owner resource identities.
#[test]
fn checks_cross_owner_resources() {
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
    expected_card["chatProviderId"] = Value::Null;
    expected_card.as_object_mut().unwrap().remove("isDefault");
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
    let card_id = configs.iter().find(|config| config.name == expected_card["name"].as_str().unwrap()).unwrap().id.as_str();
    let group_id = configs.iter().find(|config| config.name == expected_group["name"].as_str().unwrap()).unwrap().id.as_str();
    uuid::Uuid::parse_str(card_id).unwrap();
    uuid::Uuid::parse_str(group_id).unwrap();
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

/// Discards malformed legacy appearance without retaining old keys or creating unbound themes.
#[test]
fn invalid_legacy_theme_is_discarded_without_blocking_cleanup() {
    for (key, value) in [
        ("character_card_theme_legacy_theme_mode", "invalid"),
        ("character_card_theme_legacy_font_scale", "NaN"),
        ("character_card_theme_legacy_input_image_scale", "NaN"),
        (
            "character_card_theme_legacy_input_use_image",
            "not-a-boolean",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.writer().import("operit2-v27", data()).unwrap();
        let preferences = themePreferences(&fixture);
        preferences
            .edit(|saved| saved.set(&stringPreferencesKey(key), value.to_string()))
            .unwrap();
        fixture.writer().migrateCoreThemes().unwrap();
        assert!(
            !preferences
                .data()
                .unwrap()
                .contains(&stringPreferencesKey(key))
        );
        assert!(
            ThemeConfigManager::new(fixture.host.clone())
                .list()
                .unwrap()
                .is_empty()
        );
        assert!(
            fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"]["themeConfigId"]
                .is_null()
        );
        assert_eq!(
            fixture
                .writer()
                .get("imports", "operit2-v27:themes")
                .unwrap()
                .unwrap()["value"],
            json!({"complete":true})
        );
    }
}

/// Keeps explicit user bindings and discards their obsolete scoped appearance without creating a theme.
#[test]
fn theme_migration_never_replaces_a_saved_role_binding() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "dark".to_string(),
            )
        })
        .unwrap();
    let mut card = fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"].clone();
    card["themeConfigId"] = json!("user-choice");
    fixture
        .writer()
        .commit(vec![
            fixture
                .writer()
                .mutation("cards", "legacy", card.clone())
                .unwrap(),
        ])
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(
        fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"],
        card
    );
    assert!(
        ThemeConfigManager::new(fixture.host.clone())
            .list()
            .unwrap()
            .is_empty()
    );
    assert!(!preferences.data().unwrap().contains(&stringPreferencesKey(
        "character_card_theme_legacy_theme_mode"
    )));
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
    );
}

/// Deletes a partially published theme when its role disappears and accepts old plans containing source snapshots.
#[test]
fn interrupted_theme_migration_removes_deleted_role_artifacts() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "dark".to_string(),
            )
        })
        .unwrap();
    let allocated = persistThemePlan(&fixture);
    let mut saved_plan = fixture
        .writer()
        .get("imports", "operit2-v27:themes")
        .unwrap()
        .unwrap()["value"]
        .clone();
    saved_plan["sourceEntries"] = json!([["character_card_theme_legacy_theme_mode", "dark"]]);
    fixture
        .writer()
        .commit(vec![
            fixture
                .writer()
                .mutation("imports", "operit2-v27:themes", saved_plan.clone())
                .unwrap(),
        ])
        .unwrap();
    let plan: ThemeMigrationPlan = serde_json::from_value(saved_plan).unwrap();
    let manager = ThemeConfigManager::new(fixture.host.clone());
    manager
        .importLegacyConfigurations(&[plan.bindings[0].config.clone()], Some(&allocated), &[])
        .unwrap();
    let mut deletion = fixture
        .writer()
        .mutation("cards", "legacy", Value::Null)
        .unwrap();
    deletion.deleted = true;
    fixture.writer().commit(vec![deletion]).unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert!(manager.list().unwrap().is_empty());
    let saved = preferences.data().unwrap();
    assert_eq!(
        saved.get(&stringPreferencesKey("active_theme_config_id")),
        Some(&Value::Null)
    );
    assert!(!saved.contains(&stringPreferencesKey(
        "character_card_theme_legacy_theme_mode"
    )));
    assert!(!saved.contains(&stringPreferencesKey(&format!("theme_config_{allocated}"))));
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
    );
    fixture.writer().migrateCoreThemes().unwrap();
    assert!(manager.list().unwrap().is_empty());
}

/// Removes an unbound migration theme while retaining the user's replacement binding and independent theme.
#[test]
fn interrupted_theme_migration_cleans_artifacts_after_user_rebinding() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    persistThemePlan(&fixture);
    let plan: ThemeMigrationPlan = serde_json::from_value(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"]
            .clone(),
    )
    .unwrap();
    let manager = ThemeConfigManager::new(fixture.host.clone());
    let allocated = plan.bindings[0].config.clone();
    manager
        .importLegacyConfigurations(&[allocated.clone()], Some(&allocated.id), &[])
        .unwrap();
    let mut replacement = allocated.clone();
    replacement.id = "user-choice".to_string();
    replacement.name = "User theme".to_string();
    manager
        .importLegacyConfigurations(&[replacement.clone()], None, &[])
        .unwrap();
    let mut card = fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"].clone();
    card["themeConfigId"] = json!(replacement.id);
    fixture
        .writer()
        .commit(vec![
            fixture
                .writer()
                .mutation("cards", "legacy", card.clone())
                .unwrap(),
        ])
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(
        fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"],
        card
    );
    assert_eq!(manager.list().unwrap(), vec![replacement]);
    assert_eq!(
        themePreferences(&fixture)
            .data()
            .unwrap()
            .get(&stringPreferencesKey("active_theme_config_id")),
        Some(&Value::Null)
    );
}

/// Keeps a migration theme still referenced by another role instead of deleting a live configuration.
#[test]
fn interrupted_theme_migration_keeps_other_role_references() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let allocated = persistThemePlan(&fixture);
    let plan: ThemeMigrationPlan = serde_json::from_value(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"]
            .clone(),
    )
    .unwrap();
    let config = plan.bindings[0].config.clone();
    let manager = ThemeConfigManager::new(fixture.host.clone());
    manager
        .importLegacyConfigurations(&[config.clone()], None, &[])
        .unwrap();
    let mut card = fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"].clone();
    card["id"] = json!("other-role");
    card["themeConfigId"] = json!(allocated);
    let mut deletion = fixture
        .writer()
        .mutation("cards", "legacy", Value::Null)
        .unwrap();
    deletion.deleted = true;
    fixture
        .writer()
        .commit(vec![
            deletion,
            fixture
                .writer()
                .mutation("cards", "other-role", card)
                .unwrap(),
        ])
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(manager.list().unwrap(), vec![config]);
    assert_eq!(
        fixture
            .writer()
            .get("cards", "other-role")
            .unwrap()
            .unwrap()["value"]["themeConfigId"],
        allocated
    );
}

/// Skips missing and repeated targets without creating duplicate themes or retaining their old namespaces.
#[test]
fn theme_migration_discards_missing_and_duplicate_targets() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
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
            "operit2-v27",
            &preferences.data().unwrap(),
            &[
                ("cards".to_string(), "missing".to_string()),
                ("cards".to_string(), "legacy".to_string()),
                ("cards".to_string(), "legacy".to_string()),
                ("groups".to_string(), "missing".to_string()),
            ],
            true,
        )
        .unwrap();
    assert_eq!(
        ThemeConfigManager::new(fixture.host.clone())
            .list()
            .unwrap()
            .len(),
        1
    );
    assert!(
        !preferences
            .data()
            .unwrap()
            .iterEntries()
            .any(|(key, _)| key.starts_with("character_card_theme_")
                || key.starts_with("character_group_theme_"))
    );
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

/// Completes migration while preserving a conflicting named theme and leaving its role unbound.
#[test]
fn theme_configuration_conflict_does_not_block_cleanup() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "dark".to_string(),
            )
        })
        .unwrap();
    let allocated = persistThemePlan(&fixture);
    let manager = ThemeConfigManager::new(fixture.host.clone());
    let conflict = ThemeConfig {
        id: allocated,
        name: "User edited theme".to_string(),
        snapshot: readLegacyCharacterThemeSnapshot(
            &operit_store::PreferencesDataStore::emptyPreferences(),
            "character_card_theme_legacy_",
        )
        .unwrap()
        .0,
        createdAt: 100,
        updatedAt: 300,
    };
    manager
        .importLegacyConfigurations(&[conflict.clone()], None, &[])
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(manager.list().unwrap(), vec![conflict]);
    assert!(
        fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"]["themeConfigId"]
            .is_null()
    );
    assert!(!preferences.data().unwrap().contains(&stringPreferencesKey(
        "character_card_theme_legacy_theme_mode"
    )));
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
    );
}

/// Uses the durable snapshot and removes changed and newly added obsolete keys after an interruption.
#[test]
fn theme_cleanup_discards_source_changes_during_interruption() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let preferences = themePreferences(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "dark".to_string(),
            )
        })
        .unwrap();
    let allocated = persistThemePlan(&fixture);
    preferences
        .edit(|saved| {
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                "light".to_string(),
            );
            saved.set(
                &stringPreferencesKey("character_group_theme_deleted_unknown"),
                json!({"obsolete":true}),
            );
            saved.set(
                &stringPreferencesKey("unrelated_setting"),
                "keep".to_string(),
            );
        })
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(
        ThemeConfigManager::new(fixture.host.clone())
            .get(allocated)
            .unwrap()
            .snapshot["themeMode"],
        "dark"
    );
    let saved = preferences.data().unwrap();
    assert!(
        !saved
            .iterEntries()
            .any(|(key, _)| key.starts_with("character_card_theme_")
                || key.starts_with("character_group_theme_"))
    );
    assert_eq!(
        saved.get(&stringPreferencesKey("unrelated_setting")),
        Some(&json!("keep"))
    );
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
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
            .get("imports", "operit1:archive:1:themes")
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

/// Marks version-one source records for the TS upgrader without changing a current plugin schema.
#[test]
fn current_plugin_import_delegates_legacy_identity_upgrade_to_typescript() {
    let fixture = Fixture::new();
    let writer = fixture.writer();
    let metadata = json!({"version":1,"active":{"Model":{"id":"chat"}},
        "memoryPreferences":{"userDescriptionEnabled":true,"autoUpdateEnabled":true}});
    writer.commit(vec![writer.mutation("meta", "state", metadata).unwrap()]).unwrap();
    writer.import("operit1-new-source", data()).unwrap();
    let current = writer.get("meta", "state").unwrap().unwrap()["value"].clone();
    assert_eq!(current["version"], 1);
    assert_eq!(current["legacyImportPending"], true);
    assert_eq!(current["migrationStatus"], "complete");
    assert_eq!(writer.get("cards", "legacy").unwrap().unwrap()["value"]["id"], "legacy");
    assert!(writer.get("owners", "character:legacy").unwrap().is_some());
}

/// Refuses source adoption while the TS migration owns the canonical record namespace.
#[test]
fn source_import_cannot_write_during_typescript_schema_upgrade() {
    let fixture = Fixture::new();
    let writer = fixture.writer();
    let metadata = json!({"version":1,"active":{"Model":{"id":"chat"}},
        "memoryPreferences":{"userDescriptionEnabled":true,"autoUpdateEnabled":true}});
    writer.commit(vec![writer.mutation("meta", "state", metadata.clone()).unwrap()]).unwrap();
    PreferencesDataStore::newWithStorage(fixture.host.clone(), format!("{}/migration.preferences.json", writer.directory))
        .edit(|preferences| preferences.set(&stringPreferencesKey("plan"), json!({"from":"released","to":1}))).unwrap();
    assert!(writer.import("blocked-source", data()).unwrap_err().contains("legacy import is incomplete"));
    assert_eq!(writer.get("meta", "state").unwrap().unwrap()["value"], metadata);
    assert!(writer.get("cards", "legacy").unwrap().is_none());
}

/// Publishes an interrupted theme plan before injecting a conflicting destination record.
fn persistThemePlan(fixture: &Fixture) -> String {
    let writer = fixture.writer();
    let source = writer.get("imports", "operit2-v27").unwrap().unwrap();
    let targets: Vec<(String, String)> = serde_json::from_value(source["value"]["themeTargets"].clone()).unwrap();
    let plan = writer.planThemes(&themePreferences(fixture).data().unwrap(), &targets).unwrap();
    let identity = plan.bindings[0].config.id.clone();
    writer.commit(vec![writer.mutation("imports", "operit2-v27:themes", serde_json::to_value(plan).unwrap()).unwrap()]).unwrap();
    identity
}

/// Preserves edited named themes retained alongside restored v27 role preferences.
#[test]
fn restored_legacy_theme_labels_do_not_occupy_new_theme_identities() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let manager = ThemeConfigManager::new(fixture.host.clone());
    let retained = ThemeConfig {
        id: "legacy-theme:operit2-v27:cards:legacy".to_string(),
        name: "Retained edited theme".to_string(),
        snapshot: readLegacyCharacterThemeSnapshot(&operit_store::PreferencesDataStore::emptyPreferences(), "character_card_theme_legacy_").unwrap().0,
        createdAt: 100,
        updatedAt: 300,
    };
    manager.importLegacyConfigurations(&[retained.clone()], None, &[]).unwrap();
    themePreferences(&fixture).edit(|saved| {
        saved.set(&stringPreferencesKey("character_card_theme_legacy_theme_mode"), "dark".to_string());
    }).unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(manager.get(retained.id.clone()).unwrap(), retained);
    let role = fixture.writer().get("cards", "legacy").unwrap().unwrap();
    let migrated_id = role["value"]["themeConfigId"].as_str().unwrap();
    uuid::Uuid::parse_str(migrated_id).unwrap();
    assert_eq!(manager.get(migrated_id.to_string()).unwrap().snapshot["themeMode"], "dark");
    let committed = manager.list().unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(manager.list().unwrap(), committed);
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

/// Imports dangling tag references from both released sources and confirms durable completion without invented tags.
#[test]
fn imports_cards_with_deleted_tags_from_both_legacy_sources() {
    for source_id in ["operit1", "operit2-v27"] {
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
}

/// Prunes deleted mount targets while retaining permissions and the explicit primary library selection.
#[test]
fn removes_deleted_shared_memory_mounts_without_rebinding_primary_memory() {
    for source_id in ["operit1", "operit2-v27"] {
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
    fixture.writer().import("operit2-v27", source).unwrap();
    let card = fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"].clone();
    assert_eq!(card["attachedTagIds"], json!([]));
    assert_eq!(card["sharedMemoryMounts"], json!([]));
    assert!(fixture.writer().get("imports", "operit2-v27").unwrap().is_some());
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

/// Removes obsolete fields and temporary plans after verified adoption without duplicating themes on restart.
#[test]
fn migrates_supported_role_themes_and_discards_obsolete_fields() {
    for (source_id, clean_preferences) in [
        ("operit2-v27", true),
        ("operit1:archive:no-cleanup", false),
        ("operit1:archive:cleanup", true),
    ] {
        let mut fixture = Fixture::new();
        let mut source = data();
        source.groups.push(group("legacy-group", &[("legacy", 0)]));
        fixture.writer().import(source_id, source).unwrap();
        let preferences = themePreferences(&fixture);
        let entries = vec![
            (
                "character_card_theme_legacy_bubble_ai_bubble_liquid_glass".to_string(),
                json!("true"),
            ),
            (
                "character_group_theme_legacy-group_bubble_user_bubble_liquid_glass".to_string(),
                json!("false"),
            ),
            (
                "character_card_theme_legacy_unknown".to_string(),
                json!({"unparsed":"historical","items":[null,7]}),
            ),
            (
                "character_group_theme_legacy-group_unknown".to_string(),
                json!(9007199254740993_i64),
            ),
            (
                "character_card_theme_legacy_theme_mode".to_string(),
                json!("dark"),
            ),
            (
                "character_group_theme_legacy-group_font_scale".to_string(),
                json!("1.1"),
            ),
            (
                "character_card_theme_legacy_input_use_image".to_string(),
                json!("false"),
            ),
        ];
        preferences
            .edit(|saved| {
                for (key, value) in &entries {
                    saved.set(&stringPreferencesKey(key), value.clone());
                }
                saved.set(&stringPreferencesKey("unrelated"), "keep".to_string());
            })
            .unwrap();
        let source_snapshot = preferences.data().unwrap();
        let targets = vec![
            ("cards".to_string(), "legacy".to_string()),
            ("groups".to_string(), "legacy-group".to_string()),
        ];
        if source_id == "operit2-v27" {
            fixture.writer().migrateCoreThemes().unwrap();
        } else {
            fixture
                .writer()
                .migrateThemes(source_id, &source_snapshot, &targets, clean_preferences)
                .unwrap();
        }
        let card = fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"].clone();
        let group = fixture
            .writer()
            .get("groups", "legacy-group")
            .unwrap()
            .unwrap()["value"]
            .clone();
        let manager = ThemeConfigManager::new(fixture.host.clone());
        let card_config = manager
            .get(card["themeConfigId"].as_str().unwrap().to_string())
            .unwrap();
        let group_config = manager
            .get(group["themeConfigId"].as_str().unwrap().to_string())
            .unwrap();
        assert_eq!(card_config.snapshot["themeMode"], "dark");
        assert_eq!(card_config.snapshot["inputUseImage"], false);
        assert_eq!(group_config.snapshot["fontScale"], json!(1.1));
        let marker_key = format!("{source_id}:themes");
        let retained = fixture
            .writer()
            .get("imports", &marker_key)
            .unwrap()
            .unwrap()["value"]
            .clone();
        assert_eq!(retained, json!({"complete":true}));
        let saved = preferences.data().unwrap();
        for (key, value) in &entries {
            assert_eq!(
                saved.get(&stringPreferencesKey(key)),
                if clean_preferences { None } else { Some(value) }
            );
        }
        assert_eq!(
            saved.get(&stringPreferencesKey("unrelated")).unwrap(),
            "keep"
        );
        let configs = manager.list().unwrap();
        assert_eq!(configs.len(), 2);
        fixture.writer.take();
        fixture.writer =
            Some(CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap());
        if source_id == "operit2-v27" {
            fixture.writer().migrateCoreThemes().unwrap();
        } else {
            fixture
                .writer()
                .migrateThemes(source_id, &saved, &targets, clean_preferences)
                .unwrap();
        }
        assert_eq!(
            fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"],
            card
        );
        assert_eq!(
            fixture
                .writer()
                .get("groups", "legacy-group")
                .unwrap()
                .unwrap()["value"],
            group
        );
        assert_eq!(
            fixture
                .writer()
                .get("imports", &marker_key)
                .unwrap()
                .unwrap()["value"],
            retained
        );
        assert_eq!(manager.list().unwrap(), configs);
    }
}

/// Resumes interrupted cleanup with stable theme identities and deletes the temporary migration inputs on completion.
#[test]
fn resumes_theme_plan_across_cleanup_and_discards_temporary_inputs() {
    for after_cleanup in [false, true] {
        let mut fixture = Fixture::new();
        fixture.writer().import("operit2-v27", data()).unwrap();
        let preferences = themePreferences(&fixture);
        preferences
            .edit(|saved| {
                saved.set(
                    &stringPreferencesKey("character_card_theme_legacy_unknown"),
                    json!({"bytes":"retained"}),
                );
                saved.set(
                    &stringPreferencesKey("character_card_theme_legacy_theme_mode"),
                    "dark".to_string(),
                );
            })
            .unwrap();
        let allocated = persistThemePlan(&fixture);
        let pending = fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"]
            .clone();
        if after_cleanup {
            fixture.writer().migrateCoreThemes().unwrap();
            // Model a process exit after source cleanup but before the completion marker was written.
            fixture
                .writer()
                .commit(vec![fixture
                    .writer()
                    .mutation("imports", "operit2-v27:themes", pending.clone())
                    .unwrap()])
                .unwrap();
            assert!(!preferences
                .data()
                .unwrap()
                .contains(&stringPreferencesKey("character_card_theme_legacy_unknown")));
        }
        fixture.writer.take();
        fixture.writer =
            Some(CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap());
        fixture.writer().migrateCoreThemes().unwrap();
        let retained = fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"]
            .clone();
        assert_eq!(retained, json!({"complete":true}));
        assert_eq!(
            fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"]["themeConfigId"],
            allocated
        );
        let manager = ThemeConfigManager::new(fixture.host.clone());
        assert_eq!(manager.list().unwrap().len(), 1);
        assert_eq!(
            manager.get(allocated).unwrap().snapshot["themeMode"],
            "dark"
        );
        assert!(!preferences
            .data()
            .unwrap()
            .contains(&stringPreferencesKey("character_card_theme_legacy_unknown")));
    }
}

/// Removes modified unknown historical fields and discards the temporary migration plan.
#[test]
fn theme_cleanup_discards_changed_unrecognized_source_values() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let preferences = themePreferences(&fixture);
    let key = stringPreferencesKey("character_card_theme_legacy_unknown");
    preferences
        .edit(|saved| saved.set(&key, json!({"version":1})))
        .unwrap();
    persistThemePlan(&fixture);
    preferences
        .edit(|saved| saved.set(&key, json!({"version":2})))
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    assert!(!preferences.data().unwrap().contains(&key));
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
    );
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

/// Cleans empty and deleted role namespaces without publishing themes for those obsolete owners.
#[test]
fn orphaned_role_theme_preferences_do_not_abort_startup() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let preferences = themePreferences(&fixture);
    let keys = [
        "character_card_theme__custom_ai_avatar_uri",
        "character_group_theme__unknown",
        "character_card_theme_deleted_theme_mode",
        "character_group_theme_deleted_custom_primary_color",
    ];
    preferences
        .edit(|saved| {
            for key in keys {
                saved.set(&stringPreferencesKey(key), json!({"obsolete":true}));
            }
            saved.set(
                &stringPreferencesKey("character_card_theme_legacy_custom_primary_color"),
                "-16711936".to_string(),
            );
            saved.set(&stringPreferencesKey("unrelated"), json!("keep"));
        })
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    let saved = preferences.data().unwrap();
    for key in keys {
        assert!(saved.get(&stringPreferencesKey(key)).is_none());
    }
    assert!(saved
        .get(&stringPreferencesKey(
            "character_card_theme_legacy_custom_primary_color"
        ))
        .is_none());
    assert_eq!(
        saved.get(&stringPreferencesKey("unrelated")),
        Some(&json!("keep"))
    );
    let manager = ThemeConfigManager::new(fixture.host.clone());
    let themes = manager.list().unwrap();
    assert_eq!(themes.len(), 1);
    assert_eq!(
        themes[0].snapshot["customPrimaryColor"],
        json!(4278255360_u32)
    );
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
    );
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(manager.list().unwrap(), themes);
}

/// Starts both role migrations from the October 8 writer's complete string map and verifies canonical data and cleanup.
#[test]
fn october_eighth_role_themes_migrate_before_startup_validation() {
    let released: Value = serde_json::from_str(include_str!(
        "../preferences/theme_preferences_oct08_fixture.json"
    ))
    .unwrap();
    let expected: BTreeMap<String, Value> =
        serde_json::from_value(released["snapshot"].clone()).unwrap();
    let fixture = Fixture::new();
    let mut source = data();
    source.groups.push(CharacterGroupCard {
        id: "group".to_string(),
        name: "Group".to_string(),
        description: String::new(),
        members: vec![GroupMemberConfig {
            characterCardId: "legacy".to_string(),
            orderIndex: 0,
        }],
        createdAt: 100,
        updatedAt: 200,
    });
    source.active = Some(json!({"CharacterGroup":{"id":"group"}}));
    fixture.writer().import("operit2-v27", source).unwrap();
    let mut saved = released["preferences"].clone();
    for prefix in [
        "character_card_theme_legacy_",
        "character_group_theme_group_",
    ] {
        for (key, value) in released["preferences"].as_object().unwrap() {
            if key != "custom_user_avatar_uri" {
                saved[format!("{prefix}{key}")] = value.clone();
            }
        }
    }
    fixture
        .host
        .writeBytes(
            operit_util::OperitPaths::USER_PREFERENCES_PATH,
            &serde_json::to_vec(&saved).unwrap(),
        )
        .unwrap();
    fixture.writer().migrateCoreThemes().unwrap();
    let manager = ThemeConfigManager::new(fixture.host.clone());
    let themes = manager.list().unwrap();
    assert_eq!(themes.len(), 2);
    for theme in &themes {
        assert_eq!(theme.snapshot, expected);
    }
    for (collection, id) in [("cards", "legacy"), ("groups", "group")] {
        let record = fixture.writer().get(collection, id).unwrap().unwrap();
        assert!(themes
            .iter()
            .any(|theme| record["value"]["themeConfigId"] == theme.id));
    }
    assert_eq!(manager.getActive().unwrap().unwrap().name, "Group");
    let migrated = themePreferences(&fixture).data().unwrap();
    assert!(!migrated
        .iterEntries()
        .any(|(key, _)| key.starts_with("character_card_theme_")
            || key.starts_with("character_group_theme_")));
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
    );
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(manager.list().unwrap(), themes);
}

/// Resumes an installed schema-three runtime after the released validator rejected a role color during startup.
#[test]
fn installed_startup_theme_failure_resumes_after_process_restart() {
    use crate::data::preferences::ThemePreferenceSnapshot::validateThemePreferenceSnapshot;
    let mut fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let saved = json!({
        "__operit_preferences_schema_version":3,
        "character_card_theme_legacy_custom_primary_color":"-16711936",
        "character_card_theme_legacy_theme_mode":"dark"
    });
    fixture
        .host
        .writeBytes(
            operit_util::OperitPaths::USER_PREFERENCES_PATH,
            &serde_json::to_vec(&saved).unwrap(),
        )
        .unwrap();
    // The released reader decoded this stored string and then rejected the signed integer.
    let mut old_snapshot: BTreeMap<String, Value> =
        serde_json::from_str(include_str!("../preferences/theme_snapshot_fixture.json")).unwrap();
    old_snapshot.insert("customPrimaryColor".into(), json!(-16711936));
    let error = validateThemePreferenceSnapshot(&old_snapshot)
        .unwrap_err()
        .to_string();
    assert!(
        error.starts_with("Invalid theme snapshot field customPrimaryColor:"),
        "{error}"
    );
    assert!(fixture
        .writer()
        .get("imports", "operit2-v27:themes")
        .unwrap()
        .is_none());
    let original_card = fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"].clone();
    fixture.writer.take();
    fixture.writer =
        Some(CharacterPluginWriter::open(fixture.host.clone(), fixture.host.clone()).unwrap());
    fixture.writer().migrateCoreThemes().unwrap();
    let manager = ThemeConfigManager::new(fixture.host.clone());
    let themes = manager.list().unwrap();
    assert_eq!(themes.len(), 1);
    assert_eq!(
        themes[0].snapshot["customPrimaryColor"],
        json!(4278255360_u32)
    );
    assert_eq!(themes[0].snapshot["themeMode"], "dark");
    let mut expected_card = original_card;
    expected_card["themeConfigId"] = json!(themes[0].id);
    assert_eq!(
        fixture.writer().get("cards", "legacy").unwrap().unwrap()["value"],
        expected_card
    );
    let saved = themePreferences(&fixture).data().unwrap();
    assert_eq!(
        saved.get(&stringPreferencesKey("__operit_preferences_schema_version")),
        Some(&json!(5))
    );
    assert!(!saved
        .iterEntries()
        .any(|(key, _)| key.starts_with("character_card_theme_")));
    assert_eq!(
        fixture
            .writer()
            .get("imports", "operit2-v27:themes")
            .unwrap()
            .unwrap()["value"],
        json!({"complete":true})
    );
    fixture.writer().migrateCoreThemes().unwrap();
    assert_eq!(manager.list().unwrap(), themes);
}

/// Preserves missing primary libraries through both import sources without selecting another owner.
#[test]
fn retains_unresolved_primary_memory_bindings_without_blocking_import() {
    for source_id in ["operit1", "operit2-v27"] {
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
fn retains_deleted_active_roles_through_both_legacy_imports() {
    for source_id in ["operit1", "operit2-v27"] {
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
}

/// Rejects malformed active metadata before any record or cleanup marker is published.
#[test]
fn invalid_active_shapes_do_not_publish_a_legacy_import() {
    for source_id in ["operit1", "operit2-v27"] {
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

/// Retains memory text and document chunks when an attachment was lost before startup migration.
#[test]
fn missing_memory_attachments_do_not_block_migration() {
    let fixture = Fixture::new();
    let mut record = json!({
        "content":"retained user memory", "isDocumentNode":true,
        "documentPath":"/app/data/data/memory/characters/legacy/missing.txt",
        "chunkIndexFilePath":"/app/workspaces/missing-index.bin"
    });
    let mut resources = BTreeMap::new();
    migrateMemoryResources(fixture.host.as_ref(), &fixture.writer().directory, "character:legacy", &mut record, &mut resources).unwrap();
    assert_eq!(record["content"], "retained user memory");
    assert_eq!(record["isDocumentNode"], true);
    assert!(record["documentPath"].is_null());
    assert!(record["chunkIndexFilePath"].is_null());
    assert!(resources.is_empty());
}

/// Copies existing legacy resources while leaving a missing companion index unbound.
#[test]
fn available_memory_attachment_survives_a_missing_index() {
    let fixture = Fixture::new();
    let source = format!("{DATA_MEMORY_CHARACTERS_DIR_PATH}/legacy/document.txt");
    fixture.host.writeBytes(&source, b"original document").unwrap();
    let mut record = json!({
        "content":"retained user memory",
        "documentPath":"/app/data/data/memory/characters/legacy/document.txt",
        "chunkIndexFilePath":"/app/data/data/memory/characters/legacy/missing-index.bin"
    });
    let mut resources = BTreeMap::new();
    migrateMemoryResources(fixture.host.as_ref(), &fixture.writer().directory, "character:legacy", &mut record, &mut resources).unwrap();
    assert_eq!(record["content"], "retained user memory");
    assert!(record["chunkIndexFilePath"].is_null());
    let (relative, bytes) = resources.first_key_value().unwrap();
    assert_eq!(bytes, b"original document");
    assert_eq!(record["documentPath"], publicResourcePath(&format!("{}/{relative}", fixture.writer().directory)).unwrap());
}

/// Skips all plugin and database access when only current plugin data remains after migration.
#[test]
fn startup_without_legacy_characters_does_not_open_current_plugin_data() {
    let fixture = Fixture::new();
    fixture.writer().import("operit2-v27", data()).unwrap();
    let metadata = format!("{}/meta.preferences.json", fixture.writer().directory);
    fixture.host.writeBytes(&metadata, b"must not be opened by startup migration").unwrap();
    let paths = RuntimeStorePaths::new(fixture.root.join("runtime"), fixture.root.join("workspaces"));
    migrateLegacyCharactersWithStorage(paths, fixture.host.clone()).unwrap();
    assert_eq!(fixture.host.readBytes(&metadata).unwrap(), b"must not be opened by startup migration");
    assert!(!fixture.host.exists(CHARACTER_CARDS_PREFERENCES_PATH).unwrap());
}

/// A clean installation also exits before creating any migration records or opening chat history.
#[test]
fn startup_without_legacy_characters_does_not_create_an_import() {
    let fixture = Fixture::new();
    let paths = RuntimeStorePaths::new(fixture.root.join("runtime"), fixture.root.join("workspaces"));
    migrateLegacyCharactersWithStorage(paths, fixture.host.clone()).unwrap();
    assert!(fixture.writer().get("imports", "operit2-v27").unwrap().is_none());
    assert!(fixture.writer().get("meta", "state").unwrap().is_none());
}

/// A failed cleanup retains its startup trigger even when the manifest lists that file first.
#[test]
fn cleanup_failure_retains_the_startup_trigger_until_the_next_success() {
    let fixture = Fixture::new();
    fixture.host.writeBytes(CHARACTER_CARDS_PREFERENCES_PATH, b"legacy card source").unwrap();
    let blocked = format!("{CHARACTER_GROUPS_PREFERENCES_PATH}/unexpected-child");
    fixture.host.writeBytes(&blocked, b"prevents nonrecursive directory deletion").unwrap();
    let mut source = data();
    source.cleanup_paths = vec![CHARACTER_CARDS_PREFERENCES_PATH.into(), CHARACTER_GROUPS_PREFERENCES_PATH.into()];
    fixture.writer().import("operit2-v27", source).unwrap();
    assert!(cleanupLegacyCharacters(fixture.writer()).is_err());
    assert!(fixture.host.exists(CHARACTER_CARDS_PREFERENCES_PATH).unwrap());
    assert_eq!(fixture.writer().get("imports", "operit2-v27").unwrap().unwrap()["value"]["cleanupComplete"], false);
    fixture.host.delete(&blocked, false).unwrap();
    cleanupLegacyCharacters(fixture.writer()).unwrap();
    assert!(!fixture.host.exists(CHARACTER_CARDS_PREFERENCES_PATH).unwrap());
    assert!(!fixture.host.exists(CHARACTER_GROUPS_PREFERENCES_PATH).unwrap());
    assert_eq!(fixture.writer().get("imports", "operit2-v27").unwrap().unwrap()["value"]["cleanupComplete"], true);
}

/// Completes the final source deletion after an interruption following the cleanup marker write.
#[test]
fn cleanup_completion_marker_does_not_skip_the_remaining_startup_trigger() {
    let fixture = Fixture::new();
    fixture.host.writeBytes(CHARACTER_CARDS_PREFERENCES_PATH, b"legacy card source").unwrap();
    let mut source = data();
    source.cleanup_paths.push(CHARACTER_CARDS_PREFERENCES_PATH.into());
    fixture.writer().import("operit2-v27", source).unwrap();
    let mut marker = fixture.writer().get("imports", "operit2-v27").unwrap().unwrap()["value"].clone();
    marker["cleanupComplete"] = json!(true);
    fixture.writer().commit(vec![fixture.writer().mutation("imports", "operit2-v27", marker).unwrap()]).unwrap();
    crate::data::backup::CharacterPluginLayout::prepare(fixture.host.clone()).unwrap();
    assert!(operit_store::PreferenceLayout::retirements(fixture.host.clone()).unwrap().is_empty());
    cleanupLegacyCharacters(fixture.writer()).unwrap();
    assert!(!fixture.host.exists(CHARACTER_CARDS_PREFERENCES_PATH).unwrap());
    assert!(operit_store::PreferenceLayout::retirements(fixture.host.clone()).unwrap().contains(CHARACTER_CARDS_PREFERENCES_PATH));
}

/// A completed layout consumes retired originals without rewriting the plugin's current values.
#[test]
fn character_layout_retires_old_transactions_only_after_import_and_cleanup() {
    for retained_document in [false, true] {
        assert_character_layout_retirement(retained_document);
    }
}

/// Exercises fresh cleanup and an existing installation with a retained legacy document.
fn assert_character_layout_retirement(retained_document: bool) {
    use operit_store::{
        PreferenceMerge,
        SyncOperationStore::{SyncOperation, SyncOperationKnowledge, SyncOperationSemantics},
    };
    let fixture = Fixture::new();
    let memory_root = format!("{DATA_MEMORY_CHARACTERS_DIR_PATH}/legacy");
    let settings = format!("{memory_root}/settings/memory_search_settings.preferences.json");
    let missing = format!("{DATA_MEMORY_CHARACTERS_DIR_PATH}/old-peer-only/settings/memory_search_settings.preferences.json");
    let paths = [
        CHARACTER_CARDS_PREFERENCES_PATH,
        settings.as_str(),
        missing.as_str(),
    ];
    let originals = paths.iter().enumerate().map(|(index, path)| SyncOperation {
        opId: format!("legacy-peer:{}", index + 1), originDeviceId: "legacy-peer".into(),
        sequence: index as i64 + 1, domain: "preferences".into(), entityType: "entry".into(),
        entityId: serde_json::to_string(&(path, "legacy_setting")).unwrap(), operation: "set".into(),
        semantics: SyncOperationSemantics::EntityState,
        payload: json!({"storagePath":path,"key":"legacy_setting","value":"late history","encrypted":false,"formatVersion":1}),
        createdAt: index as i64 + 1, schemaVersion: 1,
    }).collect::<Vec<_>>();
    let schema = crate::data::preferences::PreferenceSpaceMerge::schema();
    let journal = SyncOperationStore::new(fixture.host.clone(), RUNTIME_SYNC_DIR_PATH);
    journal.receiveOperations(&originals).unwrap();
    crate::data::backup::CharacterPluginLayout::prepare(fixture.host.clone()).unwrap();
    assert!(
        operit_store::PreferenceLayout::retirements(fixture.host.clone())
            .unwrap()
            .is_empty()
    );
    fixture
        .host
        .writeBytes(CHARACTER_CARDS_PREFERENCES_PATH, b"{}")
        .unwrap();
    fixture.host.writeBytes(&settings, b"{}").unwrap();
    let mut source = data();
    source.cleanup_paths = vec![CHARACTER_CARDS_PREFERENCES_PATH.into(), memory_root.clone()];
    fixture.writer().import("operit2-v27", source).unwrap();
    crate::data::backup::CharacterPluginLayout::prepare(fixture.host.clone()).unwrap();
    let registry =
        PreferenceMerge::registry(fixture.host.clone(), &schema, [], |_, _| true).unwrap();
    let before = registry.receive(&[]).unwrap();
    assert!(before.pending.is_empty());
    assert_eq!(before.skipped, originals.len());
    assert!(before.errors.is_empty());
    let document = format!("{memory_root}/USER.md");
    if retained_document {
        // Reproduce an already-cleaned installation before the new layout has ever run.
        fixture.host.delete(CHARACTER_CARDS_PREFERENCES_PATH, false).unwrap();
        fixture.host.delete(&memory_root, true).unwrap();
        fixture.host.writeBytes(&document, b"retained user document").unwrap();
        let mut marker = fixture.writer().get("imports", "operit2-v27").unwrap().unwrap()["value"].clone();
        marker["cleanupComplete"] = json!(true);
        fixture.writer().commit(vec![fixture.writer().mutation("imports", "operit2-v27", marker).unwrap()]).unwrap();
    } else {
        cleanupLegacyCharacters(fixture.writer()).unwrap();
    }
    let card = fixture.writer().get("cards", "legacy").unwrap();
    let owner = fixture.writer().get("owners", "character:legacy").unwrap();
    let active = fixture.writer().get("meta", "state").unwrap();
    for attempt in 0..2 {
        // Startup publishes persisted layout rules, and a retry has no new incoming batch.
        crate::data::preferences::PreferenceSpaceMerge::prepareLayout(fixture.host.clone())
            .unwrap();
        let registry =
            PreferenceMerge::registry(fixture.host.clone(), &schema, [], |_, _| true).unwrap();
        let report = registry.receive(&[]).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(report.pending.is_empty());
        assert!(report.applied.is_empty());
        assert_eq!(
            report.skipped,
            if attempt == 0 { originals.len() } else { 0 }
        );
        let (snapshots, pending, errors) = registry.snapshots().unwrap();
        assert!(pending.is_empty());
        assert!(errors.is_empty(), "{errors:?}");
        assert!(snapshots
            .iter()
            .all(|snapshot| !paths.contains(&snapshot.data.owner.path.as_str())));
        assert_eq!(fixture.writer().get("cards", "legacy").unwrap(), card);
        assert_eq!(
            fixture.writer().get("owners", "character:legacy").unwrap(),
            owner
        );
        assert_eq!(fixture.writer().get("meta", "state").unwrap(), active);
        if retained_document {
            assert_eq!(fixture.host.readBytes(&document).unwrap(), b"retained user document");
        }
        for path in &paths {
            assert!(!fixture.host.exists(path).unwrap());
        }
    }
    let history = journal.preferenceHistory().unwrap();
    let receipt = journal.receipt().unwrap();
    for operation in originals {
        assert!(history.contains(&operation));
        assert!(receipt.containsSequence(&operation.originDeviceId, operation.sequence));
    }
}
