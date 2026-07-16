use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::ptr::null_mut;
use std::slice;

use operit_host_api::{
    HostError, HostResult, HostSecretStore, RuntimeSqliteConnection, RuntimeSqliteHost,
    RuntimeSqliteTransaction, RuntimeStorageEntry, RuntimeStorageHost, SqliteRow, SqliteValue,
};
use rusqlite::types::Value;
use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND, FILETIME};
use windows_sys::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
    CRED_TYPE_GENERIC,
};

#[derive(Clone, Debug)]
pub struct WindowsRuntimeStorageHost {
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
}

impl WindowsRuntimeStorageHost {
    /// Returns the default Windows runtime data root.
    #[allow(non_snake_case)]
    pub fn defaultRuntimeRoot() -> PathBuf {
        let appdata =
            env::var_os("APPDATA").expect("APPDATA is required for Operit2 runtime storage");
        PathBuf::from(appdata).join("Operit2").join("runtime")
    }

    /// Returns the default Windows workspace collection root.
    #[allow(non_snake_case)]
    pub fn defaultWorkspaceRoot() -> PathBuf {
        let appdata =
            env::var_os("APPDATA").expect("APPDATA is required for Operit2 runtime storage");
        PathBuf::from(appdata).join("Operit2").join("workspaces")
    }

    /// Creates a Windows runtime storage host with explicit roots.
    #[allow(non_snake_case)]
    pub fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
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

impl RuntimeStorageHost for WindowsRuntimeStorageHost {
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.runtimeRoot.clone())
    }

    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.workspaceRoot.clone())
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(fs::read(self.resolve(path)?)?)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
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

impl HostSecretStore for WindowsRuntimeStorageHost {
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        let target = windowsSecretTargetName(key)?;
        let mut credential = null_mut::<CREDENTIALW>();
        let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
        if ok == 0 {
            let error = unsafe { GetLastError() };
            if error == ERROR_NOT_FOUND {
                return Ok(None);
            }
            return Err(lastWindowsCredentialError("read Windows credential"));
        }
        let credentialRef = unsafe { &*credential };
        let content = unsafe {
            slice::from_raw_parts(
                credentialRef.CredentialBlob,
                credentialRef.CredentialBlobSize as usize,
            )
            .to_vec()
        };
        unsafe { CredFree(credential.cast()) };
        Ok(Some(content))
    }

    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        let mut target = windowsSecretTargetName(key)?;
        let mut userName = widestring("Operit2")?;
        let mut credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            Comment: null_mut(),
            LastWritten: FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: content.len() as u32,
            CredentialBlob: content.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: null_mut(),
            TargetAlias: null_mut(),
            UserName: userName.as_mut_ptr(),
        };
        let ok = unsafe { CredWriteW(&mut credential, 0) };
        if ok == 0 {
            return Err(lastWindowsCredentialError("write Windows credential"));
        }
        Ok(())
    }

    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        let target = windowsSecretTargetName(key)?;
        let ok = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok == 0 {
            let error = unsafe { GetLastError() };
            if error != ERROR_NOT_FOUND {
                return Err(lastWindowsCredentialError("delete Windows credential"));
            }
        }
        Ok(())
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

/// Builds the Windows Credential Manager target name for a host secret key.
fn windowsSecretTargetName(key: &str) -> HostResult<Vec<u16>> {
    let name = validateSecretKey(key)?;
    widestring(&format!("Operit2:{name}"))
}

/// Converts Rust text into a nul-terminated Windows wide string.
fn widestring(value: &str) -> HostResult<Vec<u16>> {
    if value.encode_utf16().any(|unit| unit == 0) {
        return Err(HostError::new("Windows credential text contains nul"));
    }
    Ok(value.encode_utf16().chain(std::iter::once(0)).collect())
}

/// Converts the last Windows credential error into a host error.
fn lastWindowsCredentialError(action: &str) -> HostError {
    HostError::new(format!("{action} failed: Windows error {}", unsafe {
        GetLastError()
    }))
}

impl RuntimeSqliteHost for WindowsRuntimeStorageHost {
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
