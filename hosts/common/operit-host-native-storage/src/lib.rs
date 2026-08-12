use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use operit_host_api::{
    ArchiveStagingHost, HostError, HostResult, RuntimeSqliteConnection, RuntimeSqliteHost,
    RuntimeSqliteTransaction, RuntimeStorageEntry, RuntimeStorageHost, RuntimeStorageWriteHost,
    RuntimeStorageWriteSession, SqliteRow, SqliteValue,
};
use rusqlite::types::Value;

#[derive(Clone, Debug)]
pub struct NativeRuntimeStorageHost {
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
}

/// Writes one runtime storage file through a private sibling staging path.
struct NativeRuntimeStorageWriteSession {
    targetPath: PathBuf,
    temporaryPath: PathBuf,
    file: Option<fs::File>,
}

impl NativeRuntimeStorageWriteSession {
    /// Closes and atomically renames one staged runtime storage file.
    fn commitInternal(&mut self, sync: bool) -> HostResult<()> {
        let file = self
            .file
            .take()
            .ok_or_else(|| HostError::new("Runtime storage write session is closed"))?;
        if sync {
            file.sync_all()?;
        }
        drop(file);
        fs::rename(&self.temporaryPath, &self.targetPath)?;
        Ok(())
    }
}

impl RuntimeStorageWriteSession for NativeRuntimeStorageWriteSession {
    /// Appends one chunk to the private staging file.
    fn writeChunk(&mut self, chunk: &[u8]) -> HostResult<()> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| HostError::new("Runtime storage write session is closed"))?;
        file.write_all(chunk)?;
        Ok(())
    }

    /// Flushes the staged file and atomically publishes it at the requested path.
    fn commit(mut self: Box<Self>) -> HostResult<()> {
        self.commitInternal(true)
    }

    /// Atomically publishes the staged file without forcing a device flush.
    fn commitFast(mut self: Box<Self>) -> HostResult<()> {
        self.commitInternal(false)
    }

    /// Removes the private staging file without modifying the published target.
    fn discard(mut self: Box<Self>) -> HostResult<()> {
        self.file.take();
        if self.temporaryPath.exists() {
            fs::remove_file(&self.temporaryPath)?;
        }
        Ok(())
    }
}

/// Stages streamed archive uploads in the runtime-private temporary directory.
#[derive(Clone, Debug)]
pub struct NativeArchiveStagingHost {
    runtimeRoot: PathBuf,
    uploads: Arc<Mutex<HashMap<String, NativeArchiveUpload>>>,
}

/// Tracks the declared length and lifecycle state of one current-process archive upload.
#[derive(Clone, Debug)]
struct NativeArchiveUpload {
    expectedByteLength: u64,
    sealed: bool,
}

impl NativeArchiveStagingHost {
    /// Creates native archive staging rooted under one runtime data directory.
    pub fn new(runtimeRoot: PathBuf) -> Self {
        Self {
            runtimeRoot,
            uploads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns the private directory containing staged archive files.
    fn stagingRoot(&self) -> PathBuf {
        self.runtimeRoot.join("temp").join("archive_staging")
    }

    /// Validates an opaque archive identifier supplied by the Core API.
    fn validateArchiveId(&self, archiveId: &str) -> HostResult<()> {
        if archiveId.is_empty()
            || !archiveId
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(HostError::new("Archive staging ID is invalid"));
        }
        Ok(())
    }

    /// Resolves one upload-in-progress path from a validated opaque archive ID.
    fn uploadingArchivePath(&self, archiveId: &str) -> HostResult<PathBuf> {
        self.validateArchiveId(archiveId)?;
        Ok(self.stagingRoot().join(format!("{archiveId}.upload")))
    }

    /// Resolves one immutable sealed path from a validated opaque archive ID.
    fn sealedArchivePath(&self, archiveId: &str) -> HostResult<PathBuf> {
        self.validateArchiveId(archiveId)?;
        Ok(self.stagingRoot().join(format!("{archiveId}.sealed")))
    }
}

impl ArchiveStagingHost for NativeArchiveStagingHost {
    /// Creates one empty private staging file without replacing an existing upload.
    fn createArchive(&self, archiveId: &str, expectedByteLength: u64) -> HostResult<()> {
        let path = self.uploadingArchivePath(archiveId)?;
        let sealedPath = self.sealedArchivePath(archiveId)?;
        let mut uploads = self
            .uploads
            .lock()
            .map_err(|_| HostError::new("Archive staging upload state is poisoned"))?;
        if uploads.contains_key(archiveId) {
            return Err(HostError::new("Archive staging ID already exists"));
        }
        fs::create_dir_all(self.stagingRoot())?;
        if sealedPath.exists() {
            return Err(HostError::new("Archive staging ID already exists"));
        }
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        uploads.insert(
            archiveId.to_string(),
            NativeArchiveUpload {
                expectedByteLength,
                sealed: false,
            },
        );
        Ok(())
    }

