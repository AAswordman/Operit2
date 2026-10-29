//! Canonical Rust declarations for plugin-owned database facades.
use super::core::AbortSignal;
use super::storage::{StorageMergeDescriptor, StorageRequest, StorageTable};
use super::{JsAny, JsAsyncIterableIterator, JsFuture, JsNullable};
use std::collections::BTreeMap;

/// Represents a structured value stored by the object database.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum StorageValue {
    Null,
    Boolean(bool),
    Number(f64),
    Text(String),
    Array(Vec<StorageValue>),
    Object(StorageValueObject),
}
/// Stores recursive structured properties as a named object declaration.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StorageValueObject {
    #[serde(flatten)]
    pub additional_properties: BTreeMap<String, StorageValue>,
}
/// Preserves native JavaScript SQLite values.
pub enum StorageFacadeSqlValue {
    Null,
    Text(String),
    Real(f64),
    Integer(super::JsBigInt),
    Blob(super::JsUint8Array),
}
/// Selects a plugin-owned path and its merge contract.
pub struct StorageOpenOptions {
    pub path: String,
    pub merge: Option<StorageMergeDescriptor>,
}
/// Selects the exact previous record version, including explicit absence.
pub struct StorageVersionOptions {
    pub expectedVersion: super::JsOptional<String>,
}
/// Reports the committed logical version.
pub struct StorageFacadeReceipt {
    pub version: String,
}
/// Carries one decoded record and its exact version.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StorageEntry<T = StorageValue> {
    pub value: T,
    pub version: String,
}
/// Carries one primary-key-ordered record.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StorageListedEntry<T = StorageValue> {
    #[serde(flatten)]
    pub entry: StorageEntry<T>,
    pub key: String,
}
/// Cancels a pending storage read when the owning collector is released.
pub struct StorageWatchOptions {
    pub signal: Option<AbortSignal>,
}
/// Owns a database handle within one authenticated engine.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StorageHandle {
    pub path: String,
}
pub trait StorageHandleMethods: Send + Sync {
    /// Releases the current execution handle without deleting durable data.
    fn close(&self) -> JsFuture<()>;
}
/// Selects one parameterized SQL write.
pub struct StorageFacadeStatement {
    pub sql: String,
    pub params: Option<Vec<StorageFacadeSqlValue>>,
    pub expectedAffectedRows: Option<u32>,
}
/// Reports the result of one committed SQL statement.
pub struct StorageSqlReceipt {
    pub affectedRows: u32,
    pub lastInsertId: String,
}
/// Selects one collection record identity.
pub struct StorageFacadeWhenAbsent {
    pub collection: String,
    pub key: String,
}
/// Guards a SQL batch against an existing record.
pub struct StorageTransactionOptions {
    pub whenAbsent: StorageFacadeWhenAbsent,
}
/// Owns a parameterized SQL database.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StorageSqliteDatabase {
    #[serde(flatten)]
    pub handle: StorageHandle,
}
pub trait StorageSqliteDatabaseMethods: Send + Sync {
    /// Queries caller-selected columns while preserving integer precision.
    fn query(
        &self,
        sql: String,
        params: Option<Vec<StorageFacadeSqlValue>>,
    ) -> JsFuture<Vec<BTreeMap<String, StorageFacadeSqlValue>>>;
    /// Commits a parameterized statement and its change journal atomically.
    fn execute(
        &self,
        sql: String,
        params: Option<Vec<StorageFacadeSqlValue>>,
    ) -> JsFuture<StorageSqlReceipt>;
    /// Commits a bounded SQL transaction with an optional absence guard.
    fn transaction(
        &self,
        statements: Vec<StorageFacadeStatement>,
        options: Option<StorageTransactionOptions>,
    ) -> JsFuture<Vec<StorageSqlReceipt>>;
    /// Declares a tracked table with a nonnullable primary key.
    fn defineTable(&self, table: StorageTable) -> JsFuture<StorageDefineTableReceipt>;
}
/// Reports whether a tracked table was newly defined.
pub struct StorageDefineTableReceipt {
    pub changed: bool,
}
/// Retains a mutable object with its original exact version.
pub struct StorageEditor<T = StorageValue> {
    pub value: T,
}
pub trait StorageEditorMethods<T>: Send + Sync {
    /// Atomically flushes tracked edits against the original version.
    fn flush(&self) -> JsFuture<StorageFacadeReceipt>;
}
/// Selects a collection within the owned database.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StorageCollection<T = StorageValue> {
    marker: std::marker::PhantomData<T>,
}
pub trait StorageCollectionMethods<T>: Send + Sync {
    /// Reads a record without conflating a stored null with an absent key.
    fn get(&self, key: String) -> JsFuture<JsNullable<StorageEntry<T>>>;
    /// Reads ordered keys, retaining missing entries, in one host transaction snapshot.
    fn getMany(&self, keys: Vec<String>) -> JsFuture<Vec<JsNullable<StorageEntry<T>>>>;
    /// Emits the current record or absence and subsequent changes to this exact key.
    fn watch(
        &self,
        key: String,
        options: Option<StorageWatchOptions>,
    ) -> JsAsyncIterableIterator<JsNullable<StorageEntry<T>>>;
    /// Lists primary-key-ordered records with a bounded page size.
    fn list(
        &self,
        after: Option<JsNullable<String>>,
        limit: Option<u32>,
    ) -> JsFuture<Vec<StorageListedEntry<T>>>;
    /// Saves one structured record with an optional version precondition.
    fn put(
        &self,
        key: String,
        value: T,
        options: Option<StorageVersionOptions>,
    ) -> JsFuture<StorageFacadeReceipt>;
    /// Removes one record with an optional version precondition.
    fn remove(
        &self,
        key: String,
        options: Option<StorageVersionOptions>,
    ) -> JsFuture<StorageFacadeReceipt>;
    /// Opens an existing object for tracked edits.
    fn edit(&self, key: String) -> JsFuture<StorageEditor<T>>;
}
/// Describes a bounded atomic object mutation.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "op")]
pub enum StorageObjectMutation {
    #[serde(rename = "put")]
    Put {
        collection: String,
        key: String,
        value: StorageValue,
        expectedVersion: super::JsOptional<String>,
    },
    #[serde(rename = "delete")]
    Delete {
        collection: String,
        key: String,
        expectedVersion: super::JsOptional<String>,
    },
}
/// Owns an object database with exact-key collection subscriptions.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StorageObjectDatabase {
    #[serde(flatten)]
    pub handle: StorageHandle,
}
pub trait StorageObjectDatabaseMethods: Send + Sync {
    /// Captures the current head with null, or reads committed changes after an opaque cursor.
    fn changes(&self, after: super::JsNullable<String>, limit: Option<u32>) -> JsFuture<super::storage::StorageChangePage>;

