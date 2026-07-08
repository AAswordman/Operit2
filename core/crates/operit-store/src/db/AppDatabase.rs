use std::sync::{Arc, Mutex};

use crate::RuntimeStorePaths::RuntimeStorePaths;
use crate::SqliteStore::{SqliteRowGet, SqliteStore, SqliteStoreError};
use thiserror::Error;

use crate::dao::ChatDao::ChatDao;
use crate::dao::MessageDao::MessageDao;
use crate::dao::MessageVariantDao::MessageVariantDao;

/// Current SQLite schema version expected by the runtime.
pub const DATABASE_VERSION: i32 = 22;

#[derive(Debug, Error)]
/// Error surface for opening and migrating the application database.
pub enum AppDatabaseError {
    #[error(transparent)]
    Store(#[from] SqliteStoreError),
    #[error("database version {actual} is newer than runtime version {expected}")]
    DatabaseVersionTooNew { actual: i32, expected: i32 },
    #[error("missing migration from version {from} to {to}")]
    MissingMigration { from: i32, to: i32 },
}

#[derive(Clone)]
/// Shared SQLite database wrapper exposing typed DAO factories.
pub struct AppDatabase {
    store: SqliteStore,
}

static INSTANCE: Mutex<Option<Arc<AppDatabase>>> = Mutex::new(None);

impl AppDatabase {
    /// Creates a DAO for chat metadata rows.
    pub fn chatDao(&self) -> ChatDao {
        ChatDao::new(self.store.clone())
    }

    /// Creates a DAO for message rows.
    pub fn messageDao(&self) -> MessageDao {
        MessageDao::new(self.store.clone())
    }

    /// Creates a DAO for message variant rows.
    pub fn messageVariantDao(&self) -> MessageVariantDao {
        MessageVariantDao::new(self.store.clone())
    }

    /// Opens the shared database instance and applies required migrations.
    pub fn getDatabase(paths: RuntimeStorePaths) -> Result<Arc<AppDatabase>, AppDatabaseError> {
        let mut instance = INSTANCE
            .lock()
            .expect("AppDatabase.INSTANCE mutex must not be poisoned");
        if let Some(database) = instance.as_ref() {
            return Ok(database.clone());
        }

        let database = Arc::new(AppDatabase {
            store: SqliteStore::open(paths.sqlite_database_path())?,
        });
        database.openWithMigrations()?;
        *instance = Some(database.clone());
        Ok(database)
    }

    /// Opens the shared database using default runtime paths.
    pub fn default() -> Result<Arc<AppDatabase>, AppDatabaseError> {
        Self::getDatabase(RuntimeStorePaths::default())
    }

    /// Clears the process-wide database instance handle.
    pub fn closeDatabase() {
        let mut instance = INSTANCE
            .lock()
            .expect("AppDatabase.INSTANCE mutex must not be poisoned");
        *instance = None;
    }

    /// Returns the underlying SQLite store.
    pub fn store(&self) -> &SqliteStore {
        &self.store
    }

    fn openWithMigrations(&self) -> Result<(), AppDatabaseError> {
        let currentVersion = self.store.getUserVersion()?;
        if currentVersion == DATABASE_VERSION {
            return Ok(());
        }
        if currentVersion > DATABASE_VERSION {
            return Err(AppDatabaseError::DatabaseVersionTooNew {
                actual: currentVersion,
                expected: DATABASE_VERSION,
            });
        }
        if currentVersion == 0 {
            self.createAllTables()?;
            self.store.setUserVersion(DATABASE_VERSION)?;
            return Ok(());
        }

        match currentVersion {
            1 => MIGRATION_1_2(self)?,
            2 => MIGRATION_2_3(self)?,
            3 => MIGRATION_3_4(self)?,
            4 => MIGRATION_4_5(self)?,
            5 => MIGRATION_5_6(self)?,
            6 => MIGRATION_6_7(self)?,
            7 => MIGRATION_7_8(self)?,
            8 => MIGRATION_8_9(self)?,
            9 => MIGRATION_9_10(self)?,
            10 => MIGRATION_10_11(self)?,
            11 => MIGRATION_11_12(self)?,
            12 => MIGRATION_12_13(self)?,
            13 => MIGRATION_13_14(self)?,
            14 => MIGRATION_14_15(self)?,
            15 => MIGRATION_15_16(self)?,
            16 => MIGRATION_16_17(self)?,
            17 => MIGRATION_17_18(self)?,
            18 => MIGRATION_18_19(self)?,
            19 => MIGRATION_19_20(self)?,
            20 => MIGRATION_20_21(self)?,
            21 => MIGRATION_21_22(self)?,
            version => {
                return Err(AppDatabaseError::MissingMigration {
                    from: version,
                    to: version + 1,
                })
            }
        }
        self.openWithMigrations()
    }

