//! Shared test-host scaffolding for tests that need the process-level default
//! runtime hosts (storage + sqlite + store-root config) to point at a throwaway
//! filesystem root.
//!
//! Registering these globals is inherently process-wide: installing them from
//! two independent `OnceLock`s (as the first version of ObjectBoxStoreSyncTests
//! did alongside SqlChatSyncStoreTests) lets the later installer silently
//! repoint the globals and split-brain the earlier suite's connections. Both
//! suites therefore go through this one installer so there is exactly one root
//! and one installation for the whole test binary.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::{
    HostError, HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeSqliteTransaction,
    RuntimeStorageEntry, RuntimeStorageHost, SqliteRow as HostSqliteRow, SqliteValue,
};
use rusqlite::types::Value as RusqliteValue;

use crate::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};

static SHARED_HOSTS: OnceLock<PathBuf> = OnceLock::new();

/// Serializes tests that touch the shared host root across modules.
pub static SHARED_TEST_HOST_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug)]
pub struct TestRuntimeHost {
    root: PathBuf,
}

impl TestRuntimeHost {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn resolve(&self, path: &str) -> HostResult<PathBuf> {
        let path = Path::new(path);
        if path.is_absolute() {
            return Err(HostError::new(format!(
                "Runtime storage path must be relative: {}",
                path.display()
            )));
        }
        let mut resolved = self.root.clone();
        for component in path.components() {
            match component {
                Component::Normal(segment) => resolved.push(segment),
                Component::CurDir => {}
                _ => {
                    return Err(HostError::new(format!(
                        "Invalid runtime storage path: {}",
                        path.display()
                    )))
                }
            }
        }
        Ok(resolved)
    }
}

impl RuntimeStorageHost for TestRuntimeHost {
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }

    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.root.join("workspace"))
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(fs::read(self.resolve(path)?)?)
    }

    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let content = self.readBytes(path)?;
        let start = usize::try_from(offset)
            .map_err(|_| HostError::new("runtime storage offset does not fit usize"))?;
        if start >= content.len() {
            return Ok(Vec::new());
        }
        let end = start
            .checked_add(length)
            .ok_or_else(|| HostError::new("runtime storage byte range overflows usize"))?
            .min(content.len());
        Ok(content[start..end].to_vec())
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        std::io::Write::write_all(&mut file, content)?;
        Ok(())
    }

    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        let path = self.resolve(path)?;
        if !path.exists() {
            return Ok(());
        }
        if path.is_dir() {
            if recursive {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_dir(path)?;
            }
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.resolve(path)?.exists())
    }

    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let directory = self.resolve(prefix)?;
        let mut entries = Vec::new();
        if !directory.exists() {
            return Ok(entries);
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry
                .path()
                .strip_prefix(&self.root)
                .map_err(|error| HostError::new(error.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(RuntimeStorageEntry {
                path,
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
            });
        }
        Ok(entries)
    }
}

impl RuntimeSqliteHost for TestRuntimeHost {
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection =
            rusqlite::Connection::open(path).map_err(|error| HostError::new(error.to_string()))?;
        connection
            .execute_batch(
                r#"
                    PRAGMA journal_mode = MEMORY;
                    PRAGMA synchronous = OFF;
                    PRAGMA temp_store = MEMORY;
                    "#,
            )
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(TestRuntimeSqliteConnection { connection }))
    }
}

struct TestRuntimeSqliteConnection {
    connection: rusqlite::Connection,
}

impl RuntimeSqliteConnection for TestRuntimeSqliteConnection {
    fn executeBatch(&mut self, sql: &str) -> HostResult<()> {
        self.connection
            .execute_batch(sql)
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
        self.connection
            .execute(sql, rusqlite::params_from_iter(params))
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<HostSqliteRow>> {
        queryRows(&self.connection, sql, params)
    }

    fn lastInsertRowId(&self) -> HostResult<i64> {
        Ok(self.connection.last_insert_rowid())
    }

    fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(TestRuntimeSqliteTransaction { transaction }))
    }
}

struct TestRuntimeSqliteTransaction<'a> {
    transaction: rusqlite::Transaction<'a>,
}

impl RuntimeSqliteTransaction for TestRuntimeSqliteTransaction<'_> {
    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
        self.transaction
            .execute(sql, rusqlite::params_from_iter(params))
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<HostSqliteRow>> {
        queryRows(&self.transaction, sql, params)
    }

    fn lastInsertRowId(&self) -> HostResult<i64> {
        Ok(self.transaction.last_insert_rowid())
    }

    fn commit(self: Box<Self>) -> HostResult<()> {
        self.transaction
            .commit()
            .map_err(|error| HostError::new(error.to_string()))
    }
}

trait TestRusqliteConnection {
    fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>>;
}

impl TestRusqliteConnection for rusqlite::Connection {
    fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>> {
        self.prepare(sql)
    }
}

impl TestRusqliteConnection for rusqlite::Transaction<'_> {
    fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>> {
        self.prepare(sql)
    }
}

pub fn queryRows(
    connection: &impl TestRusqliteConnection,
    sql: &str,
    params: Vec<SqliteValue>,
) -> HostResult<Vec<HostSqliteRow>> {
    let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
    let mut statement = connection
        .prepareStatement(sql)
        .map_err(|error| HostError::new(error.to_string()))?;
    let columns = statement
        .column_names()
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut rows = statement
        .query(rusqlite::params_from_iter(params))
        .map_err(|error| HostError::new(error.to_string()))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| HostError::new(error.to_string()))?
    {
        let mut values = Vec::new();
        for index in 0..columns.len() {
            let value = row
                .get::<_, RusqliteValue>(index)
                .map_err(|error| HostError::new(error.to_string()))?;
            values.push(fromRusqliteValue(value));
        }
        out.push(HostSqliteRow {
            columns: columns.clone(),
            values,
        });
    }
    Ok(out)
}

fn toRusqliteValue(value: SqliteValue) -> RusqliteValue {
    match value {
        SqliteValue::Null => RusqliteValue::Null,
        SqliteValue::Integer(value) => RusqliteValue::Integer(value),
        SqliteValue::Real(value) => RusqliteValue::Real(value),
        SqliteValue::Text(value) => RusqliteValue::Text(value),
        SqliteValue::Blob(value) => RusqliteValue::Blob(value),
    }
}

fn fromRusqliteValue(value: RusqliteValue) -> SqliteValue {
    match value {
        RusqliteValue::Null => SqliteValue::Null,
        RusqliteValue::Integer(value) => SqliteValue::Integer(value),
        RusqliteValue::Real(value) => SqliteValue::Real(value),
        RusqliteValue::Text(value) => SqliteValue::Text(value),
        RusqliteValue::Blob(value) => SqliteValue::Blob(value),
    }
}

/// Installs the shared throwaway host root exactly once per test binary and
/// returns it. Safe to call from every suite: the globals are configured a
/// single time and never repointed afterwards.
pub fn installSharedTestHost() -> PathBuf {
    SHARED_HOSTS
        .get_or_init(|| {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test clock must be after UNIX_EPOCH")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "operit2-store-shared-tests-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("shared test host root must be created");
            let host = Arc::new(TestRuntimeHost::new(root.clone()));
            setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
                root.clone(),
                root.join("workspace"),
            ));
            setDefaultRuntimeStorageHost(host.clone());
            setDefaultRuntimeSqliteHost(host);
            root
        })
        .clone()
}
