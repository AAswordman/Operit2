//! Plugin-owned SQLite, object collections and atomic keys over the existing Host.
#![allow(non_snake_case)]

use crate::SqliteStore::{SqliteRowGet, SqliteStore, SqliteStoreError, SqliteTransaction};
use crate::SyncOperationStore::{
    SyncClock, SyncOperation, SyncOperationSemantics, publishSyncMutation,
};
use operit_host_api::PluginStorage::*;
use operit_host_api::{RuntimeSqliteHost, RuntimeStorageHost, SqliteValue};
use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership, PLUGIN_DATA_SPACE_DIR_PATH, PLUGIN_DATA_DEVICE_DIR_PATH};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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
CREATE TABLE IF NOT EXISTS _operit_manifest (id INTEGER PRIMARY KEY CHECK(id=1), owner TEXT NOT NULL, path TEXT NOT NULL, kind TEXT NOT NULL, sync INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS _operit_origins (device TEXT PRIMARY KEY, origin TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS _operit_tables (name TEXT PRIMARY KEY, definition TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS _operit_values (collection TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, version TEXT NOT NULL, PRIMARY KEY(collection,key));
CREATE TABLE IF NOT EXISTS _operit_dirty (collection TEXT NOT NULL, key TEXT NOT NULL, deleted INTEGER NOT NULL, PRIMARY KEY(collection,key));
CREATE TABLE IF NOT EXISTS _operit_log (revision INTEGER PRIMARY KEY AUTOINCREMENT, origin TEXT NOT NULL, sequence INTEGER NOT NULL, created_at INTEGER NOT NULL, payload TEXT NOT NULL, outgoing INTEGER NOT NULL, UNIQUE(origin,sequence));
CREATE INDEX IF NOT EXISTS _operit_log_time ON _operit_log(created_at);
CREATE TABLE IF NOT EXISTS _operit_versions (collection TEXT NOT NULL, key TEXT NOT NULL, created_at INTEGER NOT NULL, origin TEXT NOT NULL, sequence INTEGER NOT NULL, PRIMARY KEY(collection,key));
"#;
const CATALOG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS plugin_databases (path TEXT PRIMARY KEY, owner TEXT NOT NULL, kind TEXT NOT NULL, sync INTEGER NOT NULL);
"#;

/// Owns all handles for one authenticated JavaScript execution host.
#[derive(Clone, Default)]
pub struct PluginStorageSession {
    handles: Arc<Mutex<BTreeMap<String, Arc<Database>>>>,
}

struct Database {
    store: SqliteStore,
    path: String,
    owner: String,
    kind: StorageKind,
    sync: bool,
    origin: String,
}

/// Preserves one durable data namespace and database identity for synchronization and file exclusion.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    path: String,
    owner: String,
    kind: StorageKind,
    sync: bool,
}

/// Transports declarative row values rather than executable SQL or database file bytes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    collection: String,
    key: String,
    value: Value,
    deleted: bool,
}

/// Preserves a bounded transaction as one atomic remote commit.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    manifest: Manifest,
    tables: Vec<StorageTable>,
    changes: Vec<Change>,
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
        StorageKind::DataStore => "data_store",
    }
}