    fn createAllTables(&self) -> Result<(), SqliteStoreError> {
        createAllTables(&self.store)
    }

    /// Drops all known application tables.
    pub fn dropAllTables(&self) -> Result<(), SqliteStoreError> {
        self.store.executeBatch(
            r#"
            DROP TABLE IF EXISTS message_variants;
            DROP TABLE IF EXISTS messages;
            DROP TABLE IF EXISTS chats;
            DROP TABLE IF EXISTS usage_request_records;
            DROP TABLE IF EXISTS sync_sql_deletions;
            DROP TABLE IF EXISTS sync_sql_message_variant_rows;
            DROP TABLE IF EXISTS sync_sql_message_rows;
            DROP TABLE IF EXISTS sync_sql_chat_rows;
            DROP TABLE IF EXISTS sync_sql_operations;
            DROP TABLE IF EXISTS sync_sql_clocks;
            "#,
        )
    }
}

#[allow(non_snake_case)]
fn MIGRATION_1_2(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        r#"
        CREATE TABLE IF NOT EXISTS chats (
            id TEXT NOT NULL,
            title TEXT NOT NULL,
            createdAt INTEGER NOT NULL,
            updatedAt INTEGER NOT NULL,
            inputTokens INTEGER NOT NULL DEFAULT 0,
            outputTokens INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY(id)
        );
        CREATE TABLE IF NOT EXISTS messages (
            messageId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            chatId TEXT NOT NULL,
            sender TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            orderIndex INTEGER NOT NULL,
            FOREIGN KEY(chatId) REFERENCES chats(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS index_messages_chatId ON messages (chatId);
        PRAGMA user_version = 2;
        "#,
    )
}

#[allow(non_snake_case)]
fn MIGRATION_2_3(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database
        .store
        .executeBatch("ALTER TABLE chats ADD COLUMN \"group\" TEXT; PRAGMA user_version = 3;")
}

#[allow(non_snake_case)]
fn MIGRATION_3_4(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        r#"
        ALTER TABLE chats ADD COLUMN displayOrder INTEGER NOT NULL DEFAULT 0;
        UPDATE chats SET displayOrder = updatedAt;
        PRAGMA user_version = 4;
        "#,
    )
}

#[allow(non_snake_case)]
fn MIGRATION_4_5(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database
        .store
        .executeBatch("ALTER TABLE chats ADD COLUMN workspace TEXT; PRAGMA user_version = 5;")
}

#[allow(non_snake_case)]
fn MIGRATION_5_6(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        "ALTER TABLE chats ADD COLUMN currentWindowSize INTEGER NOT NULL DEFAULT 0; PRAGMA user_version = 6;",
    )
}

#[allow(non_snake_case)]
fn MIGRATION_6_7(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        "ALTER TABLE messages ADD COLUMN roleName TEXT NOT NULL DEFAULT ''; PRAGMA user_version = 7;",
    )
}

#[allow(non_snake_case)]
fn MIGRATION_7_8(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        r#"
        ALTER TABLE chats ADD COLUMN parentChatId TEXT;
        ALTER TABLE chats ADD COLUMN characterCardName TEXT;
        PRAGMA user_version = 8;
        "#,
    )
}

#[allow(non_snake_case)]
fn MIGRATION_8_9(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        r#"
        ALTER TABLE messages ADD COLUMN provider TEXT NOT NULL DEFAULT '';
        ALTER TABLE messages ADD COLUMN modelName TEXT NOT NULL DEFAULT '';
        PRAGMA user_version = 9;
        "#,
    )
}

#[allow(non_snake_case)]
fn MIGRATION_9_10(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        "ALTER TABLE chats ADD COLUMN locked INTEGER NOT NULL DEFAULT 0; PRAGMA user_version = 10;",
    )
}

#[allow(non_snake_case)]
fn MIGRATION_10_11(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database
        .store
        .executeBatch("ALTER TABLE chats ADD COLUMN workspaceEnv TEXT; PRAGMA user_version = 11;")
}

#[allow(non_snake_case)]
fn MIGRATION_11_12(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        "ALTER TABLE chats ADD COLUMN characterGroupId TEXT; PRAGMA user_version = 12;",
    )
}

