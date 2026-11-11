use operit_host_contract::{FileInfo, FileSource, FileTarget, HostError, HostResult};
use serde::{Deserialize, Serialize};

use crate::PathMapper::PathMapper;
use crate::VisualFileSystem::VisualFileSystem;

/// A validated absolute virtual address; serialized values are validated on input.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct VfsPath(String);

impl VfsPath {
    /// Returns the virtual root shared by managed workspaces.
    pub fn workspaceCollectionPath() -> &'static str {
        PathMapper::workspaceCollectionPath()
    }
    /// Creates a virtual workspace address from its validated identifier.
    pub fn workspacePath(id: &str) -> Result<String, String> {
        PathMapper::workspacePath(id)
    }
    /// Normalizes a virtual address without accessing a backend.
    pub fn normalizeVfsPath(path: &str) -> Result<String, String> {
        PathMapper::normalizeVfsPath(path)
    }
    /// Addresses an entry in the canonical runtime storage namespace.
    pub fn runtimeDataPath(path: &str) -> Result<String, String> {
        PathMapper::runtimeDataPath(path)
    }
    /// Converts supported virtual aliases to their canonical public identity.
    pub fn canonicalizeVfsPath(path: &str) -> Result<String, String> {
        PathMapper::canonicalizeVfsPath(path)
    }
    /// Normalizes workspace ingress paths without returning a native locator.
    pub fn normalizeWorkspaceBindingPath(path: &str) -> Result<String, String> {
        PathMapper::normalizeWorkspaceBindingPath(path)
    }
    /// Resolves a workspace resource into the virtual namespace.
    pub fn resolveWorkspaceResourcePath(base: &str, path: &str) -> Result<String, String> {
        PathMapper::resolveWorkspaceResourcePath(base, path)
    }
    /// Resolves a tool argument relative to its explicit current directory.
    pub fn resolveFileToolPath(directory: Option<&str>, path: &str) -> Result<String, String> {
        PathMapper::resolveFileToolPath(directory, path)
    }
    /// Normalizes an external file resource into a virtual address.
    pub fn resolveFileResourcePath(directory: Option<&str>, path: &str) -> Result<String, String> {
        PathMapper::resolveFileResourcePath(directory, path)
    }
    /// Validates a path relative to an existing virtual directory.
    pub fn normalizeRelativePath(path: &str) -> Result<String, String> {
        PathMapper::normalizeRelativePath(path)
    }
    /// Joins virtual segments without inspecting platform files.
    pub fn joinVfsPath(base: &str, path: &str) -> Result<String, String> {
        PathMapper::joinVfsPath(base, path)
    }
    /// Returns a descendant path under a virtual root with alias-aware comparison.
    pub fn relativePath(root: &str, path: &str) -> Result<Option<String>, String> {
        PathMapper::relativePath(root, path)
    }
    /// Expresses a virtual file relative to a current directory, including sibling paths.
    pub fn relativeFileToolPath(base: &str, path: &str) -> Result<String, String> {
        PathMapper::relativeFileToolPath(base, path)
    }

    /// Normalizes a virtual address before it enters a file capability.
    pub fn new(path: &str) -> Result<Self, String> {
        if path.contains('\0') {
            return Err("VFS paths must not contain a null byte".into());
        }
        Ok(Self(PathMapper::normalizeVfsPath(path)?))
    }

    /// Borrows the portable path used by persisted records and host requests.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for VfsPath {
    type Error = String;

    /// Validates serialized paths rather than trusting their original representation.
    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::new(&path)
    }
}

impl From<VfsPath> for String {
    /// Serializes only the validated virtual address.
    fn from(path: VfsPath) -> Self {
        path.0
    }
}

/// A file capability bound to its owning runtime; backend locations stay private.
#[derive(Clone)]
pub struct FileReference {
    path: VfsPath,
    filesystem: VisualFileSystem,
}

impl FileReference {
    /// Captures a validated virtual file without resolving it to an operating-system path.
    pub(crate) fn new(filesystem: VisualFileSystem, path: &str) -> Result<Self, String> {
        Ok(Self {
            path: VfsPath::new(path)?,
            filesystem,
        })
    }
}

