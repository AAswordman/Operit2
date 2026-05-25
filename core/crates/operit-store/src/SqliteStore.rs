use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use operit_host_api::{HostError, RuntimeSqliteConnection, RuntimeSqliteTransaction};
use thiserror::Error;

use crate::RuntimeStorageHost::{defaultRuntimeSqliteHost, runtimeStoragePath};

pub use operit_host_api::{SqliteRow, SqliteValue};

#[derive(Debug, Error)]
pub enum SqliteStoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("host error: {0}")]
    Host(#[from] HostError),
    #[error("sqlite connection mutex poisoned")]
    MutexPoisoned,
    #[error("sqlite invalidation observer mutex poisoned")]
    ObserverMutexPoisoned,
    #[error("{0}")]
    Message(String),
}

#[derive(Clone)]
pub struct SqliteStore {
    path: PathBuf,
    connection: Arc<Mutex<Box<dyn RuntimeSqliteConnection>>>,
    observers: Arc<Mutex<Vec<Arc<dyn Fn() -> Result<(), SqliteStoreError> + Send + Sync>>>>,
}

impl SqliteStore {
    pub fn open(path: PathBuf) -> Result<Self, SqliteStoreError> {
        let storagePath = runtimeStoragePath(&path);
        let mut connection = defaultRuntimeSqliteHost().openSqliteDatabase(&storagePath)?;
        connection.execute("PRAGMA foreign_keys = ON", Vec::new())?;
        Ok(Self {
            path,
            connection: Arc::new(Mutex::new(connection)),
            observers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    #[allow(non_snake_case)]
    pub fn executeBatch(&self, sql: &str) -> Result<(), SqliteStoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SqliteStoreError::MutexPoisoned)?;
        connection.executeBatch(sql)?;
        Ok(())
    }

    pub fn execute(&self, sql: &str, params: Vec<SqliteValue>) -> Result<usize, SqliteStoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SqliteStoreError::MutexPoisoned)?;
        Ok(connection.execute(sql, params)?)
    }

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

    pub fn queryScalar<T: FromSqliteValue>(
        &self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<T, SqliteStoreError> {
        let row = self
            .queryOne(sql, params)?
            .ok_or_else(|| SqliteStoreError::Message("sqlite query returned no rows".to_string()))?;
        row.get(0)
    }

    #[allow(non_snake_case)]
    pub fn getUserVersion(&self) -> Result<i32, SqliteStoreError> {
        self.queryScalar("PRAGMA user_version", Vec::new())
    }

    #[allow(non_snake_case)]
    pub fn setUserVersion(&self, version: i32) -> Result<(), SqliteStoreError> {
        self.execute(&format!("PRAGMA user_version = {version}"), Vec::new())?;
        Ok(())
    }

    #[allow(non_snake_case)]
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
        let result = action(&mut transaction)?;
        transaction.inner.commit()?;
        Ok(result)
    }
}

pub struct SqliteTransaction<'a> {
    inner: Box<dyn RuntimeSqliteTransaction + 'a>,
}

impl SqliteTransaction<'_> {
    pub fn execute(
        &mut self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<usize, SqliteStoreError> {
        Ok(self.inner.execute(sql, params)?)
    }

    pub fn queryRows(
        &mut self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<Vec<SqliteRow>, SqliteStoreError> {
        Ok(self.inner.query(sql, params)?)
    }

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
    pub fn lastInsertRowId(&self) -> Result<i64, SqliteStoreError> {
        Ok(self.inner.lastInsertRowId()?)
    }
}

pub trait SqliteRowGet {
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

pub trait SqliteColumnKey {
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

pub trait FromSqliteValue: Sized {
    #[allow(non_snake_case)]
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

pub trait ToSqliteValue {
    #[allow(non_snake_case)]
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
