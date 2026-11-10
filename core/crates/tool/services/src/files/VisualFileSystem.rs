use std::sync::Arc;

use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, GrepFileMatch, HostError, HttpHost, HttpRequestData,
};

use operit_tools::files::PathMapper::{PathMapper, ResolvedVfsPath};

/// Presents host file APIs through normalized virtual file-system paths.
#[derive(Clone)]
pub struct VisualFileSystem {
    host: Arc<dyn FileSystemHost>,
    mapper: PathMapper,
}

impl VisualFileSystem {
    /// Binds a VFS to one runtime's storage roots and registered mounts.
    #[allow(non_snake_case)]
    pub fn fromContext(context: &operit_host_api::HostManager::HostManager) -> Result<Self, String> {
        let storage = context.runtimeStorageHost.clone().ok_or("RuntimeStorageHost is required for VFS")?;
        let runtimeRoot = storage.runtimeRootDir().ok_or("Runtime storage root is unavailable")?;
        let workspaceRoot = storage.workspaceRootDir().ok_or("Workspace storage root is unavailable")?;
        let host = context.fileSystemHost.clone().ok_or("FileSystemHost is required for VFS")?;
        Ok(Self::new(host, PathMapper::new(runtimeRoot, workspaceRoot, storage)))
    }

    /// Creates a VFS wrapper around a host file system and path mapper.
    pub fn new(host: Arc<dyn FileSystemHost>, mapper: PathMapper) -> Self {
        Self { host, mapper }
    }

    /// Returns the mapper used to translate between VFS and host paths.
    pub fn mapper(&self) -> &PathMapper {
        &self.mapper
    }

    /// Resolves a virtual path into the mapped host path and VFS metadata.
    #[allow(non_snake_case)]
    pub fn resolvePath(&self, path: &str) -> Result<ResolvedVfsPath, String> {
        self.mapper.resolve(path)
    }

    /// Returns the VFS address of a declared runtime storage entry.
    #[allow(non_snake_case)]
    pub fn runtimePath(storagePath: &str) -> Result<String, String> {
        PathMapper::runtimeDataPath(storagePath)
    }

    /// Downloads into a VFS destination while keeping host locators inside this boundary.
    #[allow(non_snake_case)]
    pub fn downloadFile(&self, http: &dyn HttpHost, request: HttpRequestData, path: &str) -> Result<u64, String> {
        let resolved = self.resolvePath(path)?;
        let result = http.downloadToFile(request, resolved.physicalPath).map_err(hostErrorMessage)?;
        Ok(result.downloadedBytes)
    }

    /// Lists child entries for a virtual directory.
    #[allow(non_snake_case)]
    pub fn listFiles(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        if let Some(mut entries) = self.mapper.virtualDirectoryEntries(path)? {
            // Registered namespaces may overlay a built-in native directory
            // (e.g. /virtual/macos/bookmarks). Keep its accessible native children.
            if let Ok(resolved) = self.resolvePath(path) {
                if resolved.nativePath().is_ok() {
                    if let Ok(nativeEntries) = self.host.listFiles(&resolved.physicalPath) {
                        for entry in nativeEntries {
                            if !entries.iter().any(|existing| existing.name == entry.name) {
                                entries.push(entry);
                            }
                        }
                    }
                }
            }
            return Ok(entries);
        }
        let resolved = self.resolvePath(path)?;
        self.host
            .listFiles(&resolved.physicalPath)
            .map_err(hostErrorMessage)
    }

