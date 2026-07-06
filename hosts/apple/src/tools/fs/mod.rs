use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
use std::time::UNIX_EPOCH;

use globset::GlobBuilder;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};
use ignore::WalkBuilder;
use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, GrepFileMatch, GrepLineMatch, HostEnvironmentDescriptor, HostError, HostResult,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Clone, Debug, Default)]
pub struct AppleFileSystemHost;

impl AppleFileSystemHost {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemHost for AppleFileSystemHost {
    fn envLabel(&self) -> &str {
        applePlatformName()
    }

    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        appleEnvironmentDescriptor()
    }

    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()> {
        if path.trim().is_empty() {
            return Err(HostError::new(format!("{paramName} parameter is required")));
        }
        let pathValue = Path::new(path);
        if !pathValue.is_absolute() {
            return Err(HostError::new(format!(
                "Invalid path: '{path}'. Path must be an absolute Apple platform path."
            )));
        }
        Ok(())
    }

    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
        self.validatePath(path, "path")?;
        let directory = Path::new(path);
        if !directory.exists() {
            return Err(HostError::new(format!("Directory does not exist: {path}")));
        }
        if !directory.is_dir() {
            return Err(HostError::new(format!("Path is not a directory: {path}")));
        }
        let mut entries = Vec::new();
        for item in fs::read_dir(directory)? {
            let item = item?;
            let metadata = item.metadata()?;
            let itemPath = item.path();
            entries.push(FileEntry {
                name: item.file_name().to_string_lossy().to_string(),
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
                permissions: permissions_string(&itemPath, &metadata),
                lastModified: modified_string(&metadata),
            });
        }
        Ok(entries)
    }

    fn readFile(&self, path: &str) -> HostResult<String> {
        self.validateReadableFile(path)?;
        fs::read_to_string(path).map_err(HostError::from)
    }

    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String> {
        self.validateReadableFile(path)?;
        let mut file = File::open(path)?;
        let mut buffer = vec![0; maxBytes];
        let readCount = file.read(&mut buffer)?;
        buffer.truncate(readCount);
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.validateReadableFile(path)?;
        fs::read(path).map_err(HostError::from)
    }

    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()> {
        self.validatePath(path, "path")?;
        ensure_parent_directory(path)?;
        let mut options = fs::OpenOptions::new();
        options.create(true).write(true);
        if append {
            options.append(true);
        } else {
            options.truncate(true);
        }
        let mut file = options.open(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.validatePath(path, "path")?;
        ensure_parent_directory(path)?;
        fs::write(path, content).map_err(HostError::from)
    }

    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.validatePath(path, "path")?;
        let target = Path::new(path);
        if !target.exists() {
            return Err(HostError::new(format!(
                "File or directory does not exist: {path}"
            )));
        }
        if target.is_dir() {
            if recursive {
                fs::remove_dir_all(target)?;
            } else {
                fs::remove_dir(target)?;
            }
        } else {
            fs::remove_file(target)?;
        }
        Ok(())
    }

    fn fileExists(&self, path: &str) -> HostResult<FileExistence> {
        self.validatePath(path, "path")?;
        let target = Path::new(path);
        if !target.exists() {
            return Ok(FileExistence {
                exists: false,
                isDirectory: false,
                size: 0,
            });
        }
        let metadata = fs::metadata(target)?;
        Ok(FileExistence {
            exists: true,
            isDirectory: metadata.is_dir(),
            size: metadata.len() as i64,
        })
    }

    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        self.validatePath(source, "source")?;
        self.validatePath(destination, "destination")?;
        if !Path::new(source).exists() {
            return Err(HostError::new(format!(
                "Source file does not exist: {source}"
            )));
        }
        ensure_parent_directory(destination)?;
        fs::rename(source, destination).map_err(HostError::from)
    }

    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
        self.validatePath(source, "source")?;
        self.validatePath(destination, "destination")?;
        let sourcePath = Path::new(source);
        if !sourcePath.exists() {
            return Err(HostError::new(format!(
                "Source path does not exist: {source}"
            )));
        }
        ensure_parent_directory(destination)?;
        if sourcePath.is_dir() {
            if !recursive {
                return Err(HostError::new(
                    "Source is a directory and recursive flag is not set",
                ));
            }
            copy_directory(sourcePath, Path::new(destination))?;
        } else {
            fs::copy(source, destination)?;
        }
        Ok(())
    }

    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()> {
        self.validatePath(path, "path")?;
        if createParents {
            fs::create_dir_all(path)?;
        } else {
            fs::create_dir(path)?;
        }
        Ok(())
    }

    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>> {
        self.validatePath(&request.path, "path")?;
        if request.pattern.trim().is_empty() {
            return Err(HostError::new("pattern parameter is required"));
        }
        let target = Path::new(&request.path);
        if !target.exists() {
            return Err(HostError::new(format!(
                "Base path does not exist: {}",
                request.path
            )));
        }
        let matcher = GlobBuilder::new(&request.pattern)
            .case_insensitive(request.caseInsensitive)
            .build()
            .map_err(|error| HostError::new(format!("Invalid file pattern: {error}")))?
            .compile_matcher();
        let mut walkBuilder = WalkBuilder::new(target);
        if request.maxDepth >= 0 {
            walkBuilder.max_depth(Some(request.maxDepth as usize + 1));
        }
        let mut files = Vec::new();
        for entry in walkBuilder.build() {
            let entry = entry.map_err(|error| HostError::new(format!("walk error: {error}")))?;
            let entryPath = entry.path();
            if !entryPath.is_file() {
                continue;
            }
            let candidate = if request.usePathPattern {
                entryPath
            } else {
                Path::new(
                    entryPath
                        .file_name()
                        .expect("walk file entry must have file name"),
                )
            };
            if matcher.is_match(candidate) {
                files.push(entryPath.to_string_lossy().to_string());
            }
        }
        Ok(files)
    }

    fn fileInfo(&self, path: &str) -> HostResult<FileInfo> {
        self.validatePath(path, "path")?;
        let target = Path::new(path);
        if !target.exists() {
            return Ok(FileInfo {
                path: path.to_string(),
                exists: false,
                fileType: String::new(),
                size: 0,
                permissions: String::new(),
                owner: String::new(),
                group: String::new(),
                lastModified: String::new(),
                rawStatOutput: String::new(),
            });
        }
        let metadata = fs::metadata(target)?;
        let fileType = if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        };
        let permissions = permissions_string(target, &metadata);
        let lastModified = modified_string(&metadata);
        let rawStatOutput = format!(
            "File: {path}\nSize: {} bytes\nType: {fileType}\nPermissions: {permissions}\nLast Modified: {lastModified}\n",
            metadata.len()
        );
        Ok(FileInfo {
            path: path.to_string(),
            exists: true,
            fileType: fileType.to_string(),
            size: metadata.len() as i64,
            permissions,
            owner: String::new(),
            group: String::new(),
            lastModified,
            rawStatOutput,
        })
    }

    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        self.validatePath(&request.path, "path")?;
        if request.pattern.trim().is_empty() {
            return Err(HostError::new("Pattern parameter is required"));
        }
        if request.filePattern.trim().is_empty() {
            return Err(HostError::new("file_pattern parameter is required"));
        }
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(request.caseInsensitive)
            .build(&request.pattern)
            .map_err(|error| HostError::new(format!("Invalid regex pattern: {error}")))?;
        let fileMatcher = GlobBuilder::new(&request.filePattern)
            .case_insensitive(request.caseInsensitive)
            .build()
            .map_err(|error| HostError::new(format!("Invalid file pattern: {error}")))?
            .compile_matcher();
        let mut searcher = SearcherBuilder::new()
            .line_number(true)
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .max_matches(Some(request.maxResults as u64))
            .build();
        let mut matches = Vec::new();
        let mut filesSearched = 0usize;
        let mut totalMatches = 0usize;

        for entry in WalkBuilder::new(&request.path).build() {
            let entry = entry.map_err(|error| HostError::new(format!("walk error: {error}")))?;
            let filePath = entry.path();
            if !filePath.is_file() || !fileMatcher.is_match(filePath) {
                continue;
            }
            filesSearched += 1;
            let mut sink = RipgrepGrepSink::new();
            searcher
                .search_path(&matcher, filePath, &mut sink)
                .map_err(HostError::from)?;
            if sink.lineMatches.is_empty() {
                continue;
            }
            let content = match fs::read_to_string(filePath) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let lines = content.lines().collect::<Vec<_>>();
            let mut lineMatches = sink.lineMatches;
            if request.contextLines > 0 {
                for lineMatch in &mut lineMatches {
                    let lineIndex = lineMatch.lineNumber.saturating_sub(1);
                    let start = lineIndex.saturating_sub(request.contextLines);
                    let end = (lineIndex + request.contextLines + 1).min(lines.len());
                    lineMatch.matchContext = Some(lines[start..end].join("\n"));
                }
            }
            totalMatches += lineMatches.len();
            if !lineMatches.is_empty() {
                matches.push(GrepFileMatch {
                    filePath: filePath.to_string_lossy().to_string(),
                    lineMatches,
                });
            }
        }
        Ok(GrepCodeResult {
            matches,
            totalMatches,
            filesSearched,
        })
    }

    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.validatePath(source, "source")?;
        self.validatePath(destination, "destination")?;
        let sourcePath = Path::new(source);
        if !sourcePath.exists() {
            return Err(HostError::new(format!(
                "Source path does not exist: {source}"
            )));
        }
        ensure_parent_directory(destination)?;
        let destinationFile = File::create(destination)?;
        let mut zipWriter = ZipWriter::new(destinationFile);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        if sourcePath.is_dir() {
            let baseName = match sourcePath.file_name() {
                Some(value) => value.to_string_lossy().to_string(),
                None => {
                    return Err(HostError::new(format!(
                        "Invalid source directory path: {source}"
                    )))
                }
            };
            zip_directory(sourcePath, sourcePath, &baseName, &mut zipWriter, options)?;
        } else {
            let fileName = match sourcePath.file_name() {
                Some(value) => value.to_string_lossy().to_string(),
                None => return Err(HostError::new(format!("Invalid source path: {source}"))),
            };
            zip_file(sourcePath, &fileName, &mut zipWriter, options)?;
        }
        zipWriter
            .finish()
            .map_err(|error| HostError::new(format!("Error finalizing zip archive: {error}")))?;
        Ok(())
    }

    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()> {
        self.validatePath(source, "source")?;
        self.validatePath(destination, "destination")?;
        validate_readable_file(self, source)?;
        fs::create_dir_all(destination)?;
        let sourceFile = File::open(source)?;
        let mut archive = ZipArchive::new(sourceFile)
            .map_err(|error| HostError::new(format!("Error opening zip archive: {error}")))?;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| HostError::new(format!("Error reading zip entry: {error}")))?;
            let enclosedPath = match entry.enclosed_name() {
                Some(path) => path.to_path_buf(),
                None => return Err(HostError::new("Zip entry has invalid path")),
            };
            let outputPath = Path::new(destination).join(enclosedPath);
            if entry.is_dir() {
                fs::create_dir_all(&outputPath)?;
                continue;
            }
            if let Some(parent) = outputPath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outputFile = File::create(&outputPath)?;
            std::io::copy(&mut entry, &mut outputFile)?;
        }
        Ok(())
    }

    fn openFile(&self, path: &str) -> HostResult<()> {
        self.validateReadableFile(path)?;
        let status = Command::new("open").arg(path).status().map_err(|error| {
            HostError::new(format!(
                "Failed to open Apple platform file request: {error}"
            ))
        })?;
        if !status.success() {
            return Err(HostError::new(format!(
                "Apple platform open request exited with {status}"
            )));
        }
        Ok(())
    }

    fn shareFile(&self, path: &str, title: &str) -> HostResult<()> {
        self.validateReadableFile(path)?;
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("open")
                .arg("-R")
                .arg(path)
                .status()
                .map_err(|error| {
                    HostError::new(format!("Failed to reveal Apple platform file: {error}"))
                })?;
            if !status.success() {
                return Err(HostError::new(format!(
                    "Apple platform reveal request exited with {status}"
                )));
            }
            let _ = title;
            Ok(())
        }
        #[cfg(target_os = "ios")]
        {
            Err(HostError::new(format!(
                "iOS file sharing must be initiated by the Flutter owner UI: {path}; title={title}"
            )))
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            let _ = title;
            Err(HostError::new(format!(
                "Apple file sharing host is available only on iOS or macOS: {path}"
            )))
        }
    }
}

