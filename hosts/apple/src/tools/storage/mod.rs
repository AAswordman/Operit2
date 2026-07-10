use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[cfg(any(target_os = "ios", target_os = "macos"))]
use operit_host_api::HostSecretStore;
use operit_host_api::{
    HostError, HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeSqliteTransaction,
    RuntimeStorageEntry, RuntimeStorageHost, SqliteRow, SqliteValue,
};
use rusqlite::types::Value;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

#[derive(Clone, Debug)]
pub struct AppleRuntimeStorageHost {
    root: PathBuf,
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
}

impl AppleRuntimeStorageHost {
    /// Returns the default Apple application support root.
    #[allow(non_snake_case)]
    pub fn defaultRoot() -> PathBuf {
        let base = appleApplicationSupportDirectory();
        base.join("Operit2")
    }

    /// Creates an Apple runtime storage host rooted at the supplied directory.
    pub fn new(root: PathBuf) -> Self {
        let runtimeRoot = root.join("runtime");
        let workspaceRoot = root.join("workspaces");
        Self::newWithRoots(root, runtimeRoot, workspaceRoot)
    }

    /// Creates an Apple runtime storage host with explicit runtime and workspace roots.
    #[allow(non_snake_case)]
    pub fn newWithRoots(root: PathBuf, runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            root,
            runtimeRoot,
            workspaceRoot,
        }
    }

    fn resolve(&self, path: &str) -> HostResult<PathBuf> {
        let normalized = normalizeStoragePath(path)?;
        let segments = normalized.iter().map(String::as_str).collect::<Vec<_>>();
        match segments.as_slice() {
            ["runtime", rest @ ..] => Ok(joinSegments(&self.runtimeRoot, rest)),
            ["workspaces", rest @ ..] => Ok(joinSegments(&self.workspaceRoot, rest)),
            _ => Ok(joinSegments(&self.root, &segments)),
        }
    }

    fn storagePathForPhysical(&self, path: &Path) -> HostResult<String> {
        if let Ok(relative) = path.strip_prefix(&self.runtimeRoot) {
            return Ok(prefixedPath("runtime", relative));
        }
        if let Ok(relative) = path.strip_prefix(&self.workspaceRoot) {
            return Ok(prefixedPath("workspaces", relative));
        }
        Ok(path
            .strip_prefix(&self.root)
            .map_err(|error| HostError::new(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/"))
    }
}

impl RuntimeStorageHost for AppleRuntimeStorageHost {
    /// Returns the Apple runtime storage data root directory.
    #[allow(non_snake_case)]
    fn rootDir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }

    /// Returns the Apple runtime files root directory.
    #[allow(non_snake_case)]
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.runtimeRoot.clone())
    }

    /// Returns the Apple workspace collection root directory.
    #[allow(non_snake_case)]
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.workspaceRoot.clone())
    }

    /// Reads bytes from Apple runtime storage.
    #[allow(non_snake_case)]
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(fs::read(self.resolve(path)?)?)
    }

    /// Writes bytes into Apple runtime storage.
    #[allow(non_snake_case)]
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    /// Deletes an entry from Apple runtime storage.
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

    /// Checks whether an Apple runtime storage path exists.
    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.resolve(path)?.exists())
    }

    /// Lists entries under an Apple runtime storage prefix.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let directory = self.resolve(prefix)?;
        let mut entries = Vec::new();
        if !directory.exists() {
            return Ok(entries);
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            entries.push(RuntimeStorageEntry {
                path: self.storagePathForPhysical(&entry.path())?,
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
            });
        }
        Ok(entries)
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl HostSecretStore for AppleRuntimeStorageHost {
    /// Reads secret bytes from Apple Keychain.
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        let account = validateSecretKey(key)?;
        match get_generic_password(APPLE_SECRET_SERVICE, &account) {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.code() == -25300 => Ok(None),
            Err(error) => Err(HostError::new(format!(
                "read Apple Keychain item failed: {error}"
            ))),
        }
    }

    /// Writes secret bytes into Apple Keychain.
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        let account = validateSecretKey(key)?;
        set_generic_password(APPLE_SECRET_SERVICE, &account, content)
            .map_err(|error| HostError::new(format!("write Apple Keychain item failed: {error}")))
    }

    /// Deletes secret bytes from Apple Keychain.
    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        let account = validateSecretKey(key)?;
        match delete_generic_password(APPLE_SECRET_SERVICE, &account) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == -25300 => Ok(()),
            Err(error) => Err(HostError::new(format!(
                "delete Apple Keychain item failed: {error}"
            ))),
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
const APPLE_SECRET_SERVICE: &str = "Operit2.HostSecretStore";

impl RuntimeSqliteHost for AppleRuntimeStorageHost {
    /// Opens an SQLite database under Apple runtime storage.
    #[allow(non_snake_case)]
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection =
            rusqlite::Connection::open(path).map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(RusqliteRuntimeConnection { connection }))
    }
}

