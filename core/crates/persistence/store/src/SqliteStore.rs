use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use operit_host_api::{HostError, RuntimeSqliteConnection, RuntimeSqliteTransaction};
use thiserror::Error;

use crate::RuntimeStorageHost::{defaultRuntimeSqliteHost, runtimeStoragePath};

pub use operit_host_api::{SqliteRow, SqliteValue};

#[derive(Debug, Error)]
/// Error type for the runtime SQLite storage adapter.
pub enum SqliteStoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("host error: {0}")]
    Host(#[from] HostError),
    #[error("sqlite connection mutex poisoned")]
    MutexPoisoned,
    #[error("sqlite invalidation observer mutex poisoned")]
    ObserverMutexPoisoned,
    #[error("transaction failed: {action}; explicit rollback failed: {rollback}")]
    TransactionRollback {
        action: Box<SqliteStoreError>,
        rollback: HostError,
    },
    #[error("{0}")]
    Message(String),
}

#[derive(Clone)]
/// Shared SQLite connection wrapper backed by the registered runtime host.
pub struct SqliteStore {
    path: PathBuf,
    connection: Arc<Mutex<Box<dyn RuntimeSqliteConnection>>>,
    observers: Arc<Mutex<Vec<Arc<dyn Fn() -> Result<(), SqliteStoreError> + Send + Sync>>>>,
    pub(crate) executionLeases: Arc<Mutex<crate::ChatExecutionLease::ChatExecutionLeaseState>>,
}

impl SqliteStore {
    /// Opens a runtime-hosted SQLite database and enables foreign keys.
    pub fn open(path: PathBuf) -> Result<Self, SqliteStoreError> {
        let storagePath = runtimeStoragePath(&path);
        let mut store = Self::openWithHost(defaultRuntimeSqliteHost(), &storagePath)?;
        store.path = path;
        Ok(store)
    }

    /// Opens an explicitly validated host path without platform-specific filesystem access.
    pub fn openWithHost(host: Arc<dyn operit_host_api::RuntimeSqliteHost>, storagePath: &str) -> Result<Self, SqliteStoreError> {
        let mut connection = host.openSqliteDatabase(storagePath)?;
        connection.execute("PRAGMA foreign_keys = ON", Vec::new())?;
        Ok(Self {
            path: PathBuf::from(storagePath),
            connection: Arc::new(Mutex::new(connection)),
            observers: Arc::new(Mutex::new(Vec::new())),
            executionLeases: Arc::new(Mutex::new(
                crate::ChatExecutionLease::ChatExecutionLeaseState::default(),
            )),
        })
    }

