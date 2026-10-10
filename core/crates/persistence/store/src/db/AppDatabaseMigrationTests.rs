use super::*;
use crate::sync::SqlChatSyncStore::tests::{openTestStore, DATABASE_MUTEX};
use crate::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
use crate::SqliteStore::{SqliteRow, SqliteRowGet, SqliteValue};
use std::collections::BTreeMap;

const VERSION_27_FIXTURE: &str = include_str!("fixtures/version27-chat-records.sql");
const EXTENSION_TABLES: [&str; 6] = [
    "chats",
    "messages",
    "message_variants",
    "sync_sql_chat_rows",
    "sync_sql_message_rows",
    "sync_sql_message_variant_rows",
];
const RETIRED_CHAT_COLUMNS: [&str; 2] = ["characterCardName", "characterGroupId"];

/// Creates the genuine production version-27 schema with populated records, legacy columns and sync logs.
fn version27Database(name: &str) -> (RuntimeStorePaths, Arc<AppDatabase>) {
    let (paths, database, _) = openTestStore(name);
    database.dropAllTables().unwrap();
    database.store.executeBatch(VERSION_27_FIXTURE).unwrap();
    assert_eq!(database.store.getUserVersion().unwrap(), 27);
    (paths, database)
}

/// Returns every table name in a stable order, including SQLite's preserved autoincrement state.
fn tableNames(store: &SqliteStore) -> Vec<String> {
    store
        .queryRows(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
            Vec::new(),
        )
        .unwrap()
        .iter()
        .map(|row| row.get("name").unwrap())
        .collect()
}

/// Captures every persisted field of every record instead of checking selected counts or sample fields.
fn tableRows(store: &SqliteStore) -> BTreeMap<String, Vec<SqliteRow>> {
    tableNames(store)
        .into_iter()
        .map(|table| {
            let rows = store
                .queryRows(
                    &format!("SELECT * FROM \"{table}\" ORDER BY rowid"),
                    Vec::new(),
                )
                .unwrap();
            (table, rows)
        })
        .collect()
}

/// Computes the only permitted migration changes: retired columns removed, empty extensions added and chat logs advanced.
fn expectedVersion28Rows(
    before: &BTreeMap<String, Vec<SqliteRow>>,
) -> BTreeMap<String, Vec<SqliteRow>> {
    before
        .iter()
        .map(|(table, rows)| {
            let rows = rows
                .iter()
                .map(|row| {
                    let mut result = SqliteRow {
                        columns: Vec::new(),
                        values: Vec::new(),
                    };
                    for (column, value) in row.columns.iter().zip(&row.values) {
                        if (table == "chats" || table == "sync_sql_chat_rows")
                            && RETIRED_CHAT_COLUMNS.iter().any(|retired| column == retired)
                        {
                            continue;
                        }
                        result.columns.push(column.clone());
                        result.values.push(value.clone());
                    }
                    if EXTENSION_TABLES.iter().any(|candidate| table == candidate) {
                        result.columns.push("pluginExtensions".to_string());
                        result.values.push(SqliteValue::Text("{}".to_string()));
                    }
                    if table == "sync_sql_operations"
                        && row.get::<_, String>("domain").unwrap() == "chat"
                        && row.get::<_, i32>("schemaVersion").unwrap() == 6
                    {
                        let index = result
                            .columns
                            .iter()
                            .position(|column| column == "schemaVersion")
                            .unwrap();
                        result.values[index] = SqliteValue::Integer(7);
                    }
                    result
                })
                .collect();
            (table.clone(), rows)
        })
        .collect()
}