/// Normalizes a storage path into safe relative path segments.
#[allow(non_snake_case)]
fn normalizeStoragePath(path: &str) -> HostResult<Vec<String>> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Err(HostError::new(format!(
            "Runtime storage path must be relative: {}",
            path.display()
        )));
    }
    let mut segments = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => segments.push(segment.to_string_lossy().to_string()),
            Component::CurDir => {}
            _ => {
                return Err(HostError::new(format!(
                    "Invalid runtime storage path: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(segments)
}

/// Joins normalized relative path segments under a physical root.
#[allow(non_snake_case)]
fn joinSegments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut resolved = root.to_path_buf();
    for segment in segments {
        resolved.push(segment);
    }
    resolved
}

/// Builds a runtime-storage path with the requested top-level prefix.
#[allow(non_snake_case)]
fn prefixedPath(prefix: &str, relative: &Path) -> String {
    let relative = relative.to_string_lossy().replace('\\', "/");
    if relative.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{relative}")
    }
}

struct RusqliteRuntimeConnection {
    connection: rusqlite::Connection,
}

impl RuntimeSqliteConnection for RusqliteRuntimeConnection {
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

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
        queryRowsConnection(&self.connection, sql, params)
    }

    fn lastInsertRowId(&self) -> HostResult<i64> {
        Ok(self.connection.last_insert_rowid())
    }

    fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(RusqliteRuntimeTransaction { transaction }))
    }
}

struct RusqliteRuntimeTransaction<'a> {
    transaction: rusqlite::Transaction<'a>,
}

impl RuntimeSqliteTransaction for RusqliteRuntimeTransaction<'_> {
    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
        self.transaction
            .execute(sql, rusqlite::params_from_iter(params))
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
        queryRowsTransaction(&self.transaction, sql, params)
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

/// Queries SQLite rows from a connection and converts values to host rows.
#[allow(non_snake_case)]
fn queryRowsConnection(
    connection: &rusqlite::Connection,
    sql: &str,
    params: Vec<SqliteValue>,
) -> HostResult<Vec<SqliteRow>> {
    let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| HostError::new(error.to_string()))?;
    collectRows(&mut statement, params)
}

/// Queries SQLite rows from a transaction and converts values to host rows.
#[allow(non_snake_case)]
fn queryRowsTransaction(
    transaction: &rusqlite::Transaction<'_>,
    sql: &str,
    params: Vec<SqliteValue>,
) -> HostResult<Vec<SqliteRow>> {
    let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
    let mut statement = transaction
        .prepare(sql)
        .map_err(|error| HostError::new(error.to_string()))?;
    collectRows(&mut statement, params)
}

/// Collects all rows from a prepared SQLite statement.
#[allow(non_snake_case)]
fn collectRows(
    statement: &mut rusqlite::Statement<'_>,
    params: Vec<Value>,
) -> HostResult<Vec<SqliteRow>> {
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
                .get::<_, Value>(index)
                .map_err(|error| HostError::new(error.to_string()))?;
            values.push(fromRusqliteValue(value));
        }
        out.push(SqliteRow {
            columns: columns.clone(),
            values,
        });
    }
    Ok(out)
}

/// Converts a host SQLite value into a rusqlite value.
#[allow(non_snake_case)]
fn toRusqliteValue(value: SqliteValue) -> Value {
    match value {
        SqliteValue::Null => Value::Null,
        SqliteValue::Integer(value) => Value::Integer(value),
        SqliteValue::Real(value) => Value::Real(value),
        SqliteValue::Text(value) => Value::Text(value),
        SqliteValue::Blob(value) => Value::Blob(value),
    }
}

/// Converts a rusqlite value into a host SQLite value.
#[allow(non_snake_case)]
fn fromRusqliteValue(value: Value) -> SqliteValue {
    match value {
        Value::Null => SqliteValue::Null,
        Value::Integer(value) => SqliteValue::Integer(value),
        Value::Real(value) => SqliteValue::Real(value),
        Value::Text(value) => SqliteValue::Text(value),
        Value::Blob(value) => SqliteValue::Blob(value),
    }
}

/// Validates and returns a platform-safe host secret key.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn validateSecretKey(key: &str) -> HostResult<String> {
    if key.is_empty()
        || key.chars().any(|character| {
            !(character.is_ascii_alphanumeric()
                || character == '.'
                || character == '_'
                || character == '-')
        })
    {
        return Err(HostError::new(format!("invalid host secret key: {key}")));
    }
    Ok(key.to_string())
}

/// Returns the Apple application support directory.
#[allow(non_snake_case)]
fn appleApplicationSupportDirectory() -> PathBuf {
    let home = env::var_os("HOME").expect("HOME is required for Apple runtime storage");
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
}