/// Validates supported persistent data paths without restricting access by the calling plugin identity.
pub fn resolveStoragePath(path: &str) -> Result<String, SqliteStoreError> {
    let path = path.replace('\\', "/");
    if path.is_empty()
        || path.split('/').any(|segment| {
            segment.is_empty()
                || segment == "."
                || segment == ".."
                || segment.chars().any(char::is_control)
        })
    {
        return Err(failure("Storage path contains invalid segments"));
    }
    storageNamespace(&path)?;
    if [".operit-storage.json", "-wal", "-shm", "-journal"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
    {
        return Err(failure("Storage path uses a reserved sidecar suffix"));
    }
    Ok(path)
}

/// Reads the durable data namespace from the canonical path independently of the script opening it.
fn storageNamespace(path: &str) -> Result<&str, SqliteStoreError> {
    for root in [PLUGIN_DATA_SPACE_DIR_PATH, PLUGIN_DATA_DEVICE_DIR_PATH] {
        if let Some(relative) = path.strip_prefix(&format!("{root}/")) {
            let (namespace, file) = relative.split_once('/')
                .ok_or_else(|| failure("Storage path requires a namespace and database filename"))?;
            if !namespace.is_empty() && !file.is_empty() {
                return Ok(namespace);
            }
        }
    }
    Err(failure("Databases must use persistent plugin_data directories independent of installation scope"))
}

/// Opens the node-local catalog used for storage discovery, export and file exclusions.
fn catalog(host: Arc<dyn RuntimeSqliteHost>) -> Result<SqliteStore, SqliteStoreError> {
    let store = SqliteStore::openWithHost(host, CATALOG_PATH)?;
    store.executeBatch(CATALOG_SCHEMA)?;
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
        if row.get::<_, String>(0)? != definition {
            return Err(failure(
                "Table declaration differs from the persisted schema",
            ));
        }
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
    let name = identifier(&table.name)?;
    let key = identifier(&table.primary_key)?;
    for (suffix, event, body) in [
        (
            "insert",
            "INSERT",
            format!(
                "INSERT OR REPLACE INTO _operit_dirty VALUES('{}',NEW.{key},0);",
                table.name
            ),
        ),
        (
            "delete",
            "DELETE",
            format!(
                "INSERT OR REPLACE INTO _operit_dirty VALUES('{}',OLD.{key},1);",
                table.name
            ),
        ),
        (
            "update",
            "UPDATE",
            format!(
                "INSERT OR REPLACE INTO _operit_dirty VALUES('{}',OLD.{key},1); INSERT OR REPLACE INTO _operit_dirty VALUES('{}',NEW.{key},0);",
                table.name, table.name
            ),
        ),
    ] {
        tx.execute(&format!("CREATE TRIGGER \"_operit_track_{}_{suffix}\" AFTER {event} ON {name} BEGIN {body} END", table.name), Vec::new())?;
    }
    Ok(true)
}

/// Opens only a database carrying the same owner, path, kind and synchronization policy.
fn openDatabase(
    host: Arc<dyn RuntimeSqliteHost>,
    manifest: &Manifest,
    mut origin: String,
) -> Result<Database, SqliteStoreError> {
    let store = SqliteStore::openWithHost(host, &manifest.path)?;
    store.execute("PRAGMA recursive_triggers = ON", Vec::new())?;
    store.transaction(|tx| {
        let tables = tx.queryRows("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", Vec::new())?;
        if !tables.is_empty() && !tables.iter().any(|row| matches!(row.values.first(), Some(SqliteValue::Text(name)) if name == "_operit_manifest")) {
            return Err(failure("Existing database does not carry a plugin storage manifest"));
        }
        let existing = !tables.is_empty();
        for sql in DATABASE_SCHEMA.split(';').filter(|sql| !sql.trim().is_empty()) { tx.execute(sql, Vec::new())?; }

        if let Some(row) = tx.queryOne(
            "SELECT owner,path,kind,sync FROM _operit_manifest WHERE id=1",
            Vec::new(),
        )? {
            if row.get::<_, String>(0)? != manifest.owner
                || row.get::<_, String>(1)? != manifest.path
                || row.get::<_, String>(2)? != kindName(&manifest.kind)
                || row.get::<_, i64>(3)? != i64::from(manifest.sync)
            {
                return Err(failure("Database identity or storage policy mismatch"));
            }
        } else {
            if existing { return Err(failure("Existing database has no persisted plugin identity")); }
            tx.execute(
                "INSERT INTO _operit_manifest VALUES(1,?1,?2,?3,?4)",
                vec![
                    SqliteValue::Text(manifest.owner.clone()),
                    SqliteValue::Text(manifest.path.clone()),
                    SqliteValue::Text(kindName(&manifest.kind).into()),
                    SqliteValue::Integer(i64::from(manifest.sync)),
                ],
            )?;
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
        Ok(())
    })?;
    Ok(Database {
        store,
        path: manifest.path.clone(),
        owner: manifest.owner.clone(),
        kind: manifest.kind.clone(),
        sync: manifest.sync,
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
            "SELECT owner,kind,sync FROM plugin_databases WHERE path=?1",
            vec![SqliteValue::Text(manifest.path.clone())],
        )? {
            if row.get::<_, String>(0)? != manifest.owner
                || row.get::<_, String>(1)? != kindName(&manifest.kind)
                || row.get::<_, i64>(2)? != i64::from(manifest.sync)
            {
                return Err(failure("Storage catalog identity mismatch"));
            }
        } else {
            tx.execute(
                "INSERT INTO plugin_databases VALUES(?1,?2,?3,?4)",
                vec![
                    SqliteValue::Text(manifest.path.clone()),
                    SqliteValue::Text(manifest.owner.clone()),
                    SqliteValue::Text(kindName(&manifest.kind).into()),
                    SqliteValue::Integer(i64::from(manifest.sync)),
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
    /// Performs one operation using only this session's owner-authenticated retained handles.
    pub fn request(
        &self,
        host: Arc<dyn RuntimeSqliteHost>,
        storage: Arc<dyn RuntimeStorageHost>,
        request: StorageRequest,
        deviceId: &str,
    ) -> Result<Value, SqliteStoreError> {
        if let StorageRequest::Open { path, kind } = &request {
            let path = resolveStoragePath(path)?;
            let sync = match runtimeStorageOwnership(&path).map_err(failure)? {
                RuntimeStorageOwnership::Space => true,
                RuntimeStorageOwnership::CoreNode => false,
                RuntimeStorageOwnership::Ephemeral => {
                    return Err(failure("Plugin databases require durable storage ownership"));
                }
            };
            let manifest = Manifest {
                path: path.clone(),
                owner: storageNamespace(&path)?.to_string(),
                kind: *kind,
                sync,
            };
            let digest = format!("{:x}", Sha256::digest(path.as_bytes()));
            let database = Arc::new(openDatabase(
                host.clone(),
                &manifest,
                format!("{deviceId}:storage:{digest}"),
            )?);
            registerDatabase(host, &manifest)?;
            registerFile(storage.clone(), &manifest)?;
            let handle = Uuid::new_v4().to_string();
            self.handles
                .lock()
                .map_err(|_| failure("Storage handles poisoned"))?
                .insert(handle.clone(), database);
            return Ok(json!({"handle":handle,"path":path}));
        }
        let handle = match &request {
            StorageRequest::Close { handle }
            | StorageRequest::Query { handle, .. }
            | StorageRequest::Execute { handle, .. }
            | StorageRequest::Transaction { handle, .. }
            | StorageRequest::DefineTable { handle, .. }
            | StorageRequest::Get { handle, .. }
            | StorageRequest::List { handle, .. }
            | StorageRequest::Commit { handle, .. }
            | StorageRequest::Changes { handle, .. } => handle,
            StorageRequest::Open { .. } => unreachable!(),
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
            return Ok(Value::Null);
        }
        resolveStoragePath(&database.path)?;
        if !storage.exists(&database.path)? || !ownsFile(storage.clone(), &database.path)? {
            return Err(failure("Storage database has been removed"));
        }
        let mutating = matches!(
            &request,
            StorageRequest::Execute { .. }
                | StorageRequest::Transaction { .. }
                | StorageRequest::DefineTable { .. }
                | StorageRequest::Commit { .. }
        );
        let result = database.store.transaction(|tx| match request {
            StorageRequest::Query {statement,..} => {
                if database.kind != StorageKind::Sqlite { return Err(failure("SQL access requires a SQLite handle")); }
                validateStatement(&statement,true,database.sync)?;
                let rows = tx.queryRows(&statement.sql,statement.params.into_iter().map(sqlValue).collect::<Result<Vec<_>,_>>()?)?;
                Ok(json!(rows.into_iter().map(|r| json!({"columns":r.columns,"values":r.values.into_iter().map(wireValue).collect::<Vec<_>>()})).collect::<Vec<_>>()))
            }
            StorageRequest::Execute {statement,..} => executeStatements(&database, tx, vec![statement]),
            StorageRequest::Transaction {statements,..} => executeStatements(&database, tx, statements),
            StorageRequest::DefineTable {table,..} => {
                if database.kind != StorageKind::Sqlite { return Err(failure("Table declarations require a SQLite handle")); }
                let changed = defineTable(tx,&table)?;
                if changed { database.record(tx,Some(table),Vec::new())?; }
                Ok(json!({"changed":changed}))
            }
            StorageRequest::Get {collection,key,..} => {
                validateCollection(&database, &collection)?;
                let row = tx.queryOne("SELECT value,version FROM _operit_values WHERE collection=?1 AND key=?2", vec![SqliteValue::Text(collection),SqliteValue::Text(key)])?;
                match row { Some(r) => Ok(json!({"value":serde_json::from_str::<Value>(&r.get::<_,String>(0)?).map_err(|e| failure(e.to_string()))?,"version":r.get::<_,String>(1)?})), None => Ok(Value::Null) }
            }
            StorageRequest::List {collection,after,limit,..} => {
                validateCollection(&database, &collection)?; bounded(limit)?;
                let rows = match after {
                    Some(after) => tx.queryRows("SELECT key,value,version FROM _operit_values WHERE collection=?1 AND key>?2 ORDER BY key LIMIT ?3",vec![SqliteValue::Text(collection),SqliteValue::Text(after),SqliteValue::Integer(i64::from(limit))])?,
                    None => tx.queryRows("SELECT key,value,version FROM _operit_values WHERE collection=?1 ORDER BY key LIMIT ?2",vec![SqliteValue::Text(collection),SqliteValue::Integer(i64::from(limit))])?,
                };
                rows.into_iter().map(|r| Ok(json!({"key":r.get::<_,String>(0)?,"value":serde_json::from_str::<Value>(&r.get::<_,String>(1)?).map_err(|e| failure(e.to_string()))?,"version":r.get::<_,String>(2)?}))).collect::<Result<Vec<_>,SqliteStoreError>>().map(|v| json!(v))
            }
            StorageRequest::Commit {mutations,..} => commitValues(&database,tx,mutations),
            StorageRequest::Changes {after,limit,..} => {
                bounded(limit)?; let after = after.parse::<i64>().map_err(|_| failure("Invalid change cursor"))?;
                if after < 0 { return Err(failure("Negative change cursor")); }
                let rows = tx.queryRows("SELECT revision,payload FROM _operit_log WHERE revision>?1 ORDER BY revision LIMIT ?2",vec![SqliteValue::Integer(after),SqliteValue::Integer(i64::from(limit))])?;
                rows.into_iter().map(|r| Ok(json!({"cursor":r.get::<_,i64>(0)?.to_string(),"transaction":serde_json::from_str::<Value>(&r.get::<_,String>(1)?).map_err(|e| failure(e.to_string()))?}))).collect::<Result<Vec<_>,SqliteStoreError>>().map(|v| json!(v))
            }
            StorageRequest::Open {..} | StorageRequest::Close {..} => unreachable!(),
        })?;
        if database.sync && mutating {
            publishSyncMutation();
        }
        Ok(result)
    }
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
            "Object operations require an object or DataStore handle",
        ));
    }
    if collection.is_empty() || collection.len() > 256 || collection.chars().any(char::is_control) {
        return Err(failure("Invalid collection"));
    }
    if database.kind == StorageKind::DataStore && collection != "keys" {
        return Err(failure("DataStore collection must be keys"));
    }
    Ok(())
}

/// Executes bounded SQL writes and captures only trigger-marked rows before the commit.
fn executeStatements(
    database: &Database,
    tx: &mut SqliteTransaction<'_>,
    statements: Vec<StorageStatement>,
) -> Result<Value, SqliteStoreError> {
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
        results.push(
            json!({"affectedRows":affected,"lastInsertId":tx.lastInsertRowId()?.to_string()}),
        );
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
    Ok(json!(results))
}

/// Checks all versions and stages all key mutations inside one transaction.
fn commitValues(
    database: &Database,
    tx: &mut SqliteTransaction<'_>,
    mutations: Vec<StorageMutation>,
) -> Result<Value, SqliteStoreError> {
    if mutations.is_empty() || mutations.len() > 1000 {
        return Err(failure("Object commit requires 1..1000 mutations"));
    }
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
        if mutation.key.is_empty()
            || mutation.key.len() > 1024
            || mutation.key.chars().any(char::is_control)
            || !seen.insert((mutation.collection.clone(), mutation.key.clone()))
        {
            return Err(failure("Invalid or duplicated mutation key"));
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
    Ok(json!({"version":version}))
}

/// Discovers exact registered databases, never traversing plugin data files.
fn manifests(host: Arc<dyn RuntimeSqliteHost>) -> Result<Vec<Manifest>, SqliteStoreError> {
    catalog(host)?
        .queryRows(
            "SELECT path,owner,kind,sync FROM plugin_databases",
            Vec::new(),
        )?
        .into_iter()
        .map(|row| {
            let kind = match row.get::<_, String>(2)?.as_str() {
                "sqlite" => StorageKind::Sqlite,
                "objects" => StorageKind::Objects,
                "data_store" => StorageKind::DataStore,
                _ => return Err(failure("Unknown registered storage kind")),
            };
            Ok(Manifest {
                path: row.get(0)?,
                owner: row.get(1)?,
                kind,
                sync: row.get::<_, i64>(3)? == 1,
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
    let marker = format!("{database}.operit-storage.json");
    if !host.exists(&marker)? {
        return Ok(false);
    }
    let manifest: Manifest =
        serde_json::from_slice(&host.readBytes(&marker)?).map_err(|e| failure(e.to_string()))?;
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
    let content = serde_json::to_vec(manifest).map_err(|e| failure(e.to_string()))?;
    if host.exists(&marker)? {
        if host.readBytes(&marker)? != content {
            return Err(failure("Storage marker conflicts with database identity"));
        }
    } else {
        host.writeBytes(&marker, &content)?;
    }
    Ok(())
}

/// Combines the durable clocks recorded atomically inside synchronized plugin databases.
pub fn localClock(host: Arc<dyn RuntimeSqliteHost>) -> Result<SyncClock, SqliteStoreError> {
    let mut clock = SyncClock::empty();
    for manifest in manifests(host.clone())?.into_iter().filter(|m| m.sync) {
        let store = SqliteStore::openWithHost(host.clone(), &manifest.path)?;
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

/// Exports bounded record transactions directly from database outboxes without copying database bytes.
pub fn operationsSince(
    host: Arc<dyn RuntimeSqliteHost>,
    clock: &SyncClock,
    domains: &[String],
    limit: usize,
) -> Result<Vec<SyncOperation>, SqliteStoreError> {
    if !domains.is_empty() && !domains.iter().any(|d| d == PLUGIN_STORAGE_SYNC_DOMAIN) {
        return Ok(Vec::new());
    }
    let mut operations = Vec::new();
    for manifest in manifests(host.clone())?.into_iter().filter(|m| m.sync) {
        let store = SqliteStore::openWithHost(host.clone(), &manifest.path)?;
        for originRow in store.queryRows(
            "SELECT DISTINCT origin FROM _operit_log WHERE outgoing=1",
            Vec::new(),
        )? {
            let origin: String = originRow.get(0)?;
            for row in store.queryRows("SELECT sequence,created_at,payload FROM _operit_log WHERE origin=?1 AND sequence>?2 AND outgoing=1 ORDER BY sequence LIMIT ?3",vec![SqliteValue::Text(origin.clone()),SqliteValue::Integer(clock.sequenceFor(&origin)),SqliteValue::Integer(i64::try_from(limit).map_err(|_|failure("Sync limit overflows int64"))?)])? {
                let sequence:i64=row.get(0)?;
                operations.push(SyncOperation {opId:format!("{origin}:{sequence}"),originDeviceId:origin.clone(),sequence,domain:PLUGIN_STORAGE_SYNC_DOMAIN.into(),entityType:"transaction".into(),entityId:format!("{origin}:{sequence}"),operation:"commit".into(),semantics:SyncOperationSemantics::Transaction,payload:serde_json::from_str(&row.get::<_,String>(2)?).map_err(|e|failure(e.to_string()))?,createdAt:row.get(1)?,schemaVersion:1});
            }
        }
    }
    operations.sort_by(|a, b| {
        (a.createdAt, &a.originDeviceId, a.sequence).cmp(&(
            b.createdAt,
            &b.originDeviceId,
            b.sequence,
        ))
    });
    operations.truncate(limit);
    Ok(operations)
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
            || (payload.manifest.kind == StorageKind::DataStore && change.collection != "keys")
        {
            return Err(failure("Invalid remote storage collection"));
        }
    }
    Ok(())
}

/// Atomically replays declarative record changes and forwards their original operation identities.
pub fn applyOperation(
    host: Arc<dyn RuntimeSqliteHost>,
    storage: Arc<dyn RuntimeStorageHost>,
    operation: &SyncOperation,
    force: bool,
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
    let payload: Payload =
        serde_json::from_value(operation.payload.clone()).map_err(|e| failure(e.to_string()))?;
    if !payload.manifest.sync {
        return Err(failure("Remote storage must declare synchronization"));
    }
    validatePayload(&payload)?;
    let path = resolveStoragePath(&payload.manifest.path)?;
    if runtimeStorageOwnership(&path).map_err(failure)? != RuntimeStorageOwnership::Space
        || storageNamespace(&path)? != payload.manifest.owner
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
            let recorded: Payload = serde_json::from_str(&row.get::<_,String>(0)?).map_err(|e|failure(e.to_string()))?;
            if serde_json::to_value(recorded).map_err(|e|failure(e.to_string()))? != serde_json::to_value(&payload).map_err(|e|failure(e.to_string()))? || row.get::<_,i64>(1)? != operation.createdAt { return Err(failure("Remote operation identity has conflicting content")); }
            if !force { return Ok(()); }
        }
        if payload.tables.len() > 1000 {return Err(failure("Remote transaction exceeds table declaration limit"));}
        let mut tableNames=BTreeSet::new();
        for table in &payload.tables {
            if database.kind!=StorageKind::Sqlite {return Err(failure("Remote object database contains a SQL schema"));}
            if !tableNames.insert(&table.name) {return Err(failure("Duplicate remote table declaration"));}
            defineTable(tx,table)?;
        }
        let mut seen=BTreeSet::new();
        if payload.changes.len()>1000 {return Err(failure("Remote transaction exceeds mutation limit"));}
        for change in &payload.changes {
            if change.key.is_empty() || change.key.len() > 2048 || change.key.chars().any(char::is_control) || !seen.insert((&change.collection,&change.key)) {return Err(failure("Invalid or duplicate remote key"));}
            if let Some(row)=tx.queryOne("SELECT created_at,origin,sequence FROM _operit_versions WHERE collection=?1 AND key=?2",vec![SqliteValue::Text(change.collection.clone()),SqliteValue::Text(change.key.clone())])? {
                let current=(row.get::<_,i64>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?);
                let incoming=(operation.createdAt,operation.originDeviceId.clone(),operation.sequence);
                if !force && incoming<=current {continue;}
            }
            if database.kind==StorageKind::Sqlite {applyRow(tx,change)?;} else {
                validateCollection(&database,&change.collection)?;
                if !change.deleted { let value = &change.value; tx.execute("INSERT INTO _operit_values VALUES(?1,?2,?3,?4) ON CONFLICT(collection,key) DO UPDATE SET value=excluded.value,version=excluded.version",vec![SqliteValue::Text(change.collection.clone()),SqliteValue::Text(change.key.clone()),SqliteValue::Text(serde_json::to_string(value).map_err(|e|failure(e.to_string()))?),SqliteValue::Text(operation.opId.clone())])?;
                } else { tx.execute("DELETE FROM _operit_values WHERE collection=?1 AND key=?2",vec![SqliteValue::Text(change.collection.clone()),SqliteValue::Text(change.key.clone())])?;
                }
            }
            tx.execute("INSERT INTO _operit_versions VALUES(?1,?2,?3,?4,?5) ON CONFLICT(collection,key) DO UPDATE SET created_at=excluded.created_at,origin=excluded.origin,sequence=excluded.sequence",vec![SqliteValue::Text(change.collection.clone()),SqliteValue::Text(change.key.clone()),SqliteValue::Integer(operation.createdAt),SqliteValue::Text(operation.originDeviceId.clone()),SqliteValue::Integer(operation.sequence)])?;
        }
        tx.execute("DELETE FROM _operit_dirty",Vec::new())?;
        tx.execute("INSERT INTO _operit_log(origin,sequence,created_at,payload,outgoing) VALUES(?1,?2,?3,?4,1) ON CONFLICT(origin,sequence) DO NOTHING",vec![SqliteValue::Text(operation.originDeviceId.clone()),SqliteValue::Integer(operation.sequence),SqliteValue::Integer(operation.createdAt),SqliteValue::Text(serde_json::to_string(&payload).map_err(|e|failure(e.to_string()))?)])?;
        Ok(())
    })
}
