//! Plugin-owned databases and native JSON preferences over the existing Host.
#![allow(non_snake_case)]
#[path = "PluginDataStore.rs"]
mod data_store;

use crate::MergeRules::{canonicalContent, uniqueNames};
use crate::SqliteStore::{SqliteRowGet, SqliteStore, SqliteStoreError, SqliteTransaction};
use crate::SyncOperationStore::{
    subscribeSyncMutations, SyncClock, SyncOperation,
    SyncOperationSemantics, SyncOperationKnowledge, SyncReceipt, publishSyncMutation,
};
use operit_host_api::PluginStorage::*;
use operit_host_api::{RuntimeSqliteHost, RuntimeStorageHost, SqliteValue};
use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership, PLUGIN_DATA_SPACE_DIR_PATH, PLUGIN_DATA_DEVICE_DIR_PATH};
pub use operit_util::PluginStoragePaths::resolveStoragePath;
use operit_util::PluginStoragePaths::storageNamespace;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlparser::{
    ast::{Ident, Statement, Visit, Visitor},
    dialect::SQLiteDialect,
    parser::Parser,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub const PLUGIN_STORAGE_SYNC_DOMAIN: &str = "plugin_storage";
const CATALOG_PATH: &str = "runtime/extensions/device/storage/catalog.sqlite";

const DATABASE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS _operit_manifest (id INTEGER PRIMARY KEY CHECK(id=1), owner TEXT NOT NULL, path TEXT NOT NULL, kind TEXT NOT NULL, sync INTEGER NOT NULL, merge TEXT);
CREATE TABLE IF NOT EXISTS _operit_origins (device TEXT PRIMARY KEY, origin TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS _operit_tables (name TEXT PRIMARY KEY, definition TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS _operit_values (collection TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, version TEXT NOT NULL, PRIMARY KEY(collection,key));
CREATE TABLE IF NOT EXISTS _operit_dirty (collection TEXT NOT NULL, key TEXT NOT NULL, deleted INTEGER NOT NULL, PRIMARY KEY(collection,key));
CREATE TABLE IF NOT EXISTS _operit_log (revision INTEGER PRIMARY KEY AUTOINCREMENT, origin TEXT NOT NULL, sequence INTEGER NOT NULL, created_at INTEGER NOT NULL, payload TEXT NOT NULL, outgoing INTEGER NOT NULL, UNIQUE(origin,sequence));
CREATE INDEX IF NOT EXISTS _operit_log_time ON _operit_log(created_at);
CREATE TABLE IF NOT EXISTS _operit_versions (collection TEXT NOT NULL, key TEXT NOT NULL, created_at INTEGER NOT NULL, origin TEXT NOT NULL, sequence INTEGER NOT NULL, PRIMARY KEY(collection,key));
CREATE TABLE IF NOT EXISTS _operit_merge_map (collection TEXT NOT NULL, origin TEXT NOT NULL, remote_key TEXT NOT NULL, canonical_key TEXT NOT NULL, PRIMARY KEY(collection,origin,remote_key));
CREATE TABLE IF NOT EXISTS _operit_merge_names (collection TEXT NOT NULL, key TEXT NOT NULL, name TEXT NOT NULL, PRIMARY KEY(collection,key));
"#;
const OBJECT_CHANGE_SCHEMA: &[&str] = &[
    r#"CREATE TABLE IF NOT EXISTS _operit_changes (revision INTEGER PRIMARY KEY AUTOINCREMENT, collection TEXT NOT NULL, key TEXT NOT NULL)"#,
    r#"CREATE TRIGGER IF NOT EXISTS _operit_changes_insert AFTER INSERT ON _operit_values BEGIN
  INSERT INTO _operit_changes(collection,key) VALUES(NEW.collection,NEW.key);
END;"#,
    r#"CREATE TRIGGER IF NOT EXISTS _operit_changes_update AFTER UPDATE OF value,collection,key ON _operit_values BEGIN
  INSERT INTO _operit_changes(collection,key) VALUES(NEW.collection,NEW.key);
END;"#,
    r#"CREATE TRIGGER IF NOT EXISTS _operit_changes_delete AFTER DELETE ON _operit_values BEGIN
  INSERT INTO _operit_changes(collection,key) VALUES(OLD.collection,OLD.key);
END;"#,
    r#"CREATE TRIGGER IF NOT EXISTS _operit_changes_trim AFTER INSERT ON _operit_changes WHEN NEW.revision % 1024 = 0 BEGIN
  DELETE FROM _operit_changes WHERE revision <= NEW.revision - 65536;
END;"#,
];
const CATALOG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS plugin_databases (path TEXT PRIMARY KEY, owner TEXT NOT NULL, kind TEXT NOT NULL, sync INTEGER NOT NULL, merge TEXT);
"#;

/// Owns all handles for one authenticated JavaScript execution host.
#[derive(Clone, Default)]
pub struct PluginStorageSession {
    handles: Arc<Mutex<BTreeMap<String, Handle>>>,
}

#[derive(Clone)]
/// Retains the selected persistence engine rather than disguising key storage as SQLite.
enum Handle {
    Database(Arc<Database>),
    DataStore(Arc<data_store::DataStore>),
}

struct Database {
    store: SqliteStore,
    path: String,
    owner: String,
    kind: StorageKind,
    sync: bool,
    merge: Option<StorageMergeDescriptor>,
    origin: String,
}

/// Preserves the durable database identity carried by a change transaction.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    pub path: String,
    pub owner: String,
    pub kind: StorageKind,
    pub sync: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge: Option<StorageMergeDescriptor>,
}

/// Describes one declarative record mutation with an application-owned JSON value.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    pub collection: String,
    pub key: String,
    pub value: Value,
    pub deleted: bool,
}

/// Preserves all schema and record changes from one atomic storage transaction.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    pub manifest: Manifest,
    pub tables: Vec<StorageTable>,
    pub changes: Vec<Change>,
}


/// Describes the published collection-only key-reference encoding.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalKeyReference {
    collection: String,
    #[serde(default)]
    prefix: String,
}

/// Migrates the explicitly identified historical key-reference schema before strict contract validation.
fn decodeMergeDescriptor(mut value: Value) -> Result<StorageMergeDescriptor, SqliteStoreError> {
    let version = value
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| failure("Merge contract version is missing"))?;
    if !(1..=4).contains(&version) {
        return Err(failure("Unsupported database merge contract version"));
    }
    if let Some(entities) = value.get_mut("entities").and_then(Value::as_array_mut) {
        for entity in entities {
            if let Some(references) = entity
                .get_mut("keyReferences")
                .and_then(Value::as_array_mut)
            {
                for reference in references {
                    if reference.get("collection").is_some() {
                        if version >= 4 {
                            return Err(failure(
                                "Collection-only key references require their historical contract version",
                            ));
                        }
                        let historical: HistoricalKeyReference =
                            serde_json::from_value(reference.clone())
                                .map_err(|error| failure(error.to_string()))?;
                        *reference = serde_json::json!({
                            "target": {"kind": "collection", "collection": historical.collection}, "prefix": historical.prefix,
                        });
                    }
                }
            }
        }
    }
    serde_json::from_value(value).map_err(|error| failure(error.to_string()))
}

/// Decodes persisted metadata through its explicit historical contract migration.
fn decodeManifest(mut value: Value) -> Result<Manifest, SqliteStoreError> {
    if let Some(merge) = value.get_mut("merge").filter(|value| !value.is_null()) {
        *merge = serde_json::to_value(decodeMergeDescriptor(merge.clone())?)
            .map_err(|error| failure(error.to_string()))?;
    }
    serde_json::from_value(value).map_err(|error| failure(error.to_string()))
}

/// Decodes a transaction for materialization while its original JSON remains immutable in the log.
fn decodePayload(mut value: Value) -> Result<Payload, SqliteStoreError> {
    let manifest = value
        .get_mut("manifest")
        .ok_or_else(|| failure("Storage transaction manifest is missing"))?;
    *manifest = serde_json::to_value(decodeManifest(manifest.clone())?)
        .map_err(|error| failure(error.to_string()))?;
    serde_json::from_value(value).map_err(|error| failure(error.to_string()))
}

/// Maps a store validation failure into its original explicit error channel.
fn failure(message: impl Into<String>) -> SqliteStoreError {
    SqliteStoreError::Message(message.into())
}

/// Encodes a stable database kind without relying on enum debug spellings.
fn kindName(kind: &StorageKind) -> &'static str {
    match kind {
        StorageKind::Sqlite => "sqlite",
        StorageKind::Objects => "objects",
    }
}

/// Opens the node-local catalog used for storage discovery, export and file exclusions.
fn catalog(host: Arc<dyn RuntimeSqliteHost>) -> Result<SqliteStore, SqliteStoreError> {
    let store = SqliteStore::openWithHost(host, CATALOG_PATH)?;
    store.executeBatch(CATALOG_SCHEMA)?;
    let columns = store.queryRows("PRAGMA table_info(plugin_databases)", Vec::new())?;
    if !columns
        .iter()
        .any(|row| row.get::<_, String>(1).ok().as_deref() == Some("merge"))
    {
        store.execute("ALTER TABLE plugin_databases ADD COLUMN merge TEXT", Vec::new())?;
    }
    Ok(store)
}

/// Converts typed SQL arguments while checking exact int64 and finite real values.
fn sqlValue(value: StorageSqlValue) -> Result<SqliteValue, SqliteStoreError> {
    Ok(match value {
        StorageSqlValue::Null => SqliteValue::Null,
        StorageSqlValue::Integer(value) => {
            let parsed = value
                .parse::<i64>()
                .map_err(|_| failure("Invalid SQLite int64"))?;
            if parsed.to_string() != value {
                return Err(failure(
                    "SQLite integer must use canonical decimal spelling",
                ));
            }
            SqliteValue::Integer(parsed)
        }
        StorageSqlValue::Real(value) => {
            if !value.is_finite() {
                return Err(failure("SQLite real must be finite"));
            }
            SqliteValue::Real(value)
        }
        StorageSqlValue::Text(value) => SqliteValue::Text(value),
        StorageSqlValue::Blob(value) => SqliteValue::Blob(value),
    })
}

/// Preserves every SQL result value using explicit portable type tags.
fn wireValue(value: SqliteValue) -> StorageSqlValue {
    match value {
        SqliteValue::Null => StorageSqlValue::Null,
        SqliteValue::Integer(v) => StorageSqlValue::Integer(v.to_string()),
        SqliteValue::Real(v) => StorageSqlValue::Real(v),
        SqliteValue::Text(v) => StorageSqlValue::Text(v),
        SqliteValue::Blob(v) => StorageSqlValue::Blob(v),
    }
}

/// Parses actual SQL and rejects bridge escape, schema rewrites and protected runtime tables.
fn validateStatement(
    statement: &StorageStatement,
    query: bool,
    sync: bool,
) -> Result<(), SqliteStoreError> {
    if query && statement.expected_affected_rows.is_some() {
        return Err(failure("Affected-row preconditions require a mutation"));
    }
    if statement.sql.len() > 1_000_000 || statement.params.len() > 32766 {
        return Err(failure("SQLite statement exceeds request limits"));
    }
    let dialect = SQLiteDialect {};
    let parsed = Parser::parse_sql(&dialect, &statement.sql).map_err(|e| failure(e.to_string()))?;
    if parsed.len() != 1 {
        return Err(failure("Exactly one SQL statement is required"));
    }
    let allowed = match &parsed[0] {
        Statement::Query(_) => query,
        Statement::Insert(_) | Statement::Update { .. } | Statement::Delete(_) => !query,
        Statement::CreateTable(_) => !query && !sync,
        Statement::CreateIndex(index) => !query && (!sync || !index.unique),
        _ => false,
    };
    if !allowed {
        return Err(failure(
            "SQL statement is not permitted by this storage contract",
        ));
    }
    if let ControlFlow::Break(error) = parsed.visit(&mut SqlAccessValidator) {
        return Err(error);
    }
    Ok(())
}

/// Checks resolved identifiers, including quoted relations and function names.
struct SqlAccessValidator;
impl Visitor for SqlAccessValidator {
    type Break = SqliteStoreError;
    /// Rejects internal tables, SQLite introspection and external filesystem functions in the AST.
    fn pre_visit_ident(&mut self, ident: &Ident) -> ControlFlow<Self::Break> {
        let name = ident.value.to_ascii_lowercase();
        if name.starts_with("_operit_")
            || name.starts_with("sqlite_")
            || name.starts_with("pragma_")
            || matches!(
                name.as_str(),
                "load_extension" | "readfile" | "writefile" | "eval"
            )
        {
            return ControlFlow::Break(failure("SQL references a protected storage capability"));
        }
        ControlFlow::Continue(())
    }
}

/// Validates an identifier structurally before generating runtime-controlled SQL.
fn identifier(value: &str) -> Result<String, SqliteStoreError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit()))
        || value.to_ascii_lowercase().starts_with("_operit_")
        || value.to_ascii_lowercase().starts_with("sqlite_")
    {
        return Err(failure("Invalid or reserved SQLite identifier"));
    }
    Ok(format!("\"{value}\""))
}

/// Produces a portable declared schema with deterministic column ordering.
fn tableSql(table: &StorageTable) -> Result<String, SqliteStoreError> {
    let name = identifier(&table.name)?;
    if table.columns.is_empty() || table.columns.len() > 100 {
        return Err(failure("A synchronized table requires 1..100 columns"));
    }
    let mut seen = BTreeSet::new();
    let mut columns = Vec::new();
    let mut primary = false;
    for column in &table.columns {
        let name = identifier(&column.name)?;
        if !seen.insert(column.name.to_ascii_lowercase()) {
            return Err(failure("Duplicate SQLite column"));
        }
        let isPrimary = column.name == table.primary_key;
        if isPrimary {
            if column.affinity != StorageColumnType::Text || column.nullable {
                return Err(failure("Synchronized primary key must be nonnullable TEXT"));
            }
            primary = true;
        }
        let affinity = match column.affinity {
            StorageColumnType::Text => "TEXT",
            StorageColumnType::Integer => "INTEGER",
            StorageColumnType::Real => "REAL",
            StorageColumnType::Blob => "BLOB",
        };
        columns.push(format!(
            "{name} {affinity}{}{}",
            if column.nullable { "" } else { " NOT NULL" },
            if isPrimary { " PRIMARY KEY" } else { "" }
        ));
    }
    if !primary {
        return Err(failure("Primary key is not present in declared columns"));
    }
    Ok(format!("CREATE TABLE {name} ({})", columns.join(",")))
}