    /// Returns the logical database path used to open this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[allow(non_snake_case)]
    /// Executes a SQL batch against the store connection.
    pub fn executeBatch(&self, sql: &str) -> Result<(), SqliteStoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SqliteStoreError::MutexPoisoned)?;
        connection.executeBatch(sql)?;
        Ok(())
    }

    /// Executes a single SQL statement and returns the affected row count.
    pub fn execute(&self, sql: &str, params: Vec<SqliteValue>) -> Result<usize, SqliteStoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SqliteStoreError::MutexPoisoned)?;
        Ok(connection.execute(sql, params)?)
    }

    /// Runs a query and returns all rows.
    pub fn queryRows(
        &self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<Vec<SqliteRow>, SqliteStoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SqliteStoreError::MutexPoisoned)?;
        Ok(connection.query(sql, params)?)
    }

    /// Runs a query and returns the first row, if one exists.
    pub fn queryOne(
        &self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<Option<SqliteRow>, SqliteStoreError> {
        let mut rows = self.queryRows(sql, params)?;
        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(rows.remove(0)))
        }
    }

    /// Runs a query and converts the first column of the first row.
    pub fn queryScalar<T: FromSqliteValue>(
        &self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<T, SqliteStoreError> {
        let row = self.queryOne(sql, params)?.ok_or_else(|| {
            SqliteStoreError::Message("sqlite query returned no rows".to_string())
        })?;
        row.get(0)
    }

    #[allow(non_snake_case)]
    /// Reads SQLite's `PRAGMA user_version` value.
    pub fn getUserVersion(&self) -> Result<i32, SqliteStoreError> {
        self.queryScalar("PRAGMA user_version", Vec::new())
    }

    #[allow(non_snake_case)]
    /// Writes SQLite's `PRAGMA user_version` value.
    pub fn setUserVersion(&self, version: i32) -> Result<(), SqliteStoreError> {
        self.execute(&format!("PRAGMA user_version = {version}"), Vec::new())?;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Checks whether a table with the supplied name exists.
    pub fn tableExists(&self, tableName: &str) -> Result<bool, SqliteStoreError> {
        let found: Option<i32> = self
            .queryOne(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
                vec![toSqliteValue(tableName)],
            )?
            .map(|row| row.get(0))
            .transpose()?;
        Ok(found.is_some())
    }

    #[allow(non_snake_case)]
    /// Registers a callback invoked by `notifyInvalidated`.
    pub fn addInvalidationObserver<F>(&self, observer: F) -> Result<(), SqliteStoreError>
    where
        F: Fn() -> Result<(), SqliteStoreError> + Send + Sync + 'static,
    {
        let mut observers = self
            .observers
            .lock()
            .map_err(|_| SqliteStoreError::ObserverMutexPoisoned)?;
        observers.push(Arc::new(observer));
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Notifies all registered invalidation observers.
    pub fn notifyInvalidated(&self) -> Result<(), SqliteStoreError> {
        let observers = self
            .observers
            .lock()
            .map_err(|_| SqliteStoreError::ObserverMutexPoisoned)?
            .clone();
        for observer in observers {
            observer()?;
        }
        Ok(())
    }

    /// Commits successful actions and explicitly rolls back failed actions on every runtime host.
    pub fn transaction<T, F>(&self, action: F) -> Result<T, SqliteStoreError>
    where
        F: FnOnce(&mut SqliteTransaction<'_>) -> Result<T, SqliteStoreError>,
    {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SqliteStoreError::MutexPoisoned)?;
        let transaction = connection.beginTransaction()?;
        let mut transaction = SqliteTransaction { inner: transaction };
        match action(&mut transaction) {
            Ok(result) => {
                transaction.inner.commit()?;
                Ok(result)
            }
            Err(action) => match transaction.inner.rollback() {
                Ok(()) => Err(action),
                Err(rollback) => Err(SqliteStoreError::TransactionRollback {
                    action: Box::new(action),
                    rollback,
                }),
            },
        }
    }
}

/// Active SQLite transaction wrapper exposed to store migrations and writes.
pub struct SqliteTransaction<'a> {
    inner: Box<dyn RuntimeSqliteTransaction + 'a>,
}

impl SqliteTransaction<'_> {
    /// Executes a statement within this transaction.
    pub fn execute(
        &mut self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<usize, SqliteStoreError> {
        Ok(self.inner.execute(sql, params)?)
    }

    /// Runs a query within this transaction and returns all rows.
    pub fn queryRows(
        &mut self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<Vec<SqliteRow>, SqliteStoreError> {
        Ok(self.inner.query(sql, params)?)
    }

    /// Runs a query within this transaction and returns the first row, if present.
    pub fn queryOne(
        &mut self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<Option<SqliteRow>, SqliteStoreError> {
        let mut rows = self.queryRows(sql, params)?;
        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(rows.remove(0)))
        }
    }

    #[allow(non_snake_case)]
    /// Returns the last inserted row id for this transaction.
    pub fn lastInsertRowId(&self) -> Result<i64, SqliteStoreError> {
        Ok(self.inner.lastInsertRowId()?)
    }
}

/// Typed row accessor for host-provided SQLite rows.
pub trait SqliteRowGet {
    /// Reads and converts a column by index or name.
    fn get<K, T>(&self, key: K) -> Result<T, SqliteStoreError>
    where
        K: SqliteColumnKey,
        T: FromSqliteValue;
}

impl SqliteRowGet for SqliteRow {
    fn get<K, T>(&self, key: K) -> Result<T, SqliteStoreError>
    where
        K: SqliteColumnKey,
        T: FromSqliteValue,
    {
        T::fromSqliteValue(key.value(self)?)
    }
}

/// Key that can resolve a value from a SQLite row.
pub trait SqliteColumnKey {
    /// Returns the SQLite value addressed by this key.
    fn value<'a>(&self, row: &'a SqliteRow) -> Result<&'a SqliteValue, SqliteStoreError>;
}

impl SqliteColumnKey for usize {
    fn value<'a>(&self, row: &'a SqliteRow) -> Result<&'a SqliteValue, SqliteStoreError> {
        Ok(row.valueAt(*self)?)
    }
}

impl SqliteColumnKey for &str {
    fn value<'a>(&self, row: &'a SqliteRow) -> Result<&'a SqliteValue, SqliteStoreError> {
        Ok(row.valueNamed(self)?)
    }
}

impl SqliteColumnKey for String {
    fn value<'a>(&self, row: &'a SqliteRow) -> Result<&'a SqliteValue, SqliteStoreError> {
        Ok(row.valueNamed(self)?)
    }
}

/// Converts host SQLite values into strongly typed Rust values.
pub trait FromSqliteValue: Sized {
    #[allow(non_snake_case)]
    /// Converts one SQLite value into this Rust type.
    fn fromSqliteValue(value: &SqliteValue) -> Result<Self, SqliteStoreError>;
}