    /// Reads a text file through the mapped host file system.
    #[allow(non_snake_case)]
    pub fn readFile(&self, path: &str) -> Result<String, String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .readFile(&resolved.physicalPath)
            .map_err(hostErrorMessage)
    }

    /// Reads a bounded amount of text from a mapped host file.
    #[allow(non_snake_case)]
    pub fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> Result<String, String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .readFileWithLimit(&resolved.physicalPath, maxBytes)
            .map_err(hostErrorMessage)
    }

    /// Reads raw bytes from a mapped host file.
    #[allow(non_snake_case)]
    pub fn readFileBytes(&self, path: &str) -> Result<Vec<u8>, String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .readFileBytes(&resolved.physicalPath)
            .map_err(hostErrorMessage)
    }

    /// Writes text to a mapped host file.
    #[allow(non_snake_case)]
    pub fn writeFile(&self, path: &str, content: &str, append: bool) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .writeFile(&resolved.physicalPath, content, append)
            .map_err(hostErrorMessage)
    }

    /// Writes raw bytes to a mapped host file.
    #[allow(non_snake_case)]
    pub fn writeFileBytes(&self, path: &str, content: &[u8]) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .writeFileBytes(&resolved.physicalPath, content)
            .map_err(hostErrorMessage)
    }

    /// Deletes a mapped host file or directory.
    #[allow(non_snake_case)]
    pub fn deleteFile(&self, path: &str, recursive: bool) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .deleteFile(&resolved.physicalPath, recursive)
            .map_err(hostErrorMessage)
    }

    /// Reports existence metadata for a virtual or mapped host path.
    #[allow(non_snake_case)]
    pub fn fileExists(&self, path: &str) -> Result<FileExistence, String> {
        if self.mapper.virtualDirectoryEntries(path)?.is_some() {
            return Ok(FileExistence {
                exists: true,
                isDirectory: true,
                size: 0,
            });
        }
        let resolved = self.resolvePath(path)?;
        self.host
            .fileExists(&resolved.physicalPath)
            .map_err(hostErrorMessage)
    }

    /// Moves a file between two mapped host paths.
    #[allow(non_snake_case)]
    pub fn moveFile(&self, source: &str, destination: &str) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .moveFile(&source.physicalPath, &destination.physicalPath)
            .map_err(hostErrorMessage)
    }

    /// Copies a file or directory between two mapped host paths.
    #[allow(non_snake_case)]
    pub fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .copyFile(&source.physicalPath, &destination.physicalPath, recursive)
            .map_err(hostErrorMessage)
    }

    /// Creates a directory through the mapped host file system.
    #[allow(non_snake_case)]
    pub fn makeDirectory(&self, path: &str, createParents: bool) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .makeDirectory(&resolved.physicalPath, createParents)
            .map_err(hostErrorMessage)
    }

    /// Finds files under a mapped path and converts results back to VFS paths.
    #[allow(non_snake_case)]
    pub fn findFiles(&self, request: FindFilesRequest) -> Result<Vec<String>, String> {
        let resolved = self.resolvePath(&request.path)?;
        let physicalRequest = FindFilesRequest {
            path: resolved.physicalPath.clone(),
            ..request
        };
        let files = self
            .host
            .findFiles(physicalRequest)
            .map_err(hostErrorMessage)?;
        files
            .into_iter()
            .map(|path| self.mapper.mapPhysicalChildToVfs(&resolved, &path))
            .collect()
    }

    /// Returns file metadata with the public path rewritten as a VFS path.
    #[allow(non_snake_case)]
    pub fn fileInfo(&self, path: &str) -> Result<FileInfo, String> {
        if self.mapper.virtualDirectoryEntries(path)?.is_some() {
            return Ok(virtualDirectoryInfo(PathMapper::normalizeVfsPath(path)?));
        }
        let resolved = self.resolvePath(path)?;
        let mut info = self
            .host
            .fileInfo(&resolved.physicalPath)
            .map_err(hostErrorMessage)?;
        info.path = resolved.vfsPath;
        Ok(info)
    }

    /// Runs code search under a mapped path and rewrites match paths to VFS paths.
    #[allow(non_snake_case)]
    pub fn grepCode(&self, request: GrepCodeRequest) -> Result<GrepCodeResult, String> {
        let resolved = self.resolvePath(&request.path)?;
        let physicalRequest = GrepCodeRequest {
            path: resolved.physicalPath.clone(),
            ..request
        };
        let mut result = self
            .host
            .grepCode(physicalRequest)
            .map_err(hostErrorMessage)?;
        result.matches = result
            .matches
            .into_iter()
            .map(|mut fileMatch| {
                fileMatch.filePath = self
                    .mapper
                    .mapPhysicalChildToVfs(&resolved, &fileMatch.filePath)?;
                Ok(fileMatch)
            })
            .collect::<Result<Vec<GrepFileMatch>, String>>()?;
        Ok(result)
    }

    /// Compresses mapped source content into a mapped destination archive.
    #[allow(non_snake_case)]
    pub fn zipFiles(&self, source: &str, destination: &str) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .zipFiles(&source.physicalPath, &destination.physicalPath)
            .map_err(hostErrorMessage)
    }

    /// Extracts a mapped archive into a mapped destination directory.
    #[allow(non_snake_case)]
    pub fn unzipFiles(&self, source: &str, destination: &str) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .unzipFiles(&source.physicalPath, &destination.physicalPath)
            .map_err(hostErrorMessage)
    }

    /// Opens a mapped host file with the platform file opener.
    #[allow(non_snake_case)]
    pub fn openFile(&self, path: &str) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .openFile(&resolved.physicalPath)
            .map_err(hostErrorMessage)
    }

    /// Shares a mapped host file through the platform share surface.
    #[allow(non_snake_case)]
    pub fn shareFile(&self, path: &str, title: &str) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .shareFile(&resolved.physicalPath, title)
            .map_err(hostErrorMessage)
    }
}