/// Installs row-level dirty tracking without encoding or walking unaffected rows.
fn defineTable(
    tx: &mut SqliteTransaction<'_>,
    table: &StorageTable,
) -> Result<bool, SqliteStoreError> {
    let sql = tableSql(table)?;
    let definition = serde_json::to_string(table).map_err(|e| failure(e.to_string()))?;
    if let Some(row) = tx.queryOne(
        "SELECT definition FROM _operit_tables WHERE name=?1",
        vec![SqliteValue::Text(table.name.clone())],
    )? {
        let previous: StorageTable = serde_json::from_str(&row.get::<_, String>(0)?)
            .map_err(|error| failure(error.to_string()))?;
        if previous.primary_key == table.primary_key
            && table.columns.iter().all(|incoming| {
                previous.columns.iter().any(|current| current == incoming)
            })
        {
            return Ok(false);
        }
        if previous.primary_key != table.primary_key
            || previous.columns.iter().any(|old| {
                table.columns.iter().find(|current| current.name == old.name)
                    != Some(old)
            })
            || table.columns.iter().any(|current| {
                !previous.columns.iter().any(|old| old.name == current.name)
                    && (!current.nullable || current.name == table.primary_key)
            })
        {
            return Err(failure("Table declaration differs from the persisted schema"));
        }
        for column in &table.columns {
            if previous.columns.iter().any(|old| old.name == column.name) {
                continue;
            }
            let name = identifier(&table.name)?;
            let columnName = identifier(&column.name)?;
            let affinity = match column.affinity {
                StorageColumnType::Text => "TEXT",
                StorageColumnType::Integer => "INTEGER",
                StorageColumnType::Real => "REAL",
                StorageColumnType::Blob => "BLOB",
            };
            tx.execute(
                &format!("ALTER TABLE {name} ADD COLUMN {columnName} {affinity}"),
                Vec::new(),
            )?;
        }
        tx.execute(
            "UPDATE _operit_tables SET definition=?1 WHERE name=?2",
            vec![
                SqliteValue::Text(definition),
                SqliteValue::Text(table.name.clone()),
            ],
        )?;
        return Ok(false);
    }
    tx.execute(&sql, Vec::new())?;
    tx.execute(
        "INSERT INTO _operit_tables VALUES(?1,?2)",
        vec![
            SqliteValue::Text(table.name.clone()),
            SqliteValue::Text(definition),
        ],
    )?;
    installDirtyTracking(tx, table)?;
    Ok(true)
}

/// Uses explicit UPSERT so an outer statement cannot override the trigger's conflict policy.
fn installDirtyTracking(tx: &mut SqliteTransaction<'_>, table: &StorageTable) -> Result<(), SqliteStoreError> {
    let name = identifier(&table.name)?;
    let key = identifier(&table.primary_key)?;
    for (suffix, event, body) in [
        (
            "insert",
            "INSERT",
            format!(
                "INSERT INTO _operit_dirty VALUES('{}',NEW.{key},0) ON CONFLICT(collection,key) DO UPDATE SET deleted=excluded.deleted;",
                table.name
            ),
        ),
        (
            "delete",
            "DELETE",
            format!(
                "INSERT INTO _operit_dirty VALUES('{}',OLD.{key},1) ON CONFLICT(collection,key) DO UPDATE SET deleted=excluded.deleted;",
                table.name
            ),
        ),
        (
            "update",
            "UPDATE",
            format!(
                "INSERT INTO _operit_dirty VALUES('{}',OLD.{key},1) ON CONFLICT(collection,key) DO UPDATE SET deleted=excluded.deleted; INSERT INTO _operit_dirty VALUES('{}',NEW.{key},0) ON CONFLICT(collection,key) DO UPDATE SET deleted=excluded.deleted;",
                table.name, table.name
            ),
        ),
    ] {
        tx.execute(&format!("DROP TRIGGER IF EXISTS \"_operit_track_{}_{suffix}\"", table.name), Vec::new())?;
        tx.execute(&format!("CREATE TRIGGER \"_operit_track_{}_{suffix}\" AFTER {event} ON {name} BEGIN {body} END", table.name), Vec::new())?;
    }
    Ok(())
}

/// Installs monotonic declarative contract updates while leaving owner data migrations to their owner.
fn mergeDescriptorForOpen(
    stored: Option<StorageMergeDescriptor>,
    requested: Option<StorageMergeDescriptor>,
) -> Result<Option<StorageMergeDescriptor>, SqliteStoreError> {
    for descriptor in stored.iter().chain(requested.iter()) {
        if !(1..=4).contains(&descriptor.version) {
            return Err(failure("Unsupported database merge contract version"));
        }
    }
    match (stored, requested) {
        (None, requested) => Ok(requested),
        (Some(stored), None) => Ok(Some(stored)),
        (Some(stored), Some(requested)) if stored == requested => Ok(Some(requested)),
        (Some(stored), Some(requested)) if requested.version > stored.version => Ok(Some(requested)),
        (Some(stored), Some(requested)) if stored.version > requested.version => Ok(Some(stored)),
        _ => Err(failure("Database merge contract has conflicting definitions at the same version")),
    }
}

/// Opens only a database carrying the same owner, path, kind and synchronization policy.
fn openDatabase(
    host: Arc<dyn RuntimeSqliteHost>,
    manifest: &Manifest,
    mut origin: String,
) -> Result<Database, SqliteStoreError> {
    let store = SqliteStore::openWithHost(host, &manifest.path)?;
    store.execute("PRAGMA recursive_triggers = ON", Vec::new())?;
    let effectiveMerge = store.transaction(|tx| {
        let mut effectiveMerge = manifest.merge.clone();
        let tables = tx.queryRows("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", Vec::new())?;
        if !tables.is_empty() && !tables.iter().any(|row| matches!(row.values.first(), Some(SqliteValue::Text(name)) if name == "_operit_manifest")) {
            return Err(failure("Existing database does not carry a plugin storage manifest"));
        }
        let existing = !tables.is_empty();
        for sql in DATABASE_SCHEMA.split(';').filter(|sql| !sql.trim().is_empty()) { tx.execute(sql, Vec::new())?; }
        if manifest.kind == StorageKind::Objects { for sql in OBJECT_CHANGE_SCHEMA { tx.execute(sql, Vec::new())?; } }
        let manifestColumns = tx.queryRows("PRAGMA table_info(_operit_manifest)", Vec::new())?;
        if !manifestColumns.iter().any(|row| row.get::<_, String>(1).ok().as_deref() == Some("merge")) {
            tx.execute("ALTER TABLE _operit_manifest ADD COLUMN merge TEXT", Vec::new())?;
        }

        if let Some(row) = tx.queryOne(
            "SELECT owner,path,kind,sync,merge FROM _operit_manifest WHERE id=1",
            Vec::new(),
        )? {
            if row.get::<_, String>(0)? != manifest.owner
                || row.get::<_, String>(1)? != manifest.path
                || row.get::<_, String>(2)? != kindName(&manifest.kind)
                || row.get::<_, i64>(3)? != i64::from(manifest.sync)
            {
                return Err(failure("Database identity or storage policy mismatch"));
            }
            let storedEncoding: Option<String> = row.get(4)?;
            let storedMerge = storedEncoding.as_ref()
                .map(|value| serde_json::from_str(value).map_err(|error| failure(error.to_string())).and_then(decodeMergeDescriptor))
                .transpose()?;
            let merge = mergeDescriptorForOpen(storedMerge, manifest.merge.clone())?;
            effectiveMerge = merge.clone();
            let encoding = merge.as_ref().map(serde_json::to_string).transpose().map_err(|error| failure(error.to_string()))?;
            if encoding != storedEncoding {
                tx.execute(
                    "UPDATE _operit_manifest SET merge=?1 WHERE id=1",
                    vec![match encoding { Some(value) => SqliteValue::Text(value), None => SqliteValue::Null }],
                )?;
            }
        } else {
            if existing { return Err(failure("Existing database has no persisted plugin identity")); }
            tx.execute(
                "INSERT INTO _operit_manifest VALUES(1,?1,?2,?3,?4,?5)",
                vec![
                    SqliteValue::Text(manifest.owner.clone()),
                    SqliteValue::Text(manifest.path.clone()),
                    SqliteValue::Text(kindName(&manifest.kind).into()),
                    SqliteValue::Integer(i64::from(manifest.sync)),
                    manifest
                        .merge
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()
                        .map_err(|error| failure(error.to_string()))?
                        .map_or(SqliteValue::Null, SqliteValue::Text),
                ],
            )?;
        }
        if manifest.kind == StorageKind::Objects {
            tx.execute("CREATE VIEW IF NOT EXISTS plugin_objects AS SELECT collection,key,value,version FROM _operit_values", Vec::new())?;
        }
        // Upgrade existing databases once; fresh databases already have UPSERT triggers.
        let legacy = tx.queryRows(
            "SELECT DISTINCT t.definition FROM _operit_tables t JOIN sqlite_master s ON s.tbl_name=t.name WHERE s.type='trigger' AND s.name GLOB '_operit_track_*' AND instr(s.sql,'INSERT OR REPLACE INTO _operit_dirty')>0",
            Vec::new(),
        )?;
        for row in legacy {
            let table: StorageTable = serde_json::from_str(&row.get::<_, String>(0)?)
                .map_err(|error| failure(error.to_string()))?;
            installDirtyTracking(tx, &table)?;
        }
        if !origin.is_empty() {
            let row = tx.queryOne("SELECT origin FROM _operit_origins WHERE device=?1", vec![SqliteValue::Text(origin.clone())])?;
            match row {
                Some(row) => origin = row.get(0)?,
                None => {
                    let actual = format!("{origin}:{}", Uuid::new_v4());
                    tx.execute("INSERT INTO _operit_origins VALUES(?1,?2)",vec![SqliteValue::Text(origin.clone()),SqliteValue::Text(actual.clone())])?;
                    origin = actual;
                }
            }
        }
        Ok(effectiveMerge)
    })?;
    Ok(Database {
        store,
        path: manifest.path.clone(),
        owner: manifest.owner.clone(),
        kind: manifest.kind.clone(),
        sync: manifest.sync,
        merge: effectiveMerge,
        origin,
    })
}

/// Registers storage durably before any user data or outgoing change is written.
fn registerDatabase(
    host: Arc<dyn RuntimeSqliteHost>,
    manifest: &Manifest,
) -> Result<(), SqliteStoreError> {
    let catalog = catalog(host)?;
    catalog.transaction(|tx| {
        if let Some(row) = tx.queryOne(
            "SELECT owner,kind,sync,merge FROM plugin_databases WHERE path=?1",
            vec![SqliteValue::Text(manifest.path.clone())],
        )? {
            if row.get::<_, String>(0)? != manifest.owner
                || row.get::<_, String>(1)? != kindName(&manifest.kind)
                || row.get::<_, i64>(2)? != i64::from(manifest.sync)
            {
                return Err(failure("Storage catalog identity mismatch"));
            }
            let storedEncoding: Option<String> = row.get(3)?;
            let storedMerge = storedEncoding.as_ref()
                .map(|value| serde_json::from_str(value).map_err(|error| failure(error.to_string())).and_then(decodeMergeDescriptor))
                .transpose()?;
            let merge = mergeDescriptorForOpen(storedMerge, manifest.merge.clone())?;
            let encoding = merge.as_ref().map(serde_json::to_string).transpose().map_err(|error| failure(error.to_string()))?;
            if encoding != storedEncoding {
                tx.execute(
                    "UPDATE plugin_databases SET merge=?1 WHERE path=?2",
                    vec![
                        match encoding { Some(value) => SqliteValue::Text(value), None => SqliteValue::Null },
                        SqliteValue::Text(manifest.path.clone()),
                    ],
                )?;
            }
        } else {
            tx.execute(
                "INSERT INTO plugin_databases VALUES(?1,?2,?3,?4,?5)",
                vec![
                    SqliteValue::Text(manifest.path.clone()),
                    SqliteValue::Text(manifest.owner.clone()),
                    SqliteValue::Text(kindName(&manifest.kind).into()),
                    SqliteValue::Integer(i64::from(manifest.sync)),
                    manifest
                        .merge
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()
                        .map_err(|error| failure(error.to_string()))?
                        .map_or(SqliteValue::Null, SqliteValue::Text),
                ],
            )?;
        }
        Ok(())
    })
}

impl Database {
    /// Constructs the durable owner descriptor carried by every fine-grained operation.
    fn manifest(&self) -> Manifest {
        Manifest {
            path: self.path.clone(),
            owner: self.owner.clone(),
            kind: self.kind.clone(),
            sync: self.sync,
            merge: self.merge.clone(),
        }
    }

    /// Records only changed rows in the same transaction as the original SQL writes.
    fn dirtyChanges(
        &self,
        tx: &mut SqliteTransaction<'_>,
    ) -> Result<Vec<Change>, SqliteStoreError> {
        let mut changes = Vec::new();
        for row in tx.queryRows(
            "SELECT collection,key,deleted FROM _operit_dirty ORDER BY collection,key",
            Vec::new(),
        )? {
            let collection: String = row.get(0)?;
            let key: String = row.get(1)?;
            let value = if row.get::<_, i64>(2)? == 1 {
                None
            } else {
                let definition = tx
                    .queryOne(
                        "SELECT definition FROM _operit_tables WHERE name=?1",
                        vec![SqliteValue::Text(collection.clone())],
                    )?
                    .ok_or_else(|| failure("Dirty table has no declaration"))?;
                let table: StorageTable = serde_json::from_str(&definition.get::<_, String>(0)?)
                    .map_err(|e| failure(e.to_string()))?;
                let record = tx
                    .queryOne(
                        &format!(
                            "SELECT {} FROM {} WHERE {}=?1",
                            table
                                .columns
                                .iter()
                                .map(|c| identifier(&c.name))
                                .collect::<Result<Vec<_>, _>>()?
                                .join(","),
                            identifier(&collection)?,
                            identifier(&table.primary_key)?
                        ),
                        vec![SqliteValue::Text(key.clone())],
                    )?
                    .ok_or_else(|| failure("Dirty row disappeared"))?;
                validateRow(&table, &key, &record.values)?;
                Some(
                    serde_json::to_value(
                        record.values.into_iter().map(wireValue).collect::<Vec<_>>(),
                    )
                    .map_err(|e| failure(e.to_string()))?,
                )
            };
            changes.push(Change {
                collection,
                key,
                deleted: value.is_none(),
                value: value.unwrap_or(Value::Null),
            });
        }
        tx.execute("DELETE FROM _operit_dirty", Vec::new())?;
        Ok(changes)
    }

    /// Advances one database-local sequence and publishes one atomic record transaction.
    fn record(
        &self,
        tx: &mut SqliteTransaction<'_>,
        table: Option<StorageTable>,
        changes: Vec<Change>,
    ) -> Result<String, SqliteStoreError> {
        let sequence = tx
            .queryOne(
                "SELECT COALESCE(MAX(sequence),0)+1 FROM _operit_log WHERE origin=?1",
                vec![SqliteValue::Text(self.origin.clone())],
            )?
            .ok_or_else(|| failure("Missing local sequence"))?
            .get::<_, i64>(0)?;
        let logical = tx
            .queryOne(
                "SELECT COALESCE(MAX(created_at),0)+1 FROM _operit_log",
                Vec::new(),
            )?
            .ok_or_else(|| failure("Missing logical time"))?
            .get::<_, i64>(0)?;
        let createdAt = operit_host_api::TimeUtils::tryCurrentTimeMillis()
            .map_err(|e| failure(e.to_string()))?
            .max(logical);
        let version = format!("{}:{sequence}", self.origin);
        for change in &changes {
            tx.execute("INSERT INTO _operit_versions VALUES(?1,?2,?3,?4,?5) ON CONFLICT(collection,key) DO UPDATE SET created_at=excluded.created_at,origin=excluded.origin,sequence=excluded.sequence", vec![SqliteValue::Text(change.collection.clone()), SqliteValue::Text(change.key.clone()), SqliteValue::Integer(createdAt), SqliteValue::Text(self.origin.clone()), SqliteValue::Integer(sequence)])?;
            if self.kind != StorageKind::Sqlite {
                tx.execute(
                    "UPDATE _operit_values SET version=?3 WHERE collection=?1 AND key=?2",
                    vec![
                        SqliteValue::Text(change.collection.clone()),
                        SqliteValue::Text(change.key.clone()),
                        SqliteValue::Text(version.clone()),
                    ],
                )?;
            }
        }
        let mut tables = table.into_iter().collect::<Vec<_>>();
        if self.kind == StorageKind::Sqlite {
            for collection in changes
                .iter()
                .map(|change| &change.collection)
                .collect::<BTreeSet<_>>()
            {
                let row = tx
                    .queryOne(
                        "SELECT definition FROM _operit_tables WHERE name=?1",
                        vec![SqliteValue::Text(collection.clone())],
                    )?
                    .ok_or_else(|| failure("Changed table has no declaration"))?;
                let declaration: StorageTable = serde_json::from_str(&row.get::<_, String>(0)?)
                    .map_err(|e| failure(e.to_string()))?;
                if !tables.iter().any(|table| &table.name == collection) {
                    tables.push(declaration);
                }
            }
        }
        if let Some(descriptor) = &self.merge {
            rememberPayloadNames(tx, self.kind, descriptor, &changes)?;
        }
        let payload = serde_json::to_string(&Payload {
            manifest: self.manifest(),
            tables,
            changes,
        })
        .map_err(|e| failure(e.to_string()))?;
        tx.execute("INSERT INTO _operit_log(origin,sequence,created_at,payload,outgoing) VALUES(?1,?2,?3,?4,1)", vec![SqliteValue::Text(self.origin.clone()), SqliteValue::Integer(sequence), SqliteValue::Integer(createdAt), SqliteValue::Text(payload)])?;
        Ok(version)
    }
}

