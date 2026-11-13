use std::sync::Arc;

use operit_host_contract::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, GrepFileMatch, HostError, HttpHost, HttpRequestData,
};

use crate::PathMapper::{PathMapper, ResolvedVfsPath};

/// Presents host file APIs through normalized virtual file-system paths.
#[derive(Clone)]
pub struct VisualFileSystem {
    host: Arc<dyn FileSystemHost>,
    mapper: PathMapper,
}

impl VisualFileSystem {
    /// Observes successful VFS writes, removals and replacements under one directory.
    pub fn watchChanges(&self, root: &str, changed: Arc<dyn Fn() + Send + Sync>) -> Result<crate::FileChanges::FileChangeSubscription, String> {
        Ok(crate::FileChanges::subscribe(self.resolvePath(root)?.physicalPath, changed))
    }

    /// Validates a virtual address at the backend boundary without exposing its locator.
    pub fn validatePath(&self, path: &str, parameter: &str) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .validatePath(&resolved.physicalPath, parameter)
            .map_err(hostErrorMessage)
    }

    /// Opens a backend-owned append handle while keeping the file address virtual.
    pub fn openAppendFile(
        &self,
        path: &str,
    ) -> Result<Box<dyn operit_host_contract::FileAppendHandle>, String> {
        let resolved = self.resolvePath(path)?;
        let handle = self.host
            .openAppendFile(&resolved.physicalPath)
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&resolved.physicalPath])?;
        Ok(Box::new(ObservedAppend { handle, path: resolved.physicalPath }))
    }

    /// Describes the filesystem backend without granting direct backend access.
    pub fn environmentDescriptor(&self) -> operit_host_contract::HostEnvironmentDescriptor {
        self.host.environmentDescriptor()
    }

    /// Returns the backend's display label without exposing platform paths.
    pub fn envLabel(&self) -> &str {
        self.host.envLabel()
    }

    /// Returns a portable file capability; platform consumers resolve its backing resource.
    pub fn file(&self, path: &str) -> Result<crate::VfsFile, String> {
        crate::VfsFile::new(self.clone(), path)
    }

    /// Converts an ingress resource path into a reference owned by this VFS.
    pub fn fileResource(
        &self,
        directory: Option<&str>,
        path: &str,
    ) -> Result<crate::VfsFile, String> {
        self.file(&PathMapper::resolveFileResourcePath(directory, path)?)
    }

    /// Assembles one filesystem from its backend and owning storage capability.
    pub fn fromStorage(
        host: Arc<dyn FileSystemHost>,
        storage: Arc<dyn operit_host_contract::RuntimeStorageHost>,
    ) -> Result<Self, String> {
        let runtimeRoot = storage
            .runtimeRootDir()
            .ok_or("Runtime storage root is unavailable")?;
        let workspaceRoot = storage
            .workspaceRootDir()
            .ok_or("Workspace storage root is unavailable")?;
        Ok(Self::new(
            host,
            PathMapper::new(runtimeRoot, workspaceRoot, storage),
        ))
    }

    /// Creates an explicitly nonpersistent filesystem for isolated hosts and fixtures.
    pub fn builtinOnly(
        host: Arc<dyn FileSystemHost>,
        runtimeRoot: std::path::PathBuf,
        workspaceRoot: std::path::PathBuf,
    ) -> Self {
        Self::new(host, PathMapper::builtinOnly(runtimeRoot, workspaceRoot))
    }

    /// Builds the internal mapping boundary; callers never construct a mapper themselves.
    pub(crate) fn new(host: Arc<dyn FileSystemHost>, mapper: PathMapper) -> Self {
        Self { host, mapper }
    }

    /// Normalizes an ingress path once against its explicit current directory.
    pub fn resolveResourcePath(
        &self,
        directory: Option<&str>,
        path: &str,
    ) -> Result<String, String> {
        PathMapper::resolveFileResourcePath(directory, path)
    }

    /// Resolves a terminal directory through the selected platform consumer.
    pub fn terminalDirectory(
        &self,
        host: &dyn operit_host_contract::TerminalHost,
        path: &str,
    ) -> Result<String, String> {
        self.resolvePath(path)?.terminalDirectory(host)
    }

    /// Checks workspace containment using canonical mount identities within this VFS.
    pub fn isWithin(&self, root: &str, path: &str) -> Result<bool, String> {
        let root = self.resolvePath(root)?;
        let path = self.resolvePath(path)?;
        use operit_host_contract::FileSystemResource::FileSystemResource;
        match (
            FileSystemResource::parse(&root.physicalPath).map_err(hostErrorMessage)?,
            FileSystemResource::parse(&path.physicalPath).map_err(hostErrorMessage)?,
        ) {
            (Some(root), Some(path)) if root.backend == path.backend && root.root == path.root => {
                Ok(PathMapper::relativePath(
                    &format!("/{}", root.path),
                    &format!("/{}", path.path),
                )?
                .is_some())
            }
            (Some(_), _) | (_, Some(_)) => Ok(false),
            (None, None) => {
                let root = PathMapper::resolveFileResourcePath(None, &root.physicalPath)?;
                let path = PathMapper::resolveFileResourcePath(None, &path.physicalPath)?;
                let (root, path) = if root.starts_with("/virtual/windows/")
                    && path.starts_with("/virtual/windows/")
                {
                    (root.to_lowercase(), path.to_lowercase())
                } else {
                    (root, path)
                };
                Ok(PathMapper::relativePath(&root, &path)?.is_some())
            }
        }
    }

    /// Resolves a virtual path into the mapped host path and VFS metadata.
    #[allow(non_snake_case)]
    pub(crate) fn resolvePath(&self, path: &str) -> Result<ResolvedVfsPath, String> {
        self.mapper.resolve(path)
    }

    /// Returns the VFS address of a declared runtime storage entry.
    #[allow(non_snake_case)]
    pub fn runtimePath(storagePath: &str) -> Result<String, String> {
        PathMapper::runtimeDataPath(storagePath)
    }

    /// Downloads into a VFS destination while keeping host locators inside this boundary.
    #[allow(non_snake_case)]
    pub fn downloadFile(
        &self,
        http: &dyn HttpHost,
        request: HttpRequestData,
        path: &str,
    ) -> Result<u64, String> {
        let target = self.file(path)?;
        self.resolvePath(path)?;
        let result = http
            .downloadToFile(request, &target)
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&self.resolvePath(path)?.physicalPath])?;
        Ok(result.downloadedBytes)
    }

    /// Downloads a batch through file capabilities owned by this filesystem.
    pub fn downloadFiles(
        &self,
        http: &dyn HttpHost,
        request: operit_host_contract::HttpDownloadRequest,
        control: operit_host_contract::HttpDownloadControl,
        progress: operit_host_contract::HttpDownloadProgressCallback,
    ) -> operit_host_contract::HostResult<operit_host_contract::HttpDownloadResult> {
        let targets = request
            .files
            .iter()
            .map(|file| self.file(&file.targetPath).map_err(HostError::new))
            .collect::<Result<Vec<_>, _>>()?;
        let references = targets
            .iter()
            .map(|target| target as &dyn operit_host_contract::FileTarget)
            .collect::<Vec<_>>();
        let paths = request.files.iter().map(|file| self.resolvePath(&file.targetPath))
            .collect::<Result<Vec<_>, _>>().map_err(HostError::new)?;
        let result = http.downloadFilesTo(request, &references, control, progress)?;
        crate::FileChanges::publish(&paths.iter().map(|path| path.physicalPath.as_str()).collect::<Vec<_>>()).map_err(HostError::new)?;
        Ok(result)
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

    /// Reads a bounded byte range directly from the selected backend.
    pub fn readFileBytesRange(
        &self,
        path: &str,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .readFileBytesRange(&resolved.physicalPath, offset, length)
            .map_err(hostErrorMessage)
    }

    /// Writes text to a mapped host file.
    #[allow(non_snake_case)]
    pub fn writeFile(&self, path: &str, content: &str, append: bool) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .writeFile(&resolved.physicalPath, content, append)
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&resolved.physicalPath])
    }

    /// Writes raw bytes to a mapped host file.
    #[allow(non_snake_case)]
    pub fn writeFileBytes(&self, path: &str, content: &[u8]) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .writeFileBytes(&resolved.physicalPath, content)
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&resolved.physicalPath])
    }

    /// Appends a bounded chunk through the selected filesystem backend.
    pub fn appendFileBytes(&self, path: &str, content: &[u8]) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .appendFileBytes(&resolved.physicalPath, content)
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&resolved.physicalPath])
    }

    /// Deletes a mapped host file or directory.
    #[allow(non_snake_case)]
    pub fn deleteFile(&self, path: &str, recursive: bool) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .deleteFile(&resolved.physicalPath, recursive)
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&resolved.physicalPath])
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

    /// Rejects self/descendant transfers, including aliases of overlapping mounts.
    pub fn validateTransfer(&self, source: &str, destination: &str, directory: bool) -> Result<(), String> {
        let source = self.resolvePath(source)?.physicalPath;
        let destination = self.resolvePath(destination)?.physicalPath;
        let source = source.trim_end_matches('/');
        let destination = destination.trim_end_matches('/');
        if source == destination || (directory && destination.starts_with(&format!("{source}/"))) {
            return Err("Cannot paste an entry onto itself or into its own subfolder".to_string());
        }
        Ok(())
    }

    /// Moves a file between two mapped host paths.
    #[allow(non_snake_case)]
    pub fn moveFile(&self, source: &str, destination: &str) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .moveFile(&source.physicalPath, &destination.physicalPath)
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&source.physicalPath, &destination.physicalPath])
    }

    /// Copies a file or directory between two mapped host paths.
    #[allow(non_snake_case)]
    pub fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .copyFile(&source.physicalPath, &destination.physicalPath, recursive)
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&destination.physicalPath])
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
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&destination.physicalPath])
    }

    /// Extracts a mapped archive into a mapped destination directory.
    #[allow(non_snake_case)]
    pub fn unzipFiles(&self, source: &str, destination: &str) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .unzipFiles(&source.physicalPath, &destination.physicalPath)
            .map_err(hostErrorMessage)?;
        crate::FileChanges::publish(&[&destination.physicalPath])
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