    /// Queries the read-only plugin_objects view through the shared SQLite host.
    fn query(
        &self,
        sql: String,
        params: Option<Vec<StorageFacadeSqlValue>>,
    ) -> JsFuture<Vec<BTreeMap<String, StorageFacadeSqlValue>>>;
    /// Selects a collection on this retained database handle.
    fn collection<T>(&self, name: String) -> StorageCollection<T>;
    /// Commits bounded mutations across collections atomically.
    fn commit(&self, mutations: Vec<StorageObjectMutation>, guards: Option<Vec<super::storage::StorageRecordGuard>>) -> JsFuture<StorageFacadeReceipt>;
}
/// Selects one native JSON preference file.
pub struct StorageDataStoreOpenOptions {
    pub path: String,
    pub merge: Option<super::storage::StoragePreferencesMergeDescriptor>,
}
/// Exposes a complete immutable preference snapshot.
pub struct StoragePreferences {}
pub trait StoragePreferencesMethods: Send + Sync {
    /// Reads one typed value, preserving absence independently of a stored null.
    fn get<T>(&self, key: String) -> super::JsUndefined<T>;
    /// Reports whether a key is present, including a stored null value.
    fn contains(&self, key: String) -> bool;
    /// Enumerates all application preference entries from this snapshot.
    fn entries(&self) -> Vec<(String, StorageValue)>;
}
/// Exposes the mutable snapshot scoped to one edit or schema callback.
pub struct StorageMutablePreferences {}
pub trait StorageMutablePreferencesMethods: Send + Sync {
    /// Reads one typed value from the current edit snapshot.
    fn get<T>(&self, key: String) -> super::JsUndefined<T>;
    /// Reports whether the edit snapshot contains this key.
    fn contains(&self, key: String) -> bool;
    /// Enumerates the edit snapshot's application preference entries.
    fn entries(&self) -> Vec<(String, StorageValue)>;
    /// Writes one typed JSON value into the current transaction.
    fn set<T>(&self, key: String, value: T);
    /// Removes one key from the current transaction.
    fn remove(&self, key: String);
    /// Records an integer delta using native transaction semantics across synchronization.
    fn incrementInteger(&self, key: String, delta: i64);
    /// Updates one object field without overwriting concurrent edits to other fields.
    fn setObjectField<T>(&self, key: String, field: String, value: T);
}
/// Accepts synchronous completion or an awaited asynchronous callback.
pub enum StoragePreferencesCallbackResult {
    Complete(()),
    Pending(JsFuture<()>),
}
/// Owns a native JSON preference file with Rust-style snapshot and edit semantics.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct StorageDataStore {
    #[serde(flatten)]
    pub handle: StorageHandle,
}
pub trait StorageDataStoreMethods: Send + Sync {
    /// Reads the complete immutable snapshot after applying the declared schema.
    fn data(&self) -> JsFuture<StoragePreferences>;
    /// Publishes one callback atomically; conflicts reject without rerunning the callback.
    fn edit(&self, transform: std::sync::Arc<dyn Fn(StorageMutablePreferences) -> StoragePreferencesCallbackResult + Send + Sync>) -> JsFuture<StoragePreferences>;
    /// Declares ordered one-version upgrades before subsequent reads or edits.
    fn withSchema(&self, currentVersion: u32, migrate: std::sync::Arc<dyn Fn(u32, StorageMutablePreferences) -> StoragePreferencesCallbackResult + Send + Sync>) -> StorageDataStore;
    /// Reads the declared store's current schema version after applying pending upgrades.
    fn schemaVersion(&self) -> JsFuture<u32>;
    /// Observes complete snapshots with collector-owned cancellation.
    fn dataFlow(&self, options: Option<StorageWatchOptions>) -> JsAsyncIterableIterator<StoragePreferences>;
}
/// Opens SQLite at an explicit plugin-owned path.
pub trait StorageSqliteHost: Send + Sync {
    /// Opens a parameterized SQLite database.
    fn open(&self, options: StorageOpenOptions) -> JsFuture<StorageSqliteDatabase>;
}
/// Opens generic object collections.
pub trait StorageObjectsHost: Send + Sync {
    /// Opens a transactional object database.
    fn open(&self, options: StorageOpenOptions) -> JsFuture<StorageObjectDatabase>;
}
/// Opens native typed JSON preferences through the host abstraction.
pub trait StorageDataStoreHost: Send + Sync {
    /// Opens a native preference file without a SQLite object database.
    fn open(&self, options: StorageDataStoreOpenOptions) -> JsFuture<StorageDataStore>;
}
/// Dispatches the exact wire contract through the authenticated host.
pub trait StorageFacadeHost: Send + Sync {
    /// Dispatches one typed wire request.
    fn request(&self, request: StorageRequest) -> JsFuture<JsAny>;
}