impl PluginStorageSession {
    /// Identifies native preference requests without requiring a SQLite host registration.
    pub fn isPreferencesRequest(&self, request: &StorageRequest) -> Result<bool, SqliteStoreError> {
        match request {
            StorageRequest::OpenPreferences { .. } | StorageRequest::PreferencesData { .. }
            | StorageRequest::PreferencesBeginEdit { .. } | StorageRequest::PreferencesFinishEdit { .. }
            | StorageRequest::PreferencesCancelEdit { .. } => Ok(true),
            StorageRequest::Close { handle } => {
                let handles = self.handles.lock().map_err(|_| failure("Storage handles poisoned"))?;
                let owned = handles.get(handle).ok_or_else(|| failure("Unknown or closed storage handle"))?;
                Ok(matches!(owned, Handle::DataStore(_)))
            }
            _ => Ok(false),
        }
    }

    /// Executes native JSON preference operations using only the existing filesystem host abstraction.
    pub fn requestPreferences(&self, storage: Arc<dyn RuntimeStorageHost>, request: StorageRequest) -> Result<StorageResponse, SqliteStoreError> {
        if let StorageRequest::OpenPreferences { path, merge } = &request {
            let path = resolveStoragePath(path).map_err(failure)?;
            let store = Arc::new(data_store::DataStore::open(storage, &path, merge.as_ref())?);
            let handle = Uuid::new_v4().to_string();
            self.handles.lock().map_err(|_| failure("Storage handles poisoned"))?
                .insert(handle.clone(), Handle::DataStore(store));
            return Ok(StorageResponse::Open(StorageOpenResult { handle, path }));
        }
        let handle = match &request {
            StorageRequest::Close { handle } | StorageRequest::PreferencesData { handle }
            | StorageRequest::PreferencesBeginEdit { handle } | StorageRequest::PreferencesFinishEdit { handle, .. }
            | StorageRequest::PreferencesCancelEdit { handle, .. } => handle,
            _ => return Err(failure("Native preference requests require DataStore operations")),
        };
        let owned = self.handles.lock().map_err(|_| failure("Storage handles poisoned"))?
            .get(handle).cloned().ok_or_else(|| failure("Unknown or closed storage handle"))?;
        let Handle::DataStore(store) = owned else { return Err(failure("Preference operations require a DataStore handle")); };
        if matches!(&request, StorageRequest::Close { .. }) {
            self.handles.lock().map_err(|_| failure("Storage handles poisoned"))?.remove(handle);
            // Closing a handle writes nothing. Publishing here would schedule a
            // synchronization pass for every plugin read, and the pass itself
            // opens plugin databases, creating a self-sustaining idle loop.
            return Ok(StorageResponse::Close);
        }
        store.request(request)
    }

    /// Emits an atomic initial record and waits only for changes to that database, collection and key.
    pub async fn waitForRecord(
        &self,
        handle: String,
        collection: String,
        key: String,
        after: Option<StorageRecordWatchState>,
    ) -> Result<Option<StorageValueEntry>, SqliteStoreError> {
        let wake = Arc::new(Mutex::new(None::<std::task::Waker>));
        let listenerWake = wake.clone();
        let _subscription = subscribeSyncMutations(move || {
            let waker = listenerWake.lock().expect("Storage record waker mutex is poisoned").take();
            if let Some(waker) = waker { waker.wake(); }
        });
        std::future::poll_fn(|context| {
            *wake.lock().expect("Storage record waker mutex is poisoned") = Some(context.waker().clone());
            let result = (|| {
                let database = self.handles.lock()
                    .map_err(|_| failure("Storage handle registry is poisoned"))?
                    .get(&handle).cloned().ok_or_else(|| failure("Storage handle is not active"))?;
                let database = match database {
                    Handle::DataStore(store) => {
                        if collection != "__preferences" || key != "snapshot" {
                            return Err(failure("DataStore observations require a complete preferences snapshot"));
                        }
                        return store.watchSnapshot().map(Some);
                    }
                    Handle::Database(database) => database,
                };
                validateCollection(&database, &collection)?;
                database.store.transaction(|tx| {
                    let row = tx.queryOne(
                        "SELECT value,version FROM _operit_values WHERE collection=?1 AND key=?2",
                        vec![SqliteValue::Text(collection.clone()), SqliteValue::Text(key.clone())],
                    )?;
                    row.map(|row| -> Result<StorageValueEntry, SqliteStoreError> {
                        Ok(StorageValueEntry {
                            value: serde_json::from_str(&row.get::<_, String>(0)?)
                                .map_err(|error| failure(error.to_string()))?,
                            version: row.get::<_, String>(1)?,
                        })
                    }).transpose()
                })
            })();
            match result {
                Ok(entry) if after.as_ref().is_some_and(|state| {
                    state.version.as_deref() == entry.as_ref().map(|entry| entry.version.as_str())
                }) => std::task::Poll::Pending,
                Ok(entry) => std::task::Poll::Ready(Ok(entry)),
                Err(error) => std::task::Poll::Ready(Err(error)),
            }
        }).await
    }

    /// Performs one operation using only this session's owner-authenticated retained handles.
    pub fn request(
        &self,
        host: Arc<dyn RuntimeSqliteHost>,
        storage: Arc<dyn RuntimeStorageHost>,
        request: StorageRequest,
        deviceId: &str,
    ) -> Result<StorageResponse, SqliteStoreError> {
        if self.isPreferencesRequest(&request)? { return self.requestPreferences(storage, request); }
        if let StorageRequest::Open { path, kind, merge } = &request {
            let path = resolveStoragePath(path).map_err(failure)?;
            if path.ends_with(".preferences.json") {return Err(failure("Preference JSON paths require a DataStore handle"));}
            let sync = match runtimeStorageOwnership(&path).map_err(failure)? {
                RuntimeStorageOwnership::Space => true,
                RuntimeStorageOwnership::CoreNode => false,
                RuntimeStorageOwnership::Ephemeral => {
                    return Err(failure(
                        "Plugin databases require durable storage ownership",
                    ));
                }
            };
            let manifest = Manifest {
                path: path.clone(),
                owner: storageNamespace(&path).map_err(failure)?.to_string(),
                kind: *kind,
                sync,
                merge: merge.clone(),
            };
            let digest = format!("{:x}", Sha256::digest(path.as_bytes()));
            let database = Arc::new(openDatabase(
                host.clone(),
                &manifest,
                format!("{deviceId}:storage:{digest}"),
            )?);
            registerDatabase(host, &manifest)?;
            registerFile(storage.clone(), &manifest)?;
            if manifest.merge.is_some() {
                let descriptor = manifest.merge.clone().expect("merge descriptor checked");
                database.store.transaction(|tx| {
                    if hasDurableAlias(tx)? {
                        reconcileDurableMappings(tx, manifest.kind, &descriptor)?;
                        tx.execute("DELETE FROM _operit_dirty", Vec::new())?;
                    }
                    Ok(())
                })?;
            }
            let handle = Uuid::new_v4().to_string();
            self.handles
                .lock()
                .map_err(|_| failure("Storage handles poisoned"))?
                .insert(handle.clone(), Handle::Database(database));
            return Ok(StorageResponse::Open(StorageOpenResult { handle, path }));
        }
        let handle = match &request {
            StorageRequest::PreferencesData { handle }
            | StorageRequest::PreferencesBeginEdit { handle }
            | StorageRequest::PreferencesFinishEdit { handle, .. }
            | StorageRequest::PreferencesCancelEdit { handle, .. }
            | StorageRequest::Close { handle }
            | StorageRequest::Query { handle, .. }
            | StorageRequest::Execute { handle, .. }
            | StorageRequest::Transaction { handle, .. }
            | StorageRequest::DefineTable { handle, .. }
            | StorageRequest::Get { handle, .. }
            | StorageRequest::GetMany { handle, .. }
            | StorageRequest::Changes { handle, .. }
            | StorageRequest::List { handle, .. }
            | StorageRequest::Commit { handle, .. } => handle,
            StorageRequest::Open { .. } | StorageRequest::OpenPreferences { .. } => unreachable!(),
        };
        let database = self
            .handles
            .lock()
            .map_err(|_| failure("Storage handles poisoned"))?
            .get(handle)
            .cloned()
            .ok_or_else(|| failure("Unknown or closed storage handle"))?;
        if let StorageRequest::Close { handle } = &request {
            self.handles
                .lock()
                .map_err(|_| failure("Storage handles poisoned"))?
                .remove(handle);
            // See requestPreferences: a close has no data to publish, and
            // treating it as a mutation feeds every plugin read back into the
            // synchronization scheduler.
            return Ok(StorageResponse::Close);
        }
        let database = match database {
            Handle::DataStore(store) => {
                return store.request(request);
            }
            Handle::Database(database) => database,
        };
        resolveStoragePath(&database.path).map_err(failure)?;
        if !storage.exists(&database.path)? || !ownsFile(storage.clone(), &database.path)? {
            return Err(failure("Storage database has been removed"));
        }
        let request = match request {
            StorageRequest::Query { statement, .. } => {
                validateStatement(&statement, true, database.sync)?;
                let rows = database.store.queryRows(
                    &statement.sql,
                    statement.params.into_iter().map(sqlValue).collect::<Result<Vec<_>, _>>()?,
                )?;
                return Ok(StorageResponse::Query(rows.into_iter().map(|row| StorageSqlRow {
                    columns: row.columns,
                    values: row.values.into_iter().map(wireValue).collect(),
                }).collect()));
            }
            request => request,
        };
        let mutating = matches!(
            &request,
            StorageRequest::Execute { .. }
                | StorageRequest::Transaction { .. }
                | StorageRequest::DefineTable { .. }
                | StorageRequest::Commit { .. }
        );
        let result = database.store.transaction(|tx| match request {
            StorageRequest::Execute {statement,..} => executeStatements(&database, tx, vec![statement]).map(StorageResponse::Execute),
            StorageRequest::Transaction {statements, when_absent, ..} => {
                if let Some(identity) = when_absent {
                    let definition = tx.queryOne("SELECT definition FROM _operit_tables WHERE name=?1",
                        vec![SqliteValue::Text(identity.collection.clone())])?
                        .ok_or_else(|| failure("Transaction guard table is not declared"))?;
                    let table: StorageTable = serde_json::from_str(&definition.get::<_, String>(0)?)
                        .map_err(|error| failure(error.to_string()))?;
                    if tx.queryOne(&format!("SELECT 1 FROM {} WHERE {}=?1", identifier(&table.name)?, identifier(&table.primary_key)?),
                        vec![SqliteValue::Text(identity.key)])?.is_some() {
                        return Ok(StorageResponse::Transaction(Vec::new()));
                    }
                }
                executeStatements(&database, tx, statements).map(StorageResponse::Transaction)
            },
            StorageRequest::DefineTable {table,..} => {
                if database.kind != StorageKind::Sqlite { return Err(failure("Table declarations require a SQLite handle")); }
                let changed = defineTable(tx,&table)?;
                if changed { database.record(tx,Some(table),Vec::new())?; }
                Ok(StorageResponse::DefineTable(StorageTableDefinitionResult { changed }))
            }
            StorageRequest::Get {collection,key,..} => {
                validateCollection(&database, &collection)?;
                readValue(tx, &collection, &key).map(StorageResponse::Get)
            }
            StorageRequest::GetMany {collection,keys,..} => {
                validateCollection(&database, &collection)?;
                if keys.is_empty() || keys.len() > 1000 {
                    return Err(failure("Object batch read requires 1..1000 keys"));
                }
                keys.iter().map(|key| readValue(tx, &collection, key))
                    .collect::<Result<Vec<_>, _>>().map(StorageResponse::GetMany)
            }
            StorageRequest::List {collection,after,limit,..} => {
                validateCollection(&database, &collection)?; bounded(limit)?;
                let rows = match after {
                    Some(after) => tx.queryRows("SELECT key,value,version FROM _operit_values WHERE collection=?1 AND key>?2 ORDER BY key LIMIT ?3",vec![SqliteValue::Text(collection),SqliteValue::Text(after),SqliteValue::Integer(i64::from(limit))])?,
                    None => tx.queryRows("SELECT key,value,version FROM _operit_values WHERE collection=?1 ORDER BY key LIMIT ?2",vec![SqliteValue::Text(collection),SqliteValue::Integer(i64::from(limit))])?,
                };
                rows.into_iter().map(|row| Ok(StorageKeyValueEntry {
                    key: row.get::<_, String>(0)?,
                    value: serde_json::from_str(&row.get::<_, String>(1)?).map_err(|error| failure(error.to_string()))?,
                    version: row.get::<_, String>(2)?,
                })).collect::<Result<Vec<_>, SqliteStoreError>>().map(StorageResponse::List)
            }
            StorageRequest::Changes {after,limit,..} => objectChanges(&database, tx, after, limit).map(StorageResponse::Changes),
            StorageRequest::Commit {mutations,guards,..} => {
                if guards.len() > 1000 { return Err(failure("Object commit exceeds 1000 record guards")); }
                for guard in guards {
                    validateCollection(&database, &guard.collection)?;
                    let actual = readValue(tx, &guard.collection, &guard.key)?.map(|entry| entry.version);
                    if actual != guard.expected_version { return Err(failure("Storage record guard failed")); }
                }
                commitValues(&database,tx,mutations).map(StorageResponse::Commit)
            },
            StorageRequest::PreferencesData { .. }
            | StorageRequest::PreferencesBeginEdit { .. }
            | StorageRequest::PreferencesFinishEdit { .. }
            | StorageRequest::PreferencesCancelEdit { .. } => Err(failure("Preference operations require a DataStore handle")),
            StorageRequest::Open {..} | StorageRequest::OpenPreferences {..} | StorageRequest::Close {..} | StorageRequest::Query {..} => unreachable!(),
        })?;
        if mutating {
            publishSyncMutation();
        }
        Ok(result)
    }
}

/// Reads an exact object identity from the transaction snapshot shared by a batch.
fn readValue(
    tx: &mut SqliteTransaction<'_>,
    collection: &str,
    key: &str,
) -> Result<Option<StorageValueEntry>, SqliteStoreError> {
    validateObjectKey(key)?;
    tx.queryOne(
        "SELECT value,version FROM _operit_values WHERE collection=?1 AND key=?2",
        vec![SqliteValue::Text(collection.into()), SqliteValue::Text(key.into())],
    )?.map(|row| Ok(StorageValueEntry {
        value: serde_json::from_str(&row.get::<_, String>(0)?).map_err(|error| failure(error.to_string()))?,
        version: row.get::<_, String>(1)?,
    })).transpose()
}

