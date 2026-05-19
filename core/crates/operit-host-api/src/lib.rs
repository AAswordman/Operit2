use std::error::Error;
use std::fmt::{Display, Formatter};

pub type HostResult<T> = Result<T, HostError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostError {
    pub message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for HostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for HostError {}

impl From<std::io::Error> for HostError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub isDirectory: bool,
    pub size: i64,
    pub permissions: String,
    pub lastModified: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileExistence {
    pub exists: bool,
    pub isDirectory: bool,
    pub size: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileInfo {
    pub path: String,
    pub exists: bool,
    pub fileType: String,
    pub size: i64,
    pub permissions: String,
    pub owner: String,
    pub group: String,
    pub lastModified: String,
    pub rawStatOutput: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindFilesRequest {
    pub path: String,
    pub pattern: String,
    pub maxDepth: i32,
    pub usePathPattern: bool,
    pub caseInsensitive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepCodeRequest {
    pub path: String,
    pub pattern: String,
    pub filePattern: String,
    pub caseInsensitive: bool,
    pub contextLines: usize,
    pub maxResults: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepLineMatch {
    pub lineNumber: usize,
    pub lineContent: String,
    pub matchContext: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepFileMatch {
    pub filePath: String,
    pub lineMatches: Vec<GrepLineMatch>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepCodeResult {
    pub matches: Vec<GrepFileMatch>,
    pub totalMatches: usize,
    pub filesSearched: usize,
}

pub trait FileSystemHost: Send + Sync {
    fn envLabel(&self) -> &str;
    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()>;
    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>>;
    fn readFile(&self, path: &str) -> HostResult<String>;
    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String>;
    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>>;
    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()>;
    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()>;
    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()>;
    fn fileExists(&self, path: &str) -> HostResult<FileExistence>;
    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()>;
    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()>;
    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()>;
    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>>;
    fn fileInfo(&self, path: &str) -> HostResult<FileInfo>;
    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult>;
}