impl FileSource for FileReference {
    /// Keeps the file's public identity portable across platform storage roots.
    fn vfsPath(&self) -> &str {
        self.path.as_str()
    }

    /// Reads using the exact filesystem that owns this reference.
    fn readBytes(&self) -> HostResult<Vec<u8>> {
        self.filesystem
            .readFileBytes(self.path.as_str())
            .map_err(HostError::new)
    }

    /// Bounds reads at the backend before returning data to the host consumer.
    fn readBytesRange(&self, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        self.filesystem
            .readFileBytesRange(self.path.as_str(), offset, length)
            .map_err(HostError::new)
    }

    /// Returns virtual metadata instead of propagating backend search paths.
    fn fileInfo(&self) -> HostResult<FileInfo> {
        self.filesystem
            .fileInfo(self.path.as_str())
            .map_err(HostError::new)
    }

    /// Resolves native interoperation only when the platform consumer requests it.
    fn nativePath(&self) -> HostResult<String> {
        self.filesystem
            .resolvePath(self.path.as_str())
            .map_err(HostError::new)?
            .nativePath()
            .map_err(HostError::new)
    }
}

impl FileTarget for FileReference {
    /// Keeps writable destinations in the same portable namespace as file sources.
    fn vfsPath(&self) -> &str {
        self.path.as_str()
    }

    /// Writes through the original backend, including document-backed mounts.
    fn writeBytes(&self, bytes: &[u8]) -> HostResult<()> {
        self.filesystem
            .writeFileBytes(self.path.as_str(), bytes)
            .map_err(HostError::new)
    }

    /// Exposes native paths only to platform operations that require native interoperation.
    fn nativePath(&self) -> HostResult<String> {
        FileSource::nativePath(self)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use operit_host_contract::*;
    use operit_host_native_filesystem::PosixFileSystemHost;
    use operit_host_native_storage::NativeRuntimeStorageHost;
    use std::{path::PathBuf, sync::Arc};

    /// Creates an independent runtime with a real backend for capability identity checks.
    fn filesystem() -> (VisualFileSystem, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("operit-vfs-reference-{}", uuid::Uuid::new_v4()));
        let storage = Arc::new(NativeRuntimeStorageHost::new(
            root.clone(),
            root.join("workspaces"),
        ));
        let files = VisualFileSystem::new(
            Arc::new(PosixFileSystemHost::new()),
            PathMapper::new(root.clone(), root.join("workspaces"), storage),
        );
        files.makeDirectory("/operitapp/data/files", true).unwrap();
        (files, root)
    }

    /// Ensures equal virtual addresses never resolve through another runtime's backend.
    #[test]
    fn references_retain_runtime_identity_and_bounded_reads() {
        let (first, firstRoot) = filesystem();
        let (second, secondRoot) = filesystem();
        let path = "/operitapp/data/files/shared.txt";
        first.writeFileBytes(path, b"first-runtime").unwrap();
        second.writeFileBytes(path, b"second-runtime").unwrap();
        let file = first.file(path).unwrap();
        drop(first);
        assert_eq!(file.readBytes().unwrap(), b"first-runtime");
        assert_eq!(file.readBytesRange(6, 4).unwrap(), b"runt");
        assert_eq!(file.readBytesRange(100, 4).unwrap(), b"");
        assert_eq!(file.fileInfo().unwrap().path, path);
        assert_eq!(
            second.file(path).unwrap().readBytes().unwrap(),
            b"second-runtime"
        );
        FileTarget::writeBytes(&file, b"changed").unwrap();
        assert_eq!(file.readBytes().unwrap(), b"changed");
        assert_eq!(second.readFileBytes(path).unwrap(), b"second-runtime");
        std::fs::remove_dir_all(firstRoot).unwrap();
        std::fs::remove_dir_all(secondRoot).unwrap();
    }