/// Enforces the same exact object-key contract for reads and writes.
fn validateObjectKey(key: &str) -> Result<(), SqliteStoreError> {
    if key.is_empty() || key.len() > 1024 || key.chars().any(char::is_control) {
        return Err(failure("Invalid object key"));
    }
    Ok(())
}

/// Bounds pagination without substituting a default for malformed requests.
fn bounded(limit: u32) -> Result<(), SqliteStoreError> {
    if !(1..=1000).contains(&limit) {
        return Err(failure("Storage page limit must be 1..1000"));
    }
    Ok(())
}

/// Enforces the declared collection contract and the DataStore's one exact keyspace.
fn validateCollection(database: &Database, collection: &str) -> Result<(), SqliteStoreError> {
    if database.kind == StorageKind::Sqlite {
        return Err(failure(
            "Object operations require an object database handle",
        ));
    }
    if collection.is_empty() || collection.len() > 256 || collection.chars().any(char::is_control) {
        return Err(failure("Invalid collection"));
    }
    Ok(())
}

/// Executes bounded SQL writes and captures only trigger-marked rows before the commit.
fn executeStatements(
    database: &Database,
    tx: &mut SqliteTransaction<'_>,
    statements: Vec<StorageStatement>,
) -> Result<Vec<StorageSqlExecutionResult>, SqliteStoreError> {
    if database.kind != StorageKind::Sqlite {
        return Err(failure("SQL writes require a SQLite handle"));
    }
    if statements.is_empty() || statements.len() > 1000 {
        return Err(failure("SQL transaction requires 1..1000 statements"));
    }
    let mut results = Vec::new();
    for statement in statements {
        validateStatement(&statement, false, database.sync)?;
        let affected = tx.execute(
            &statement.sql,
            statement
                .params
                .into_iter()
                .map(sqlValue)
                .collect::<Result<Vec<_>, _>>()?,
        )?;
        if statement.expected_affected_rows.is_some_and(|expected| affected as u64 != expected) {
            return Err(failure("SQLite statement affected-row precondition failed"));
        }
        results.push(StorageSqlExecutionResult {
            affected_rows: affected as u64,
            last_insert_id: tx.lastInsertRowId()?.to_string(),
        });
    }
    let dirty = tx
        .queryOne("SELECT COUNT(*) FROM _operit_dirty", Vec::new())?
        .ok_or_else(|| failure("Missing dirty row count"))?
        .get::<_, i64>(0)?;
    if dirty > 1000 {
        return Err(failure("SQL transaction changes more than 1000 rows"));
    }
    let changes = database.dirtyChanges(tx)?;
    if !changes.is_empty() {
        database.record(tx, None, changes)?;
    }
    Ok(results)
}

/// Redirects writes that still use a pre-merge identity onto the surviving canonical rows.
///
/// A live plugin session keeps its loaded identities across a background merge
/// that aliased them away; without this redirect its next commit would recreate
/// orphaned rows under the retired key.
fn redirectAliasedMutations(
    database: &Database,
    tx: &mut SqliteTransaction<'_>,
    mut mutations: Vec<StorageMutation>,
) -> Result<Vec<StorageMutation>, SqliteStoreError> {
    let Some(descriptor) = database.merge.as_ref() else { return Ok(mutations); };
    let mut aliases: Option<BTreeMap<String, String>> = None;
    for mutation in mutations.iter_mut() {
        let Some(entity) = descriptor.entities.iter().find(|entity| {
            entity.collection == mutation.collection
                || entity.child_collections.iter().any(|name| name == &mutation.collection)
        }) else { continue; };
        let isRoot = entity.collection == mutation.collection;
        let root = if isRoot {
            mutation.key.clone()
        } else {
            let Some(root) = rootFromChildKey(&mutation.key, entity.key_encoding)? else { continue; };
            root
        };
        let Some(canonical) = mergeMapping(tx, &entity.collection, &root)? else { continue };
        if canonical == root { continue; }
        if aliases.is_none() { aliases = Some(durablePreferenceAliases(tx)?); }
        if isRoot {
            mutation.key = canonical.clone();
        } else {
            mutation.key = rewriteChildKey(&mutation.key, &root, &canonical, entity.key_encoding)?;
        }
        if mutation.deleted { continue; }
        let collection = MergeCollection::open(tx, database.kind, &mutation.collection)?;
        let mut value = mutation.value.clone();
        if isRoot {
            collection.mapText(&mut value, &entity.identity_column, &mut |_| Ok(canonical.clone()))?;
        }
        let pending = BTreeMap::from([((entity.collection.clone(), root.clone()), canonical.clone())]);
        rewriteMergeReferences(tx, &collection, descriptor, &mut value, &pending, aliases.as_ref().expect("aliases loaded"))?;
        mutation.value = value;
    }
    Ok(mutations)
}

/// Reads journal identities and their current records in one snapshot; no vector JSON is examined on idle polls.
fn objectChanges(database: &Database, tx: &mut SqliteTransaction<'_>, after: Option<String>, limit: u32) -> Result<StorageChangePage, SqliteStoreError> {
    if database.kind != StorageKind::Objects { return Err(failure("Changes require an object database")); }
    if limit == 0 || limit > 1000 { return Err(failure("Change page limit must be between 1 and 1000")); }
    let bounds = tx.queryOne("SELECT COALESCE((SELECT revision FROM _operit_changes ORDER BY revision LIMIT 1),0),COALESCE((SELECT revision FROM _operit_changes ORDER BY revision DESC LIMIT 1),0)", Vec::new())?.ok_or_else(|| failure("Missing change journal bounds"))?;
    let oldest = bounds.get::<_, i64>(0)?;
    let head = bounds.get::<_, i64>(1)?;
    let cursor = |revision| format!("{}#{revision}", database.origin);
    let Some(after) = after else { return Ok(StorageChangePage { cursor: cursor(head), reset: false, has_more: false, changes: Vec::new() }); };
    let (origin, position) = after.rsplit_once('#').ok_or_else(|| failure("Invalid object change cursor"))?;
    let position = position.parse::<i64>().map_err(|_| failure("Invalid object change cursor"))?;
    if position < 0 { return Err(failure("Invalid object change cursor")); }
    if origin != database.origin || position > head || (oldest > 0 && position < oldest - 1) {
        return Ok(StorageChangePage { cursor: cursor(head), reset: true, has_more: false, changes: Vec::new() });
    }
    let rows = tx.queryRows("SELECT revision,collection,key FROM _operit_changes WHERE revision>?1 ORDER BY revision LIMIT ?2", vec![SqliteValue::Integer(position), SqliteValue::Integer(limit.into())])?;
    let mut last = position;
    let mut identities = BTreeMap::new();
    for row in rows {
        last = row.get(0)?;
        identities.insert((row.get::<_, String>(1)?, row.get::<_, String>(2)?), ());
    }
    let changes = identities.into_keys().map(|(collection,key)| {
        let entry = readValue(tx, &collection, &key)?;
        Ok(StorageObjectChange { collection, key, entry })
    }).collect::<Result<Vec<_>, SqliteStoreError>>()?;
    Ok(StorageChangePage { cursor: cursor(last), reset: false, has_more: last < head, changes })
}

/// Checks all versions and stages all key mutations inside one transaction.
fn commitValues(
    database: &Database,
    tx: &mut SqliteTransaction<'_>,
    mutations: Vec<StorageMutation>,
) -> Result<StorageCommitResult, SqliteStoreError> {
    if mutations.is_empty() || mutations.len() > 1000 {
        return Err(failure("Object commit requires 1..1000 mutations"));
    }
    let mutations = redirectAliasedMutations(database, tx, mutations)?;
    let mut seen = BTreeSet::new();
    let mut changes = Vec::new();
    for mutation in mutations {
        validateCollection(database, &mutation.collection)?;
        if (mutation.deleted && !mutation.value.is_null())
            || (!mutation.check_version && mutation.expected_version.is_some())
        {
            return Err(failure(
                "Storage mutation has inconsistent deletion or version metadata",
            ));
        }
        validateObjectKey(&mutation.key)?;
        if !seen.insert((mutation.collection.clone(), mutation.key.clone())) {
            return Err(failure("Duplicated mutation key"));
        }
        let args = vec![
            SqliteValue::Text(mutation.collection.clone()),
            SqliteValue::Text(mutation.key.clone()),
        ];
        let current = tx
            .queryOne(
                "SELECT version FROM _operit_values WHERE collection=?1 AND key=?2",
                args.clone(),
            )?
            .map(|r| r.get::<_, String>(0))
            .transpose()?;
        if mutation.check_version && current != mutation.expected_version {
            return Err(failure(format!(
                "Storage version conflict: {}/{}",
                mutation.collection, mutation.key
            )));
        }
        if !mutation.deleted {
            let value = &mutation.value;
            tx.execute("INSERT INTO _operit_values VALUES(?1,?2,?3,'pending') ON CONFLICT(collection,key) DO UPDATE SET value=excluded.value,version=excluded.version",vec![args[0].clone(),args[1].clone(),SqliteValue::Text(serde_json::to_string(value).map_err(|e| failure(e.to_string()))?)])?;
        } else {
            tx.execute(
                "DELETE FROM _operit_values WHERE collection=?1 AND key=?2",
                args,
            )?;
        }
        changes.push(Change {
            collection: mutation.collection,
            key: mutation.key,
            value: mutation.value,
            deleted: mutation.deleted,
        });
    }
    let version = database.record(tx, None, changes)?;
    Ok(StorageCommitResult { version })
}

/// Discovers exact registered databases, never traversing plugin data files.
fn manifests(host: Arc<dyn RuntimeSqliteHost>) -> Result<Vec<Manifest>, SqliteStoreError> {
    catalog(host)?
        .queryRows(
            "SELECT path,owner,kind,sync,merge FROM plugin_databases",
            Vec::new(),
        )?
        .into_iter()
        .map(|row| {
            let kind = match row.get::<_, String>(2)?.as_str() {
                "sqlite" => StorageKind::Sqlite,
                "objects" => StorageKind::Objects,
                _ => return Err(failure("Unknown registered storage kind")),
            };
            Ok(Manifest {
                path: row.get(0)?,
                owner: row.get(1)?,
                kind,
                sync: row.get::<_, i64>(3)? == 1,
                merge: row
                    .get::<_, Option<String>>(4)?
                    .map(|value| serde_json::from_str(&value).map_err(|error| failure(error.to_string())).and_then(decodeMergeDescriptor))
                    .transpose()
                    .map_err(|error| failure(error.to_string()))?,
            })
        })
        .collect()
}

/// Excludes registered databases and exact SQLite sidecars from whole-file replication.
pub fn ownsFile(host: Arc<dyn RuntimeStorageHost>, path: &str) -> Result<bool, SqliteStoreError> {
    let database = [".operit-storage.json", "-wal", "-shm", "-journal"]
        .iter()
        .find_map(|suffix| path.strip_suffix(suffix))
        .unwrap_or(path);
    if database.ends_with(".preferences.json") && [PLUGIN_DATA_SPACE_DIR_PATH, PLUGIN_DATA_DEVICE_DIR_PATH].iter().any(|root| database.strip_prefix(root).is_some_and(|relative| relative.starts_with('/'))) {
        resolveStoragePath(database).map_err(failure)?;
        return Ok(true);
    }
    let marker = format!("{database}.operit-storage.json");
    if !host.exists(&marker)? {
        return Ok(false);
    }
    let manifest: Manifest =
        decodeManifest(serde_json::from_slice(&host.readBytes(&marker)?).map_err(|error| failure(error.to_string()))?)?;
    if manifest.path != database {
        return Err(failure("Storage marker identity mismatch"));
    }
    Ok(true)
}

/// Removes discovery metadata before an explicitly authorized plugin uninstall deletes its files.
pub fn unregisterDatabase(
    host: Arc<dyn RuntimeSqliteHost>,
    path: &str,
) -> Result<(), SqliteStoreError> {
    catalog(host)?.execute(
        "DELETE FROM plugin_databases WHERE path=?1",
        vec![SqliteValue::Text(path.to_string())],
    )?;
    Ok(())
}

/// Persists an exact marker before user writes so file replication cannot capture a live database.
fn registerFile(
    host: Arc<dyn RuntimeStorageHost>,
    manifest: &Manifest,
) -> Result<(), SqliteStoreError> {
    let marker = format!("{}.operit-storage.json", manifest.path);
    let mut current = manifest.clone();
    if host.exists(&marker)? {
        let stored = decodeManifest(serde_json::from_slice(&host.readBytes(&marker)?).map_err(|error| failure(error.to_string()))?)?;
        if stored.path != manifest.path || stored.owner != manifest.owner
            || stored.kind != manifest.kind || stored.sync != manifest.sync
        {
            return Err(failure("Storage marker conflicts with database identity"));
        }
        current.merge = mergeDescriptorForOpen(stored.merge.clone(), manifest.merge.clone())?;
        if current.merge == stored.merge {
            return Ok(());
        }
    }
    let content = serde_json::to_vec(&current).map_err(|error| failure(error.to_string()))?;
    host.writeBytes(&marker, &content)?;
    Ok(())
}

/// Reads committed plugin clocks independently of merge descriptor validation.
pub fn localClock(host: Arc<dyn RuntimeSqliteHost>) -> Result<SyncClock, SqliteStoreError> {
    let mut clock = SyncClock::empty();
    let databases = catalog(host.clone())?.queryRows(
        "SELECT path FROM plugin_databases WHERE sync=1",
        Vec::new(),
    )?;
    for database in databases {
        let path: String = database.get(0)?;
        let store = SqliteStore::openWithHost(host.clone(), &path)?;
        for row in store.queryRows(
            "SELECT origin,MAX(sequence) FROM _operit_log GROUP BY origin",
            Vec::new(),
        )? {
            let origin: String = row.get(0)?;
            let sequence: i64 = row.get(1)?;
            clock.setSequence(origin.clone(), clock.sequenceFor(&origin).max(sequence));
        }
    }
    Ok(clock)
}

/// Reads exact committed database sequences without interpreting any merge contract.
pub fn receipt(
    host: Arc<dyn RuntimeSqliteHost>,
) -> Result<(SyncReceipt, Vec<String>), SqliteStoreError> {
    let mut receipt = SyncReceipt::default();
    let mut errors = Vec::new();
    for row in catalog(host.clone())?
        .queryRows("SELECT path FROM plugin_databases WHERE sync=1", Vec::new())?
    {
        let path: String = row.get(0)?;
        let result = (|| -> Result<(), SqliteStoreError> {
            let store = SqliteStore::openWithHost(host.clone(), &path)?;
            for row in store.queryRows(
                "SELECT origin,sequence FROM _operit_log ORDER BY origin,sequence",
                Vec::new(),
            )? {
                receipt.observe(&row.get::<_, String>(0)?, row.get(1)?);
            }
            Ok(())
        })();
        if let Err(error) = result {
            errors.push(format!("receipt path={path}: {error}"));
        }
    }
    Ok((receipt, errors))
}

/// Exports bounded record transactions directly from database outboxes without copying database bytes.
pub fn operationsSince<K: SyncOperationKnowledge>(
    host: Arc<dyn RuntimeSqliteHost>,
    clock: &K,
    domains: &[String],
    limit: usize,
) -> Result<Vec<SyncOperation>, SqliteStoreError> {
    operationsSinceWithBudget(host, clock, domains, limit, usize::MAX)
}