/// Captures canonical column definitions, foreign keys and indexes for fresh-versus-upgraded schema parity.
fn schemaDefinitions(store: &SqliteStore) -> BTreeMap<String, Vec<SqliteRow>> {
    let mut definitions = BTreeMap::new();
    for table in tableNames(store) {
        for (kind, pragma) in [
            ("columns", "table_info"),
            ("foreign-keys", "foreign_key_list"),
        ] {
            definitions.insert(
                format!("{kind}:{table}"),
                store
                    .queryRows(&format!("PRAGMA {pragma}(\"{table}\")"), Vec::new())
                    .unwrap(),
            );
        }
    }
    definitions.insert(
        "indexes".to_string(),
        store
            .queryRows(
                "SELECT name,tbl_name,sql FROM sqlite_master WHERE type='index' ORDER BY name",
                Vec::new(),
            )
            .unwrap(),
    );
    definitions
}

/// Proves that opening the actual AppDatabase upgrades all complete records and preserves preferences and file reopen behavior.
#[test]
fn opening_version27_atomically_upgrades_records_and_reopens_version28() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (paths, old) = version27Database("database-v27-v28");
    let before = tableRows(&old.store);
    let expected = expectedVersion28Rows(&before);
    let indexes = old
        .store
        .queryRows(
            "SELECT name,tbl_name,sql FROM sqlite_master WHERE type='index' ORDER BY name",
            Vec::new(),
        )
        .unwrap();
    let preferences = PreferencesDataStore::new(paths.current_chat_id_preferences_path());
    let key = stringPreferencesKey("migration-test-pref");
    preferences
        .edit(|values| {
            values.set(&key, "kept-preference".to_string());
        })
        .unwrap();
    AppDatabase::closeDatabase();
    drop(old);
    let database = AppDatabase::getDatabase(paths.clone()).unwrap();
    assert_eq!(database.store.getUserVersion().unwrap(), 28);
    assert_eq!(tableRows(&database.store), expected);
    assert_eq!(
        database
            .store
            .queryRows(
                "SELECT name,tbl_name,sql FROM sqlite_master WHERE type='index' ORDER BY name",
                Vec::new()
            )
            .unwrap(),
        indexes
    );
    assert!(database
        .store
        .queryRows("PRAGMA foreign_key_check", Vec::new())
        .unwrap()
        .is_empty());
    assert_eq!(
        database
            .store
            .queryScalar::<i32>(
                "SELECT schemaVersion FROM sync_sql_operations WHERE opId='v27:1'",
                Vec::new()
            )
            .unwrap(),
        7
    );
    assert_eq!(
        database
            .store
            .queryScalar::<i32>(
                "SELECT schemaVersion FROM sync_sql_operations WHERE opId='v27:2'",
                Vec::new()
            )
            .unwrap(),
        6
    );
    let chat = database.chatDao().getChatById("v27-chat").unwrap().unwrap();
    assert_eq!(chat.title, "Kept title");
    assert_eq!(chat.group.as_deref(), Some("legacy-sidebar"));
    assert_eq!(
        (
            chat.createdAt,
            chat.updatedAt,
            chat.inputTokens,
            chat.outputTokens,
            chat.currentWindowSize,
            chat.displayOrder
        ),
        (100, 200, 3, 5, 8, 12)
    );
    assert_eq!(chat.workspaceId.as_deref(), Some("workspace-kept"));
    assert_eq!(chat.parentChatId.as_deref(), Some("parent-kept"));
    assert!(chat.locked && chat.pinned && chat.pluginExtensions.is_empty());
    let message = database
        .messageDao()
        .getMessageByTimestamp("v27-chat", 42)
        .unwrap()
        .unwrap();
    assert_eq!(message.provider, "provider-kept");
    assert_eq!(message.modelName, "model-kept");
    assert_eq!(message.roleName, "Legacy speaker");
    assert_eq!(
        (
            message.orderIndex,
            message.selectedVariantIndex,
            message.inputTokens,
            message.outputTokens,
            message.cachedInputTokens
        ),
        (7, 2, 11, 13, 17)
    );
    assert_eq!(
        (
            message.sentAt,
            message.outputDurationMs,
            message.waitDurationMs,
            message.completedAt,
            message.completedExecutionGeneration
        ),
        (50, 19, 23, 90, 29)
    );
    assert_eq!(message.displayMode, "NORMAL");
    assert!(message.isFavorite && message.pluginExtensions.is_empty());
    let variants = database
        .messageVariantDao()
        .getVariantsForMessage("v27-chat", 42)
        .unwrap();
    assert_eq!(variants.len(), 2);
    assert!(variants
        .iter()
        .all(|variant| variant.pluginExtensions.is_empty()));
    assert_eq!(variants[0].roleName, "Variant one");
    assert_eq!(variants[1].roleName, "Variant two");
    assert_eq!(
        database
            .messagePartDao()
            .getPartsForChat("v27-chat")
            .unwrap()
            .len(),
        3
    );
    for table in EXTENSION_TABLES {
        let columns = database
            .store
            .queryRows(&format!("PRAGMA table_info({table})"), Vec::new())
            .unwrap();
        let extension = columns
            .iter()
            .find(|row| row.get::<_, String>("name").unwrap() == "pluginExtensions")
            .unwrap();
        assert_eq!(extension.get::<_, String>("type").unwrap(), "TEXT");
        assert_eq!(extension.get::<_, i32>("notnull").unwrap(), 1);
        assert_eq!(extension.get::<_, String>("dflt_value").unwrap(), "'{}'");
        assert!(columns.iter().all(|row| !RETIRED_CHAT_COLUMNS
            .iter()
            .any(|retired| row.get::<_, String>("name").unwrap() == *retired)));
    }
    assert_eq!(
        preferences
            .dataFlow()
            .first()
            .unwrap()
            .get(&key)
            .map(String::as_str),
        Some("kept-preference")
    );
    let schema = schemaDefinitions(&database.store);
    AppDatabase::closeDatabase();
    drop(database);
    let reopened = AppDatabase::getDatabase(paths).unwrap();
    assert_eq!(reopened.store.getUserVersion().unwrap(), 28);
    assert_eq!(tableRows(&reopened.store), expected);
    assert_eq!(schemaDefinitions(&reopened.store), schema);
}

