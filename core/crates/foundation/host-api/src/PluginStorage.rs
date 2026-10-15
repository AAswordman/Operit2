//! Platform-independent contracts for plugin-owned transactional storage.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Preserves SQLite integers and binary values without JavaScript number coercion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum StorageSqlValue {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "integer")]
    Integer(String),
    #[serde(rename = "real")]
    Real(f64),
    #[serde(rename = "text")]
    Text(String),
    #[serde(rename = "blob")]
    Blob(Vec<u8>),
}

/// Executes one parameterized statement inside an explicitly bounded transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageStatement {
    pub sql: String,
    pub params: Vec<StorageSqlValue>,
    /// Requires an exact write count inside the transaction for optimistic concurrency.
    #[serde(default, rename = "expectedAffectedRows", skip_serializing_if = "Option::is_none")]
    pub expected_affected_rows: Option<u64>,
}

/// Selects a database's permanently recorded data contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageKind {
    #[serde(rename = "sqlite")]
    Sqlite,
    #[serde(rename = "objects")]
    Objects,
    #[serde(rename = "data_store")]
    DataStore,
}

/// Declares a synchronized collection column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageColumnType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "real")]
    Real,
    #[serde(rename = "blob")]
    Blob,
}

/// Declares one column without accepting executable schema SQL from a remote peer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageColumn {
    pub name: String,
    pub affinity: StorageColumnType,
    pub nullable: bool,
}

/// Declares a synchronized table with one text primary key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageTable {
    pub name: String,
    #[serde(rename = "primaryKey")]
    pub primary_key: String,
    pub columns: Vec<StorageColumn>,
}

/// Stages one object or preference-key mutation and an optional exact-version precondition.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageMutation {
    pub collection: String,
    pub key: String,
    pub value: Value,
    pub deleted: bool,
    #[serde(rename = "checkVersion")]
    pub check_version: bool,
    #[serde(rename = "expectedVersion")]
    pub expected_version: Option<String>,
}

/// Dispatches storage operations only through an authenticated engine-owned handle.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum StorageRequest {
    #[serde(rename = "open")]
    Open {
        path: String,
        kind: StorageKind,
    },
    #[serde(rename = "close")]
    Close { handle: String },
    #[serde(rename = "query")]
    Query {
        handle: String,
        statement: StorageStatement,
    },
    #[serde(rename = "execute")]
    Execute {
        handle: String,
        statement: StorageStatement,
    },
    #[serde(rename = "transaction")]
    Transaction {
        handle: String,
        statements: Vec<StorageStatement>,
    },
    #[serde(rename = "define_table")]
    DefineTable { handle: String, table: StorageTable },
    #[serde(rename = "get")]
    Get {
        handle: String,
        collection: String,
        key: String,
    },
    #[serde(rename = "list")]
    List {
        handle: String,
        collection: String,
        after: Option<String>,
        limit: u32,
    },
    #[serde(rename = "commit")]
    Commit {
        handle: String,
        mutations: Vec<StorageMutation>,
    },
    #[serde(rename = "changes")]
    Changes {
        handle: String,
        after: String,
        limit: u32,
    },
}

/// Identifies one successfully opened database session.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageOpenResult {
    pub handle: String,
    pub path: String,
}

/// Preserves the exact column order and tagged SQLite values of one query row.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageSqlRow {
    pub columns: Vec<String>,
    pub values: Vec<StorageSqlValue>,
}

/// Reports one statement's affected rows and exact int64 insertion identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageSqlExecutionResult {
    pub affected_rows: u64,
    pub last_insert_id: String,
}

/// Reports whether a declarative table definition changed the schema.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageTableDefinitionResult {
    pub changed: bool,
}

/// Preserves a stored application JSON value and its exact concurrency version.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageValueEntry {
    pub value: Value,
    pub version: String,
}

/// Adds the collection key to a listed value and its concurrency version.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageKeyValueEntry {
    pub key: String,
    pub value: Value,
    pub version: String,
}

/// Reports the actual version committed by an atomic mutation batch.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageCommitResult {
    pub version: String,
}

/// Preserves the durable database identity carried by a change transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageManifest {
    pub path: String,
    pub owner: String,
    pub kind: StorageKind,
    pub sync: bool,
}

/// Describes one declarative record mutation with an application-owned JSON value.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageRecordChange {
    pub collection: String,
    pub key: String,
    pub value: Value,
    pub deleted: bool,
}

/// Preserves all schema and record changes from one atomic storage transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageChangeTransaction {
    pub manifest: StorageManifest,
    pub tables: Vec<StorageTable>,
    pub changes: Vec<StorageRecordChange>,
}

/// Carries one exact int64 cursor and its complete atomic transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageChangeEntry {
    pub cursor: String,
    pub transaction: StorageChangeTransaction,
}

/// Returns each storage operation's concrete business result.
/// Serialization preserves the database API payload; tool results add their own operation discriminator.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum StorageResponse {
    Open(StorageOpenResult),
    Close,
    Query(Vec<StorageSqlRow>),
    Execute(Vec<StorageSqlExecutionResult>),
    Transaction(Vec<StorageSqlExecutionResult>),
    DefineTable(StorageTableDefinitionResult),
    Get(Option<StorageValueEntry>),
    List(Vec<StorageKeyValueEntry>),
    Commit(StorageCommitResult),
    Changes(Vec<StorageChangeEntry>),
}
