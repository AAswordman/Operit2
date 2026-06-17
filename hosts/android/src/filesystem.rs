use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, HostEnvironmentDescriptor, HostError, HostResult,
};

#[derive(Clone, Debug, Default)]
pub struct AndroidFileSystemHost {
    inner: operit_host_linux_native::LinuxFileSystemHost,
}

impl AndroidFileSystemHost {
    pub fn new() -> Self {
        Self {
            inner: operit_host_linux_native::LinuxFileSystemHost::new(),
        }
    }
}

impl FileSystemHost for AndroidFileSystemHost {
    fn envLabel(&self) -> &str {
        "android"
    }

    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        HostEnvironmentDescriptor::android()
    }

    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()> {
        if path.trim().is_empty() {
            return Err(HostError::new(format!("{paramName} parameter is required")));
        }
        if !std::path::Path::new(path).is_absolute() {
            return Err(HostError::new(format!(
                "Invalid path: '{path}'. Path must be an absolute Android path."
            )));
        }
        Ok(())
    }

    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
        self.inner.listFiles(path)
    }

    fn readFile(&self, path: &str) -> HostResult<String> {
        self.inner.readFile(path)
    }

    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String> {
        self.inner.readFileWithLimit(path, maxBytes)
    }

    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readFileBytes(path)
    }

    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
        self.inner.writeFile(path, content, append)
    }

    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeFileBytes(path, content)
    }

    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.deleteFile(path, recursive)
    }

    fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
        self.inner.fileExists(path)
    }

    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.moveFile(source, destination)
    }

    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
        self.inner.copyFile(source, destination, recursive)
    }

    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()> {
        self.inner.makeDirectory(path, createParents)
    }

    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>> {
        self.inner.findFiles(request)
    }

    fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
        self.inner.fileInfo(path)
    }

    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        self.inner.grepCode(request)
    }

    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.zipFiles(source, destination)
    }

    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.inner.unzipFiles(source, destination)
    }

    fn openFile(&self, path: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android open_file requires the Flutter Android host bridge: {path}"
        )))
    }

    fn shareFile(&self, path: &str, title: &str) -> HostResult<()> {
        Err(HostError::new(format!(
            "Android share_file requires the Flutter Android host bridge: {path} ({title})"
        )))
    }
}