impl AppleFileSystemHost {
    fn validateReadableFile(&self, path: &str) -> HostResult<()> {
        validate_readable_file(self, path)
    }
}

fn appleEnvironmentDescriptor() -> HostEnvironmentDescriptor {
    HostEnvironmentDescriptor {
        id: applePlatformName().to_string(),
        displayName: appleDisplayName().to_string(),
        pathStyleDescriptionEn:
            "Use absolute Apple platform paths such as /Users/Name/Documents or an app sandbox path."
                .to_string(),
        pathStyleDescriptionCn:
            "使用 Apple 平台绝对路径，例如 /Users/Name/Documents 或应用沙盒路径。".to_string(),
        examplePaths: vec![
            "/Users/Name/Documents".to_string(),
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
            "audio.playback".to_string(),
            "music.playback".to_string(),
            "bluetooth.classic".to_string(),
            "bluetooth.ble".to_string(),
            "tts.synthesis".to_string(),
            "tts.playback".to_string(),
            "runtime.process".to_string(),
            "runtime.storage".to_string(),
            "runtime.sqlite".to_string(),
        ],
    }
}

fn applePlatformName() -> &'static str {
    #[cfg(target_os = "ios")]
    {
        "ios"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        "apple"
    }
}

fn appleDisplayName() -> &'static str {
    #[cfg(target_os = "ios")]
    {
        "iOS"
    }
    #[cfg(target_os = "macos")]
    {
        "macOS"
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        "Apple"
    }
}

