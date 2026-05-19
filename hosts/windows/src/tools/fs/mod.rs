use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, GrepFileMatch, GrepLineMatch, HostError, HostResult,
};
use regex::RegexBuilder;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Clone, Debug, Default)]
pub struct WindowsFileSystemHost;

impl WindowsFileSystemHost {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemHost for WindowsFileSystemHost {
    fn envLabel(&self) -> &str {
        "windows"
    }

    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()> {
        if path.trim().is_empty() {
            return Err(HostError::new(format!("{paramName} parameter is required")));
        }
        let pathValue = Path::new(path);
        if !pathValue.is_absolute() {
            return Err(HostError::new(format!(
                "Invalid path: '{path}'. Path must be an absolute Windows path."
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
            return Err(HostError::new(format!("File or directory does not exist: {path}")));
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
            return Err(HostError::new(format!("Source file does not exist: {source}")));
        }
        ensure_parent_directory(destination)?;
        fs::rename(source, destination).map_err(HostError::from)
    }

    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()> {
        self.validatePath(source, "source")?;
        self.validatePath(destination, "destination")?;
        let sourcePath = Path::new(source);
        if !sourcePath.exists() {
            return Err(HostError::new(format!("Source path does not exist: {source}")));
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
        let target = PathBuf::from(&request.path);
        if !target.exists() {
            return Err(HostError::new(format!(
                "Base path does not exist: {}",
                request.path
            )));
        }
        let mut files = Vec::new();
        collect_matching_files(&target, &request, 0, &mut files)?;
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

        let regex = RegexBuilder::new(&request.pattern)
            .case_insensitive(request.caseInsensitive)
            .build()
            .map_err(|error| HostError::new(format!("Invalid regex pattern: {error}")))?;
        let fileRequest = FindFilesRequest {
            path: request.path.clone(),
            pattern: request.filePattern.clone(),
            maxDepth: -1,
            usePathPattern: false,
            caseInsensitive: request.caseInsensitive,
        };
        let candidates = self.findFiles(fileRequest)?;
        let mut matches = Vec::new();
        let mut filesSearched = 0usize;
        let mut totalMatches = 0usize;
        for filePath in candidates {
            filesSearched += 1;
            let content = match fs::read_to_string(&filePath) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let mut lineMatches = Vec::new();
            let lines = content.lines().collect::<Vec<_>>();
            for (index, line) in lines.iter().enumerate() {
                if regex.is_match(line) {
                    totalMatches += 1;
                    let lineNumber = index + 1;
                    let start = index.saturating_sub(request.contextLines);
                    let end = (index + request.contextLines + 1).min(lines.len());
                    let matchContext = if request.contextLines > 0 {
                        Some(lines[start..end].join("\n"))
                    } else {
                        None
                    };
                    lineMatches.push(GrepLineMatch {
                        lineNumber,
                        lineContent: (*line).to_string(),
                        matchContext,
                    });
                    if lineMatches.len() >= request.maxResults {
                        break;
                    }
                }
            }
            if !lineMatches.is_empty() {
                matches.push(GrepFileMatch {
                    filePath,
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
            return Err(HostError::new(format!("Source path does not exist: {source}")));
        }
        ensure_parent_directory(destination)?;
        let destinationFile = File::create(destination)?;
        let mut zipWriter = ZipWriter::new(destinationFile);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);
        if sourcePath.is_dir() {
            let baseName = sourcePath
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| "root".to_string());
            zip_directory(sourcePath, sourcePath, &baseName, &mut zipWriter, options)?;
        } else {
            let fileName = sourcePath
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .ok_or_else(|| HostError::new(format!("Invalid source path: {source}")))?;
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
            let enclosedPath = entry
                .enclosed_name()
                .ok_or_else(|| HostError::new("Zip entry contains invalid path"))?
                .to_path_buf();
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
        let status = Command::new("cmd")
            .args(["/C", "start", "", path])
            .status()?;
        if !status.success() {
            return Err(HostError::new(format!(
                "Failed to open file with system default application: {path}"
            )));
        }
        Ok(())
    }

    fn shareFile(&self, path: &str, _title: &str) -> HostResult<()> {
        self.validateReadableFile(path)?;
        Err(HostError::new(
            "File sharing is not supported in Windows host yet",
        ))
    }
}

fn ensure_parent_directory(path: &str) -> HostResult<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn validate_readable_file(host: &WindowsFileSystemHost, path: &str) -> HostResult<()> {
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

impl WindowsFileSystemHost {
    fn validateReadableFile(&self, path: &str) -> HostResult<()> {
        validate_readable_file(self, path)
    }
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
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_default()
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
        format!("{zipPrefix}/{}", relative.to_string_lossy().replace('\\', "/"))
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

fn collect_matching_files(
    root: &Path,
    request: &FindFilesRequest,
    depth: i32,
    output: &mut Vec<String>,
) -> HostResult<()> {
    if request.maxDepth >= 0 && depth > request.maxDepth {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let entryPath = entry.path();
        if entryPath.is_dir() {
            collect_matching_files(&entryPath, request, depth + 1, output)?;
        } else if glob_matches(&request.pattern, &entryPath.to_string_lossy(), request.caseInsensitive) {
            output.push(entryPath.to_string_lossy().to_string());
        }
    }
    Ok(())
}

fn glob_matches(pattern: &str, value: &str, caseInsensitive: bool) -> bool {
    let patternValue = if caseInsensitive {
        pattern.to_ascii_lowercase()
    } else {
        pattern.to_string()
    };
    let valueValue = if caseInsensitive {
        value.to_ascii_lowercase()
    } else {
        value.to_string()
    };
    glob_match_bytes(patternValue.as_bytes(), valueValue.as_bytes())
}

fn glob_match_bytes(pattern: &[u8], value: &[u8]) -> bool {
    let mut patternIndex = 0usize;
    let mut valueIndex = 0usize;
    let mut starIndex = None;
    let mut matchIndex = 0usize;
    while valueIndex < value.len() {
        if patternIndex < pattern.len() && (pattern[patternIndex] == b'?' || pattern[patternIndex] == value[valueIndex]) {
            patternIndex += 1;
            valueIndex += 1;
        } else if patternIndex < pattern.len() && pattern[patternIndex] == b'*' {
            starIndex = Some(patternIndex);
            matchIndex = valueIndex;
            patternIndex += 1;
        } else if let Some(starIndexValue) = starIndex {
            patternIndex = starIndexValue + 1;
            matchIndex += 1;
            valueIndex = matchIndex;
        } else {
            return false;
        }
    }
    while patternIndex < pattern.len() && pattern[patternIndex] == b'*' {
        patternIndex += 1;
    }
    patternIndex == pattern.len()
}
