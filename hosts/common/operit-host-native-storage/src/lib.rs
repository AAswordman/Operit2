use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
#[cfg(windows)]
use std::{ffi::OsStr, os::windows::ffi::OsStrExt};

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
        atomicReplace(&self.temporaryPath, &self.targetPath)?;
        Ok(())
    }
}

impl Drop for NativeRuntimeStorageWriteSession {
    /// Removes an unpublished staging file when its write session is abandoned.
    fn drop(&mut self) {
        self.file.take();
        if self.temporaryPath.exists() {
            let _ = fs::remove_file(&self.temporaryPath);
        }
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
    stagingRoot: PathBuf,
    uploads: Arc<Mutex<HashMap<String, NativeArchiveUpload>>>,
}

/// Tracks the declared length and lifecycle state of one current-process archive upload.
#[derive(Clone, Debug)]
struct NativeArchiveUpload {
    expectedByteLength: Option<u64>,
    sealed: bool,
}

impl NativeArchiveStagingHost {
    /// Creates native archive staging beside one runtime data directory.
    pub fn new(runtimeRoot: PathBuf) -> Self {
        Self {
            stagingRoot: Self::stagingRootForRuntimeRoot(&runtimeRoot),
            uploads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Resolves the archive staging root owned by one runtime root.
    fn stagingRootForRuntimeRoot(runtimeRoot: &Path) -> PathBuf {
        let parent = runtimeRoot
            .parent()
            .expect("runtime root must have a parent for archive staging");
        let name = runtimeRoot
            .file_name()
            .expect("runtime root must have a final segment for archive staging");
        parent.join("archive_staging").join(name)
    }

    /// Creates an archive with an explicit upload or growable-export length contract.
    fn createPendingArchive(
        &self,
        archiveId: &str,
        expectedByteLength: Option<u64>,
    ) -> HostResult<()> {
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

    /// Returns the private directory containing staged archive files.
    fn stagingRoot(&self) -> PathBuf {
        self.stagingRoot.clone()
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
        self.createPendingArchive(archiveId, Some(expectedByteLength))
    }

    /// Creates a growable output file for a streamed snapshot export.
    fn createExportArchive(&self, archiveId: &str) -> HostResult<()> {
        self.createPendingArchive(archiveId, None)
    }

    /// Writes a bounded export range without loading the archive into memory.
    fn writeExportArchive(&self, archiveId: &str, offset: u64, chunk: &[u8]) -> HostResult<()> {
        if chunk.len() > 64 * 1024 {
            return Err(HostError::new("Snapshot export chunk exceeds 64 KiB"));
        }
        let uploads = self
            .uploads
            .lock()
            .map_err(|_| HostError::new("Archive staging state is poisoned"))?;
        let upload = uploads
            .get(archiveId)
            .ok_or_else(|| HostError::new("Archive staging ID does not exist"))?;
        if upload.sealed || upload.expectedByteLength.is_some() {
            return Err(HostError::new("Archive is not an active snapshot export"));
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(self.uploadingArchivePath(archiveId)?)?;
        if offset > file.metadata()?.len() {
            return Err(HostError::new(
                "Snapshot write starts beyond the current output",
            ));
        }
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(chunk)?;
        Ok(())
    }

    /// Exposes the sealed staging file as a platform-saveable file reference.
    fn archiveFileReference(&self, archiveId: &str) -> HostResult<String> {
        let uploads = self
            .uploads
            .lock()
            .map_err(|_| HostError::new("Archive staging state is poisoned"))?;
        let upload = uploads
            .get(archiveId)
            .ok_or_else(|| HostError::new("Archive staging ID does not exist"))?;
        if !upload.sealed {
            return Err(HostError::new("Archive must be sealed before saving"));
        }
        self.sealedArchivePath(archiveId)?
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| HostError::new("Snapshot file path is not valid UTF-8"))
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
            .ok_or_else(|| HostError::new("Export archives require ranged writes"))?
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
        if upload
            .expectedByteLength
            .is_some_and(|expected| actualByteLength != expected)
        {
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

    // Enforce non-following workspace access at the actual storage boundary too:
    // recheck components before reads/writes, rather than trusting an earlier listing.
    fn resolveFile(&self, path: &str) -> HostResult<PathBuf> {
        let resolved = self.resolve(path)?;
        let segments = normalizeStoragePath(path)?;
        if segments.first().map(String::as_str) == Some("workspaces") {
            let mut current = self.workspaceRoot.clone();
            for segment in segments.iter().skip(1) {
                current.push(segment);
                match fs::symlink_metadata(&current) {
                    Ok(metadata) if isStorageLink(&metadata) =>
                        return Err(HostError::new("Workspace storage does not follow links")),
                    Ok(metadata) if !metadata.is_dir() && !metadata.is_file() =>
                        return Err(HostError::new("Workspace storage requires a regular file or directory")),
                    Ok(_) => {},
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                    Err(error) => return Err(error.into()),
                }
            }
        }
        Ok(resolved)
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

    /// Opens a staging writer, optionally retaining identity-private catalog permissions.
    fn createWriteSessionInternal(
        &self,
        path: &str,
        privateCatalog: bool,
    ) -> HostResult<Box<dyn RuntimeStorageWriteSession>> {
        let targetPath = self.resolveFile(path)?;
        let parent = targetPath
            .parent()
            .ok_or_else(|| HostError::new("Runtime storage file has no parent directory"))?;
        fs::create_dir_all(storageFsPath(parent)?)?;
        let temporaryPath = Self::writeTemporaryPath(&targetPath)?;
        let mut options = fs::OpenOptions::new();
        options.create_new(true).write(true);
        if privateCatalog {
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
        }
        let file = options.open(storageFsPath(&temporaryPath)?)?;
        Ok(Box::new(NativeRuntimeStorageWriteSession {
            targetPath,
            temporaryPath,
            file: Some(file),
        }))
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
        self.createWriteSessionInternal(path, false)
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
        Ok(fs::read(self.resolveFile(path)?)?)
    }

    /// Reads one bounded byte range from native runtime storage.
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        let mut file = fs::File::open(self.resolveFile(path)?)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; length];
        let count = file.read(&mut bytes)?;
        bytes.truncate(count);
        Ok(bytes)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let mut session = self.createWriteSession(path)?;
        session.writeChunk(content)?;
        session.commitFast()
    }

    /// Flushes and atomically replaces an identity-private catalog.
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let mut session = self.createWriteSessionInternal(path, true)?;
        session.writeChunk(content)?;
        session.commit()
    }

    /// Appends bytes to a native runtime storage file.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolveFile(path)?;
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
        let path = self.resolveFile(path)?;
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
        let resolved = self.resolve(path)?;
        if normalizeStoragePath(path)?.first().map(String::as_str) == Some("workspaces") {
            // A dangling link is still an occupied destination, never a new writable file.
            return match fs::symlink_metadata(resolved) {
                Ok(_) => Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error.into()),
            };
        }
        Ok(resolved.exists())
    }

    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let directory = self.resolve(prefix)?;
        let mut entries = Vec::new();
        let managedWorkspace = normalizeStoragePath(prefix)?.first().map(String::as_str)
            == Some("workspaces");
        if managedWorkspace {
            // Never descend into links, even when a caller lists a linked directory directly.
            // The configured workspace root itself is trusted; only its descendants are scanned.
            let relative = directory.strip_prefix(&self.workspaceRoot)
                .map_err(|_| HostError::new("Workspace listing escaped its root"))?;
            let mut current = self.workspaceRoot.clone();
            for component in relative.components() {
                current.push(component);
                match fs::symlink_metadata(&current) {
                    Ok(metadata) if isStorageLink(&metadata) => return Ok(entries),
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
                    Err(error) => return Err(error.into()),
                }
            }
        }
        if !directory.exists() {
            return Ok(entries);
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = if managedWorkspace {
                // The host's atomic writer stages a private sibling until commit.
                // Never publish that transient file as user workspace content.
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with('.') && name.strip_suffix(".partial")
                    .and_then(|stem| stem.rsplit_once('.'))
                    .is_some_and(|(_, id)| uuid::Uuid::parse_str(id).is_ok()) {
                    continue;
                }
                let metadata = fs::symlink_metadata(entry.path())?;
                // Do not copy link targets or try reading sockets, devices and FIFOs as files.
                if isStorageLink(&metadata) || (!metadata.is_dir() && !metadata.is_file()) {
                    continue;
                }
                metadata
            } else {
                entry.metadata()?
            };
            entries.push(RuntimeStorageEntry {
                path: self.storagePathForPhysical(&entry.path())?,
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
            });
        }
        Ok(entries)
    }
}

/// Detects symbolic links and Windows junctions without resolving their targets.
#[allow(non_snake_case)]
fn isStorageLink(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        return metadata.file_attributes() & 0x400 != 0; // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    false
}

/// Atomically publishes one sibling staging file over its target path.
#[cfg(not(windows))]
#[allow(non_snake_case)]
fn atomicReplace(source: &Path, target: &Path) -> HostResult<()> {
    fs::rename(source, target)?;
    Ok(())
}

/// Atomically publishes one sibling staging file over its target path on Windows.
#[cfg(windows)]
#[allow(non_snake_case)]
fn atomicReplace(source: &Path, target: &Path) -> HostResult<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let sourcePath = windowsExtendedPath(source)?;
    let targetPath = windowsExtendedPath(target)?;
    let source = widePath(sourcePath.as_os_str());
    let target = widePath(targetPath.as_os_str());
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

/// Converts a storage path into the representation required by local file APIs.
#[cfg(not(windows))]
#[allow(non_snake_case)]
fn storageFsPath(path: &Path) -> HostResult<PathBuf> {
    Ok(path.to_path_buf())
}

/// Converts a storage path into extended-length Windows syntax for local file APIs.
#[cfg(windows)]
#[allow(non_snake_case)]
fn storageFsPath(path: &Path) -> HostResult<PathBuf> {
    windowsExtendedPath(path)
}

/// Converts one Windows path to extended-length syntax.
#[cfg(windows)]
#[allow(non_snake_case)]
fn windowsExtendedPath(path: &Path) -> HostResult<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let text = absolute.to_string_lossy();
    if text.starts_with(r"\\?\") || text.starts_with(r"\??\") {
        return Ok(absolute);
    }
    if text.starts_with(r"\\") {
        return Ok(PathBuf::from(format!(
            r"\\?\UNC\{}",
            text.trim_start_matches('\\')
        )));
    }
    Ok(PathBuf::from(format!(r"\\?\{}", text)))
}