fn ensure_parent_directory(path: &str) -> HostResult<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn validate_readable_file(host: &AppleFileSystemHost, path: &str) -> HostResult<()> {
    host.validatePath(path, "path")?;
    let target = Path::new(path);
    if !target.exists() {
        return Err(HostError::new(format!("File does not exist: {path}")));
    }
    if !target.is_file() {
        return Err(HostError::new(format!("Path is not a file: {path}")));
    }
    Ok(())
}

fn permissions_string(_path: &Path, metadata: &fs::Metadata) -> String {
    let canRead = 'r';
    let canWrite = if metadata.permissions().readonly() {
        '-'
    } else {
        'w'
    };
    let canExecute = if metadata.is_dir() { 'x' } else { '-' };
    format!("{canRead}{canWrite}{canExecute}")
}

fn modified_string(metadata: &fs::Metadata) -> String {
    match metadata.modified() {
        Ok(value) => match value.duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs().to_string(),
            Err(_) => String::new(),
        },
        Err(_) => String::new(),
    }
}

fn copy_directory(source: &Path, destination: &Path) -> HostResult<()> {
    if !destination.exists() {
        fs::create_dir_all(destination)?;
    }
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let sourcePath = entry.path();
        let destinationPath = destination.join(entry.file_name());
        if sourcePath.is_dir() {
            copy_directory(&sourcePath, &destinationPath)?;
        } else {
            fs::copy(&sourcePath, &destinationPath)?;
        }
    }
    Ok(())
}