    /// Appends one ordered upload chunk to the private staging file.
    fn appendArchive(&self, archiveId: &str, chunk: &[u8]) -> HostResult<()> {
        let path = self.uploadingArchivePath(archiveId)?;
        let uploads = self
            .uploads
            .lock()
            .map_err(|_| HostError::new("Archive staging upload state is poisoned"))?;
        let upload = uploads
            .get(archiveId)
            .ok_or_else(|| HostError::new("Archive staging ID does not exist"))?;
        if upload.sealed {
            return Err(HostError::new("Archive staging upload is already sealed"));
        }
        let currentByteLength = fs::metadata(&path)?.len();
        let remainingByteLength = upload
            .expectedByteLength
            .checked_sub(currentByteLength)
            .ok_or_else(|| {
                HostError::new("Archive staging upload exceeds its declared byte length")
            })?;
        if u64::try_from(chunk.len())
            .map_err(|_| HostError::new("Archive staging chunk length does not fit u64"))?
            > remainingByteLength
        {
            return Err(HostError::new(
                "Archive staging upload exceeds its declared byte length",
            ));
        }
        let mut file = fs::OpenOptions::new().append(true).open(path)?;
        file.write_all(chunk)?;
        Ok(())
    }

    /// Flushes one staged upload and returns the resulting persisted byte length.
    fn sealArchive(&self, archiveId: &str) -> HostResult<u64> {
        let uploadingPath = self.uploadingArchivePath(archiveId)?;
        let sealedPath = self.sealedArchivePath(archiveId)?;
        let mut uploads = self
            .uploads
            .lock()
            .map_err(|_| HostError::new("Archive staging upload state is poisoned"))?;
        let upload = uploads
            .get_mut(archiveId)
            .ok_or_else(|| HostError::new("Archive staging ID does not exist"))?;
        if upload.sealed {
            return Ok(fs::metadata(sealedPath)?.len());
        }
        let actualByteLength = fs::metadata(&uploadingPath)?.len();
        if actualByteLength != upload.expectedByteLength {
            return Err(HostError::new(
                "Archive staging upload does not match its declared byte length",
            ));
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&uploadingPath)?;
        file.sync_all()?;
        drop(file);
        fs::rename(uploadingPath, &sealedPath)?;
        upload.sealed = true;
        Ok(actualByteLength)
    }

    /// Reads one requested byte range from a staged upload.
    fn readArchive(&self, archiveId: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let path = self.sealedArchivePath(archiveId)?;
        let mut file = fs::File::open(path)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; length];
        let count = file.read(&mut bytes)?;
        bytes.truncate(count);
        Ok(bytes)
    }

    /// Deletes one private staged upload.
    fn removeArchive(&self, archiveId: &str) -> HostResult<()> {
        let uploadingPath = self.uploadingArchivePath(archiveId)?;
        let sealedPath = self.sealedArchivePath(archiveId)?;
        self.uploads
            .lock()
            .map_err(|_| HostError::new("Archive staging upload state is poisoned"))?
            .remove(archiveId);
        if uploadingPath.exists() {
            fs::remove_file(uploadingPath)?;
        }
        if sealedPath.exists() {
            fs::remove_file(sealedPath)?;
        }
        Ok(())
    }
}

impl NativeRuntimeStorageHost {
    /// Creates a native runtime storage host with explicit roots.
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

