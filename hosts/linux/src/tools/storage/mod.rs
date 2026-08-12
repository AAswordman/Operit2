use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use operit_host_api::{
    HostError, HostResult, HostSecretStore, RuntimeSqliteConnection, RuntimeSqliteHost,
    RuntimeSqliteTransaction, RuntimeStorageEntry, RuntimeStorageHost, SqliteRow, SqliteValue,
};
use rusqlite::types::Value;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct LinuxRuntimeStorageHost {
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
    secretRoot: PathBuf,
}

impl LinuxRuntimeStorageHost {
    /// Returns the default Linux runtime data root.
    #[allow(non_snake_case)]
    pub fn defaultRuntimeRoot() -> PathBuf {
        if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(xdg_data_home).join("operit2").join("runtime");
        }
        let home = env::var_os("HOME").expect("HOME is required for Operit2 runtime storage");
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("operit2")
            .join("runtime")
    }

    /// Returns the default Linux workspace collection root.
    #[allow(non_snake_case)]
    pub fn defaultWorkspaceRoot() -> PathBuf {
        if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(xdg_data_home)
                .join("operit2")
                .join("workspaces");
        }
        let home = env::var_os("HOME").expect("HOME is required for Operit2 runtime storage");
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("operit2")
            .join("workspaces")
    }

    /// Creates a Linux runtime storage host with explicit roots.
    #[allow(non_snake_case)]
    pub fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            runtimeRoot,
            workspaceRoot,
            secretRoot: defaultHostSecretRoot(),
        }
    }

    fn resolve(&self, path: &str) -> HostResult<PathBuf> {
        let normalized = normalizeStoragePath(path)?;
        let segments = normalized.iter().map(String::as_str).collect::<Vec<_>>();
        match segments.as_slice() {
            ["runtime", rest @ ..] => Ok(joinSegments(&self.runtimeRoot, rest)),
            ["workspaces", rest @ ..] => Ok(joinSegments(&self.workspaceRoot, rest)),
            ["secure", rest @ ..] => legacySecurePath(&self.runtimeRoot, rest),
            _ => Err(HostError::new(format!(
                "Runtime storage path must start with runtime/, workspaces/, or secure/: {path}"
            ))),
        }
    }

    fn storagePathForPhysical(&self, path: &Path) -> HostResult<String> {
        if let Ok(relative) = path.strip_prefix(&self.runtimeRoot) {
            return Ok(prefixedPath("runtime", relative));
        }
        if let Ok(relative) = path.strip_prefix(&self.workspaceRoot) {
            return Ok(prefixedPath("workspaces", relative));
        }
        let secureRoot = legacySecurePath(&self.runtimeRoot, &[])?;
        if let Ok(relative) = path.strip_prefix(&secureRoot) {
            return Ok(prefixedPath("secure", relative));
        }
        Err(HostError::new(format!(
            "Physical path is outside configured runtime and workspace roots: {}",
            path.display()
        )))
    }

    /// Reads one Linux host secret from the private secret directory.
    fn readSecretFile(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        let path = self.secretPath(key)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        self.validateSecretFile(&path, &metadata)?;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        Ok(Some(content))
    }

    /// Writes one Linux host secret through an atomic private file replacement.
    fn writeSecretFile(&self, key: &str, content: &[u8]) -> HostResult<()> {
        self.ensureSecretRoot()?;
        let path = self.secretPath(key)?;
        let temporaryPath = self.secretRoot.join(format!(
            ".{}.{}.tmp",
            validateSecretKey(key)?,
            Uuid::new_v4()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temporaryPath)?;
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(temporaryPath, path)?;
        self.syncSecretRoot()
    }

    /// Deletes one Linux host secret from the private secret directory.
    fn deleteSecretFile(&self, key: &str) -> HostResult<()> {
        let path = self.secretPath(key)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        self.validateSecretFile(&path, &metadata)?;
        fs::remove_file(path)?;
        self.syncSecretRoot()
    }

    /// Ensures the Linux host secret directory exists with private permissions.
    fn ensureSecretRoot(&self) -> HostResult<()> {
        fs::create_dir_all(&self.secretRoot)?;
        let metadata = fs::symlink_metadata(&self.secretRoot)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(HostError::new(format!(
                "Linux host secret root must be a directory: {}",
                self.secretRoot.display()
            )));
        }
        fs::set_permissions(&self.secretRoot, fs::Permissions::from_mode(0o700))?;
        Ok(())
    }

    /// Resolves one validated secret key beneath the Linux host secret directory.
    fn secretPath(&self, key: &str) -> HostResult<PathBuf> {
        Ok(self.secretRoot.join(validateSecretKey(key)?))
    }

    /// Validates that one persisted Linux host secret is a private regular file.
    fn validateSecretFile(&self, path: &Path, metadata: &fs::Metadata) -> HostResult<()> {
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(HostError::new(format!(
                "Linux host secret must be a regular file: {}",
                path.display()
            )));
        }
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(HostError::new(format!(
                "Linux host secret has insecure permissions: {}",
                path.display()
            )));
        }
        Ok(())
    }

    /// Synchronizes Linux host secret directory metadata after a mutation.
    fn syncSecretRoot(&self) -> HostResult<()> {
        File::open(&self.secretRoot)?.sync_all()?;
        Ok(())
    }
}

