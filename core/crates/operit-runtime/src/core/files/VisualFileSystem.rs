use std::sync::Arc;

use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, GrepFileMatch, HostError,
};

use crate::core::files::PathMapper::{PathMapper, ResolvedVfsPath};

#[derive(Clone)]
pub struct VisualFileSystem {
    host: Arc<dyn FileSystemHost>,
    mapper: PathMapper,
}

impl VisualFileSystem {
    pub fn new(host: Arc<dyn FileSystemHost>, mapper: PathMapper) -> Self {
        Self { host, mapper }
    }

    pub fn mapper(&self) -> &PathMapper {
        &self.mapper
    }

    #[allow(non_snake_case)]
    pub fn resolvePath(&self, path: &str) -> Result<ResolvedVfsPath, String> {
        self.mapper.resolve(path)
    }

    #[allow(non_snake_case)]
    pub fn listFiles(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        if let Some(entries) = self.mapper.virtualDirectoryEntries(path)? {
            return Ok(entries);
        }
        let resolved = self.resolvePath(path)?;
        self.host
            .listFiles(&resolved.physicalPath)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn readFile(&self, path: &str) -> Result<String, String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .readFile(&resolved.physicalPath)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> Result<String, String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .readFileWithLimit(&resolved.physicalPath, maxBytes)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn readFileBytes(&self, path: &str) -> Result<Vec<u8>, String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .readFileBytes(&resolved.physicalPath)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn writeFile(&self, path: &str, content: &str, append: bool) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .writeFile(&resolved.physicalPath, content, append)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn writeFileBytes(&self, path: &str, content: &[u8]) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .writeFileBytes(&resolved.physicalPath, content)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn deleteFile(&self, path: &str, recursive: bool) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .deleteFile(&resolved.physicalPath, recursive)
            .map_err(hostErrorMessage)
    }

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

    #[allow(non_snake_case)]
    pub fn moveFile(&self, source: &str, destination: &str) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .moveFile(&source.physicalPath, &destination.physicalPath)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .copyFile(&source.physicalPath, &destination.physicalPath, recursive)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn makeDirectory(&self, path: &str, createParents: bool) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .makeDirectory(&resolved.physicalPath, createParents)
            .map_err(hostErrorMessage)
    }

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

    #[allow(non_snake_case)]
    pub fn zipFiles(&self, source: &str, destination: &str) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .zipFiles(&source.physicalPath, &destination.physicalPath)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn unzipFiles(&self, source: &str, destination: &str) -> Result<(), String> {
        let source = self.resolvePath(source)?;
        let destination = self.resolvePath(destination)?;
        self.host
            .unzipFiles(&source.physicalPath, &destination.physicalPath)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn openFile(&self, path: &str) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .openFile(&resolved.physicalPath)
            .map_err(hostErrorMessage)
    }

    #[allow(non_snake_case)]
    pub fn shareFile(&self, path: &str, title: &str) -> Result<(), String> {
        let resolved = self.resolvePath(path)?;
        self.host
            .shareFile(&resolved.physicalPath, title)
            .map_err(hostErrorMessage)
    }
}

fn hostErrorMessage(error: HostError) -> String {
    error.message
}

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