/// Converts host errors into bridge-facing message strings.
fn hostErrorMessage(error: HostError) -> String {
    error.message
}

/// Builds metadata for synthetic VFS directories.
#[allow(non_snake_case)]
fn virtualDirectoryInfo(path: String) -> FileInfo {
    FileInfo {
        path,
        exists: true,
        fileType: "directory".to_string(),
        size: 0,
        permissions: "rwx".to_string(),
        owner: String::new(),
        group: String::new(),
        lastModified: String::new(),
        rawStatOutput: String::new(),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use operit_host_api::{HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
        HttpDownloadResult, HttpFileDownloadResult, HttpImageDelivery, HttpResponseData, HttpStreamHost,
        HttpStreamOpenedCallback, HttpStreamChunkCallback, HttpStreamClosedCallback};
    use operit_host_native_filesystem::PosixFileSystemHost;
    use operit_host_native_storage::NativeRuntimeStorageHost;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct DownloadHost { calls: AtomicUsize }

    impl HttpStreamHost for DownloadHost {
        /// Rejects stream operations outside this download boundary test.
        fn openHttpByteStream(&self, _: String, _: HttpRequestData, _: HttpStreamOpenedCallback,
            _: HttpStreamChunkCallback, _: HttpStreamClosedCallback) -> HostResult<()> { unreachable!() }
        /// Rejects stream cancellation outside this download boundary test.
        fn closeHttpByteStream(&self, _: &str) -> HostResult<()> { unreachable!() }
    }

    impl HttpHost for DownloadHost {
        /// Declares the test host's ordinary byte delivery capability.
        fn imageDelivery(&self) -> HttpImageDelivery { HttpImageDelivery::Bytes }
        /// Rejects buffered requests so a download cannot silently change transport.
        fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> { unreachable!() }
        /// Writes only the physical destination delivered at the VFS boundary.
        fn downloadToFile(&self, request: HttpRequestData, targetPath: String) -> HostResult<HttpFileDownloadResult> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if request.method != "GET" { return Err(HostError::new("original download failure")); }
            std::fs::write(&targetPath, b"downloaded through VFS").map_err(|error| HostError::new(error.to_string()))?;
            Ok(HttpFileDownloadResult { finalUrl: request.url, targetPath, downloadedBytes: 22 })
        }
        /// Rejects unrelated batch downloads.
        fn downloadFiles(&self, _: HttpDownloadRequest, _: HttpDownloadControl,
            _: HttpDownloadProgressCallback) -> HostResult<HttpDownloadResult> { unreachable!() }
    }

    /// Verifies downloads, ordinary reads and cleanup share the same VFS identity and error boundary.
    #[test]
    fn downloads_share_file_io_paths_and_preserve_host_errors() {
        let (vfs, root) = vfs();
        let http = DownloadHost { calls: AtomicUsize::new(0) };
        let directory = VisualFileSystem::runtimePath("runtime/cache/toolpkg/download-test").unwrap();
        vfs.makeDirectory(&directory, true).unwrap();
        let path = PathMapper::joinVfsPath(&directory, "plugin.toolpkg").unwrap();
        let request = HttpRequestData { url: "https://market.test/asset".into(), method: "GET".into(),
            headers: vec![], body: vec![], formFields: vec![], fileParts: vec![], connectTimeoutSeconds: 1,
            readTimeoutSeconds: 1, followRedirects: true, ignoreSsl: false, proxyHost: String::new(), proxyPort: 0 };
        let bytes = vfs.downloadFile(&http, request.clone(), &path).unwrap();
        assert_eq!(bytes as usize, vfs.readFileBytes(&path).unwrap().len());
        assert_eq!(vfs.readFile(&path).unwrap(), "downloaded through VFS");
        assert!(vfs.downloadFile(&http, request.clone(), "/unknown/download").is_err());
        assert_eq!(http.calls.load(Ordering::SeqCst), 1);
        let mut failed = request; failed.method = "FAIL".into();
        assert_eq!(vfs.downloadFile(&http, failed, &path).unwrap_err(), "original download failure");
        vfs.deleteFile(&directory, true).unwrap();
        assert!(!vfs.fileExists(&path).unwrap().exists);
        std::fs::remove_dir_all(root).unwrap();
    }

    fn vfs() -> (VisualFileSystem, PathBuf) {
        let root = std::env::temp_dir()
            .join(format!("operit-vfs-tools-test-{}", uuid::Uuid::new_v4()));
        let storage = Arc::new(NativeRuntimeStorageHost::new(
            root.clone(),
            root.join("workspaces"),
        ));
        let vfs = VisualFileSystem::new(
            Arc::new(PosixFileSystemHost::new()),
            PathMapper::new(root.clone(), root.join("workspaces"), storage),
        );
        (vfs, root)
    }

    #[test]
    fn resource_inputs_keep_vfs_native_and_relative_compatibility() {
        let (vfs, root) = vfs();
        vfs.makeDirectory("/operitapp/data/uploads", true).unwrap();
        vfs.writeFile("/operitapp/data/uploads/input.txt", "through VFS", false).unwrap();
        for input in ["/operitapp/data/uploads/input.txt", "/app/data/uploads/input.txt", "input.txt", "./input.txt", "../uploads/input.txt"] {
            let path = PathMapper::resolveFileResourcePath(Some("/operitapp/data/uploads"), input).unwrap();
            assert_eq!(vfs.readFile(&path).unwrap(), "through VFS", "{input}");
        }
        let native = root.join("uploads/input.txt").to_string_lossy().into_owned();
        let path = PathMapper::resolveFileResourcePath(None, &native).unwrap();
        assert_eq!(vfs.readFile(&path).unwrap(), "through VFS");
        assert_eq!(PathMapper::resolveFileResourcePath(None, "/").unwrap(), "/");
        assert!(!vfs.listFiles("/").unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn both_root_mounts_share_files_and_preserve_result_paths() {
        let (vfs, root) = vfs();
        vfs.makeDirectory("/app/workspaces/demo", true).unwrap();
        vfs.writeFile("/app/workspaces/demo/file.txt", "shared", false).unwrap();
        assert_eq!(vfs.readFile("/operitapp/workspaces/demo/file.txt").unwrap(), "shared");
        vfs.writeFile("/operitapp/workspaces/demo/file.txt", "updated", false).unwrap();
        assert_eq!(vfs.readFile("/app/workspaces/demo/file.txt").unwrap(), "updated");
        for base in ["/app", "/operitapp"] {
            let path = format!("{base}/workspaces/demo/file.txt");
            assert_eq!(vfs.fileInfo(&path).unwrap().path, path);
            assert_eq!(vfs.fileInfo(base).unwrap().path, base);
        }
        let names: Vec<_> = vfs.listFiles("/").unwrap().into_iter().map(|entry| entry.name).collect();
        assert!(names.iter().any(|name| name == "operitapp"));
        assert!(!names.iter().any(|name| matches!(name.as_str(), "app" | "mnt" | "sdcard" | "data")));
        assert_eq!(vfs.listFiles("/app").unwrap(), vfs.listFiles("/operitapp").unwrap());
        let registry = super::super::MountRegistry::MountRegistry::withStorage(Arc::new(
            NativeRuntimeStorageHost::new(root.clone(), root.join("workspaces")),
        ));
        // Either registry spelling must appear in both directory trees.
        for namespace in ["/mnt/test/folders", "/virtual/test/other"] {
            let mount = registry.register(namespace, "native", root.to_str().unwrap(), "Shared").unwrap();
            assert!(mount.vfsPath().starts_with(namespace));
            let names: Vec<_> = vfs.listFiles("/").unwrap().into_iter().map(|entry| entry.name).collect();
            assert!(names.iter().any(|name| name == "virtual"));
            assert!(!names.iter().any(|name| name == "mnt" || name == "app"));
            let relative = super::super::MountRegistry::mountRelativePath(&mount.vfsPath()).unwrap().to_string();
            let first = format!("/mnt/{relative}/mounted.txt");
            let second = format!("/virtual/{relative}/mounted.txt");
            vfs.writeFile(&first, "mounted", false).unwrap();
            assert_eq!(vfs.readFile(&second).unwrap(), "mounted");
            vfs.writeFile(&second, "changed", false).unwrap();
            assert_eq!(vfs.readFile(&first).unwrap(), "changed");
            for path in [&first, &second] {
                assert_eq!(vfs.fileInfo(path).unwrap().path, *path);
            }
            assert_eq!(vfs.listFiles("/mnt/test").unwrap(), vfs.listFiles("/virtual/test").unwrap());
            registry.remove(&format!("/virtual/{relative}")).unwrap();
        }
        assert_eq!(vfs.listFiles("/mnt").unwrap(), vfs.listFiles("/virtual").unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }


    /// move/copy/file_info resolve VFS paths to the workspace collection root
    /// and keep echoing VFS paths in metadata.
    #[test]
    fn moveCopyFileInfoRoundTripThroughVfsWorkspacePaths() {
        let (vfs, root) = vfs();
        vfs.makeDirectory("/operitapp/workspaces/ws-a", true).unwrap();
        vfs.writeFile("/operitapp/workspaces/ws-a/src.txt", "hello vfs", false).unwrap();

        let info = vfs.fileInfo("/operitapp/workspaces/ws-a/src.txt").unwrap();
        assert!(info.exists);
        assert_eq!(info.path, "/operitapp/workspaces/ws-a/src.txt");
        assert_eq!(info.size, "hello vfs".len() as i64);

        vfs.copyFile("/operitapp/workspaces/ws-a/src.txt", "/operitapp/workspaces/ws-a/copy.txt", false)
            .unwrap();
        assert_eq!(vfs.readFile("/operitapp/workspaces/ws-a/copy.txt").unwrap(), "hello vfs");

        vfs.moveFile("/operitapp/workspaces/ws-a/src.txt", "/operitapp/workspaces/ws-a/renamed.txt")
            .unwrap();
        assert!(!vfs.fileExists("/operitapp/workspaces/ws-a/src.txt").unwrap().exists);
        assert_eq!(vfs.readFile("/operitapp/workspaces/ws-a/renamed.txt").unwrap(), "hello vfs");

        let dirInfo = vfs.fileInfo("/operitapp/workspaces/ws-a").unwrap();
        assert!(dirInfo.exists && dirInfo.fileType == "directory");

        let virtualInfo = vfs.fileInfo("/operitapp/workspaces").unwrap();
        assert!(virtualInfo.exists && virtualInfo.fileType == "directory");
        assert_eq!(virtualInfo.path, "/operitapp/workspaces");

        std::fs::remove_dir_all(root).unwrap();
    }

    /// Mounted platform roots resolve through the same VFS surface on their host.
    #[test]
    #[cfg(target_os = "linux")]
    fn fileInfoResolvesThroughTheLinuxMountRoot() {
        let (vfs, root) = vfs();
        // Address the real temp directory through the /virtual/linux mount root so
        // the physical target stays writable regardless of the active TMPDIR.
        // Some sandboxes refuse file creation directly inside the temp root,
        // so keep the marker inside a directory the host can create.
        let marker = format!(
            "/virtual/linux{}",
            std::env::temp_dir()
                .join(format!("operit-vfs-check-{}", uuid::Uuid::new_v4()))
                .join("marker.txt")
                .display()
        );
        vfs.writeFile(&marker, "mounted", false).unwrap();
        let info = vfs.fileInfo(&marker).unwrap();
        assert!(info.exists);
        assert_eq!(info.path, marker);
        vfs.deleteFile(&marker, false).unwrap();
        // The mount-root write never touches the runtime root, so it may not exist.
        drop(std::fs::remove_dir_all(root));
    }
}
