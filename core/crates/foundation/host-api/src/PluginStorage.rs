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

/// Guards an entire transaction against an already initialized primary-key identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageRecordIdentity {
    pub collection: String,
    pub key: String,
}

/// Selects a database's permanently recorded data contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageKind {
    #[serde(rename = "sqlite")]
    Sqlite,
    #[serde(rename = "objects")]
    Objects,
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

/// Guards a related record without rewriting it or publishing an extra change.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageRecordGuard {
    pub collection: String,
    pub key: String,
    #[serde(rename = "expectedVersion")]
    pub expected_version: Option<String>,
}

/// Identifies the last observed record state without exposing journal positions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageRecordWatchState {
    pub version: Option<String>,
}

/// Dispatches storage operations only through an authenticated engine-owned handle.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum StorageRequest {
    #[serde(rename = "open")]
    Open {
        path: String,
        kind: StorageKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        merge: Option<StorageMergeDescriptor>,
    },
    #[serde(rename = "open_preferences")]
    OpenPreferences {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        merge: Option<StoragePreferencesMergeDescriptor>,
    },
    #[serde(rename = "preferences_data")]
    PreferencesData { handle: String },
    #[serde(rename = "preferences_begin_edit")]
    PreferencesBeginEdit { handle: String },
    #[serde(rename = "preferences_finish_edit")]
    PreferencesFinishEdit {
        handle: String,
        token: String,
        edits: Vec<StoragePreferenceEdit>,
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
    },
    #[serde(rename = "preferences_cancel_edit")]
    PreferencesCancelEdit { handle: String, token: String },
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
        #[serde(default, rename = "whenAbsent")]
        when_absent: Option<StorageRecordIdentity>,
    },
    #[serde(rename = "define_table")]
    DefineTable { handle: String, table: StorageTable },
    #[serde(rename = "get")]
    Get {
        handle: String,
        collection: String,
        key: String,
    },
    #[serde(rename = "get_many")]
    GetMany {
        handle: String,
        collection: String,
        keys: Vec<String>,
    },
    #[serde(rename = "list")]
    List {
        handle: String,
        collection: String,
        after: Option<String>,
        limit: u32,
    },
    /// Reads committed object changes after an opaque local cursor; null captures the current head.
    #[serde(rename = "changes")]
    Changes { handle: String, after: Option<String>, limit: u32 },
    #[serde(rename = "commit")]
    Commit {
        handle: String,
        mutations: Vec<StorageMutation>,
        #[serde(default)]
        guards: Vec<StorageRecordGuard>,
    },
}

/// Identifies one successfully opened database session.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageOpenResult {
    pub handle: String,
    pub path: String,
}

/// Selects the bounded key encoding used by a declarative space merge contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageMergeKeyEncoding {
    #[serde(rename = "length_prefixed_root")]
    LengthPrefixedRoot,
    #[serde(rename = "slash_root")]
    SlashRoot,
}

/// Selects whether an entity is matched by content or by its declared identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageMergeMatchMode {
    #[serde(rename = "content")]
    Content,
    #[serde(rename = "identity")]
    Identity,
}

/// Describes a key prefix that points to another declared entity identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageMergeKeyReference {
    pub target: StorageMergeTarget,
    #[serde(default)]
    pub prefix: String,
}

/// Selects the representation used around a referenced identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageMergeReferenceEncoding {
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "uri_component")]
    UriComponent,
}

/// Selects a core-owned namespace whose identifiers may be rewritten during a space merge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields, rename_all = "snake_case")]
pub enum StorageMergeTarget {
    #[serde(rename = "collection")]
    Collection { collection: String },
    #[serde(rename = "preference")]
    Preference { path: String },
}

/// Declares one explicit scalar or child-row identifier reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageMergeReference {
    pub collection: String,
    pub column: String,
    pub target: StorageMergeTarget,
    #[serde(rename = "keyPrefix", default)]
    pub key_prefix: String,
    #[serde(rename = "keySuffix", default)]
    pub key_suffix: String,
    #[serde(rename = "valueEncoding", default = "defaultStorageMergeReferenceEncoding")]
    pub value_encoding: StorageMergeReferenceEncoding,
}

/// Supplies the content-based default for descriptors released before match modes existed.
fn defaultStorageMergeMatchMode() -> StorageMergeMatchMode {
    StorageMergeMatchMode::Content
}

/// Supplies direct identifier encoding for descriptors released before wrapped references existed.
fn defaultStorageMergeReferenceEncoding() -> StorageMergeReferenceEncoding {
    StorageMergeReferenceEncoding::Direct
}

/// Declares one root entity whose UUID, display name and references are core-mergeable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageMergeEntity {
    pub collection: String,
    #[serde(rename = "identityColumn")]
    pub identity_column: String,
    #[serde(rename = "nameColumn")]
    pub name_column: Option<String>,
    #[serde(rename = "ignoredColumns")]
    pub ignored_columns: Vec<String>,
    #[serde(rename = "keyEncoding")]
    pub key_encoding: StorageMergeKeyEncoding,
    #[serde(rename = "matchMode", default = "defaultStorageMergeMatchMode")]
    pub match_mode: StorageMergeMatchMode,
    #[serde(rename = "keyReferences", default)]
    pub key_references: Vec<StorageMergeKeyReference>,
    #[serde(rename = "childCollections", default)]
    pub child_collections: Vec<String>,
    pub references: Vec<StorageMergeReference>,
}