/// Resolves the legacy secure storage namespace beside the runtime root.
fn legacySecurePath(runtimeRoot: &Path, segments: &[&str]) -> HostResult<PathBuf> {
    let mut resolved = runtimeRoot.parent().map(Path::to_path_buf).ok_or_else(|| {
        HostError::new(format!(
            "Runtime root has no parent for secure storage: {}",
            runtimeRoot.display()
        ))
    })?;
    resolved.push("secure");
    for segment in segments {
        resolved.push(segment);
    }
    Ok(resolved)
}

/// Returns the per-user Linux directory used for private host secrets.
fn defaultHostSecretRoot() -> PathBuf {
    if let Some(xdg_state_home) = env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(xdg_state_home)
            .join("operit2")
            .join("secrets");
    }
    let home = env::var_os("HOME").expect("HOME is required for Operit2 host secrets");
    PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("operit2")
        .join("secrets")
}

impl RuntimeStorageHost for LinuxRuntimeStorageHost {
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.runtimeRoot.clone())
    }

    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.workspaceRoot.clone())
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(fs::read(self.resolve(path)?)?)
    }

    /// Reads one bounded byte range from Linux runtime storage.
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let mut file = fs::File::open(self.resolve(path)?)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; length];
        let count = file.read(&mut bytes)?;
        bytes.truncate(count);
        Ok(bytes)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    /// Appends bytes to a Linux runtime storage file.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(content)?;
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
            entries.push(RuntimeStorageEntry {
                path: self.storagePathForPhysical(&entry.path())?,
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
            });
        }
        Ok(entries)
    }
}

impl HostSecretStore for LinuxRuntimeStorageHost {
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        self.readSecretFile(key)
    }

    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        self.writeSecretFile(key, content)
    }

    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        self.deleteSecretFile(key)
    }
}

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

fn joinSegments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut resolved = root.to_path_buf();
    for segment in segments {
        resolved.push(segment);
    }
    resolved
}

fn prefixedPath(prefix: &str, relative: &Path) -> String {
    let relative = relative.to_string_lossy().replace('\\', "/");
    if relative.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{relative}")
    }
}

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

impl RuntimeSqliteHost for LinuxRuntimeStorageHost {
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

fn toRusqliteValue(value: SqliteValue) -> Value {
    match value {
        SqliteValue::Null => Value::Null,
        SqliteValue::Integer(value) => Value::Integer(value),
        SqliteValue::Real(value) => Value::Real(value),
        SqliteValue::Text(value) => Value::Text(value),
        SqliteValue::Blob(value) => Value::Blob(value),
    }
}

fn fromRusqliteValue(value: Value) -> SqliteValue {
    match value {
        Value::Null => SqliteValue::Null,
        Value::Integer(value) => SqliteValue::Integer(value),
        Value::Real(value) => SqliteValue::Real(value),
        Value::Text(value) => SqliteValue::Text(value),
        Value::Blob(value) => SqliteValue::Blob(value),
    }
}