/// Exports one byte-bounded page of record transactions.
///
/// The page is planned from outbox metadata first and only the selected payloads
/// are parsed, so one large transaction cannot make a store materialize
/// `limit × origins` operations before the page is truncated. The first
/// operation is always kept even when it alone exceeds `maxBytes`: the send path
/// reports an operation that cannot fit any message instead of stalling here.
pub fn operationsSinceWithBudget<K: SyncOperationKnowledge>(
    host: Arc<dyn RuntimeSqliteHost>,
    clock: &K,
    domains: &[String],
    limit: usize,
    maxBytes: usize,
) -> Result<Vec<SyncOperation>, SqliteStoreError> {
    let (operations, errors) = exportWithBudget(host, clock, domains, limit, maxBytes)?;
    if !errors.is_empty() {
        return Err(failure(errors.join("; ")));
    }
    Ok(operations)
}

/// Discovers and exports each database independently while preserving unavailable stores as explicit errors.
pub fn exportWithBudget<K: SyncOperationKnowledge>(
    host: Arc<dyn RuntimeSqliteHost>,
    clock: &K,
    domains: &[String],
    limit: usize,
    maxBytes: usize,
) -> Result<(Vec<SyncOperation>, Vec<String>), SqliteStoreError> {
    if !domains.is_empty()
        && !domains
            .iter()
            .any(|domain| domain == PLUGIN_STORAGE_SYNC_DOMAIN)
    {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut stores = Vec::new();
    let mut planned = Vec::new();
    let mut errors = Vec::new();
    for row in catalog(host.clone())?.queryRows(
        "SELECT path FROM plugin_databases WHERE sync=1 ORDER BY path",
        Vec::new(),
    )? {
        let path: String = row.get(0)?;
        let result = (|| -> Result<_, SqliteStoreError> {
            let store = SqliteStore::openWithHost(host.clone(), &path)?;
            let mut entries = Vec::new();
            for row in store.queryRows(
                "SELECT DISTINCT origin FROM _operit_log WHERE outgoing=1",
                Vec::new(),
            )? {
                let origin: String = row.get(0)?;
                let mut cursor = clock.sequenceFor(&origin);
                let mut missing = 0usize;
                while missing < limit {
                    let Some(next) = clock.nextMissingSequence(&origin, cursor) else {
                        break;
                    };
                    let rows = store.queryRows(
                        "SELECT sequence,created_at,length(payload) FROM _operit_log WHERE origin=?1 AND sequence>=?2 AND outgoing=1 ORDER BY sequence LIMIT ?3",
                        vec![SqliteValue::Text(origin.clone()), SqliteValue::Integer(next), SqliteValue::Integer(limit.min(256) as i64)],
                    )?;
                    if rows.is_empty() {
                        break;
                    }
                    for row in rows {
                        cursor = row.get(0)?;
                        if clock.containsSequence(&origin, cursor) {
                            continue;
                        }
                        let bytes = usize::try_from(row.get::<_, i64>(2)?)
                            .map_err(|_| failure("Invalid transaction payload length"))?;
                        entries.push((
                            row.get::<_, i64>(1)?,
                            origin.clone(),
                            cursor,
                            stores.len(),
                            bytes,
                        ));
                        missing += 1;
                        if missing == limit {
                            break;
                        }
                    }
                }
            }
            Ok((store, entries))
        })();
        match result {
            Ok((store, entries)) => {
                stores.push((path, store));
                planned.extend(entries);
            }
            Err(error) => errors.push(format!("export path={path}: {error}")),
        }
    }
    planned.sort();
    let mut operations = Vec::new();
    let mut bytes = 0usize;
    for (createdAt, origin, sequence, storeIndex, size) in planned {
        if operations.len() >= limit {
            break;
        }
        if !operations.is_empty() && bytes.saturating_add(size) > maxBytes {
            break;
        }
        let (path, store) = &stores[storeIndex];
        let result = (|| -> Result<SyncOperation, SqliteStoreError> {
            let payload: String = store
                .queryRows(
                    "SELECT payload FROM _operit_log WHERE origin=?1 AND sequence=?2",
                    vec![
                        SqliteValue::Text(origin.clone()),
                        SqliteValue::Integer(sequence),
                    ],
                )?
                .into_iter()
                .next()
                .ok_or_else(|| failure("Missing synchronization payload"))?
                .get(0)?;
            Ok(SyncOperation {
                opId: format!("{origin}:{sequence}"),
                originDeviceId: origin.clone(),
                sequence,
                domain: PLUGIN_STORAGE_SYNC_DOMAIN.into(),
                entityType: "transaction".into(),
                entityId: format!("{origin}:{sequence}"),
                operation: "commit".into(),
                semantics: SyncOperationSemantics::Transaction,
                payload: serde_json::from_str(&payload)
                    .map_err(|error| failure(error.to_string()))?,
                createdAt,
                schemaVersion: 1,
            })
        })();
        match result {
            Ok(operation) => {
                bytes = bytes.saturating_add(size);
                operations.push(operation);
            }
            Err(error) => errors.push(format!(
                "export path={path} origin={origin} sequence={sequence}: {error}"
            )),
        }
    }
    Ok((operations, errors))
}

/// Marks pre-join database outboxes unexportable while retaining actual user data.
pub fn prepareSpaceJoin(host: Arc<dyn RuntimeSqliteHost>) -> Result<(), SqliteStoreError> {
    for manifest in manifests(host.clone())?.into_iter().filter(|m| m.sync) {
        SqliteStore::openWithHost(host.clone(), &manifest.path)?
            .execute("UPDATE _operit_log SET outgoing=0 WHERE origin IN (SELECT origin FROM _operit_origins)", Vec::new())?;
    }
    Ok(())
}

/// Applies the same column and primary-key checks to local writes and remote replay.
fn validateRow(
    table: &StorageTable,
    key: &str,
    params: &[SqliteValue],
) -> Result<(), SqliteStoreError> {
    if key.is_empty() || key.len() > 2048 || key.chars().any(char::is_control) {
        return Err(failure("Invalid storage primary key"));
    }
    if params.len() != table.columns.len() {
        return Err(failure("Row has wrong column count"));
    }
    for (column, value) in table.columns.iter().zip(params) {
        let valid = match (&column.affinity, value) {
            (_, SqliteValue::Null) => column.nullable,
            (StorageColumnType::Text, SqliteValue::Text(_))
            | (StorageColumnType::Integer, SqliteValue::Integer(_))
            | (StorageColumnType::Real, SqliteValue::Real(_))
            | (StorageColumnType::Real, SqliteValue::Integer(_))
            | (StorageColumnType::Blob, SqliteValue::Blob(_)) => true,
            _ => false,
        };
        if !valid {
            return Err(failure("Row violates declared column affinity"));
        }
        if column.name == table.primary_key && value != &SqliteValue::Text(key.to_string()) {
            return Err(failure("Row primary key mismatch"));
        }
    }
    Ok(())
}

/// Validates one remote row before producing parameterized local SQL from its declared schema.
fn applyRow(tx: &mut SqliteTransaction<'_>, change: &Change) -> Result<(), SqliteStoreError> {
    let row = tx
        .queryOne(
            "SELECT definition FROM _operit_tables WHERE name=?1",
            vec![SqliteValue::Text(change.collection.clone())],
        )?
        .ok_or_else(|| failure("Synchronized row references an undeclared table"))?;
    let table: StorageTable =
        serde_json::from_str(&row.get::<_, String>(0)?).map_err(|e| failure(e.to_string()))?;
    let name = identifier(&table.name)?;
    let key = identifier(&table.primary_key)?;
    if change.deleted {
        tx.execute(
            &format!("DELETE FROM {name} WHERE {key}=?1"),
            vec![SqliteValue::Text(change.key.clone())],
        )?;
    } else {
        let value = &change.value;
        let values: Vec<StorageSqlValue> =
            serde_json::from_value(value.clone()).map_err(|e| failure(e.to_string()))?;
        if values.len() != table.columns.len() {
            return Err(failure("Remote row has wrong column count"));
        }
        let params = values
            .into_iter()
            .map(sqlValue)
            .collect::<Result<Vec<_>, _>>()?;
        validateRow(&table, &change.key, &params)?;
        let columns = table
            .columns
            .iter()
            .map(|c| identifier(&c.name))
            .collect::<Result<Vec<_>, _>>()?;
        let placeholders = (1..=params.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let update = columns
            .iter()
            .map(|c| format!("{c}=excluded.{c}"))
            .collect::<Vec<_>>()
            .join(",");
        tx.execute(&format!("INSERT INTO {name}({}) VALUES({placeholders}) ON CONFLICT({key}) DO UPDATE SET {update}",columns.join(",")),params)?;
    }
    Ok(())
}

/// Projects validated historical rows by column identity into the installed additive schema.
fn projectHistoricalRows(
    tx: &mut SqliteTransaction<'_>,
    payload: &mut Payload,
) -> Result<(), SqliteStoreError> {
    for incoming in &mut payload.tables {
        let definition = tx.queryOne(
            "SELECT definition FROM _operit_tables WHERE name=?1",
            vec![SqliteValue::Text(incoming.name.clone())],
        )?.ok_or_else(|| failure("Historical projection requires an installed table"))?;
        let current: StorageTable = serde_json::from_str(&definition.get::<_, String>(0)?)
            .map_err(|error| failure(error.to_string()))?;
        if *incoming == current {
            continue;
        }
        let columns = current.columns.iter()
            .map(|column| identifier(&column.name))
            .collect::<Result<Vec<_>, _>>()?.join(",");
        let select = format!("SELECT {columns} FROM {} WHERE {}=?1",
            identifier(&current.name)?, identifier(&current.primary_key)?);
        for change in payload.changes.iter_mut().filter(|change| {
            change.collection == incoming.name && !change.deleted
        }) {
            let values: Vec<StorageSqlValue> = serde_json::from_value(change.value.clone())
                .map_err(|error| failure(error.to_string()))?;
            let mut projected = vec![StorageSqlValue::Null; current.columns.len()];
            let retainedKey = mergeMapping(tx, &incoming.name, &change.key)?;
            let key = retainedKey.as_deref().unwrap_or(&change.key);
            if let Some(row) = tx.queryOne(&select, vec![SqliteValue::Text(key.into())])? {
                projected = row.values.into_iter().map(wireValue).collect();
            }
            for (column, value) in incoming.columns.iter().zip(values) {
                let index = current.columns.iter().position(|current| current == column)
                    .ok_or_else(|| failure("Historical column differs from the installed schema"))?;
                projected[index] = value;
            }
            change.value = serde_json::to_value(projected)
                .map_err(|error| failure(error.to_string()))?;
        }
        *incoming = current;
    }
    Ok(())
}

/// Validates a self-contained remote transaction before creating any durable database state.
fn validatePayload(payload: &Payload) -> Result<(), SqliteStoreError> {
    if payload.changes.len() > 1000
        || payload.tables.len() > 1000
        || (payload.changes.is_empty() && payload.tables.is_empty())
    {
        return Err(failure("Remote transaction has invalid mutation bounds"));
    }
    let mut tables = BTreeMap::new();
    for table in &payload.tables {
        if payload.manifest.kind != StorageKind::Sqlite
            || tables.insert(&table.name, table).is_some()
        {
            return Err(failure("Invalid or duplicate remote table declaration"));
        }
        tableSql(table)?;
    }
    let mut seen = BTreeSet::new();
    for change in &payload.changes {
        if change.key.is_empty()
            || change.key.len() > 2048
            || change.key.chars().any(char::is_control)
            || !seen.insert((&change.collection, &change.key))
        {
            return Err(failure("Invalid or duplicate remote key"));
        }
        if change.deleted && !change.value.is_null() {
            return Err(failure("Deleted storage values must be null"));
        }
        if payload.manifest.kind == StorageKind::Sqlite {
            let table = tables
                .get(&change.collection)
                .ok_or_else(|| failure("Remote row omits its table declaration"))?;
            if !change.deleted {
                let values: Vec<StorageSqlValue> = serde_json::from_value(change.value.clone())
                    .map_err(|e| failure(e.to_string()))?;
                let values = values
                    .into_iter()
                    .map(sqlValue)
                    .collect::<Result<Vec<_>, _>>()?;
                validateRow(table, &change.key, &values)?;
            }
        } else if change.collection.is_empty()
            || change.collection.len() > 256
            || change.collection.chars().any(char::is_control)
        {
            return Err(failure("Invalid remote storage collection"));
        }
    }
    Ok(())
}

/// Validates a plugin-provided merge contract before Core interprets any business record.
fn validateMergeDescriptor(
    tx: &mut SqliteTransaction<'_>,
    descriptor: &StorageMergeDescriptor,
    payload: &Payload,
) -> Result<(), SqliteStoreError> {
    if !(1..=4).contains(&descriptor.version)
        || descriptor.max_entities == 0
        || descriptor.max_entities > 10_000
        || descriptor.max_references == 0
        || descriptor.max_references > 100_000
        || descriptor.entities.len() > descriptor.max_entities as usize
    {
        return Err(failure("Invalid storage merge contract bounds"));
    }
    let mut entityNames = BTreeSet::new();
    let mut referenceCount = 0usize;
    for entity in &descriptor.entities {
        if !entityNames.insert(entity.collection.as_str()) || identifier(&entity.collection).is_err() {
            return Err(failure("Invalid storage merge entity declaration"));
        }
        let collection = MergeCollection::open(tx, payload.manifest.kind, &entity.collection)?;
        collection.validateField(&entity.identity_column)?;
        if let Some(name) = &entity.name_column { collection.validateField(name)?; }
        if payload.manifest.kind == StorageKind::Objects &&
            std::iter::once(&entity.identity_column).chain(entity.ignored_columns.iter()).any(|field| field.contains('/') || field == "*") {
            return Err(failure("JSON entity identities and content exclusions must declare direct fields"));
        }
        let mut ignored = BTreeSet::new();
        for field in &entity.ignored_columns {
            if !ignored.insert(field.as_str()) { return Err(failure("Duplicate storage merge ignored field")); }
            collection.validateField(field)?;
        }
        for reference in &entity.key_references {
            if reference.prefix.chars().any(char::is_control) { return Err(failure("Invalid storage merge key reference prefix")); }
            match &reference.target {
                StorageMergeTarget::Collection { collection } if !descriptor.entities.iter().any(|item| item.collection == *collection) => return Err(failure("Undeclared storage merge key target")),
                StorageMergeTarget::Preference { path } if !path.starts_with("runtime/") || !path.ends_with(".preferences.json") || path.len() > 512 || path.chars().any(char::is_control) || path.split('/').any(|part| part.is_empty() || part == "." || part == "..") => return Err(failure("Invalid storage merge preference key target")),
                _ => {}
            }
        }
        let mut children = BTreeSet::new();
        for child in &entity.child_collections {
            if !children.insert(child.as_str()) || child == &entity.collection || identifier(child).is_err() {
                return Err(failure("Invalid storage merge child collection declaration"));
            }
            MergeCollection::open(tx, payload.manifest.kind, child)?;
        }
        for reference in &entity.references {
            referenceCount += 1;
            if referenceCount > descriptor.max_references as usize || identifier(&reference.collection).is_err() {
                return Err(failure("Invalid storage merge reference declaration"));
            }
            MergeCollection::open(tx, payload.manifest.kind, &reference.collection)?.validateField(&reference.column)?;
            if reference.key_prefix.chars().any(char::is_control) || reference.key_suffix.chars().any(char::is_control) {
                return Err(failure("Invalid storage merge reference wrapper"));
            }
            match &reference.target {
                StorageMergeTarget::Collection { collection } => {
                    if !descriptor.entities.iter().any(|item| item.collection == *collection) {
                        return Err(failure("Storage merge target collection is not declared"));
                    }
                }
                StorageMergeTarget::Preference { path } => {
                    if path.is_empty() || path.len() > 512 || path.chars().any(char::is_control) {
                        return Err(failure("Invalid storage merge preference target"));
                    }
                }
            }
        }
    }
    let mut resolved = BTreeSet::new();
    for _ in 0..descriptor.entities.len() {
        for entity in &descriptor.entities {
            if entity.key_references.iter().all(|reference| match &reference.target { StorageMergeTarget::Collection { collection } => resolved.contains(collection), StorageMergeTarget::Preference { .. } => true }) {
                resolved.insert(entity.collection.clone());
            }
        }
    }
    if resolved.len() != descriptor.entities.len() {
        return Err(failure("Storage merge identity ownership graph contains a cycle"));
    }
    Ok(())
}

/// Adapts physical records to the existing merge algorithm without defining another merge policy.
struct MergeCollection {
    name: String,
    table: Option<StorageTable>,
}

impl MergeCollection {
    /// Opens the explicitly selected record representation for one collection.
    fn open(tx: &mut SqliteTransaction<'_>, kind: StorageKind, name: &str) -> Result<Self, SqliteStoreError> {
        let table = match kind {
            StorageKind::Sqlite => {
                let row = tx.queryOne("SELECT definition FROM _operit_tables WHERE name=?1", vec![SqliteValue::Text(name.into())])?
                    .ok_or_else(|| failure("Storage merge table is not declared"))?;
                Some(serde_json::from_str(&row.get::<_, String>(0)?).map_err(|error| failure(error.to_string()))?)
            }
            StorageKind::Objects => None,
        };
        Ok(Self { name: name.into(), table })
    }

    /// Validates a SQL column or a bounded JSON field selector in the selected representation.
    fn validateField(&self, path: &str) -> Result<(), SqliteStoreError> {
        if let Some(table) = &self.table { columnIndex(table, path)?; }
        else if path.is_empty() || path.len() > 512 || path.split('/').count() > 32
            || path.split('/').any(|field| field.is_empty() || field.chars().any(char::is_control)) {
            return Err(failure("Invalid storage merge JSON field selector"));
        }
        Ok(())
    }

    /// Reads complete materialized records in deterministic key order.
    fn read(&self, tx: &mut SqliteTransaction<'_>) -> Result<Vec<(String, Value)>, SqliteStoreError> {
        match &self.table {
            Some(table) => readTableRows(tx, table)?.into_iter().map(|(key, values)|
                Ok((key, serde_json::to_value(values).map_err(|error| failure(error.to_string()))?))).collect(),
            None => tx.queryRows("SELECT key,value FROM _operit_values WHERE collection=?1 ORDER BY key", vec![SqliteValue::Text(self.name.clone())])?
                .into_iter().map(|row| Ok((row.get(0)?, serde_json::from_str(&row.get::<_, String>(1)?).map_err(|error| failure(error.to_string()))?))).collect(),
        }
    }

    /// Reads a required scalar identity or display name from a declared field.
    fn text(&self, value: &Value, field: &str) -> Result<String, SqliteStoreError> {
        match &self.table {
            Some(table) => textColumn(table, &serde_json::from_value::<Vec<StorageSqlValue>>(value.clone()).map_err(|error| failure(error.to_string()))?, field),
            None => {
                let mut current = value;
                for part in field.split('/') {
                    current = current.get(part).ok_or_else(|| failure("Storage merge required JSON field is missing"))?;
                }
                current.as_str().map(str::to_owned).ok_or_else(|| failure("Storage merge identity or name is not text"))
            }
        }
    }

    /// Rewrites only declared text references while leaving absent optional fields untouched.
    fn mapText(&self, value: &mut Value, field: &str,
        map: &mut impl FnMut(&str) -> Result<String, SqliteStoreError>) -> Result<(), SqliteStoreError> {
        match &self.table {
            Some(table) => {
                let mut values: Vec<StorageSqlValue> = serde_json::from_value(value.clone()).map_err(|error| failure(error.to_string()))?;
                let index = columnIndex(table, field)?;
                if let StorageSqlValue::Text(text) = &values[index] { values[index] = StorageSqlValue::Text(map(text)?); }
                *value = serde_json::to_value(values).map_err(|error| failure(error.to_string()))?;
            }
            None => visitMergeJson(value, &field.split('/').collect::<Vec<_>>(), &mut |value| {
                if value.is_null() { return Ok(()); }
                let text = value.as_str().ok_or_else(|| failure("Storage merge JSON reference is not text"))?;
                *value = Value::String(map(text)?);
                Ok(())
            })?,
        }
        Ok(())
    }

    /// Uses the established content exclusions for either complete JSON or relational records.
    fn content(&self, entity: &StorageMergeEntity, value: &Value) -> Result<String, SqliteStoreError> {
        match &self.table {
            Some(table) => entityContent(table, entity, &serde_json::from_value::<Vec<StorageSqlValue>>(value.clone()).map_err(|error| failure(error.to_string()))?),
            None => {
                canonicalContent(value, &entity.identity_column, entity.ignored_columns.iter().map(String::as_str))
                    .map_err(failure)
            }
        }
    }

    /// Rewrites physical SQL ownership fields and the declared root identity without altering user content.
    fn identity(&self, value: &mut Value, entity: &StorageMergeEntity, key: &str, old: &str, new: &str) -> Result<(), SqliteStoreError> {
        if let Some(table) = &self.table {
            self.mapText(value, &table.primary_key, &mut |_| Ok(key.into()))?;
            for field in ["_root", "_parent"] {
                if table.columns.iter().any(|column| column.name == field) {
                    self.mapText(value, field, &mut |text| if text == old { Ok(new.into()) }
                        else if field == "_parent" { rewriteChildKey(text, old, new, entity.key_encoding) } else { Ok(text.into()) })?;
                }
            }
        }
        if self.name == entity.collection {
            self.text(value, &entity.identity_column)?;
            self.mapText(value, &entity.identity_column, &mut |_| Ok(new.into()))?;
        }
        Ok(())
    }

    /// Writes a derived materialization without adding a synchronization operation.
    fn write(&self, tx: &mut SqliteTransaction<'_>, key: &str, value: Option<&Value>) -> Result<(), SqliteStoreError> {
        if self.table.is_some() {
            return applyRow(tx, &Change { collection: self.name.clone(), key: key.into(), value: match value { Some(value) => value.clone(), None => Value::Null }, deleted: value.is_none() });
        }
        match value {
            Some(value) => {
                let encoded = serde_json::to_string(value).map_err(|error| failure(error.to_string()))?;
                let version = Uuid::new_v4().to_string();
                tx.execute("INSERT INTO _operit_values(collection,key,value,version) VALUES(?1,?2,?3,?4) ON CONFLICT(collection,key) DO UPDATE SET value=excluded.value,version=excluded.version",
                    vec![SqliteValue::Text(self.name.clone()), SqliteValue::Text(key.into()), SqliteValue::Text(encoded), SqliteValue::Text(version)])?;
            }
            None => { tx.execute("DELETE FROM _operit_values WHERE collection=?1 AND key=?2", vec![SqliteValue::Text(self.name.clone()), SqliteValue::Text(key.into())])?; }
        }
        Ok(())
    }
}

/// Traverses explicitly declared JSON fields and array or object members without inferring references.
fn visitMergeJson(value: &mut Value, path: &[&str], visit: &mut impl FnMut(&mut Value) -> Result<(), SqliteStoreError>) -> Result<(), SqliteStoreError> {
    let Some((field, tail)) = path.split_first() else { return visit(value); };
    if value.is_null() { return Ok(()); }
    if *field == "*" {
        match value {
            Value::Array(values) => for value in values { visitMergeJson(value, tail, visit)?; },
            Value::Object(values) => for value in values.values_mut() { visitMergeJson(value, tail, visit)?; },
            _ => return Err(failure("Storage merge wildcard requires a JSON array or object")),
        }
    } else {
        let object = value.as_object_mut().ok_or_else(|| failure("Storage merge field selector requires a JSON object"))?;
        if let Some(value) = object.get_mut(*field) { visitMergeJson(value, tail, visit)?; }
    }
    Ok(())
}

/// Reads one declared SQL row using only the already validated table identifiers.
fn readTableRows(
    tx: &mut SqliteTransaction<'_>,
    table: &StorageTable,
) -> Result<Vec<(String, Vec<StorageSqlValue>)>, SqliteStoreError> {
    let columns = table
        .columns
        .iter()
        .map(|column| identifier(&column.name))
        .collect::<Result<Vec<_>, _>>()?;
    tx.queryRows(
        &format!(
            "SELECT {} FROM {} ORDER BY {}",
            columns.join(","),
            identifier(&table.name)?,
            identifier(&table.primary_key)?,
        ),
        Vec::new(),
    )?
    .into_iter()
    .map(|row| {
        let key = row
            .values
            .get(columnIndex(table, &table.primary_key)?)
            .and_then(|value| match value {
                SqliteValue::Text(value) => Some(value.clone()),
                _ => None,
            })
            .ok_or_else(|| failure("Storage merge row has no text primary key"))?;
        Ok((key, row.values.into_iter().map(wireValue).collect()))
    })
    .collect()
}

/// Returns one column position from a declared table without accepting an undeclared field.
fn columnIndex(table: &StorageTable, column: &str) -> Result<usize, SqliteStoreError> {
    table
        .columns
        .iter()
        .position(|item| item.name == column)
        .ok_or_else(|| failure("Storage merge column is not declared"))
}

/// Extracts one text field from a portable SQL row.
fn textColumn(
    table: &StorageTable,
    values: &[StorageSqlValue],
    column: &str,
) -> Result<String, SqliteStoreError> {
    let index = columnIndex(table, column)?;
    match values.get(index) {
        Some(StorageSqlValue::Text(value)) => Ok(value.clone()),
        _ => Err(failure("Storage merge identity or name is not text")),
    }
}

/// Builds stable content identity while excluding physical and explicitly ignored columns.
fn entityContent(
    table: &StorageTable,
    entity: &StorageMergeEntity,
    values: &[StorageSqlValue],
) -> Result<String, SqliteStoreError> {
    let ignored = entity
        .ignored_columns
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let content = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, column)| {
            column.name != entity.identity_column
                && column.name != "_key"
                && column.name != "_revision"
                && !ignored.contains(column.name.as_str())
        })
        .map(|(index, _)| values.get(index).cloned().ok_or_else(|| failure("Storage merge row is incomplete")))
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_string(&content).map_err(|error| failure(error.to_string()))
}

