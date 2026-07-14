use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, HostEnvironmentDescriptor, HostError, HostResult,
};
use operit_host_native_common::PosixFileSystemHost;
use std::path::Path;
use std::sync::Arc;

pub type OhosFileOpener = Arc<dyn Fn(&str) -> HostResult<()> + Send + Sync>;
pub type OhosFileSharer = Arc<dyn Fn(&str, &str) -> HostResult<()> + Send + Sync>;

#[derive(Clone)]
pub struct OhosFileSystemHost {
    inner: PosixFileSystemHost,
    opener: Option<OhosFileOpener>,
    sharer: Option<OhosFileSharer>,
}

impl OhosFileSystemHost {
    /// Creates the OpenHarmony file-system host.
    pub fn new() -> Self {
        Self {
            inner: PosixFileSystemHost::newForEnvironment(
                "ohos",
                HostEnvironmentDescriptor::ohos(),
            ),
            opener: None,
            sharer: None,
        }
    }

    /// Creates the OpenHarmony file-system host with platform file actions.
    pub fn fromPlatformActions(opener: OhosFileOpener, sharer: OhosFileSharer) -> Self {
        Self {
            inner: PosixFileSystemHost::newForEnvironment(
                "ohos",
                HostEnvironmentDescriptor::ohos(),
            ),
            opener: Some(opener),
            sharer: Some(sharer),
        }
    }

    /// Verifies that a path points to a readable OpenHarmony file.
    fn validateReadableFile(&self, path: &str) -> HostResult<()> {
        self.validatePath(path, "path")?;
        let target = Path::new(path);
        if !target.exists() {
            return Err(HostError::new(format!("File does not exist: {path}")));
        }
        if !target.is_file() {
            return Err(HostError::new(format!("Path is not a file: {path}")));
        }
        Ok(())
    }
}

impl Default for OhosFileSystemHost {
    /// Creates the default OpenHarmony file-system host.
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemHost for OhosFileSystemHost {
    /// Returns the OpenHarmony file-system environment label.
    fn envLabel(&self) -> &str {
        self.inner.envLabel()
    }

    /// Returns the OpenHarmony file-system environment descriptor.
    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        self.inner.environmentDescriptor()
    }

    /// Validates an OpenHarmony absolute application path.
    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()> {
        self.inner.validatePath(path, paramName)
    }

    /// Lists files in an OpenHarmony directory.
    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
        self.inner.listFiles(path)
    }

    /// Reads a UTF-8 text file from OpenHarmony storage.
    fn readFile(&self, path: &str) -> HostResult<String> {
        self.inner.readFile(path)
    }

    /// Reads a bounded UTF-8 text prefix from OpenHarmony storage.
    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String> {
        self.inner.readFileWithLimit(path, maxBytes)
    }

    /// Reads raw file bytes from OpenHarmony storage.
    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readFileBytes(path)
    }

    /// Writes a UTF-8 text file into OpenHarmony storage.
    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
        self.inner.writeFile(path, content, append)
    }

    /// Writes raw file bytes into OpenHarmony storage.
    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeFileBytes(path, content)
    }

    /// Deletes an OpenHarmony file or directory.
    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.deleteFile(path, recursive)
    }

    /// Checks whether an OpenHarmony file-system entry exists.
    fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
        self.inner.fileExists(path)
    }

    /// Moves an OpenHarmony file-system entry.
    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.moveFile(source, destination)
    }

    /// Copies an OpenHarmony file-system entry.
    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
        self.inner.copyFile(source, destination, recursive)
    }

    /// Creates an OpenHarmony directory.
    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()> {
        self.inner.makeDirectory(path, createParents)
    }

    /// Finds files under an OpenHarmony directory using a glob request.
    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>> {
        self.inner.findFiles(request)
    }

    /// Reads metadata for an OpenHarmony file-system entry.
    fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
        self.inner.fileInfo(path)
    }

    /// Searches code text under an OpenHarmony directory.
    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        self.inner.grepCode(request)
    }

    /// Creates a zip archive from OpenHarmony storage entries.
    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.zipFiles(source, destination)
    }

    /// Extracts a zip archive into OpenHarmony storage.
    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.unzipFiles(source, destination)
    }

    /// Reports that OpenHarmony open-file requires a platform owner bridge.
    fn openFile(&self, path: &str) -> HostResult<()> {
        self.validateReadableFile(path)?;
        let opener = self
            .opener
            .as_ref()
            .ok_or_else(|| HostError::new("OpenHarmony file opener is not registered"))?;
        opener(path)
    }

    /// Reports that OpenHarmony share-file requires a platform owner bridge.
    fn shareFile(&self, path: &str, title: &str) -> HostResult<()> {
        self.validateReadableFile(path)?;
        let sharer = self
            .sharer
            .as_ref()
            .ok_or_else(|| HostError::new("OpenHarmony file sharer is not registered"))?;
        sharer(path, title)
    }
}
