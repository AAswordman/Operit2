//! Small node-local namespace, atomically persisted inside the app sandbox.
use operit_edge_sdk::host::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
use std::{collections::BTreeMap, fs, io::Write, path::PathBuf, sync::Mutex};

const MAX_SNAPSHOT_BYTES: usize = 2 * 1024 * 1024;
const MAX_FILES: usize = 128;
type Files = BTreeMap<String, Vec<u8>>;

pub struct PhoneStorage {
    root: PathBuf,
    files: Mutex<Files>,
}

impl PhoneStorage {
    /// Loads the committed namespace; invalid state is an error, never a new identity.
    pub fn open(root: PathBuf) -> HostResult<Self> {
        fs::create_dir_all(&root)?;
        let path = root.join("node.json");
        let files = match fs::File::open(path) {
            Ok(file) => {
                if file.metadata()?.len() > MAX_SNAPSHOT_BYTES as u64 {
                    return Err(HostError::new("Edge node storage exceeds its byte budget"));
                }
                let files: Files = serde_json::from_reader(file).map_err(error)?;
                if files.len() > MAX_FILES {
                    return Err(HostError::new("Edge node storage exceeds its entry budget"));
                }
                for path in files.keys() {
                    validate(path)?;
                }
                files
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Files::new(),
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            root,
            files: Mutex::new(files),
        })
    }

    /// Publishes a complete namespace before changing the in-memory committed state.
    fn update(&self, edit: impl FnOnce(&mut Files) -> HostResult<()>) -> HostResult<()> {
        let mut committed = self.files.lock().map_err(error)?;
        let mut next = committed.clone();
        edit(&mut next)?;
        if next.len() > MAX_FILES {
            return Err(HostError::new("Too many Edge node storage entries"));
        }
        let bytes = serde_json::to_vec(&next).map_err(error)?;
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err(HostError::new("Edge node storage is full"));
        }
        let pending = self.root.join("node.pending");
        let mut file = fs::File::create(&pending)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(pending, self.root.join("node.json"))?;
        *committed = next;
        // Android/Linux requires the parent directory to be flushed after rename.
        #[cfg(unix)]
        fs::File::open(&self.root)?.sync_all()?;
        Ok(())
    }
}

/// Accepts relative SDK storage keys without exposing filesystem paths.
fn validate(path: &str) -> HostResult<()> {
    if path.is_empty()
        || path.len() > 512
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(HostError::new("Invalid Edge storage key"));
    }
    Ok(())
}

/// Converts codec and synchronization errors to the shared platform error type.
fn error(value: impl std::fmt::Display) -> HostError {
    HostError::new(value.to_string())
}

impl RuntimeStorageHost for PhoneStorage {
    /// Makes accidental business-journal access fail at its actual call site in tests.
    #[cfg(test)]
    fn openTransactionDatabase(
        &self,
        _path: &str,
    ) -> HostResult<Box<dyn operit_edge_sdk::host::RuntimeSqliteConnection>> {
        panic!("Phone Edge must not open a SQL journal");
    }
    /// Uses the SDK's bounded endpoint projection and releases idle parsed caches.
    fn retainPreferencesCache(&self) -> bool {
        false
    }
    /// Keeps bounded node history rather than full business replica history.
    fn nodeLocalApplicationHistoryLimit(&self) -> Option<usize> {
        Some(8)
    }
    /// Keeps physical storage private to this product's platform adapter.
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        None
    }
    /// This device does not own a Core workspace.
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        None
    }
    /// Reads exactly one committed logical file.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        validate(path)?;
        self.files
            .lock()
            .map_err(error)?
            .get(path)
            .cloned()
            .ok_or_else(|| HostError::new(format!("Edge storage entry does not exist: {path}")))
    }
    /// Reads bounded control-record slices without cloning the complete logical file.
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        validate(path)?;
        let files = self.files.lock().map_err(error)?;
        let bytes = files
            .get(path)
            .ok_or_else(|| HostError::new("Edge storage entry does not exist"))?;
        if offset >= bytes.len() as u64 {
            return Ok(Vec::new());
        }
        let start = usize::try_from(offset).map_err(error)?;
        Ok(bytes[start..start.saturating_add(length).min(bytes.len())].to_vec())
    }
    /// Publishes one complete logical file atomically.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        validate(path)?;
        if content.len() > MAX_SNAPSHOT_BYTES {
            return Err(HostError::new("Edge storage entry is too large"));
        }
        self.update(|files| {
            files.insert(path.to_owned(), content.to_vec());
            Ok(())
        })
    }
    /// Uses the same atomic persistence contract for SDK preference transactions.
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.writeBytes(path, content)
    }
    /// Appends within the bounded namespace and commits the resulting complete file.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        validate(path)?;
        self.update(|files| {
            let file = files.entry(path.to_owned()).or_default();
            if content.len() > MAX_SNAPSHOT_BYTES.saturating_sub(file.len()) {
                return Err(HostError::new("Edge storage entry is too large"));
            }
            file.extend_from_slice(content);
            Ok(())
        })
    }
    /// Every publication already flushes the complete backing file.
    fn flushFile(&self, path: &str) -> HostResult<()> {
        if !self.exists(path)? {
            return Err(HostError::new("Edge storage entry does not exist"));
        }
        Ok(())
    }
    /// Removes a file or an explicitly selected logical subtree.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        validate(path)?;
        let prefix = format!("{path}/");
        self.update(|files| {
            if !recursive && files.keys().any(|key| key.starts_with(&prefix)) {
                return Err(HostError::new("Edge storage directory is not empty"));
            }
            files.retain(|key, _| key != path && !(recursive && key.starts_with(&prefix)));
            Ok(())
        })
    }
    /// Checks a committed key or logical directory.
    fn exists(&self, path: &str) -> HostResult<bool> {
        validate(path)?;
        let files = self.files.lock().map_err(error)?;
        Ok(
            files.contains_key(path)
                || files.keys().any(|key| key.starts_with(&format!("{path}/"))),
        )
    }
    /// Lists committed files without exposing private staging files.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        if !prefix.is_empty() {
            validate(prefix.trim_end_matches('/'))?;
        }
        let directory = format!("{}/", prefix.trim_end_matches('/'));
        Ok(self
            .files
            .lock()
            .map_err(error)?
            .iter()
            .filter(|(path, _)| {
                prefix.is_empty() || path.as_str() == prefix || path.starts_with(&directory)
            })
            .map(|(path, bytes)| RuntimeStorageEntry {
                path: path.clone(),
                isDirectory: false,
                size: bytes.len() as i64,
            })
            .collect())
    }
}