    /// Creates a unique sibling path used while one storage file is streamed.
    fn writeTemporaryPath(targetPath: &Path) -> HostResult<PathBuf> {
        let parent = targetPath
            .parent()
            .ok_or_else(|| HostError::new("Runtime storage file has no parent directory"))?;
        let fileName = targetPath
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| HostError::new("Runtime storage file has an invalid name"))?;
        Ok(parent.join(format!(".{fileName}.{}.partial", uuid::Uuid::new_v4())))
    }
}

impl RuntimeStorageWriteHost for NativeRuntimeStorageHost {
    /// Opens one private streaming write session for a validated storage path.
    fn createWriteSession(&self, path: &str) -> HostResult<Box<dyn RuntimeStorageWriteSession>> {
        let targetPath = self.resolve(path)?;
        let parent = targetPath
            .parent()
            .ok_or_else(|| HostError::new("Runtime storage file has no parent directory"))?;
        fs::create_dir_all(parent)?;
        let temporaryPath = Self::writeTemporaryPath(&targetPath)?;
        let file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporaryPath)?;
        Ok(Box::new(NativeRuntimeStorageWriteSession {
            targetPath,
            temporaryPath,
            file: Some(file),
        }))
    }
}

impl RuntimeStorageHost for NativeRuntimeStorageHost {
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.runtimeRoot.clone())
    }

    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.workspaceRoot.clone())
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(fs::read(self.resolve(path)?)?)
    }

    /// Reads one bounded byte range from native runtime storage.
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

    /// Appends bytes to a native runtime storage file.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
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

/// Normalizes a runtime storage path into safe relative segments.
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

/// Joins normalized storage path segments under a physical root.
fn joinSegments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut resolved = root.to_path_buf();
    for segment in segments {
        resolved.push(segment);
    }
    resolved
}

/// Builds a storage path prefixed with its virtual top-level root.
fn prefixedPath(prefix: &str, relative: &Path) -> String {
    let relative = relative.to_string_lossy().replace('\\', "/");
    if relative.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{relative}")
    }
}

/// Opens a SQLite database under the native runtime storage root.
impl RuntimeSqliteHost for NativeRuntimeStorageHost {
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

#[cfg(test)]
mod tests {
    use std::fs;

    use operit_host_api::{ArchiveStagingHost, RuntimeStorageHost};

    use super::{NativeArchiveStagingHost, NativeRuntimeStorageHost};

    /// Verifies that a native staged archive rejects excess bytes and seals only at its declared length.
    #[test]
    fn archive_staging_enforces_declared_length() {
        let root =
            std::env::temp_dir().join(format!("operit-archive-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("temporary archive test root must be created");
        let host = NativeArchiveStagingHost::new(root.clone());

        host.createArchive("archive", 4)
            .expect("archive must be created");
        host.appendArchive("archive", b"ab")
            .expect("first archive chunk must append");
        assert!(host.appendArchive("archive", b"cde").is_err());
        assert!(host.sealArchive("archive").is_err());

        host.appendArchive("archive", b"cd")
            .expect("final archive chunk must append");
        assert_eq!(host.sealArchive("archive").expect("archive must seal"), 4);
        assert_eq!(
            host.readArchive("archive", 0, 4)
                .expect("sealed archive must be readable"),
            b"abcd"
        );

        host.removeArchive("archive")
            .expect("sealed archive must be removed");
        fs::remove_dir_all(root).expect("temporary archive test root must be removed");
    }

    /// Verifies that native runtime storage appends without rewriting earlier content.
    #[test]
    fn runtime_storage_appends_bytes() {
        let root = std::env::temp_dir().join(format!(
            "operit-runtime-storage-test-{}",
            uuid::Uuid::new_v4()
        ));
        let workspace_root = root.join("workspaces");
        let host = NativeRuntimeStorageHost::new(root.clone(), workspace_root);

        host.appendBytes("runtime/state/client.log", b"first\n")
            .expect("first log entry must append");
        host.appendBytes("runtime/state/client.log", b"second\n")
            .expect("second log entry must append");

        assert_eq!(
            host.readBytes("runtime/state/client.log")
                .expect("appended log must be readable"),
            b"first\nsecond\n"
        );

        fs::remove_dir_all(root).expect("temporary runtime storage root must be removed");
    }
}