fn zip_directory(
    root: &Path,
    current: &Path,
    zipPrefix: &str,
    zipWriter: &mut ZipWriter<File>,
    options: SimpleFileOptions,
) -> HostResult<()> {
    let relative = current
        .strip_prefix(root)
        .map_err(|error| HostError::new(format!("Error building zip path: {error}")))?;
    let entryName = if relative.as_os_str().is_empty() {
        zipPrefix.to_string()
    } else {
        format!(
            "{zipPrefix}/{}",
            relative.to_string_lossy().replace('\\', "/")
        )
    };
    if !entryName.is_empty() {
        zipWriter
            .add_directory(format!("{entryName}/"), options)
            .map_err(|error| HostError::new(format!("Error writing zip directory: {error}")))?;
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            zip_directory(root, &path, zipPrefix, zipWriter, options)?;
        } else {
            let fileRelative = path
                .strip_prefix(root)
                .map_err(|error| HostError::new(format!("Error building zip path: {error}")))?;
            let fileName = format!(
                "{zipPrefix}/{}",
                fileRelative.to_string_lossy().replace('\\', "/")
            );
            zip_file(&path, &fileName, zipWriter, options)?;
        }
    }
    Ok(())
}

fn zip_file(
    path: &Path,
    entryName: &str,
    zipWriter: &mut ZipWriter<File>,
    options: SimpleFileOptions,
) -> HostResult<()> {
    zipWriter
        .start_file(entryName, options)
        .map_err(|error| HostError::new(format!("Error writing zip entry: {error}")))?;
    let mut file = File::open(path)?;
    std::io::copy(&mut file, zipWriter)?;
    Ok(())
}

struct RipgrepGrepSink {
    lineMatches: Vec<GrepLineMatch>,
}

impl RipgrepGrepSink {
    fn new() -> Self {
        Self {
            lineMatches: Vec::new(),
        }
    }
}

impl Sink for RipgrepGrepSink {
    type Error = io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        let lineContent = bytes_to_search_line(mat.bytes());
        let lineNumber = match mat.line_number() {
            Some(value) => value as usize,
            None => 0,
        };
        self.lineMatches.push(GrepLineMatch {
            lineNumber,
            lineContent,
            matchContext: None,
        });
        Ok(true)
    }
}

fn bytes_to_search_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches(&['\r', '\n'][..])
        .to_string()
}