/// Compares the actual ordered relational values rather than only their root-level element counts.
fn sameEntityChildren(tx: &mut SqliteTransaction<'_>, kind: StorageKind, descriptor: &StorageMergeDescriptor, entity: &StorageMergeEntity,
    localKey: &str, remoteKey: &str, incoming: &[Change], pending: &BTreeMap<(String, String), String>, preferenceReferences: &BTreeMap<String, String>) -> Result<bool, SqliteStoreError> {
    for name in &entity.child_collections {
        let collection = MergeCollection::open(tx, kind, name)?;
        let mut content = |tx: &mut SqliteTransaction<'_>, value: &Value| -> Result<String, SqliteStoreError> {
            let mut value = value.clone();
            rewriteMergeReferences(tx, &collection, descriptor, &mut value, pending, preferenceReferences)?;
            match &collection.table {
                Some(table) => childContent(table, &serde_json::from_value::<Vec<StorageSqlValue>>(value.clone()).map_err(|error| failure(error.to_string()))?),
                None => {
                    let mut value = value.as_object().ok_or_else(|| failure("Storage merge child must be a JSON object"))?.clone();
                    value.remove(&entity.identity_column);
                    for reference in &entity.references {
                        if reference.collection == *name && reference.column == entity.identity_column { value.remove(&reference.column); }
                    }
                    serde_json::to_string(&value).map_err(|error| failure(error.to_string()))
                }
            }
        };
        let mut local = Vec::new();
        for (key, value) in collection.read(tx)? {
            if rootFromChildKey(&key, entity.key_encoding)?.as_deref() == Some(localKey) {
                local.push((rewriteChildKey(&key, localKey, "@root", entity.key_encoding)?, content(tx, &value)?));
            }
        }
        let mut remote = Vec::new();
        for change in incoming.iter().filter(|change| change.collection == *name && !change.deleted) {
            if rootFromChildKey(&change.key, entity.key_encoding)?.as_deref() == Some(remoteKey) {
                remote.push((rewriteChildKey(&change.key, remoteKey, "@root", entity.key_encoding)?, content(tx, &change.value)?));
            }
        }
        local.sort(); remote.sort();
        if local != remote { return Ok(false); }
    }
    Ok(true)
}

/// Excludes only physical relation keys while retaining ordered positions, map keys and all user data.
fn childContent(table: &StorageTable, values: &[StorageSqlValue]) -> Result<String, SqliteStoreError> {
    let values = table.columns.iter().zip(values).filter(|(column,_)|
        !matches!(column.name.as_str(), "_key" | "_revision" | "_root" | "_parent"))
        .map(|(_,value)| value).collect::<Vec<_>>();
    serde_json::to_string(&values).map_err(|error| failure(error.to_string()))
}

/// Extracts a root identity from a declared relational child-key encoding.
fn rootFromChildKey(
    key: &str,
    encoding: StorageMergeKeyEncoding,
) -> Result<Option<String>, SqliteStoreError> {
    match encoding {
        StorageMergeKeyEncoding::LengthPrefixedRoot => {
            let Some((length, remainder)) = key.split_once(':') else {
                return Ok(None);
            };
            let length = length
                .parse::<usize>()
                .map_err(|_| failure("Invalid storage merge child key length"))?;
            if remainder.len() < length + 1 || !remainder.is_char_boundary(length) {
                return Err(failure("Invalid storage merge child key"));
            }
            let (segment, suffix) = remainder.split_at(length);
            if !suffix.starts_with(':') || segment.is_empty() {
                return Err(failure("Invalid storage merge child key"));
            }
            if segment.split_once(':').is_some_and(|(length, _)| !length.is_empty() && length.bytes().all(|byte| byte.is_ascii_digit())) {
                return rootFromChildKey(segment, encoding);
            }
            Ok(Some(segment.to_string()))
        }
        StorageMergeKeyEncoding::SlashRoot => Ok(key.split_once('/').map(|(root, _)| {
            root.to_string()
        })),
    }
}

/// Rewrites one relational child key after a declared root identity was canonicalized.
fn rewriteChildKey(
    key: &str,
    remote: &str,
    canonical: &str,
    encoding: StorageMergeKeyEncoding,
) -> Result<String, SqliteStoreError> {
    match encoding {
        StorageMergeKeyEncoding::LengthPrefixedRoot => {
            let Some((length, remainder)) = key.split_once(':') else {
                return Ok(key.to_string());
            };
            let length = length
                .parse::<usize>()
                .map_err(|_| failure("Invalid storage merge child key length"))?;
            if remainder.len() < length + 1 || !remainder.is_char_boundary(length) {
                return Err(failure("Invalid storage merge child key"));
            }
            let (segment, suffix) = remainder.split_at(length);
            if !suffix.starts_with(':') {
                return Err(failure("Invalid storage merge child key"));
            }
            let rewritten = if segment == remote {
                canonical.to_string()
            } else if segment.split_once(':').is_some_and(|(length, _)| !length.is_empty() && length.bytes().all(|byte| byte.is_ascii_digit())) {
                rewriteChildKey(segment, remote, canonical, encoding)?
            } else {
                return Ok(key.to_string());
            };
            if rewritten == segment {
                return Ok(key.to_string());
            }
            Ok(format!("{}:{}{}", rewritten.len(), rewritten, suffix))
        }
        StorageMergeKeyEncoding::SlashRoot => {
            let Some((root, suffix)) = key.split_once('/') else {
                return Ok(key.to_string());
            };
            if root != remote {
                return Ok(key.to_string());
            }
            Ok(format!("{canonical}/{suffix}"))
        }
    }
}

