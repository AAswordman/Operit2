//! Identity-local persistent mount catalog, independent of platform filesystem backends.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use operit_host_api::RuntimeStorageHost;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use operit_host_api::FileSystemResource::FileSystemResource;

pub const MOUNT_SOURCE_PREFIX: &str = "operit-mount:";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MountSource {
    pub namespace: String,
    pub backend: String,
    pub root: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VfsMount {
    pub id: String,
    pub namespace: String,
    pub name: String,
    pub backend: String,
    pub root: String,
}

impl VfsMount {
    pub fn vfsPath(&self) -> String { format!("{}/{}", self.namespace, self.id) }
    fn validate(&self) -> Result<(), String> {
        let parts: Vec<_> = self.namespace.split('/').collect();
        if parts.len() != 4 || parts[0] != "" || parts[1] != "mnt"
            || !parts[2..].iter().all(|s| safeSegment(s))
            || !safeSegment(&self.id)
            || (parts[2] == "windows" && parts[3].len() == 1)
            || ["/mnt/android/root", "/mnt/android/sdcard"].contains(&self.namespace.as_str()) {
            return Err("Mount namespace must be /mnt/<platform>/<kind> and must not shadow a built-in mount".into());
        }
        FileSystemResource { backend: self.backend.clone(), root: self.root.clone(), path: String::new() }
            .validate().map_err(|e| e.to_string())?;
        if self.backend == "native" && !Path::new(&self.root).is_absolute() {
            return Err("Native mount root must be an absolute host path".into());
        }
        if self.backend == "android_documents" {
            let uri = url::Url::parse(&self.root).map_err(|e| e.to_string())?;
            if uri.scheme() != "content" || uri.host_str().is_none() || !uri.path().starts_with("/tree/")
                || uri.query().is_some() || uri.fragment().is_some() {
                return Err("Android document mount requires an authorized content tree URI".into());
            }
        }
        Ok(())
    }
}

fn safeSegment(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}

#[derive(Serialize, Deserialize)]
struct Catalog { version: u32, mounts: Vec<VfsMount> }

const CATALOG_PATH: &str = "runtime/config/vfs_mounts.json";

#[derive(Clone)]
pub struct MountRegistry {
    catalog: PathBuf,
    storage: Option<Arc<dyn RuntimeStorageHost>>,
}

impl std::fmt::Debug for MountRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MountRegistry").field("catalog", &self.catalog).finish_non_exhaustive()
    }
}

impl MountRegistry {
    /// Creates a built-in-only mapper catalog; persistence requires an explicit Host.
    pub(super) fn withoutStorage(runtimeRoot: &Path) -> Self {
        Self { catalog: runtimeRoot.join("config/vfs_mounts.json"), storage: None }
    }