/// Publishes committed append chunks through the same mutation boundary as ordinary writes.
struct ObservedAppend {
    handle: Box<dyn operit_host_contract::FileAppendHandle>,
    path: String,
}

impl operit_host_contract::FileAppendHandle for ObservedAppend {
    /// Preserves the backend's exact committed length.
    fn byteLength(&self) -> u64 { self.handle.byteLength() }

    /// Notifies observers only after the backend accepted the complete chunk.
    fn append(&mut self, content: &[u8]) -> operit_host_contract::HostResult<()> {
        self.handle.append(content)?;
        crate::FileChanges::publish(&[&self.path]).map_err(HostError::new)
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
    use operit_host_contract::{
        HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
        HttpDownloadResult, HttpFileDownloadResult, HttpImageDelivery, HttpResponseData,
        HttpStreamChunkCallback, HttpStreamClosedCallback, HttpStreamHost,
        HttpStreamOpenedCallback,
    };
    use operit_host_native_filesystem::PosixFileSystemHost;
    use operit_host_native_storage::NativeRuntimeStorageHost;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct DownloadHost {
        calls: AtomicUsize,
    }

    impl HttpStreamHost for DownloadHost {
        /// Rejects stream operations outside this download boundary test.
        fn openHttpByteStream(
            &self,
            _: String,
            _: HttpRequestData,
            _: HttpStreamOpenedCallback,
            _: HttpStreamChunkCallback,
            _: HttpStreamClosedCallback,
        ) -> HostResult<()> {
            unreachable!()
        }
        /// Rejects stream cancellation outside this download boundary test.
        fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
            unreachable!()
        }
    }