/// Encodes a byte sequence as one URI component without changing identifier semantics.
fn encodeUriComponent(value: &str) -> String {
    value.bytes().fold(String::new(), |mut result, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'!' | b'\'' | b'(' | b')' | b'*') {
            result.push(byte as char);
        } else {
            result.push('%');
            result.push_str(&format!("{byte:02X}"));
        }
        result
    })
}

/// Decodes one URI component and rejects malformed percent escapes.
fn decodeUriComponent(value: &str) -> Result<String, SqliteStoreError> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        if index + 2 >= bytes.len() {
            return Err(failure("Invalid URI component reference"));
        }
        let hex = |byte: u8| match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        };
        let high = hex(bytes[index + 1]).ok_or_else(|| failure("Invalid URI component reference"))?;
        let low = hex(bytes[index + 2]).ok_or_else(|| failure("Invalid URI component reference"))?;
        decoded.push(high * 16 + low);
        index += 3;
    }
    String::from_utf8(decoded).map_err(|_| failure("URI component is not UTF-8"))
}

/// Resolves a persisted identity equivalence chain and rejects cycles.
fn mergeMapping(
    tx: &mut SqliteTransaction<'_>, collection: &str, remoteKey: &str,
) -> Result<Option<String>, SqliteStoreError> {
    let mut current = remoteKey.to_string();
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(current.clone()) { return Err(failure("Storage merge identity cycle")); }
        let rows = tx.queryRows(
            "SELECT DISTINCT canonical_key FROM _operit_merge_map WHERE collection=?1 AND remote_key=?2",
            vec![SqliteValue::Text(collection.to_string()), SqliteValue::Text(current.clone())])?;
        if rows.len() > 1 { return Err(failure("Storage merge UUID maps to conflicting canonical identities")); }
        let Some(row) = rows.into_iter().next() else { break; };
        let next: String = row.get(0)?;
        if next == current { break; }
        current = next;
    }
    Ok((current != remoteKey).then_some(current))
}

/// Persists an explicit alias while keeping every earlier alias pointed at the retained identity.
fn saveMergeMapping(
    tx: &mut SqliteTransaction<'_>, collection: &str, origin: &str,
    remoteKey: &str, canonicalKey: &str,
) -> Result<(), SqliteStoreError> {
    tx.execute("UPDATE _operit_merge_map SET canonical_key=?3 WHERE collection=?1 AND canonical_key=?2",
        vec![SqliteValue::Text(collection.to_string()), SqliteValue::Text(remoteKey.to_string()), SqliteValue::Text(canonicalKey.to_string())])?;
    tx.execute("INSERT INTO _operit_merge_map VALUES(?1,?2,?3,?4) ON CONFLICT(collection,origin,remote_key) DO UPDATE SET canonical_key=excluded.canonical_key",
        vec![SqliteValue::Text(collection.to_string()), SqliteValue::Text(origin.to_string()),
            SqliteValue::Text(remoteKey.to_string()), SqliteValue::Text(canonicalKey.to_string())])?;
    Ok(())
}

/// Reserved merge-map collection mirroring preference identity aliases into every plugin database.
const PREFERENCE_ALIAS_COLLECTION: &str = "_preference_aliases";