    /// Binds a catalog explicitly to its identity-owned storage host.
    pub fn withStorage(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        let root = storage.runtimeRootDir().unwrap_or_default();
        Self { catalog: root.join("config/vfs_mounts.json"), storage: Some(storage) }
    }
    pub fn list(&self) -> Result<Vec<VfsMount>, String> {
        // Without an installed storage capability, built-in VFS mounts still work.
        let Some(storage) = &self.storage else { return Ok(Vec::new()) };
        if !storage.exists(CATALOG_PATH).map_err(|e| format!("Cannot inspect VFS mounts: {e}"))? {
            return Ok(Vec::new());
        }
        let bytes = storage.readBytes(CATALOG_PATH)
            .map_err(|e| format!("Cannot read VFS mounts: {e}"))?;
        let catalog: Catalog = serde_json::from_slice(&bytes).map_err(|e| format!("Invalid VFS mount catalog: {e}"))?;
        if catalog.version != 1 { return Err("Unsupported VFS mount catalog version".into()) }
        let mut paths = std::collections::HashSet::new();
        for mount in &catalog.mounts {
            mount.validate()?;
            if !paths.insert(mount.vfsPath()) { return Err("Duplicate VFS mount path".into()) }
        }
        Ok(catalog.mounts)
    }
    pub fn register(&self, namespace: &str, backend: &str, root: &str, name: &str) -> Result<VfsMount, String> {
        let _guard = catalogLock().lock().map_err(|_| "VFS mount catalog lock is poisoned")?;
        let candidate = VfsMount { id: Uuid::new_v4().simple().to_string(), namespace: namespace.trim_end_matches('/').into(), name: name.into(), backend: backend.into(), root: root.into() };
        candidate.validate()?;
        let mut mounts = self.list()?;
        if let Some(existing) = mounts.iter().find(|m| m.namespace == candidate.namespace && m.backend == backend && m.root == root) {
            return Ok(existing.clone());
        }
        mounts.push(candidate.clone());
        self.save(mounts)?;
        Ok(candidate)
    }
    pub fn remove(&self, vfsPath: &str) -> Result<(), String> {
        let _guard = catalogLock().lock().map_err(|_| "VFS mount catalog lock is poisoned")?;
        let mut mounts = self.list()?;
        let before = mounts.len();
        mounts.retain(|m| m.vfsPath() != vfsPath.trim_end_matches('/'));
        if before == mounts.len() { return Err("VFS mount not found".into()) }
        self.save(mounts)
    }
    fn save(&self, mounts: Vec<VfsMount>) -> Result<(), String> {
        let storage = self.storage.as_ref().ok_or("VFS mount storage host is not configured for this runtime root")?;
        let bytes = serde_json::to_vec_pretty(&Catalog { version: 1, mounts }).map_err(|e| e.to_string())?;
        storage.writeBytesAtomically(CATALOG_PATH, &bytes).map_err(|e| e.to_string())
    }
}
fn catalogLock() -> &'static Mutex<()> { static LOCK: OnceLock<Mutex<()>> = OnceLock::new(); LOCK.get_or_init(|| Mutex::new(())) }

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::fs;
    struct TempRoot(PathBuf);
    impl TempRoot {
        fn new() -> Self { Self(std::env::temp_dir().join(format!("operit-mount-test-{}", Uuid::new_v4()))) }
        fn registry(&self) -> MountRegistry { testRegistry(&self.0) }
    }
    impl Drop for TempRoot { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
    /// No native paths are read until a matching identity-owned Host is installed.
    #[test]
    fn unavailableStorageKeepsBuiltinMountsUsableButRejectsWrites() {
        let registry = MountRegistry { catalog: PathBuf::from("runtime/config/vfs_mounts.json"), storage: None };
        assert!(registry.list().unwrap().is_empty());
        let error = registry.register("/mnt/test/resources", "test", "opaque", "Test").unwrap_err();
        assert!(error.contains("storage host is not configured"), "{error}");
    }

    /// Models OPFS-like storage that exposes a logical root, not a native filesystem.
    #[derive(Default)]
    struct VirtualStorage {
        bytes: Mutex<Option<Vec<u8>>>,
        rejectWrites: std::sync::atomic::AtomicBool,
    }
    impl RuntimeStorageHost for VirtualStorage {
        fn runtimeRootDir(&self) -> Option<PathBuf> { Some(PathBuf::from("runtime")) }
        fn workspaceRootDir(&self) -> Option<PathBuf> { Some(PathBuf::from("workspaces")) }
        fn readBytes(&self, path: &str) -> operit_host_api::HostResult<Vec<u8>> {
            assert_eq!(path, CATALOG_PATH);
            self.bytes.lock().unwrap().clone().ok_or_else(|| operit_host_api::HostError::new("missing catalog"))
        }
        fn writeBytes(&self, _: &str, _: &[u8]) -> operit_host_api::HostResult<()> {
            panic!("catalog must use atomic publication")
        }
        fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
            assert_eq!(path, CATALOG_PATH);
            if self.rejectWrites.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(operit_host_api::HostError::new("scripted write failure"));
            }
            *self.bytes.lock().unwrap() = Some(content.to_vec());
            Ok(())
        }
        fn appendBytes(&self, _: &str, _: &[u8]) -> operit_host_api::HostResult<()> { unreachable!() }
        fn delete(&self, _: &str, _: bool) -> operit_host_api::HostResult<()> { unreachable!() }
        fn exists(&self, path: &str) -> operit_host_api::HostResult<bool> {
            assert_eq!(path, CATALOG_PATH);
            Ok(self.bytes.lock().unwrap().is_some())
        }
        fn list(&self, _: &str) -> operit_host_api::HostResult<Vec<operit_host_api::RuntimeStorageEntry>> { unreachable!() }
    }

    #[test]
    fn virtualHostCatalogPersistsAcrossRegistriesAndFailedWrites() {
        let host = Arc::new(VirtualStorage::default());
        let registry = MountRegistry::withStorage(host.clone());
        let mount = registry.register("/mnt/web/resources", "test", "opaque", "Web").unwrap();
        let reopened = MountRegistry::withStorage(host.clone());
        assert_eq!(reopened.list().unwrap(), vec![mount.clone()]);
        host.rejectWrites.store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(registry.remove(&mount.vfsPath()).unwrap_err().contains("scripted write failure"));
        assert_eq!(reopened.list().unwrap(), vec![mount.clone()]);
        host.rejectWrites.store(false, std::sync::atomic::Ordering::Relaxed);
        reopened.remove(&mount.vfsPath()).unwrap();
        assert!(registry.list().unwrap().is_empty());
    }

    /// Identical virtual roots are not an identity: the owning Host selects the catalog.
    #[test]
    fn virtualStorageHostsDoNotShareMountResolution() {
        let first = Arc::new(VirtualStorage::default());
        let second = Arc::new(VirtualStorage::default());
        let firstMount = MountRegistry::withStorage(first.clone())
            .register("/mnt/web/resources", "test", "first-root", "First").unwrap();
        let secondMount = MountRegistry::withStorage(second.clone())
            .register("/mnt/web/resources", "test", "second-root", "Second").unwrap();
        let firstMapper = super::super::PathMapper::PathMapper::new("runtime".into(), "workspaces".into())
            .withMountStorage(first);
        let secondMapper = super::super::PathMapper::PathMapper::new("runtime".into(), "workspaces".into())
            .withMountStorage(second);
        assert!(firstMapper.resolve(&firstMount.vfsPath()).is_ok());
        assert!(secondMapper.resolve(&secondMount.vfsPath()).is_ok());
        assert!(firstMapper.resolve(&secondMount.vfsPath()).is_err());
        assert!(secondMapper.resolve(&firstMount.vfsPath()).is_err());
    }

    #[test]
    fn duplicateVirtualCatalogEntriesAreRejected() {
        let host = Arc::new(VirtualStorage::default());
        let registry = MountRegistry::withStorage(host.clone());
        let mount = registry.register("/mnt/web/resources", "test", "opaque", "Web").unwrap();
        *host.bytes.lock().unwrap() = Some(serde_json::to_vec(&Catalog { version: 1, mounts: vec![mount.clone(), mount] }).unwrap());
        assert!(registry.list().unwrap_err().contains("Duplicate VFS mount path"));
    }

    #[cfg(unix)]
    #[test]
    fn nativeCatalogRetainsPrivatePermissions() {
        use std::os::unix::fs::PermissionsExt;
        let root = TempRoot::new();
        let registry = root.registry();
        registry.register("/mnt/test/resources", "test", "opaque", "Test").unwrap();
        assert_eq!(fs::metadata(registry.catalog).unwrap().permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn persistentCatalogIsStableAndIdentityLocal() {
        let first = TempRoot::new();
        let second = TempRoot::new();
        let mount = first.registry().register("/mnt/android/documents", "android_documents", "content://com.termux.documents/tree/opaque%2Fhome", "Project").unwrap();
        assert_eq!(first.registry().list().unwrap(), vec![mount.clone()]);
        assert!(second.registry().list().unwrap().is_empty());
        assert_eq!(first.registry().register("/mnt/android/documents", "android_documents", &mount.root, "New name").unwrap().id, mount.id);
        first.registry().remove(&mount.vfsPath()).unwrap();
        assert!(first.registry().list().unwrap().is_empty());
    }
    #[test]
    fn nativeAndFuturePlatformBackendsUseTheSameCatalog() {
        let temp = TempRoot::new();
        temp.registry().register("/mnt/local/folders", "native", std::env::temp_dir().to_str().unwrap(), "Local").unwrap();
        temp.registry().register("/mnt/macos/bookmarks", "macos_bookmark", "opaque bookmark", "Future").unwrap();
        assert_eq!(temp.registry().list().unwrap().len(), 2);
    }
    #[test]
    fn invalidSourcesAndNamespacesAreRejectedBeforePersistence() {
        let temp = TempRoot::new();
        for ns in ["/mnt", "/mnt/android", "/mnt/android/root", "/mnt/android/sdcard", "/mnt/windows/c", "/mnt/../documents", "/app/data/mounts"] {
            assert!(temp.registry().register(ns, "test", "opaque", "Bad").is_err(), "{ns}");
        }
        for uri in ["/data/data/com.termux", "content://provider/document/id", "https://provider/tree/id", "content://provider/tree/id?x=1"] {
            assert!(temp.registry().register("/mnt/android/documents", "android_documents", uri, "Bad").is_err(), "{uri}");
        }
        assert!(temp.registry().register("/mnt/local/folders", "native", "relative", "Bad").is_err());
        assert!(temp.registry().list().unwrap().is_empty());
    }
    #[test]
    fn corruptCatalogIsAnErrorNotAnEmptyList() {
        let temp = TempRoot::new();
        let registry = temp.registry();
        fs::create_dir_all(registry.catalog.parent().unwrap()).unwrap();
        fs::write(&registry.catalog, "broken").unwrap();
        assert!(registry.list().is_err());
        assert!(registry.register("/mnt/local/folders", "native", "/tmp", "Local").is_err());
    }
    #[test]
    fn concurrentRegistrationDoesNotLoseMounts() {
        let temp = TempRoot::new();
        let registry = temp.registry();
        let jobs: Vec<_> = (0..8).map(|index| {
            let registry = registry.clone();
            std::thread::spawn(move || registry.register("/mnt/test/resources", "test", &format!("root-{index}"), "Test").unwrap())
        }).collect();
        for job in jobs { job.join().unwrap(); }
        assert_eq!(registry.list().unwrap().len(), 8);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) fn testRegistry(root: &Path) -> MountRegistry {
    MountRegistry::withStorage(Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
        root.to_path_buf(), root.join("workspaces"),
    )))
}