#[allow(non_snake_case)]
fn MIGRATION_12_13(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        r#"
        ALTER TABLE messages ADD COLUMN inputTokens INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE messages ADD COLUMN outputTokens INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE messages ADD COLUMN cachedInputTokens INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE messages ADD COLUMN sentAt INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE messages ADD COLUMN outputDurationMs INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE messages ADD COLUMN waitDurationMs INTEGER NOT NULL DEFAULT 0;
        PRAGMA user_version = 13;
        "#,
    )
}

#[allow(non_snake_case)]
fn MIGRATION_13_14(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database
        .store
        .executeBatch("DROP TABLE IF EXISTS problem_records; PRAGMA user_version = 14;")
}

#[allow(non_snake_case)]
fn MIGRATION_14_15(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        r#"
        ALTER TABLE messages ADD COLUMN selectedVariantIndex INTEGER NOT NULL DEFAULT 0;
        CREATE TABLE IF NOT EXISTS message_variants (
            variantId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            chatId TEXT NOT NULL,
            messageTimestamp INTEGER NOT NULL,
            variantIndex INTEGER NOT NULL,
            content TEXT NOT NULL,
            roleName TEXT NOT NULL DEFAULT '',
            provider TEXT NOT NULL DEFAULT '',
            modelName TEXT NOT NULL DEFAULT '',
            inputTokens INTEGER NOT NULL DEFAULT 0,
            outputTokens INTEGER NOT NULL DEFAULT 0,
            cachedInputTokens INTEGER NOT NULL DEFAULT 0,
            sentAt INTEGER NOT NULL DEFAULT 0,
            outputDurationMs INTEGER NOT NULL DEFAULT 0,
            waitDurationMs INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(chatId) REFERENCES chats(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS index_message_variants_chatId_messageTimestamp
            ON message_variants (chatId, messageTimestamp);
        CREATE UNIQUE INDEX IF NOT EXISTS index_message_variants_chatId_messageTimestamp_variantIndex
            ON message_variants (chatId, messageTimestamp, variantIndex);
        PRAGMA user_version = 15;
        "#,
    )
}

#[allow(non_snake_case)]
fn MIGRATION_15_16(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        "ALTER TABLE messages ADD COLUMN displayMode TEXT NOT NULL DEFAULT 'NORMAL'; PRAGMA user_version = 16;",
    )
}

#[allow(non_snake_case)]
fn MIGRATION_16_17(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        r#"
        CREATE INDEX IF NOT EXISTS index_messages_chatId_timestamp
            ON messages (chatId, timestamp);
        PRAGMA user_version = 17;
        "#,
    )
}

#[allow(non_snake_case)]
fn MIGRATION_17_18(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        "ALTER TABLE messages ADD COLUMN isFavorite INTEGER NOT NULL DEFAULT 0; PRAGMA user_version = 18;",
    )
}

#[allow(non_snake_case)]
fn MIGRATION_18_19(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    database.store.executeBatch(
        r#"
        ALTER TABLE messages ADD COLUMN completedAt INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE message_variants ADD COLUMN completedAt INTEGER NOT NULL DEFAULT 0;
        PRAGMA user_version = 19;
        "#,
    )
}

#[allow(non_snake_case)]
fn MIGRATION_19_20(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    createSyncTables(&database.store)?;
    database.store.setUserVersion(20)
}

#[allow(non_snake_case)]
fn MIGRATION_20_21(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    addColumnIfMissing(
        &database.store,
        "chats",
        "pinned",
        "ALTER TABLE chats ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0",
    )?;
    addColumnIfMissing(
        &database.store,
        "sync_sql_chat_rows",
        "pinned",
        "ALTER TABLE sync_sql_chat_rows ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0",
    )?;
    database.store.setUserVersion(21)
}

#[allow(non_snake_case)]
fn MIGRATION_21_22(database: &AppDatabase) -> Result<(), SqliteStoreError> {
    createUsageRequestRecordsTable(&database.store)?;
    database.store.setUserVersion(22)
}

#[allow(non_snake_case)]
fn addColumnIfMissing(
    store: &SqliteStore,
    tableName: &str,
    columnName: &str,
    alterSql: &str,
) -> Result<(), SqliteStoreError> {
    let pragma = format!("PRAGMA table_info(\"{}\")", tableName.replace('"', "\"\""));
    let hasColumn = store
        .queryRows(&pragma, Vec::new())?
        .into_iter()
        .any(|row| {
            let name: Result<String, SqliteStoreError> = row.get("name");
            name.map(|name| name == columnName).unwrap_or(false)
        });
    if !hasColumn {
        store.execute(alterSql, Vec::new())?;
    }
    Ok(())
}