/// Reads the durable mirror of preference identity aliases recorded by earlier reconciliations.
fn durablePreferenceAliases(
    tx: &mut SqliteTransaction<'_>,
) -> Result<BTreeMap<String, String>, SqliteStoreError> {
    tx.queryRows(
        "SELECT remote_key,canonical_key FROM _operit_merge_map WHERE collection=?1 ORDER BY remote_key",
        vec![SqliteValue::Text(PREFERENCE_ALIAS_COLLECTION.to_string())],
    )?
    .into_iter()
    .map(|row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
    .collect()
}

/// Mirrors one batch of preference identity aliases so local writers can follow them later.
fn recordPreferenceAliases(
    tx: &mut SqliteTransaction<'_>,
    preferenceReferences: &BTreeMap<String, String>,
) -> Result<(), SqliteStoreError> {
    for (key, canonical) in preferenceReferences {
        saveMergeMapping(tx, PREFERENCE_ALIAS_COLLECTION, "preferences", key, canonical)?;
    }
    Ok(())
}

/// Reports whether any durable alias actually redirects one identity onto another.
fn hasDurableAlias(tx: &mut SqliteTransaction<'_>) -> Result<bool, SqliteStoreError> {
    let row = tx.queryOne(
        "SELECT EXISTS(SELECT 1 FROM _operit_merge_map WHERE canonical_key<>remote_key)",
        Vec::new(),
    )?
    .ok_or_else(|| failure("Merge alias probe returned no row"))?;
    Ok(row.get::<_, i64>(0)? != 0)
}

/// Rekeys an equivalent materialized entity and every explicitly declared dependent identity.
fn rekeyEntity(
    tx: &mut SqliteTransaction<'_>, kind: StorageKind, descriptor: &StorageMergeDescriptor,
    entity: &StorageMergeEntity, old: &str, new: &str, origin: &str,
) -> Result<(), SqliteStoreError> {
    if old == new { return Ok(()); }
    saveMergeMapping(tx, &entity.collection, origin, old, new)?;
    for name in std::iter::once(&entity.collection).chain(entity.child_collections.iter()) {
        let collection = MergeCollection::open(tx, kind, name)?;
        for (key, mut value) in collection.read(tx)? {
            let isRoot = name == &entity.collection;
            if isRoot && key != old { continue; }
            if !isRoot && rootFromChildKey(&key, entity.key_encoding)?.as_deref() != Some(old) { continue; }
            let rewritten = if isRoot { new.to_string() } else { rewriteChildKey(&key, old, new, entity.key_encoding)? };
            collection.identity(&mut value, entity, &rewritten, old, new)?;
            collection.write(tx, &key, None)?;
            collection.write(tx, &rewritten, Some(&value))?;
            // The canonical row may already carry its own version when a local
            // writer resurrected the retired key; the moved row replaces it so
            // the published content and its version stay one atomic truth.
            tx.execute(
                "INSERT INTO _operit_versions(collection,key,created_at,origin,sequence) SELECT collection,?3,created_at,origin,sequence FROM _operit_versions WHERE collection=?1 AND key=?2 ON CONFLICT(collection,key) DO UPDATE SET created_at=excluded.created_at,origin=excluded.origin,sequence=excluded.sequence",
                vec![SqliteValue::Text(name.clone()), SqliteValue::Text(key.clone()), SqliteValue::Text(rewritten.clone())],
            )?;
            tx.execute("DELETE FROM _operit_versions WHERE collection=?1 AND key=?2",
                vec![SqliteValue::Text(name.clone()), SqliteValue::Text(key)])?;
        }
    }
    tx.execute(
        "INSERT INTO _operit_merge_names(collection,key,name) SELECT collection,?3,name FROM _operit_merge_names WHERE collection=?1 AND key=?2 ON CONFLICT(collection,key) DO UPDATE SET name=excluded.name",
        vec![SqliteValue::Text(entity.collection.clone()), SqliteValue::Text(old.to_string()), SqliteValue::Text(new.to_string())],
    )?;
    tx.execute("DELETE FROM _operit_merge_names WHERE collection=?1 AND key=?2",
        vec![SqliteValue::Text(entity.collection.clone()), SqliteValue::Text(old.to_string())])?;
    for dependent in &descriptor.entities {
        for reference in &dependent.key_references {
            if matches!(&reference.target, StorageMergeTarget::Collection { collection } if collection == &entity.collection) {
                rekeyEntity(tx, kind, descriptor, dependent, &format!("{}{old}", reference.prefix), &format!("{}{new}", reference.prefix), origin)?;
            }
        }
    }
    Ok(())
}

/// Retains a declared original display name independently from deterministic collision rendering.
fn rememberMergeName(tx: &mut SqliteTransaction<'_>, collection: &str, key: &str, name: &str) -> Result<(), SqliteStoreError> {
    tx.execute("INSERT INTO _operit_merge_names VALUES(?1,?2,?3) ON CONFLICT(collection,key) DO UPDATE SET name=excluded.name",
        vec![SqliteValue::Text(collection.to_string()), SqliteValue::Text(key.to_string()), SqliteValue::Text(name.to_string())])?;
    Ok(())
}

/// Records only the names explicitly written by an accepted user transaction.
fn rememberPayloadNames(tx: &mut SqliteTransaction<'_>, kind: StorageKind, descriptor: &StorageMergeDescriptor,
    changes: &[Change]) -> Result<(), SqliteStoreError> {
    for entity in &descriptor.entities {
        let Some(field) = &entity.name_column else { continue; };
        for change in changes.iter().filter(|change| change.collection == entity.collection) {
            if change.deleted {
                tx.execute("DELETE FROM _operit_merge_names WHERE collection=?1 AND key=?2",
                    vec![SqliteValue::Text(entity.collection.clone()), SqliteValue::Text(change.key.clone())])?;
            } else {
                let collection = MergeCollection::open(tx, kind, &entity.collection)?;
                rememberMergeName(tx, &entity.collection, &change.key, &collection.text(&change.value, field)?)?;
            }
        }
    }
    Ok(())
}

/// Assigns unique names in UUID order while retaining every entity's original requested name.
fn reconcileMergeNames(tx: &mut SqliteTransaction<'_>, kind: StorageKind, descriptor: &StorageMergeDescriptor) -> Result<(), SqliteStoreError> {
    for entity in &descriptor.entities {
        let Some(field) = &entity.name_column else { continue; };
        let collection = MergeCollection::open(tx, kind, &entity.collection)?;
        let mut rows = Vec::new();
        for (key, value) in collection.read(tx)? {
            let stored = tx.queryOne("SELECT name FROM _operit_merge_names WHERE collection=?1 AND key=?2",
                vec![SqliteValue::Text(entity.collection.clone()), SqliteValue::Text(key.clone())])?;
            let name = match stored { Some(row) => row.get(0)?, None => collection.text(&value, field)? };
            rememberMergeName(tx, &entity.collection, &key, &name)?;
            rows.push((key, name, value));
        }
        let names = rows.iter().map(|(key, name, _)| (key.clone(), name.clone())).collect();
        let mut names = uniqueNames(&names);
        for (key, _, mut value) in rows {
            let name = names.remove(&key).ok_or_else(|| failure("Declared merge name is missing"))?;
            if collection.text(&value, field)? != name {
                collection.mapText(&mut value, field, &mut |_| Ok(name.clone()))?;
                collection.write(tx, &key, Some(&value))?;
            }
        }
    }
    Ok(())
}

/// Resolves one target identifier through pending mappings, persisted mappings or preference remaps.
fn resolveMergeReference(
    tx: &mut SqliteTransaction<'_>,
    reference: &StorageMergeReference,
    value: &str,
    pending: &BTreeMap<(String, String), String>,
    preferenceReferences: &BTreeMap<String, String>,
) -> Result<String, SqliteStoreError> {
    if !value.starts_with(&reference.key_prefix) || !value.ends_with(&reference.key_suffix) {
        return Ok(value.to_string());
    }
    let end = value.len() - reference.key_suffix.len();
    let encoded = &value[reference.key_prefix.len()..end];
    let identifier = match reference.value_encoding {
        StorageMergeReferenceEncoding::Direct => encoded.to_string(),
        StorageMergeReferenceEncoding::UriComponent => decodeUriComponent(encoded)?,
    };
    let mapped = match &reference.target {
        StorageMergeTarget::Collection { collection } => pending
            .get(&(collection.clone(), identifier.clone()))
            .cloned()
            .or(mergeMapping(tx, collection, &identifier)?)
            .unwrap_or(identifier),
        StorageMergeTarget::Preference { path } => preferenceReferences
            .get(&format!("{path}\u{001f}{identifier}"))
            .cloned()
            .unwrap_or(identifier),
    };
    let encoded = match reference.value_encoding {
        StorageMergeReferenceEncoding::Direct => mapped,
        StorageMergeReferenceEncoding::UriComponent => encodeUriComponent(&mapped),
    };
    Ok(format!("{}{encoded}{}", reference.key_prefix, reference.key_suffix))
}

/// Resolves a root key through an optional declared identity relationship.
fn resolveEntityKey(
    tx: &mut SqliteTransaction<'_>,
    entity: &StorageMergeEntity,
    remoteKey: &str,
    pending: &BTreeMap<(String, String), String>,
    preferenceReferences: &BTreeMap<String, String>,
) -> Result<String, SqliteStoreError> {
    for reference in &entity.key_references {
        if !remoteKey.starts_with(&reference.prefix) {
            continue;
        }
        let identity = &remoteKey[reference.prefix.len()..];
        let declaration = StorageMergeReference {
            collection: entity.collection.clone(), column: entity.identity_column.clone(),
            target: reference.target.clone(), key_prefix: String::new(), key_suffix: String::new(),
            value_encoding: StorageMergeReferenceEncoding::Direct,
        };
        let mapped = resolveMergeReference(tx, &declaration, identity, pending, preferenceReferences)?;
        return Ok(format!("{}{mapped}", reference.prefix));
    }
    Ok(remoteKey.to_string())
}


/// Applies the existing reference resolver to explicitly declared fields in either record representation.
fn rewriteMergeReferences(
    tx: &mut SqliteTransaction<'_>, collection: &MergeCollection, descriptor: &StorageMergeDescriptor,
    value: &mut Value, pending: &BTreeMap<(String, String), String>, preferenceReferences: &BTreeMap<String, String>,
) -> Result<(), SqliteStoreError> {
    for reference in descriptor.entities.iter().flat_map(|entity| entity.references.iter())
        .filter(|reference| reference.collection == collection.name) {
        collection.mapText(value, &reference.column, &mut |value|
            resolveMergeReference(tx, reference, value, pending, preferenceReferences))?;
    }
    Ok(())
}

/// Reports whether a declared entity owns a root or one of its explicit child collections.
fn entityOwnsCollection(entity: &StorageMergeEntity, collection: &str) -> bool {
    entity.collection == collection || entity.child_collections.iter().any(|item| item == collection)
}
/// Applies the Core-owned declarative UUID merge rules to every incoming transaction.
fn mergePayload(
    tx: &mut SqliteTransaction<'_>,
    payload: &mut Payload,
    operation: &SyncOperation,
    preferenceReferences: &BTreeMap<String, String>,
) -> Result<(), SqliteStoreError> {
    let Some(descriptor) = payload.manifest.merge.as_ref() else {
        return Ok(());
    };
    validateMergeDescriptor(tx, descriptor, payload)?;
    let sourceChanges = payload.changes.clone();
    let mut pending = BTreeMap::<(String, String), String>::new();
    for entity in &descriptor.entities {
        let collection = MergeCollection::open(tx, payload.manifest.kind, &entity.collection)?;
        let localRows = collection.read(tx)?;
        let localContent = localRows.iter().map(|(key, value)| {
            let mut value = value.clone();
            rewriteMergeReferences(tx, &collection, descriptor, &mut value, &pending, preferenceReferences)?;
            Ok((key.clone(), collection.content(entity, &value)?))
        }).collect::<Result<Vec<_>, SqliteStoreError>>()?;
        for change in payload
            .changes
            .iter_mut()
            .filter(|change| change.collection == entity.collection)
        {
            let remoteKey = change.key.clone();
            let canonicalKey = if let Some(mapped) =
                mergeMapping(tx, &entity.collection, &remoteKey)?
            {
                mapped
            } else if !entity.key_references.is_empty()
                || entity.match_mode == StorageMergeMatchMode::Identity
            {
                resolveEntityKey(tx, entity, &remoteKey, &pending, preferenceReferences)?
            } else if change.deleted || localRows.iter().any(|(key, _)| key == &remoteKey) {
                remoteKey.clone()
            } else {
                let mut candidate = change.value.clone();
                rewriteMergeReferences(tx, &collection, descriptor, &mut candidate, &pending, preferenceReferences)?;
                let content = collection.content(entity, &candidate)?;
                let mut equivalent = None;
                for (key, item) in &localContent {
                    if item == &content && sameEntityChildren(tx, payload.manifest.kind, descriptor, entity, key, &remoteKey, &sourceChanges, &pending, preferenceReferences)? {
                        equivalent = Some(key);
                        break;
                    }
                }
                if let Some(key) = equivalent {
                    if remoteKey < *key {
                        rekeyEntity(tx, payload.manifest.kind, descriptor, entity, key, &remoteKey, &operation.originDeviceId)?;
                        remoteKey.clone()
                    } else {
                        key.clone()
                    }
                } else {
                    remoteKey.clone()
                }
            };
            saveMergeMapping(
                tx,
                &entity.collection,
                &operation.originDeviceId,
                &remoteKey,
                &canonicalKey,
            )?;
            pending.insert(
                (entity.collection.clone(), remoteKey.clone()),
                canonicalKey.clone(),
            );

        }
    }
    for change in &mut payload.changes {
        for entity in &descriptor.entities {
            let remoteRoot = if change.collection == entity.collection {
                Some(change.key.clone())
            } else if entityOwnsCollection(entity, &change.collection) {
                rootFromChildKey(&change.key, entity.key_encoding)?
            } else {
                None
            };
            let Some(remoteRoot) = remoteRoot else {
                continue;
            };
            let Some(canonicalRoot) = pending
                .get(&(entity.collection.clone(), remoteRoot.clone()))
                .cloned()
                .or(mergeMapping(tx, &entity.collection, &remoteRoot)?)
            else {
                continue;
            };
            if change.collection == entity.collection {
                change.key = canonicalRoot.clone();
            } else {
                change.key = rewriteChildKey(&change.key, &remoteRoot, &canonicalRoot, entity.key_encoding)?;
            }
            if change.deleted {
                continue;
            }
            let collection = MergeCollection::open(tx, payload.manifest.kind, &change.collection)?;
            collection.identity(&mut change.value, entity, &change.key, &remoteRoot, &canonicalRoot)?;

        }
    }
    for change in payload.changes.iter_mut().filter(|change| !change.deleted) {
        let collection = MergeCollection::open(tx, payload.manifest.kind, &change.collection)?;
        rewriteMergeReferences(tx, &collection, descriptor, &mut change.value, &pending, preferenceReferences)?;
    }

    Ok(())
}

/// Canonicalizes stored references without changing their originating write versions or journal payloads.
fn reconcileLocalReferences(
    tx: &mut SqliteTransaction<'_>, kind: StorageKind,
    descriptor: &StorageMergeDescriptor,
    preferenceReferences: &BTreeMap<String, String>,
) -> Result<(), SqliteStoreError> {
    for entity in &descriptor.entities {
        let collection = MergeCollection::open(tx, kind, &entity.collection)?;
        for (key, _) in collection.read(tx)? {
            let canonical = resolveEntityKey(tx, entity, &key, &BTreeMap::new(), preferenceReferences)?;
            if canonical != key { rekeyEntity(tx, kind, descriptor, entity, &key, &canonical, "preferences")?; }
        }
    }
    let mut references = BTreeMap::<String, Vec<&StorageMergeReference>>::new();
    for reference in descriptor.entities.iter().flat_map(|entity| entity.references.iter()) {
        references.entry(reference.collection.clone()).or_default().push(reference);
    }
    for (name, references) in references {
        let collection = MergeCollection::open(tx, kind, &name)?;
        for (key, mut value) in collection.read(tx)? {
            let original = value.clone();
            for reference in &references {
                collection.mapText(&mut value, &reference.column, &mut |value|
                    resolveMergeReference(tx, reference, value, &BTreeMap::new(), preferenceReferences))?;
            }
            if value != original { collection.write(tx, &key, Some(&value))?; }
        }
    }
    Ok(())
}

/// Reconciles every registered shared database after preference identity canonicalization.
pub fn reconcilePreferenceReferences(
    host: Arc<dyn RuntimeSqliteHost>,
    preferenceReferences: &BTreeMap<String, String>,
) -> Result<(), SqliteStoreError> {
    if preferenceReferences.is_empty() {
        return Ok(());
    }
    let mut errors = Vec::new();
    for row in catalog(host.clone())?.queryRows(
        "SELECT path,owner,kind,sync,merge FROM plugin_databases WHERE sync=1",
        Vec::new(),
    )? {
        let path: String = row.get(0)?;
        let result = (|| -> Result<(), SqliteStoreError> {
            let kind = match row.get::<_, String>(2)?.as_str() {
                "sqlite" => StorageKind::Sqlite,
                "objects" => StorageKind::Objects,
                _ => return Err(failure("Unknown registered storage kind")),
            };
            let descriptor = row
                .get::<_, Option<String>>(4)?
                .map(|value| {
                    serde_json::from_str(&value)
                        .map_err(|error| failure(error.to_string()))
                        .and_then(decodeMergeDescriptor)
                })
                .transpose()?;
            let manifest = Manifest {
                path: path.clone(),
                owner: row.get(1)?,
                kind,
                sync: true,
                merge: descriptor,
            };
            if let Some(descriptor) = manifest.merge {
                let store = SqliteStore::openWithHost(host.clone(), &manifest.path)?;
                store.transaction(|tx| {
                    reconcileLocalReferences(tx, manifest.kind, &descriptor, preferenceReferences)?;
                    recordPreferenceAliases(tx, preferenceReferences)?;
                    tx.execute("DELETE FROM _operit_dirty", Vec::new())?;
                    Ok(())
                })?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            errors.push(format!("reconcile path={path}: {error}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(failure(errors.join("; ")))
    }
}

/// Heals rows written under identities that a completed merge aliased away.
///
/// Local writers can hold a pre-merge identity long after the preference and
/// object merges renamed it (a live plugin session keeps writing its loaded
/// owner keys). Their rows are moved onto the surviving canonical identities
/// using only the durable merge map, so a plugin opens consistent state even
/// when no synchronization batch runs.
fn reconcileDurableMappings(
    tx: &mut SqliteTransaction<'_>, kind: StorageKind,
    descriptor: &StorageMergeDescriptor,
) -> Result<(), SqliteStoreError> {
    for entity in &descriptor.entities {
        let collection = MergeCollection::open(tx, kind, &entity.collection)?;
        for (key, _) in collection.read(tx)? {
            if let Some(canonical) = mergeMapping(tx, &entity.collection, &key)? {
                if canonical != key {
                    rekeyEntity(tx, kind, descriptor, entity, &key, &canonical, "merge-map")?;
                }
            }
        }
        for name in &entity.child_collections {
            let child = MergeCollection::open(tx, kind, name)?;
            for (key, _) in child.read(tx)? {
                let Some(root) = rootFromChildKey(&key, entity.key_encoding)? else { continue; };
                if let Some(canonical) = mergeMapping(tx, &entity.collection, &root)? {
                    if canonical != root {
                        rekeyEntity(tx, kind, descriptor, entity, &root, &canonical, "merge-map")?;
                    }
                }
            }
        }
    }
    let preferenceAliases = durablePreferenceAliases(tx)?;
    let mut references = BTreeMap::<String, Vec<&StorageMergeReference>>::new();
    for reference in descriptor.entities.iter().flat_map(|entity| entity.references.iter()) {
        references.entry(reference.collection.clone()).or_default().push(reference);
    }
    for (name, references) in references {
        let collection = MergeCollection::open(tx, kind, &name)?;
        for (key, mut value) in collection.read(tx)? {
            let original = value.clone();
            for reference in &references {
                collection.mapText(&mut value, &reference.column, &mut |value|
                    resolveMergeReference(tx, reference, value, &BTreeMap::new(), &preferenceAliases))?;
            }
            if value != original { collection.write(tx, &key, Some(&value))?; }
        }
    }
    Ok(())
}

/// Identifies the explicit plugin database commit boundary without interpreting its record mutations.
pub fn syncDatasetPath(operation: &SyncOperation) -> Result<String, SqliteStoreError> {
    if operation.domain != PLUGIN_STORAGE_SYNC_DOMAIN { return Err(failure("Not a plugin storage operation")); }
    let path = operation.payload.get("manifest").and_then(|manifest| manifest.get("path")).and_then(Value::as_str)
        .ok_or_else(|| failure("Plugin transaction dataset path is missing"))?;
    resolveStoragePath(path).map_err(failure)
}

/// Applies one remote transaction with Core-validated entity and reference declarations.
pub fn applyOperation(
    host: Arc<dyn RuntimeSqliteHost>,
    storage: Arc<dyn RuntimeStorageHost>,
    operation: &SyncOperation,
    preferences: &BTreeMap<String, String>,
) -> Result<(), SqliteStoreError> {
    if operation.domain != PLUGIN_STORAGE_SYNC_DOMAIN
        || operation.entityType != "transaction"
        || operation.operation != "commit"
        || operation.schemaVersion != 1
        || operation.semantics != SyncOperationSemantics::Transaction
        || operation.sequence <= 0
        || operation.opId != format!("{}:{}", operation.originDeviceId, operation.sequence)
        || operation.entityId != operation.opId
    {
        return Err(failure("Invalid plugin storage sync envelope"));
    }
    let recordedPayload = operation.payload.clone();
    let payload = decodePayload(recordedPayload.clone())?;
    if !payload.manifest.sync {
        return Err(failure("Remote storage must declare synchronization"));
    }
    validatePayload(&payload)?;
    let path = resolveStoragePath(&payload.manifest.path).map_err(failure)?;
    if runtimeStorageOwnership(&path).map_err(failure)? != RuntimeStorageOwnership::Space
        || storageNamespace(&path).map_err(failure)? != payload.manifest.owner
    {
        return Err(failure("Remote storage identity does not match its shared data path"));
    }
    let digest = format!("{:x}", Sha256::digest(payload.manifest.path.as_bytes()));
    let (databaseOrigin, epoch) = operation
        .originDeviceId
        .rsplit_once(':')
        .ok_or_else(|| failure("Missing storage origin epoch"))?;
    if operation.createdAt < 0
        || operation.originDeviceId.len() > 256
        || !databaseOrigin.ends_with(&format!(":storage:{digest}"))
        || Uuid::parse_str(epoch).is_err()
    {
        return Err(failure(
            "Remote storage origin does not match its database path and epoch",
        ));
    }
    let database = openDatabase(host.clone(), &payload.manifest, String::new())?;
    registerDatabase(host, &payload.manifest)?;
    registerFile(storage, &payload.manifest)?;
    database.store.transaction(|tx| {
        if let Some(row) = tx.queryOne("SELECT payload,created_at FROM _operit_log WHERE origin=?1 AND sequence=?2",vec![SqliteValue::Text(operation.originDeviceId.clone()),SqliteValue::Integer(operation.sequence)])? {
            let recorded: Value = serde_json::from_str(&row.get::<_,String>(0)?).map_err(|e|failure(e.to_string()))?;
            if recorded != recordedPayload || row.get::<_,i64>(1)? != operation.createdAt { return Err(failure("Remote operation identity has conflicting content")); }
            return Ok(());
        }
        if payload.tables.len() > 1000 {return Err(failure("Remote transaction exceeds table declaration limit"));}
        let mut tableNames=BTreeSet::new();
        for table in &payload.tables {
            if database.kind!=StorageKind::Sqlite {return Err(failure("Remote object database contains a SQL schema"));}
            if !tableNames.insert(&table.name) {return Err(failure("Duplicate remote table declaration"));}
            defineTable(tx,table)?;
        }
        let mut payload = payload.clone();
        projectHistoricalRows(tx, &mut payload)?;
        payload.manifest.merge = database.merge.clone();
        if payload.manifest.merge.is_some() && !payload.changes.is_empty() {
            mergePayload(
                tx,
                &mut payload,
                operation,
                preferences,
            )?;
            validatePayload(&payload)?;
        }
        let mut seen=BTreeSet::new();
        if payload.changes.len()>1000 {return Err(failure("Remote transaction exceeds mutation limit"));}
        for change in &payload.changes {
            if change.key.is_empty() || change.key.len() > 2048 || change.key.chars().any(char::is_control) || !seen.insert((&change.collection,&change.key)) {return Err(failure("Invalid or duplicate remote key"));}
            if let Some(row)=tx.queryOne("SELECT created_at,origin,sequence FROM _operit_versions WHERE collection=?1 AND key=?2",vec![SqliteValue::Text(change.collection.clone()),SqliteValue::Text(change.key.clone())])? {
                let current=(row.get::<_,i64>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?);
                let incoming=(operation.createdAt,operation.originDeviceId.clone(),operation.sequence);
                if incoming<=current {continue;}
            }
            if let Some(descriptor) = &database.merge {
                rememberPayloadNames(tx, database.kind, descriptor, std::slice::from_ref(change))?;
            }
            if database.kind==StorageKind::Sqlite {applyRow(tx,change)?;} else {
                validateCollection(&database,&change.collection)?;
                if !change.deleted { let value = &change.value; tx.execute("INSERT INTO _operit_values VALUES(?1,?2,?3,?4) ON CONFLICT(collection,key) DO UPDATE SET value=excluded.value,version=excluded.version",vec![SqliteValue::Text(change.collection.clone()),SqliteValue::Text(change.key.clone()),SqliteValue::Text(serde_json::to_string(value).map_err(|e|failure(e.to_string()))?),SqliteValue::Text(operation.opId.clone())])?;
                } else { tx.execute("DELETE FROM _operit_values WHERE collection=?1 AND key=?2",vec![SqliteValue::Text(change.collection.clone()),SqliteValue::Text(change.key.clone())])?;
                }
            }
            tx.execute("INSERT INTO _operit_versions VALUES(?1,?2,?3,?4,?5) ON CONFLICT(collection,key) DO UPDATE SET created_at=excluded.created_at,origin=excluded.origin,sequence=excluded.sequence",vec![SqliteValue::Text(change.collection.clone()),SqliteValue::Text(change.key.clone()),SqliteValue::Integer(operation.createdAt),SqliteValue::Text(operation.originDeviceId.clone()),SqliteValue::Integer(operation.sequence)])?;
        }
        if !payload.changes.is_empty() {
            if let Some(descriptor) = &database.merge {
                reconcileLocalReferences(tx, database.kind, descriptor, preferences)?;
                reconcileMergeNames(tx, database.kind, descriptor)?;
            }
        }
        tx.execute("DELETE FROM _operit_dirty",Vec::new())?;
        tx.execute("INSERT INTO _operit_log(origin,sequence,created_at,payload,outgoing) VALUES(?1,?2,?3,?4,1) ON CONFLICT(origin,sequence) DO NOTHING",vec![SqliteValue::Text(operation.originDeviceId.clone()),SqliteValue::Integer(operation.sequence),SqliteValue::Integer(operation.createdAt),SqliteValue::Text(serde_json::to_string(&recordedPayload).map_err(|e|failure(e.to_string()))?)])?;
        Ok(())
    })
}