/// Declares a bounded, data-only merge contract verified and executed by Core.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageMergeDescriptor {
    pub version: u32,
    #[serde(rename = "maxEntities")]
    pub max_entities: u32,
    #[serde(rename = "maxReferences")]
    pub max_references: u32,
    pub entities: Vec<StorageMergeEntity>,
}

/// Preserves the exact column order and tagged SQLite values of one query row.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageSqlRow {
    pub columns: Vec<String>,
    pub values: Vec<StorageSqlValue>,
}

/// Carries the current value (or deletion) for one changed object identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageObjectChange {
    pub collection: String,
    pub key: String,
    pub entry: Option<StorageValueEntry>,
}

/// Pages a bounded local change journal, including synchronization and conflict-merge writes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageChangePage {
    pub cursor: String,
    pub reset: bool,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    pub changes: Vec<StorageObjectChange>,
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

/// Declares one complete-record catalog using the native preference merge executor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StoragePreferencesMergeCatalog {
    #[serde(rename = "keyPrefix")]
    pub key_prefix: String,
    #[serde(rename = "keySuffix")]
    pub key_suffix: String,
    #[serde(rename = "identityField")]
    pub identity_field: String,
    #[serde(rename = "nameField")]
    pub name_field: String,
    #[serde(rename = "ignoredFields")]
    pub ignored_fields: Vec<String>,
    #[serde(rename = "retiredKeys")]
    pub retired_keys: Vec<String>,
}

/// Selects an exact preference or complete records with explicit key boundaries.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields, rename_all = "snake_case")]
pub enum StoragePreferencesMergeKey {
    #[serde(rename = "exact")]
    Exact { key: String },
    #[serde(rename = "records")]
    Records { prefix: String, suffix: String },
}

/// Selects nested reference fields without interpreting application-specific JSON.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields, rename_all = "snake_case")]
pub enum StoragePreferencesMergePathSegment {
    #[serde(rename = "field")]
    Field { name: String },
    #[serde(rename = "object_values")]
    ObjectValues,
    #[serde(rename = "array_values")]
    ArrayValues,
    #[serde(rename = "variant")]
    Variant { name: String },
    #[serde(rename = "identity")]
    Identity { prefix: String, suffix: String },
}

/// Declares a native preference reference targeting an explicit catalog file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoragePreferencesMergeReference {
    pub key: StoragePreferencesMergeKey,
    pub path: Vec<StoragePreferencesMergePathSegment>,
    pub target: String,
}

/// Projects selected entity fields through the native preference merge executor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StoragePreferencesMergeProjection {
    #[serde(rename = "selectionKey")]
    pub selection_key: String,
    #[serde(rename = "sourceField")]
    pub source_field: String,
    pub fields: std::collections::BTreeMap<String, String>,
    #[serde(rename = "retiredKeys")]
    pub retired_keys: Vec<String>,
}

/// Carries the native merge capabilities as bounded data alongside preference journal entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoragePreferencesMergeDescriptor {
    pub version: u32,
    pub catalog: Option<StoragePreferencesMergeCatalog>,
    pub references: Vec<StoragePreferencesMergeReference>,
    pub projection: Option<StoragePreferencesMergeProjection>,
}

/// Identifies immutable merge declarations in the native preference journal.
pub const PLUGIN_MERGE_DECLARATION_KEY: &str = "__operit_preferences_merge";

impl StoragePreferencesMergeKey {
    /// Matches only the explicitly declared preference key namespace.
    pub fn matches(&self, candidate: &str) -> bool {
        match self {
            Self::Exact { key } => candidate == key,
            Self::Records { prefix, suffix } => candidate.strip_prefix(prefix.as_str())
                .and_then(|key| key.strip_suffix(suffix.as_str()))
                .is_some_and(|identity| !identity.is_empty()),
        }
    }
}

/// Preserves native preference edit intentions across the callback transport boundary.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", deny_unknown_fields, rename_all = "snake_case")]
pub enum StoragePreferenceEdit {
    #[serde(rename = "set")]
    Set { key: String, value: Value },
    #[serde(rename = "remove")]
    Remove { key: String },
    #[serde(rename = "increment_integer")]
    IncrementInteger { key: String, delta: i64 },
    #[serde(rename = "set_object_field")]
    SetObjectField { key: String, field: String, value: Value },
}

/// Carries a complete typed preference snapshot independently of object collection versions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StoragePreferencesSnapshot {
    pub values: std::collections::BTreeMap<String, Value>,
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
}

/// Identifies a single callback invocation whose publication must match its original snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoragePreferencesEdit {
    pub token: String,
    pub snapshot: StoragePreferencesSnapshot,
}

/// Returns each storage operation's concrete business result.
/// Serialization preserves the database API payload; tool results add their own operation discriminator.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum StorageResponse {
    Open(StorageOpenResult),
    PreferencesData(StoragePreferencesSnapshot),
    PreferencesBeginEdit(StoragePreferencesEdit),
    PreferencesFinishEdit(StoragePreferencesSnapshot),
    PreferencesCancelEdit,
    Close,
    Query(Vec<StorageSqlRow>),
    Execute(Vec<StorageSqlExecutionResult>),
    Transaction(Vec<StorageSqlExecutionResult>),
    DefineTable(StorageTableDefinitionResult),
    Get(Option<StorageValueEntry>),
    GetMany(Vec<Option<StorageValueEntry>>),
    List(Vec<StorageKeyValueEntry>),
    Commit(StorageCommitResult),
    Changes(StorageChangePage),
}