impl FromSqliteValue for String {
    fn fromSqliteValue(value: &SqliteValue) -> Result<Self, SqliteStoreError> {
        Ok(value.asString()?)
    }
}

impl FromSqliteValue for i64 {
    fn fromSqliteValue(value: &SqliteValue) -> Result<Self, SqliteStoreError> {
        Ok(value.asI64()?)
    }
}

impl FromSqliteValue for f64 {
    fn fromSqliteValue(value: &SqliteValue) -> Result<Self, SqliteStoreError> {
        Ok(value.asF64()?)
    }
}

impl FromSqliteValue for i32 {
    fn fromSqliteValue(value: &SqliteValue) -> Result<Self, SqliteStoreError> {
        Ok(value.asI64()? as i32)
    }
}

impl FromSqliteValue for usize {
    fn fromSqliteValue(value: &SqliteValue) -> Result<Self, SqliteStoreError> {
        Ok(value.asI64()? as usize)
    }
}

impl FromSqliteValue for bool {
    fn fromSqliteValue(value: &SqliteValue) -> Result<Self, SqliteStoreError> {
        Ok(value.asI64()? != 0)
    }
}

impl<T: FromSqliteValue> FromSqliteValue for Option<T> {
    fn fromSqliteValue(value: &SqliteValue) -> Result<Self, SqliteStoreError> {
        if value.isNull() {
            Ok(None)
        } else {
            Ok(Some(T::fromSqliteValue(value)?))
        }
    }
}

/// Converts Rust values into host SQLite values.
pub trait ToSqliteValue {
    #[allow(non_snake_case)]
    /// Converts this Rust value into a SQLite value.
    fn toSqliteValue(&self) -> SqliteValue;
}

impl<T: ToSqliteValue + ?Sized> ToSqliteValue for &T {
    fn toSqliteValue(&self) -> SqliteValue {
        (*self).toSqliteValue()
    }
}

impl<T: ToSqliteValue> ToSqliteValue for Option<T> {
    fn toSqliteValue(&self) -> SqliteValue {
        match self {
            Some(value) => value.toSqliteValue(),
            None => SqliteValue::Null,
        }
    }
}

impl ToSqliteValue for str {
    fn toSqliteValue(&self) -> SqliteValue {
        SqliteValue::Text(self.to_string())
    }
}

impl ToSqliteValue for String {
    fn toSqliteValue(&self) -> SqliteValue {
        SqliteValue::Text(self.clone())
    }
}

impl ToSqliteValue for i64 {
    fn toSqliteValue(&self) -> SqliteValue {
        SqliteValue::Integer(*self)
    }
}

impl ToSqliteValue for f64 {
    fn toSqliteValue(&self) -> SqliteValue {
        SqliteValue::Real(*self)
    }
}

impl ToSqliteValue for i32 {
    fn toSqliteValue(&self) -> SqliteValue {
        SqliteValue::Integer(*self as i64)
    }
}

impl ToSqliteValue for usize {
    fn toSqliteValue(&self) -> SqliteValue {
        SqliteValue::Integer(*self as i64)
    }
}

impl ToSqliteValue for bool {
    fn toSqliteValue(&self) -> SqliteValue {
        SqliteValue::Integer(if *self { 1 } else { 0 })
    }
}

#[allow(non_snake_case)]
/// Converts a supported Rust value into a host SQLite value.
pub fn toSqliteValue<T: ToSqliteValue + ?Sized>(value: &T) -> SqliteValue {
    value.toSqliteValue()
}

#[macro_export]
macro_rules! sqliteParams {
    () => {
        Vec::<operit_host_api::SqliteValue>::new()
    };
    ($($value:expr),+ $(,)?) => {
        vec![$($crate::SqliteStore::toSqliteValue(&$value)),+]
    };
}

#[cfg(test)]
mod transaction_lifecycle_tests {
    use super::*;
    use operit_host_api::HostResult;

    /// Records host lifecycle calls without implementing destructor rollback.
    #[derive(Default)]
    struct LifecycleState {
        commits: usize,
        rollbacks: usize,
        failRollback: bool,
    }

    /// Supplies a transaction whose only cleanup path is the explicit host method.
    struct LifecycleConnection {
        state: Arc<Mutex<LifecycleState>>,
    }

    impl RuntimeSqliteConnection for LifecycleConnection {
        /// Rejects batch statements outside the lifecycle under test.
        fn executeBatch(&mut self, _: &str) -> HostResult<()> {
            Err(HostError::new("Unexpected lifecycle batch"))
        }

