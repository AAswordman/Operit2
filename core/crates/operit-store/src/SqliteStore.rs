use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SqliteStoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("sqlite connection mutex poisoned")]
    MutexPoisoned,
}

#[derive(Clone)]
pub struct SqliteStore {
    path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    pub fn open(path: PathBuf) -> Result<Self, SqliteStoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(&path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self {
            path,
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn executeBatch(&self, sql: &str) -> Result<(), SqliteStoreError> {
        self.withConnection(|connection| {
            connection.execute_batch(sql)?;
            Ok(())
        })
    }

    pub fn getUserVersion(&self) -> Result<i32, SqliteStoreError> {
        self.withConnection(|connection| {
            connection.query_row("PRAGMA user_version", [], |row| row.get(0))
        })
    }

    pub fn setUserVersion(&self, version: i32) -> Result<(), SqliteStoreError> {
        self.withConnection(|connection| {
            connection.pragma_update(None, "user_version", version)?;
            Ok(())
        })
    }

    pub fn tableExists(&self, tableName: &str) -> Result<bool, SqliteStoreError> {
        self.withConnection(|connection| {
            let found: Option<i32> = connection
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
                    [tableName],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(found.is_some())
        })
    }

    pub fn withConnection<T, F>(&self, action: F) -> Result<T, SqliteStoreError>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SqliteStoreError::MutexPoisoned)?;
        Ok(action(&connection)?)
    }

    pub fn transaction<T, F>(&self, action: F) -> Result<T, SqliteStoreError>
    where
        F: FnOnce(&rusqlite::Transaction<'_>) -> Result<T, rusqlite::Error>,
    {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SqliteStoreError::MutexPoisoned)?;
        let transaction = connection.transaction()?;
        let result = action(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }
}
