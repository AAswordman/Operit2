use std::error::Error;
use std::fmt::{Display, Formatter};

pub type HostResult<T> = Result<T, HostError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostEnvironmentDescriptor {
    pub id: String,
    pub displayName: String,
    pub pathStyleDescriptionEn: String,
    pub pathStyleDescriptionCn: String,
    pub examplePaths: Vec<String>,
    pub usesEnvironmentParameter: bool,
    pub environmentParameterDescriptionEn: String,
    pub environmentParameterDescriptionCn: String,
    pub capabilities: Vec<String>,
}

impl HostEnvironmentDescriptor {
    pub fn android() -> Self {
        Self {
            id: "android".to_string(),
            displayName: "Android".to_string(),
            pathStyleDescriptionEn: "Use Android absolute paths such as /sdcard/Download or an attached repository path.".to_string(),
            pathStyleDescriptionCn: "使用 Android 绝对路径，例如 /sdcard/Download，或使用已附加的仓库路径。".to_string(),
            examplePaths: vec![
                "/sdcard/Download".to_string(),
                "/sdcard/Documents".to_string(),
            ],
            usesEnvironmentParameter: true,
            environmentParameterDescriptionEn: "optional, execution environment. Values: \"android\" (Android file system) | \"linux\" (local terminal environment) | \"repo:<repositoryName>\" (attached local storage repository)".to_string(),
            environmentParameterDescriptionCn: "可选，执行环境。取值：\"android\"（Android 文件系统）| \"linux\"（本地终端环境）| \"repo:<仓库名>\"（附加本地储存仓库）".to_string(),
            capabilities: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "fs.search".to_string(),
                "fs.archive".to_string(),
                "os.open".to_string(),
                "os.share".to_string(),
            ],
        }
    }

    pub fn windows() -> Self {
        Self {
            id: "windows".to_string(),
            displayName: "Windows".to_string(),
            pathStyleDescriptionEn: "Use absolute Windows paths such as C:/Users/Name/Documents or D:/Code/project.".to_string(),
            pathStyleDescriptionCn: "使用 Windows 绝对路径，例如 C:/Users/Name/Documents 或 D:/Code/project。".to_string(),
            examplePaths: vec![
                "C:/Users/Name/Documents".to_string(),
                "D:/Code/project".to_string(),
            ],
            usesEnvironmentParameter: false,
            environmentParameterDescriptionEn: String::new(),
            environmentParameterDescriptionCn: String::new(),
            capabilities: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "fs.search".to_string(),
                "fs.archive".to_string(),
                "os.open".to_string(),
                "os.share".to_string(),
            ],
        }
    }

    pub fn linux() -> Self {
        Self {
            id: "linux".to_string(),
            displayName: "Linux".to_string(),
            pathStyleDescriptionEn: "Use absolute Linux paths such as /home/user/project or /tmp/work.".to_string(),
            pathStyleDescriptionCn: "使用 Linux 绝对路径，例如 /home/user/project 或 /tmp/work。".to_string(),
            examplePaths: vec![
                "/home/user/project".to_string(),
                "/tmp/work".to_string(),
            ],
            usesEnvironmentParameter: false,
            environmentParameterDescriptionEn: String::new(),
            environmentParameterDescriptionCn: String::new(),
            capabilities: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "fs.search".to_string(),
                "fs.archive".to_string(),
                "os.open".to_string(),
                "os.share".to_string(),
            ],
        }
    }
}

impl Default for HostEnvironmentDescriptor {
    fn default() -> Self {
        Self::android()
    }
}

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
    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor;
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
    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()>;
    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()>;
    fn openFile(&self, path: &str) -> HostResult<()>;
    fn shareFile(&self, path: &str, title: &str) -> HostResult<()>;
}