/// Fails at every boundary, including after the version statement, and verifies explicit rollback preserves schema and every row.
#[test]
fn migration_failure_at_every_boundary_preserves_version27_schema_and_all_records() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_, database) = version27Database("database-v27-rollback");
    let beforeSchema = database
        .store
        .queryRows(
            "SELECT name,sql FROM sqlite_master ORDER BY name",
            Vec::new(),
        )
        .unwrap();
    let beforeRows = tableRows(&database.store);
    for boundary in 0..=migration27To28Statements().len() {
        let result = database.store.transaction(|transaction| {
            for statement in migration27To28Statements().iter().take(boundary) {
                transaction.execute(statement, Vec::new())?;
            }
            transaction.execute(
                "ALTER TABLE missing_failure_fixture ADD COLUMN injected TEXT",
                Vec::new(),
            )?;
            Ok(())
        });
        assert!(
            result.is_err(),
            "Failure boundary {boundary} must not commit"
        );
        assert_eq!(
            database.store.getUserVersion().unwrap(),
            27,
            "boundary {boundary}"
        );
        assert_eq!(
            database
                .store
                .queryRows(
                    "SELECT name,sql FROM sqlite_master ORDER BY name",
                    Vec::new()
                )
                .unwrap(),
            beforeSchema,
            "boundary {boundary}"
        );
        assert_eq!(
            tableRows(&database.store),
            beforeRows,
            "boundary {boundary}"
        );
        assert!(database
            .store
            .queryRows("PRAGMA foreign_key_check", Vec::new())
            .unwrap()
            .is_empty());
    }
}

/// Requires identical columns, defaults, foreign keys and indexes for newly created and migrated version-28 databases.
#[test]
fn fresh_and_migrated_version28_have_identical_schema() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    let (_, database) = version27Database("database-v28-schema-parity");
    database.openWithMigrations().unwrap();
    let upgraded = schemaDefinitions(&database.store);
    database.dropAllTables().unwrap();
    database.createAllTables().unwrap();
    database.store.setUserVersion(28).unwrap();
    assert_eq!(schemaDefinitions(&database.store), upgraded);
}