/// Encodes one Windows path as a null-terminated UTF-16 string.
#[cfg(windows)]
#[allow(non_snake_case)]
fn widePath(path: &OsStr) -> Vec<u16> {
    path.encode_wide().chain(std::iter::once(0)).collect()
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

    /// Aborts the exact native host transaction and propagates SQLite rollback errors.
    fn rollback(self: Box<Self>) -> HostResult<()> {
        self.transaction
            .rollback()
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

    use operit_host_api::{ArchiveStagingHost, RuntimeStorageHost, RuntimeStorageWriteHost};

    use super::{NativeArchiveStagingHost, NativeRuntimeStorageHost};

    /// Verifies that snapshot outputs grow, accept ZIP header updates, and expose sealed files.
    #[test]
    fn snapshot_export_is_file_backed_and_seekable() {
        let root = std::env::temp_dir().join(format!("operit-snapshot-export-{}", uuid::Uuid::new_v4()));
        let host = NativeArchiveStagingHost::new(root.join("runtime"));
        host.createExportArchive("snapshot").unwrap();
        assert!(host.archiveFileReference("snapshot").is_err());
        assert!(host.appendArchive("snapshot", b"invalid").is_err());
        let chunk = vec![42; 64 * 1024];
        for index in 0..16 {
            host.writeExportArchive("snapshot", index * chunk.len() as u64, &chunk).unwrap();
        }
        host.writeExportArchive("snapshot", 0, b"PK").unwrap();
        assert_eq!(host.sealArchive("snapshot").unwrap(), 16 * chunk.len() as u64);
        assert_eq!(host.readArchive("snapshot", 0, 4).unwrap(), [b'P', b'K', 42, 42]);
        let reference = host.archiveFileReference("snapshot").unwrap();
        assert_eq!(fs::metadata(&reference).unwrap().len(), 16 * chunk.len() as u64);
        assert!(host.writeExportArchive("snapshot", 0, b"invalid").is_err());
        host.removeArchive("snapshot").unwrap();
        assert!(!std::path::Path::new(&reference).exists());
        fs::remove_dir_all(root).unwrap();
    }

    /// Rejects oversized export writes and keeps exact-length upload validation unchanged.
    #[test]
    fn snapshot_export_rejects_invalid_writes_without_changing_upload_contracts() {
        let root = std::env::temp_dir().join(format!("operit-snapshot-ranges-{}", uuid::Uuid::new_v4()));
        let host = NativeArchiveStagingHost::new(root.join("runtime"));
        host.createExportArchive("snapshot").unwrap();
        assert!(host.writeExportArchive("snapshot", 1, b"gap").is_err());
        assert!(host.writeExportArchive("snapshot", 0, &vec![0; 65537]).is_err());
        assert_eq!(host.sealArchive("snapshot").unwrap(), 0);
        host.createArchive("upload", 4).unwrap();
        assert!(host.writeExportArchive("upload", 0, b"data").is_err());
        host.appendArchive("upload", b"ab").unwrap();
        assert!(host.sealArchive("upload").is_err());
        host.appendArchive("upload", b"cd").unwrap();
        assert_eq!(host.sealArchive("upload").unwrap(), 4);
        host.removeArchive("snapshot").unwrap();
        host.removeArchive("upload").unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn workspace_listing_skips_file_directory_broken_and_cyclic_links() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!("operit-workspace-links-{}", uuid::Uuid::new_v4()));
        let workspace = root.join("workspaces");
        let body = workspace.join("project");
        let outside = root.join("outside");
        fs::create_dir_all(&body).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(body.join("normal.txt"), b"portable").unwrap();
        fs::write(outside.join("secret.txt"), b"local only").unwrap();
        symlink(outside.join("secret.txt"), body.join("file-link")).unwrap();
        symlink(&outside, body.join("directory-link")).unwrap();
        symlink(&body, body.join("loop")).unwrap();
        symlink(body.join("missing"), body.join("broken-link")).unwrap();
        symlink(&outside, workspace.join("linked-workspace")).unwrap();
        let host = NativeRuntimeStorageHost::new(root.join("runtime"), workspace);
        let entries = host.list("workspaces/project").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "workspaces/project/normal.txt");
        assert_eq!(host.list("workspaces").unwrap().len(), 1);
        for path in ["workspaces/project/loop", "workspaces/project/loop/loop",
            "workspaces/project/directory-link", "workspaces/linked-workspace"] {
            assert!(host.list(path).unwrap().is_empty(), "{path}");
        }
        assert!(host.writeBytes("workspaces/project/file-link", b"overwrite").is_err());
        assert!(host.writeBytes("workspaces/project/directory-link/new.txt", b"overwrite").is_err());
        assert!(host.writeBytes("workspaces/project/broken-link", b"overwrite").is_err());
        assert!(host.readBytes("workspaces/project/file-link").is_err());
        assert!(!outside.join("new.txt").exists());
        assert_eq!(fs::read(outside.join("secret.txt")).unwrap(), b"local only");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_listing_hides_uncommitted_atomic_write_staging() {
        let root = std::env::temp_dir().join(format!("operit-workspace-stage-{}", uuid::Uuid::new_v4()));
        let host = NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces"));
        let mut session = host.createWriteSession("workspaces/project/data.txt").unwrap();
        session.writeChunk(b"new").unwrap();
        assert!(host.list("workspaces/project").unwrap().is_empty());
        session.commitFast().unwrap();
        let files = host.list("workspaces/project").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "workspaces/project/data.txt");
        fs::remove_dir_all(root).unwrap();
    }

    /// Verifies that a native staged archive rejects excess bytes and seals only at its declared length.
    #[test]
    fn archive_staging_enforces_declared_length() {
        let root =
            std::env::temp_dir().join(format!("operit-archive-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("temporary archive test root must be created");
        let host = NativeArchiveStagingHost::new(root.clone());
        let stagingRoot = host.stagingRoot();

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
        fs::remove_dir_all(stagingRoot).expect("temporary archive staging root must be removed");
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

    /// Verifies staged runtime writes keep the previous file visible until atomic publication.
    #[test]
    fn runtime_storage_replaces_files_atomically() {
        let root = std::env::temp_dir().join(format!(
            "operit-runtime-storage-atomic-test-{}",
            uuid::Uuid::new_v4()
        ));
        let workspace_root = root.join("workspaces");
        let host = NativeRuntimeStorageHost::new(root.clone(), workspace_root);
        let path = "runtime/config/preferences/atomic.preferences.json";
        host.writeBytes(path, b"previous")
            .expect("initial file must be written");

        let mut session = host
            .createWriteSession(path)
            .expect("replacement session must open");
        session
            .writeChunk(b"replacement")
            .expect("replacement content must stage");
        assert_eq!(
            host.readBytes(path)
                .expect("published file must remain readable while staging"),
            b"previous"
        );
        session
            .commitFast()
            .expect("replacement content must publish atomically");
        assert_eq!(
            host.readBytes(path)
                .expect("replacement file must be readable"),
            b"replacement"
        );

        fs::remove_dir_all(root).expect("temporary runtime storage root must be removed");
    }
}