        /// Rejects nontransactional statements in this lifecycle test.
        fn execute(&mut self, _: &str, _: Vec<SqliteValue>) -> HostResult<usize> {
            Err(HostError::new("Unexpected lifecycle execute"))
        }

        /// Rejects connection queries that are unrelated to transaction cleanup.
        fn query(&mut self, _: &str, _: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
            Err(HostError::new("Unexpected lifecycle query"))
        }

        /// Rejects row-id lookups that are unrelated to transaction cleanup.
        fn lastInsertRowId(&self) -> HostResult<i64> {
            Err(HostError::new("Unexpected lifecycle row id"))
        }

        /// Returns a host transaction with recorded explicit terminal calls.
        fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>> {
            Ok(Box::new(LifecycleTransaction {
                state: self.state.clone(),
            }))
        }
    }

    /// Uses explicit lifecycle counters rather than RAII to detect missing rollback calls.
    struct LifecycleTransaction {
        state: Arc<Mutex<LifecycleState>>,
    }

    impl RuntimeSqliteTransaction for LifecycleTransaction {
        /// Reports a deterministic statement error without rolling anything back implicitly.
        fn execute(&mut self, _: &str, _: Vec<SqliteValue>) -> HostResult<usize> {
            Err(HostError::new("Injected action SQL failure"))
        }

        /// Rejects queries that are not needed by the lifecycle test.
        fn query(&mut self, _: &str, _: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
            Err(HostError::new("Unexpected lifecycle transaction query"))
        }

        /// Rejects row-id lookups that are not needed by the lifecycle test.
        fn lastInsertRowId(&self) -> HostResult<i64> {
            Err(HostError::new("Unexpected lifecycle transaction row id"))
        }

        /// Records the sole successful commit terminal operation.
        fn commit(self: Box<Self>) -> HostResult<()> {
            self.state.lock().unwrap().commits += 1;
            Ok(())
        }

        /// Records explicit rollback and optionally reports the independent injected cleanup failure.
        fn rollback(self: Box<Self>) -> HostResult<()> {
            let mut state = self.state.lock().unwrap();
            state.rollbacks += 1;
            if state.failRollback {
                return Err(HostError::new("Injected host rollback failure"));
            }
            Ok(())
        }
    }

    /// Builds a store around the lifecycle-only host so destructor cleanup cannot make the test pass.
    fn lifecycleStore(failRollback: bool) -> (SqliteStore, Arc<Mutex<LifecycleState>>) {
        let state = Arc::new(Mutex::new(LifecycleState {
            failRollback,
            ..LifecycleState::default()
        }));
        let store = SqliteStore {
            path: PathBuf::from("transaction-lifecycle-test"),
            connection: Arc::new(Mutex::new(Box::new(LifecycleConnection {
                state: state.clone(),
            }))),
            observers: Arc::new(Mutex::new(Vec::new())),
            executionLeases: Arc::new(Mutex::new(
                crate::ChatExecutionLease::ChatExecutionLeaseState::default(),
            )),
        };
        (store, state)
    }

    /// Requires exactly one explicit rollback and preserves the original failed statement error.
    #[test]
    fn failed_action_rolls_back_explicitly_without_destructor_cleanup() {
        let (store, state) = lifecycleStore(false);
        let error = store
            .transaction::<(), _>(|transaction| {
                transaction.execute("injected failure", Vec::new())?;
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "host error: Injected action SQL failure");
        let state = state.lock().unwrap();
        assert_eq!((state.commits, state.rollbacks), (0, 1));
    }

    /// Reports both the action failure and explicit rollback failure instead of losing either error.
    #[test]
    fn rollback_failure_preserves_both_action_and_host_errors() {
        let (store, state) = lifecycleStore(true);
        let error = store
            .transaction::<(), _>(|transaction| {
                transaction.execute("injected failure", Vec::new())?;
                Ok(())
            })
            .unwrap_err();
        match error {
            SqliteStoreError::TransactionRollback { action, rollback } => {
                assert_eq!(
                    action.to_string(),
                    "host error: Injected action SQL failure"
                );
                assert_eq!(rollback.to_string(), "Injected host rollback failure");
            }
            other => panic!("Expected both transaction errors, got {other}"),
        }
        let state = state.lock().unwrap();
        assert_eq!((state.commits, state.rollbacks), (0, 1));
    }

    /// Requires a successful action to commit exactly once without calling rollback.
    #[test]
    fn successful_action_commits_once() {
        let (store, state) = lifecycleStore(false);
        assert_eq!(store.transaction(|_| Ok(7)).unwrap(), 7);
        let state = state.lock().unwrap();
        assert_eq!((state.commits, state.rollbacks), (1, 0));
    }
}