/// Creates all application database tables for a fresh store.
pub fn createAllTables(store: &SqliteStore) -> Result<(), SqliteStoreError> {
    store.executeBatch(
        r#"
        CREATE TABLE chats (
            id TEXT PRIMARY KEY NOT NULL,
            title TEXT NOT NULL,
            createdAt INTEGER NOT NULL,
            updatedAt INTEGER NOT NULL,
            inputTokens INTEGER NOT NULL DEFAULT 0,
            outputTokens INTEGER NOT NULL DEFAULT 0,
            currentWindowSize INTEGER NOT NULL DEFAULT 0,
            "group" TEXT,
            displayOrder INTEGER NOT NULL DEFAULT 0,
            workspace TEXT,
            workspaceEnv TEXT,
            parentChatId TEXT,
            characterCardName TEXT,
            characterGroupId TEXT,
            locked INTEGER NOT NULL DEFAULT 0,
            pinned INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE messages (
            messageId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            chatId TEXT NOT NULL,
            sender TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            orderIndex INTEGER NOT NULL,
            roleName TEXT NOT NULL DEFAULT '',
            selectedVariantIndex INTEGER NOT NULL DEFAULT 0,
            provider TEXT NOT NULL DEFAULT '',
            modelName TEXT NOT NULL DEFAULT '',
            inputTokens INTEGER NOT NULL DEFAULT 0,
            outputTokens INTEGER NOT NULL DEFAULT 0,
            cachedInputTokens INTEGER NOT NULL DEFAULT 0,
            sentAt INTEGER NOT NULL DEFAULT 0,
            outputDurationMs INTEGER NOT NULL DEFAULT 0,
            waitDurationMs INTEGER NOT NULL DEFAULT 0,
            completedAt INTEGER NOT NULL DEFAULT 0,
            displayMode TEXT NOT NULL DEFAULT 'NORMAL',
            isFavorite INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(chatId) REFERENCES chats(id) ON DELETE CASCADE
        );

        CREATE TABLE message_variants (
            variantId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            chatId TEXT NOT NULL,
            messageTimestamp INTEGER NOT NULL,
            variantIndex INTEGER NOT NULL,
            content TEXT NOT NULL,
            roleName TEXT NOT NULL DEFAULT '',
            provider TEXT NOT NULL DEFAULT '',
            modelName TEXT NOT NULL DEFAULT '',
            inputTokens INTEGER NOT NULL DEFAULT 0,
            outputTokens INTEGER NOT NULL DEFAULT 0,
            cachedInputTokens INTEGER NOT NULL DEFAULT 0,
            sentAt INTEGER NOT NULL DEFAULT 0,
            outputDurationMs INTEGER NOT NULL DEFAULT 0,
            waitDurationMs INTEGER NOT NULL DEFAULT 0,
            completedAt INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(chatId) REFERENCES chats(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS index_messages_chatId_timestamp
            ON messages(chatId, timestamp);
        CREATE INDEX IF NOT EXISTS index_messages_chatId_orderIndex
            ON messages(chatId, orderIndex);
        CREATE INDEX IF NOT EXISTS index_message_variants_chatId_messageTimestamp
            ON message_variants(chatId, messageTimestamp);
        CREATE UNIQUE INDEX IF NOT EXISTS index_message_variants_chatId_messageTimestamp_variantIndex
            ON message_variants(chatId, messageTimestamp, variantIndex);
        "#,
    )?;
    createUsageRequestRecordsTable(store)?;
    createSyncTables(store)
}

#[allow(non_snake_case)]
fn createUsageRequestRecordsTable(store: &SqliteStore) -> Result<(), SqliteStoreError> {
    store.executeBatch(
        r#"
        CREATE TABLE IF NOT EXISTS usage_request_records (
            id TEXT PRIMARY KEY NOT NULL,
            createdAtMs INTEGER NOT NULL,
            providerModel TEXT NOT NULL,
            provider TEXT NOT NULL,
            modelName TEXT NOT NULL,
            functionType TEXT NOT NULL,
            source TEXT NOT NULL,
            chatId TEXT,
            inputTokens INTEGER NOT NULL,
            outputTokens INTEGER NOT NULL,
            cachedInputTokens INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS index_usage_request_records_createdAtMs
            ON usage_request_records(createdAtMs);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_providerModel
            ON usage_request_records(providerModel);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_provider
            ON usage_request_records(provider);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_modelName
            ON usage_request_records(modelName);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_functionType
            ON usage_request_records(functionType);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_source
            ON usage_request_records(source);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_chatId
            ON usage_request_records(chatId);
        "#,
    )
}

/// Creates tables used by SQL sync clocks, operations, rows, and deletions.
pub fn createSyncTables(store: &SqliteStore) -> Result<(), SqliteStoreError> {
    store.executeBatch(
        r#"
        CREATE TABLE IF NOT EXISTS sync_sql_clocks (
            originDeviceId TEXT PRIMARY KEY NOT NULL,
            sequence INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sync_sql_operations (
            opId TEXT PRIMARY KEY NOT NULL,
            originDeviceId TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            domain TEXT NOT NULL,
            entityType TEXT NOT NULL,
            entityId TEXT NOT NULL,
            operation TEXT NOT NULL,
            createdAt INTEGER NOT NULL,
            schemaVersion INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS index_sync_sql_operations_origin_sequence
            ON sync_sql_operations(originDeviceId, sequence);
        CREATE INDEX IF NOT EXISTS index_sync_sql_operations_createdAt
            ON sync_sql_operations(createdAt);

        CREATE TABLE IF NOT EXISTS sync_sql_chat_rows (
            opId TEXT NOT NULL,
            id TEXT NOT NULL,
            title TEXT NOT NULL,
            createdAt INTEGER NOT NULL,
            updatedAt INTEGER NOT NULL,
            inputTokens INTEGER NOT NULL,
            outputTokens INTEGER NOT NULL,
            currentWindowSize INTEGER NOT NULL,
            "group" TEXT,
            displayOrder INTEGER NOT NULL,
            workspace TEXT,
            workspaceEnv TEXT,
            parentChatId TEXT,
            characterCardName TEXT,
            characterGroupId TEXT,
            locked INTEGER NOT NULL,
            pinned INTEGER NOT NULL,
            PRIMARY KEY(opId, id),
            FOREIGN KEY(opId) REFERENCES sync_sql_operations(opId) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS sync_sql_message_rows (
            opId TEXT NOT NULL,
            chatId TEXT NOT NULL,
            sender TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            orderIndex INTEGER NOT NULL,
            roleName TEXT NOT NULL,
            selectedVariantIndex INTEGER NOT NULL,
            provider TEXT NOT NULL,
            modelName TEXT NOT NULL,
            inputTokens INTEGER NOT NULL,
            outputTokens INTEGER NOT NULL,
            cachedInputTokens INTEGER NOT NULL,
            sentAt INTEGER NOT NULL,
            outputDurationMs INTEGER NOT NULL,
            waitDurationMs INTEGER NOT NULL,
            completedAt INTEGER NOT NULL,
            displayMode TEXT NOT NULL,
            isFavorite INTEGER NOT NULL,
            PRIMARY KEY(opId, chatId, timestamp),
            FOREIGN KEY(opId) REFERENCES sync_sql_operations(opId) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS sync_sql_message_variant_rows (
            opId TEXT NOT NULL,
            chatId TEXT NOT NULL,
            messageTimestamp INTEGER NOT NULL,
            variantIndex INTEGER NOT NULL,
            content TEXT NOT NULL,
            roleName TEXT NOT NULL,
            provider TEXT NOT NULL,
            modelName TEXT NOT NULL,
            inputTokens INTEGER NOT NULL,
            outputTokens INTEGER NOT NULL,
            cachedInputTokens INTEGER NOT NULL,
            sentAt INTEGER NOT NULL,
            outputDurationMs INTEGER NOT NULL,
            waitDurationMs INTEGER NOT NULL,
            completedAt INTEGER NOT NULL,
            PRIMARY KEY(opId, chatId, messageTimestamp, variantIndex),
            FOREIGN KEY(opId) REFERENCES sync_sql_operations(opId) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS sync_sql_deletions (
            opId TEXT NOT NULL,
            ordinal INTEGER NOT NULL,
            tableName TEXT NOT NULL,
            chatId TEXT NOT NULL,
            messageTimestamp INTEGER,
            variantIndex INTEGER,
            PRIMARY KEY(opId, ordinal),
            FOREIGN KEY(opId) REFERENCES sync_sql_operations(opId) ON DELETE CASCADE
        );
        "#,
    )
}