    /// Keeps settings locations equivalent to application aliases without exposing a resolver to Core.
    #[test]
    fn storage_management_locations_share_files_and_containment() {
        let (files, root) = filesystem();
        let locations = files.storageLocations().unwrap();
        let location = locations
            .iter()
            .find(|location| location.vfsPath == "/operitapp/data")
            .unwrap();
        assert_eq!(std::path::Path::new(&location.displayPath), root);
        let child = VfsPath::joinVfsPath(&location.locationVfsPath, "files/selected.txt").unwrap();
        files.writeFileBytes(&child, b"selected-location").unwrap();
        assert_eq!(
            files
                .readFileBytes("/operitapp/data/files/selected.txt")
                .unwrap(),
            b"selected-location"
        );
        assert!(files.isWithin("/operitapp/data", &child).unwrap());
        assert!(files
            .isWithin(&location.locationVfsPath, "/operitapp/data/files")
            .unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Validates addresses after deserialization so callers cannot forge a relative capability.
    #[test]
    fn serialized_paths_are_validated_and_normalized() {
        let path = VfsPath::new("/operitapp//data/files/").unwrap();
        assert_eq!(
            serde_json::to_string(&path).unwrap(),
            "\"/operitapp/data/files\""
        );
        for value in [
            "relative.txt",
            "D:/native.txt",
            "phone:/operitapp/data/a",
            "/app/../a",
            "/app/./a",
            "/app/\0a",
        ] {
            assert!(
                serde_json::from_value::<VfsPath>(serde_json::json!(value)).is_err(),
                "{value:?}"
            );
        }
    }

    struct ResourceHost;

    /// Rejects unrelated backend calls rather than hiding accidental native access.
    macro_rules! unused_operations {
        ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
            $(/// Rejects operations outside the resource capability test.
            fn $name(&self, $(_: $ty),*) -> HostResult<$result> { unreachable!(stringify!($name)) })*
        };
    }

    impl FileSystemHost for ResourceHost {
        /// Reads an opaque resource through its selected backend.
        fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            let resource = FileSystemResource::FileSystemResource::parse(path)?.unwrap();
            assert_eq!(resource.root, "opaque-grant");
            assert_eq!(resource.path, "image.png");
            Ok(b"resource-bytes".to_vec())
        }
        /// Identifies this resource-only backend.
        fn envLabel(&self) -> &str {
            "resource-test"
        }
        /// Rejects descriptor queries unrelated to the bound file capability.
        fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
            unreachable!()
        }
        unused_operations! {
            validatePath(path: &str, paramName: &str) -> ();
            listFiles(path: &str) -> Vec<FileEntry>;
            readFile(path: &str) -> String;
            readFileWithLimit(path: &str, maxBytes: usize) -> String;
            writeFile(path: &str, content: &str, append: bool) -> ();
            writeFileBytes(path: &str, bytes: &[u8]) -> ();
            deleteFile(path: &str, recursive: bool) -> ();
            fileExists(path: &str) -> FileExistence;
            moveFile(source: &str, target: &str) -> ();
            copyFile(source: &str, target: &str, recursive: bool) -> ();
            makeDirectory(path: &str, parents: bool) -> ();
            findFiles(request: FindFilesRequest) -> Vec<String>;
            fileInfo(path: &str) -> FileInfo;
            grepCode(request: GrepCodeRequest) -> GrepCodeResult;
            zipFiles(source: &str, target: &str) -> ();
            unzipFiles(source: &str, target: &str) -> ();
            openFile(path: &str) -> ();
            shareFile(path: &str, title: &str) -> ();
        }
    }

    /// Resource-backed reads must work without requiring a native location.
    #[test]
    fn opaque_resources_can_be_consumed_without_native_paths() {
        let (_, root) = filesystem();
        let storage = Arc::new(NativeRuntimeStorageHost::new(
            root.clone(),
            root.join("workspaces"),
        ));
        let mount = crate::MountRegistry::MountRegistry::withStorage(storage.clone())
            .register(
                "/virtual/test/documents",
                "documents",
                "opaque-grant",
                "Images",
            )
            .unwrap();
        let files = VisualFileSystem::new(
            Arc::new(ResourceHost),
            PathMapper::new(root.clone(), root.join("workspaces"), storage),
        );
        let file = files
            .file(&format!("{}/image.png", mount.vfsPath()))
            .unwrap();
        assert_eq!(file.readBytes().unwrap(), b"resource-bytes");
        assert!(FileSource::nativePath(&file)
            .unwrap_err()
            .message
            .contains("document/resource mount"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