    impl HttpHost for DownloadHost {
        /// Declares the test host's ordinary byte delivery capability.
        fn imageDelivery(&self) -> HttpImageDelivery {
            HttpImageDelivery::Bytes
        }
        /// Rejects buffered requests so a download cannot silently change transport.
        fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
            unreachable!()
        }
        /// Consumes the virtual destination without extracting its backend locator.
        fn downloadToFile(
            &self,
            request: HttpRequestData,
            target: &dyn operit_host_contract::FileTarget,
        ) -> HostResult<HttpFileDownloadResult> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if request.method != "GET" {
                return Err(HostError::new("original download failure"));
            }
            target.writeBytes(b"downloaded through VFS")?;
            Ok(HttpFileDownloadResult {
                finalUrl: request.url,
                targetPath: target.vfsPath().into(),
                downloadedBytes: 22,
            })
        }
        /// Rejects unrelated batch downloads.
        fn downloadFiles(
            &self,
            _: HttpDownloadRequest,
            _: HttpDownloadControl,
            _: HttpDownloadProgressCallback,
        ) -> HostResult<HttpDownloadResult> {
            unreachable!()
        }
    }

    /// Verifies downloads, ordinary reads and cleanup share the same VFS identity and error boundary.
    #[test]
    fn downloads_share_file_io_paths_and_preserve_host_errors() {
        let (vfs, root) = vfs();
        let http = DownloadHost {
            calls: AtomicUsize::new(0),
        };
        let directory =
            VisualFileSystem::runtimePath("runtime/cache/toolpkg/download-test").unwrap();
        vfs.makeDirectory(&directory, true).unwrap();
        let path = PathMapper::joinVfsPath(&directory, "plugin.toolpkg").unwrap();
        let request = HttpRequestData {
            url: "https://market.test/asset".into(),
            method: "GET".into(),
            headers: vec![],
            body: vec![],
            formFields: vec![],
            fileParts: vec![],
            connectTimeoutSeconds: 1,
            readTimeoutSeconds: 1,
            followRedirects: true,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        };
        let bytes = vfs.downloadFile(&http, request.clone(), &path).unwrap();
        assert_eq!(bytes as usize, vfs.readFileBytes(&path).unwrap().len());
        assert_eq!(vfs.readFile(&path).unwrap(), "downloaded through VFS");
        assert!(vfs
            .downloadFile(&http, request.clone(), "/unknown/download")
            .is_err());
        assert_eq!(http.calls.load(Ordering::SeqCst), 1);
        let mut failed = request;
        failed.method = "FAIL".into();
        assert_eq!(
            vfs.downloadFile(&http, failed, &path).unwrap_err(),
            "original download failure"
        );
        vfs.deleteFile(&directory, true).unwrap();
        assert!(!vfs.fileExists(&path).unwrap().exists);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn transfer_validation_rejects_aliases_and_descendants() {
        let (vfs, root) = vfs();
        assert!(vfs.validateTransfer("/operitapp/workspaces/a", "/app/workspaces/a", true).is_err());
        assert!(vfs.validateTransfer("/app/workspaces/a", "/operitapp/workspaces/a/child", true).is_err());
        assert!(vfs.validateTransfer("/operitapp/workspaces/a", "/operitapp/workspaces/a-copy", true).is_ok());
        assert!(vfs.validateTransfer("/operitapp/workspaces/a", "/operitapp/workspaces/b/a", true).is_ok());
        if root.exists() { std::fs::remove_dir_all(root).unwrap(); }
    }

    fn vfs() -> (VisualFileSystem, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("operit-vfs-tools-test-{}", uuid::Uuid::new_v4()));
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

    /// Observers follow committed alias writes, appends, replacements and parent removal across VFS instances.
    #[test]
    fn directory_changes_are_automatic_scoped_and_commit_ordered() {
        let (files, root) = vfs();
        let base = "/operitapp/data/extensions/device/plugins/configs/book";
        files.makeDirectory(base, true).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let subscription = files.watchChanges(base, Arc::new(move || { observed.fetch_add(1, Ordering::SeqCst); })).unwrap();
        let independent = VisualFileSystem::builtinOnly(Arc::new(PosixFileSystemHost::new()), root.clone(), root.join("workspaces"));
        independent.writeFile(&format!("{base}/rules.json"), "first", false).unwrap();
        files.writeFile("/app/data/extensions/device/plugins/configs/book/rules.json", "second", false).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2);
        let mut append = independent.openAppendFile(&format!("{base}/rules.json")).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 3);
        append.append(b" third").unwrap();
        drop(append);
        assert_eq!(count.load(Ordering::SeqCst), 4);
        let (other, other_root) = vfs();
        other.makeDirectory(base, true).unwrap();
        other.writeFile(&format!("{base}/rules.json"), "different runtime", false).unwrap();
        files.writeFile("/operitapp/data/unrelated", "unrelated", false).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 4);
        files.copyFile("/operitapp/data/unrelated", &format!("{base}/copied"), false).unwrap();
        files.moveFile(&format!("{base}/copied"), &format!("{base}/moved")).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 6, "A rename notifies once even when both paths are watched");
        assert!(files.writeFile(base, "failed", false).is_err());
        assert_eq!(count.load(Ordering::SeqCst), 6);
        files.deleteFile("/operitapp/data/extensions/device/plugins/configs", true).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 7);
        drop(subscription);
        files.makeDirectory(base, true).unwrap();
        files.writeFile(&format!("{base}/rules.json"), "unobserved", false).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 7);
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(other_root).unwrap();
    }

    /// Startup logging must reach the active identity's physical storage only.
    #[test]
    fn runtime_log_files_resolve_through_identity_storage() {
        let root = std::env::temp_dir().join(format!("operit-vfs-logs-{}", uuid::Uuid::new_v4()));
        for identity in ["identity-a", "identity-b"] {
            let runtime = root
                .join("Library/Application Support/Operit2/runtime/identities")
                .join(identity);
            let storage = Arc::new(NativeRuntimeStorageHost::new(
                runtime.clone(),
                root.join("workspaces").join(identity),
            ));
            let vfs = VisualFileSystem::fromStorage(
                Arc::new(PosixFileSystemHost::new()),
                storage,
            ).unwrap();
            vfs.makeDirectory("/operitapp/data/logs", true).unwrap();
            for name in ["operit.log", "toolpkg.log"] {
                let path = format!("/operitapp/data/logs/{name}");
                assert!(!vfs.fileExists(&path).unwrap().exists);
                vfs.writeFile(&path, "", false).unwrap();
                let mut writer = vfs.openAppendFile(&path).unwrap();
                writer.append(identity.as_bytes()).unwrap();
                assert_eq!(
                    std::fs::read_to_string(runtime.join("logs").join(name)).unwrap(),
                    identity,
                );
                assert_eq!(vfs.readFile(&path).unwrap(), identity);
            }
            assert!(!runtime.join("operitapp").exists());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_inputs_keep_vfs_native_and_relative_compatibility() {
        let (vfs, root) = vfs();
        vfs.makeDirectory("/operitapp/data/uploads", true).unwrap();
        vfs.writeFile("/operitapp/data/uploads/input.txt", "through VFS", false)
            .unwrap();
        for input in [
            "/operitapp/data/uploads/input.txt",
            "/app/data/uploads/input.txt",
            "input.txt",
            "./input.txt",
            "../uploads/input.txt",
        ] {
            let path = PathMapper::resolveFileResourcePath(Some("/operitapp/data/uploads"), input)
                .unwrap();
            assert_eq!(vfs.readFile(&path).unwrap(), "through VFS", "{input}");
        }
        let native = root
            .join("uploads/input.txt")
            .to_string_lossy()
            .into_owned();
        let path = PathMapper::resolveFileResourcePath(None, &native).unwrap();
        assert_eq!(vfs.readFile(&path).unwrap(), "through VFS");
        assert_eq!(PathMapper::resolveFileResourcePath(None, "/").unwrap(), "/");
        assert!(!vfs.listFiles("/").unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Legacy plugins write with Files and replay the same path through WebView resources.
    #[test]
    fn legacy_voice_cache_paths_share_file_and_resource_storage() {
        use operit_host_contract::FileSource;
        let (vfs, root) = vfs();
        let directory = "/sdcard/Download/voice_bar_voices";
        vfs.makeDirectory(directory, true).unwrap();
        let name = "测试 & 100%20.wav";
        let written = format!("{directory}/{name}");
        vfs.writeFileBytes(&written, b"RIFF voice cache").unwrap();
        for path in [written, format!("/storage/emulated/0/Download/voice_bar_voices/{name}")] {
            let resource = vfs.fileResource(None, &path).unwrap();
            assert_eq!(resource.readBytes().unwrap(), b"RIFF voice cache", "{path}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn both_root_mounts_share_files_and_preserve_result_paths() {
        let (vfs, root) = vfs();
        vfs.makeDirectory("/app/workspaces/demo", true).unwrap();
        vfs.writeFile("/app/workspaces/demo/file.txt", "shared", false)
            .unwrap();
        assert_eq!(
            vfs.readFile("/operitapp/workspaces/demo/file.txt").unwrap(),
            "shared"
        );
        vfs.writeFile("/operitapp/workspaces/demo/file.txt", "updated", false)
            .unwrap();
        assert_eq!(
            vfs.readFile("/app/workspaces/demo/file.txt").unwrap(),
            "updated"
        );
        for base in ["/app", "/operitapp"] {
            let path = format!("{base}/workspaces/demo/file.txt");
            assert_eq!(vfs.fileInfo(&path).unwrap().path, path);
            assert_eq!(vfs.fileInfo(base).unwrap().path, base);
        }
        let names: Vec<_> = vfs
            .listFiles("/")
            .unwrap()
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert!(names.iter().any(|name| name == "operitapp"));
        assert!(!names
            .iter()
            .any(|name| matches!(name.as_str(), "app" | "mnt" | "sdcard" | "data")));
        assert_eq!(
            vfs.listFiles("/app").unwrap(),
            vfs.listFiles("/operitapp").unwrap()
        );
        let registry = super::super::MountRegistry::MountRegistry::withStorage(Arc::new(
            NativeRuntimeStorageHost::new(root.clone(), root.join("workspaces")),
        ));
        // Either registry spelling must appear in both directory trees.
        for namespace in ["/mnt/test/folders", "/virtual/test/other"] {
            let mount = registry
                .register(namespace, "native", root.to_str().unwrap(), "Shared")
                .unwrap();
            assert!(mount.vfsPath().starts_with(namespace));
            let names: Vec<_> = vfs
                .listFiles("/")
                .unwrap()
                .into_iter()
                .map(|entry| entry.name)
                .collect();
            assert!(names.iter().any(|name| name == "virtual"));
            assert!(!names.iter().any(|name| name == "mnt" || name == "app"));
            let relative = super::super::MountRegistry::mountRelativePath(&mount.vfsPath())
                .unwrap()
                .to_string();
            let first = format!("/mnt/{relative}/mounted.txt");
            let second = format!("/virtual/{relative}/mounted.txt");
            vfs.writeFile(&first, "mounted", false).unwrap();
            assert_eq!(vfs.readFile(&second).unwrap(), "mounted");
            vfs.writeFile(&second, "changed", false).unwrap();
            assert_eq!(vfs.readFile(&first).unwrap(), "changed");
            for path in [&first, &second] {
                assert_eq!(vfs.fileInfo(path).unwrap().path, *path);
            }
            assert_eq!(
                vfs.listFiles("/mnt/test").unwrap(),
                vfs.listFiles("/virtual/test").unwrap()
            );
            registry.remove(&format!("/virtual/{relative}")).unwrap();
        }
        assert_eq!(
            vfs.listFiles("/mnt").unwrap(),
            vfs.listFiles("/virtual").unwrap()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// move/copy/file_info resolve VFS paths to the workspace collection root
    /// and keep echoing VFS paths in metadata.
    #[test]
    fn moveCopyFileInfoRoundTripThroughVfsWorkspacePaths() {
        let (vfs, root) = vfs();
        vfs.makeDirectory("/operitapp/workspaces/ws-a", true)
            .unwrap();
        vfs.writeFile("/operitapp/workspaces/ws-a/src.txt", "hello vfs", false)
            .unwrap();

        let info = vfs.fileInfo("/operitapp/workspaces/ws-a/src.txt").unwrap();
        assert!(info.exists);
        assert_eq!(info.path, "/operitapp/workspaces/ws-a/src.txt");
        assert_eq!(info.size, "hello vfs".len() as i64);

        vfs.copyFile(
            "/operitapp/workspaces/ws-a/src.txt",
            "/operitapp/workspaces/ws-a/copy.txt",
            false,
        )
        .unwrap();
        assert_eq!(
            vfs.readFile("/operitapp/workspaces/ws-a/copy.txt").unwrap(),
            "hello vfs"
        );

        vfs.moveFile(
            "/operitapp/workspaces/ws-a/src.txt",
            "/operitapp/workspaces/ws-a/renamed.txt",
        )
        .unwrap();
        assert!(
            !vfs.fileExists("/operitapp/workspaces/ws-a/src.txt")
                .unwrap()
                .exists
        );
        assert_eq!(
            vfs.readFile("/operitapp/workspaces/ws-a/renamed.txt")
                .unwrap(),
            "hello vfs"
        );

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
