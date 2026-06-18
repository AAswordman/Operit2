use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::{
    FileEntry, FileSystemHost, FindFilesRequest, GrepCodeRequest, GrepCodeResult, HttpHost,
    HttpRequestData, SystemOperationHost,
};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::FileBindingService::{
    FileBindingService, StructuredEditAction, StructuredEditOperation,
};
use crate::api::chat::enhance::ToolExecutionManager::ToolExecutionManager;
use crate::api::chat::enhance::ToolExecutionManager::{
    AITool, ToolExecutor, ToolParameter, ToolValidationResult,
};
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::files::PathMapper::PathMapper;
use crate::core::files::VisualFileSystem::VisualFileSystem;
use crate::core::tools::ToolExecutionLimits::ToolExecutionLimits;
use crate::core::tools::ToolResultDataClasses::{
    stringResultData, BinaryFileContentData, DirectoryListingData, FileApplyResultData,
    FileContentData, FileEntry as ToolFileEntry, FileExistsData, FileInfoData, FileOperationData,
    FilePartContentData, FindFilesResultData, GrepFileMatch, GrepLineMatch, GrepResultData,
    ToolResultData,
};
use crate::util::OCRUtils::{OCRUtils, Quality as OCRQuality};

use super::StandardWebVisitTool::StandardWebVisitTool;

#[derive(Clone)]
pub struct StandardFileSystemTools {
    pub host: Arc<dyn FileSystemHost>,
    pub httpHost: Arc<dyn HttpHost>,
    pub systemOperationHost: Option<Arc<dyn SystemOperationHost>>,
    runtimeStoreRoot: PathBuf,
    appFilesRoot: Option<PathBuf>,
    workspaceCollectionRoot: PathBuf,
}

impl StandardFileSystemTools {
    pub fn new(
        host: Arc<dyn FileSystemHost>,
        httpHost: Arc<dyn HttpHost>,
        systemOperationHost: Option<Arc<dyn SystemOperationHost>>,
        runtimeStoreRoot: PathBuf,
        appFilesRoot: Option<PathBuf>,
        workspaceCollectionRoot: PathBuf,
    ) -> Self {
        Self {
            host,
            httpHost,
            systemOperationHost,
            runtimeStoreRoot,
            appFilesRoot,
            workspaceCollectionRoot,
        }
    }

    fn vfs(&self) -> VisualFileSystem {
        VisualFileSystem::new(
            self.host.clone(),
            PathMapper::new(
                self.runtimeStoreRoot.clone(),
                self.appFilesRoot.clone(),
                self.workspaceCollectionRoot.clone(),
            ),
        )
    }

    #[allow(non_snake_case)]
    pub fn listFiles(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let vfs = self.vfs();
        match vfs.listFiles(&path) {
            Ok(entries) => successData(
                tool,
                ToolResultData::DirectoryListingData(DirectoryListingData {
                    path: path.clone(),
                    entries: entries.iter().map(toolFileEntry).collect(),
                }),
            ),
            Err(error) => toolError(tool, String::new(), error),
        }
    }

    #[allow(non_snake_case)]
    pub fn readFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let vfs = self.vfs();

        match vfs.fileExists(&path) {
            Ok(existence) if existence.exists && !existence.isDirectory => {
                let fileExt = fileExtension(&path);
                if isSpecialFileType(&fileExt) {
                    let fullResult = self.readFileFull(tool);
                    if !fullResult.success {
                        return fullResult;
                    }

                    let ToolResultData::FileContentData(contentData) = fullResult.result else {
                        return toolError(
                            tool,
                            String::new(),
                            "Unexpected read_file_full result".to_string(),
                        );
                    };

                    let mut content = contentData.content;
                    let isTruncated = content.len() > ToolExecutionLimits::MAX_FILE_READ_BYTES;
                    if isTruncated {
                        content = content
                            .chars()
                            .take(ToolExecutionLimits::MAX_FILE_READ_BYTES)
                            .collect();
                    }
                    let mut contentWithLineNumbers = addLineNumbers(&content, 0, 0);
                    if isTruncated {
                        contentWithLineNumbers.push_str("\n\n... (file content truncated) ...");
                    }
                    return successData(
                        tool,
                        ToolResultData::FileContentData(FileContentData {
                            path: path.clone(),
                            size: contentWithLineNumbers.len() as i64,
                            content: contentWithLineNumbers,
                        }),
                    );
                }

                match vfs.readFileWithLimit(&path, ToolExecutionLimits::MAX_FILE_READ_BYTES) {
                    Ok(content) => {
                        let mut finalContent = addLineNumbers(&content, 0, 0);
                        if existence.size > ToolExecutionLimits::MAX_FILE_READ_BYTES as i64 {
                            finalContent.push_str("\n\n... (file content truncated) ...");
                        }
                        successData(
                            tool,
                            ToolResultData::FileContentData(FileContentData {
                                path: path.clone(),
                                content: finalContent,
                                size: existence.size,
                            }),
                        )
                    }
                    Err(error) => {
                        toolError(tool, String::new(), format!("Error reading file: {error}"))
                    }
                }
            }
            Ok(_) => toolError(tool, String::new(), format!("Path is not a file: {path}")),
            Err(error) => toolError(tool, String::new(), error),
        }
    }

    #[allow(non_snake_case)]
    pub fn readFileFull(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let vfs = self.vfs();

        match vfs.fileExists(&path) {
            Ok(existence) if existence.exists && !existence.isDirectory => {
                let fileExt = fileExtension(&path);
                if isSpecialFileType(&fileExt) {
                    return self.handleSpecialFileRead(tool, &vfs, &path, &fileExt);
                }

                match vfs.readFile(&path) {
                    Ok(content) => successData(
                        tool,
                        ToolResultData::FileContentData(FileContentData {
                            path: path.clone(),
                            size: existence.size,
                            content,
                        }),
                    ),
                    Err(error) => {
                        toolError(tool, String::new(), format!("Error reading file: {error}"))
                    }
                }
            }
            Ok(existence) if !existence.exists => {
                toolError(tool, String::new(), format!("File does not exist: {path}"))
            }
            Ok(_) => toolError(tool, String::new(), format!("Path is not a file: {path}")),
            Err(error) => toolError(tool, String::new(), error),
        }
    }

    #[allow(non_snake_case)]
    fn handleSpecialFileRead(
        &self,
        tool: &AITool,
        vfs: &VisualFileSystem,
        path: &str,
        fileExt: &str,
    ) -> ToolResult {
        match fileExt {
            "jpg" | "jpeg" | "png" | "gif" | "bmp" => self.handleImageFileRead(tool, vfs, path),
            _ => toolError(
                tool,
                String::new(),
                format!("Unsupported special file type: {fileExt}"),
            ),
        }
    }

    #[allow(non_snake_case)]
    fn handleImageFileRead(&self, tool: &AITool, vfs: &VisualFileSystem, path: &str) -> ToolResult {
        let physicalPath = match vfs.resolvePath(path) {
            Ok(resolved) => resolved.physicalPath,
            Err(error) => return toolError(tool, String::new(), error),
        };
        let content = match self.recognizeImageText(&physicalPath) {
            Ok(ocrText) if ocrText.trim().is_empty() => "No text detected in image.".to_string(),
            Ok(ocrText) => ocrText,
            Err(error) => format!("Error extracting text from image: {error}"),
        };
        successData(
            tool,
            ToolResultData::FileContentData(FileContentData {
                path: path.to_string(),
                size: content.len() as i64,
                content,
            }),
        )
    }

    #[allow(non_snake_case)]
    fn recognizeImageText(&self, imagePath: &str) -> Result<String, String> {
        let Some(systemOperationHost) = self.systemOperationHost.clone() else {
            return Err("SystemOperationHost is required for OCR".to_string());
        };
        let context = OperitApplicationContext {
            systemOperationHost: Some(systemOperationHost),
            ..OperitApplicationContext::new()
        };
        let text = OCRUtils::recognizeText(&context, imagePath, OCRQuality::LOW);
        Ok(text)
    }

    #[allow(non_snake_case)]
    pub fn readFilePart(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let startLineParam = parameterValue(tool, "start_line")
            .parse::<usize>()
            .unwrap_or(1);
        let endLineParam =
            optionalParameterValue(tool, "end_line").and_then(|value| value.parse::<usize>().ok());
        let vfs = self.vfs();

        let content = match vfs.readFile(&path) {
            Ok(value) => value,
            Err(error) => {
                return toolError(
                    tool,
                    String::new(),
                    format!("Error reading file part: {error}"),
                )
            }
        };
        let lines = content.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        let totalLines = lines.len();
        let startLine = startLineParam.max(1).min(totalLines.max(1));
        let endLine = match endLineParam {
            Some(value) => value,
            None => startLine + ToolExecutionLimits::DEFAULT_FILE_READ_PART_LINES - 1,
        }
        .max(startLine)
        .min(totalLines.max(1));
        let startIndex = startLine.saturating_sub(1);
        let endIndex = endLine.min(totalLines);
        let mut partContent = if totalLines > 0 && startIndex < totalLines {
            lines[startIndex..endIndex].join("\n")
        } else {
            String::new()
        };
        let isTruncated = partContent.len() > ToolExecutionLimits::MAX_FILE_READ_BYTES;
        if isTruncated {
            partContent = partContent
                .chars()
                .take(ToolExecutionLimits::MAX_FILE_READ_BYTES)
                .collect();
        }
        let mut numbered = addLineNumbers(&partContent, startIndex, totalLines);
        if isTruncated {
            numbered.push_str("\n\n... (file content truncated) ...");
        }
        successData(
            tool,
            ToolResultData::FilePartContentData(FilePartContentData {
                path: path.clone(),
                content: numbered,
                partIndex: 0,
                totalParts: 1,
                startLine: startIndex as i32,
                endLine: endIndex as i32,
                totalLines: totalLines as i32,
            }),
        )
    }

    #[allow(non_snake_case)]
    pub fn readFileBinary(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let vfs = self.vfs();

        match vfs.readFileBytes(&path) {
            Ok(bytes) => {
                let base64Content = STANDARD.encode(&bytes);
                successData(
                    tool,
                    ToolResultData::BinaryFileContentData(BinaryFileContentData {
                        path: path.clone(),
                        size: bytes.len() as i64,
                        contentBase64: base64Content,
                    }),
                )
            }
            Err(error) => toolError(
                tool,
                String::new(),
                format!("Error reading binary file: {error}"),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn writeFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let content = parameterValue(tool, "content");
        let append = parameterBool(tool, "append");
        let vfs = self.vfs();

        match vfs.writeFile(&path, &content, append) {
            Ok(()) => {
                let operation = if append { "append" } else { "write" };
                let details = if append {
                    format!("Content appended to {path}")
                } else {
                    format!("Content written to {path}")
                };
                successData(tool, fileOperationResult(operation, &path, true, details))
            }
            Err(errorValue) => {
                let message = format!("Error writing to file: {errorValue}");
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn writeFileBinary(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let base64Content = parameterValue(tool, "base64Content");
        let vfs = self.vfs();

        let decoded = match STANDARD.decode(base64Content.as_bytes()) {
            Ok(value) => value,
            Err(errorValue) => {
                let message = format!("Invalid base64 content: {errorValue}");
                return toolError(tool, fileOperationDataToString(&message), message);
            }
        };
        match vfs.writeFileBytes(&path, &decoded) {
            Ok(()) => successData(
                tool,
                fileOperationResult(
                    "write_binary",
                    &path,
                    true,
                    format!("Binary content written to {path} ({} bytes)", decoded.len()),
                ),
            ),
            Err(errorValue) => {
                let message = format!("Error writing binary file: {errorValue}");
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn deleteFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let recursive = parameterBool(tool, "recursive");
        let vfs = self.vfs();

        match vfs.deleteFile(&path, recursive) {
            Ok(()) => successData(
                tool,
                fileOperationResult(
                    "delete",
                    &path,
                    true,
                    format!("Successfully deleted {path}"),
                ),
            ),
            Err(errorValue) => {
                let message = errorValue;
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn fileExists(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let vfs = self.vfs();

        match vfs.fileExists(&path) {
            Ok(existence) => successData(
                tool,
                ToolResultData::FileExistsData(FileExistsData {
                    path: path.clone(),
                    exists: existence.exists,
                    isDirectory: existence.isDirectory,
                    size: existence.size,
                }),
            ),
            Err(error) => toolError(tool, String::new(), error),
        }
    }

    #[allow(non_snake_case)]
    pub fn moveFile(&self, tool: &AITool) -> ToolResult {
        let sourcePath = parameterValue(tool, "source");
        let destPath = parameterValue(tool, "destination");
        let vfs = self.vfs();

        match vfs.moveFile(&sourcePath, &destPath) {
            Ok(()) => successData(
                tool,
                fileOperationResult(
                    "move",
                    &sourcePath,
                    true,
                    format!("Successfully moved {sourcePath} to {destPath}"),
                ),
            ),
            Err(errorValue) => {
                let message = errorValue;
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn copyFile(&self, tool: &AITool) -> ToolResult {
        let sourcePath = parameterValue(tool, "source");
        let destPath = parameterValue(tool, "destination");
        let recursive = parameterBoolDefaultTrue(tool, "recursive");
        let vfs = self.vfs();

        match vfs.copyFile(&sourcePath, &destPath, recursive) {
            Ok(()) => successData(
                tool,
                fileOperationResult(
                    "copy",
                    &sourcePath,
                    true,
                    format!("Successfully copied {sourcePath} to {destPath}"),
                ),
            ),
            Err(errorValue) => {
                let message = errorValue;
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn makeDirectory(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let createParents = parameterBool(tool, "create_parents");
        let vfs = self.vfs();

        match vfs.makeDirectory(&path, createParents) {
            Ok(()) => successData(
                tool,
                fileOperationResult("mkdir", &path, true, format!("Directory created: {path}")),
            ),
            Err(errorValue) => {
                let message = format!("Error creating directory: {errorValue}");
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn findFiles(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let pattern = parameterValue(tool, "pattern");
        let vfs = self.vfs();
        if pattern.trim().is_empty() {
            return toolError(
                tool,
                ToolResultData::FindFilesResultData(FindFilesResultData {
                    path: path.clone(),
                    pattern: pattern.clone(),
                    files: Vec::new(),
                })
                .toJson(),
                "pattern parameter is required".to_string(),
            );
        }

        let request = FindFilesRequest {
            path: path.clone(),
            pattern: pattern.clone(),
            maxDepth: parameterValue(tool, "max_depth")
                .parse::<i32>()
                .unwrap_or(-1),
            usePathPattern: parameterBool(tool, "use_path_pattern"),
            caseInsensitive: parameterBool(tool, "case_insensitive"),
        };
        match vfs.findFiles(request) {
            Ok(files) => successData(
                tool,
                ToolResultData::FindFilesResultData(FindFilesResultData {
                    path: path.clone(),
                    pattern: pattern.clone(),
                    files,
                }),
            ),
            Err(errorValue) => toolError(
                tool,
                ToolResultData::FindFilesResultData(FindFilesResultData {
                    path: path.clone(),
                    pattern: pattern.clone(),
                    files: Vec::new(),
                })
                .toJson(),
                errorValue,
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn fileInfo(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let vfs = self.vfs();

        match vfs.fileInfo(&path) {
            Ok(info) => successData(
                tool,
                ToolResultData::FileInfoData(FileInfoData {
                    path: info.path,
                    exists: info.exists,
                    fileType: info.fileType,
                    size: info.size,
                    permissions: info.permissions,
                    owner: info.owner,
                    group: info.group,
                    lastModified: info.lastModified,
                    rawStatOutput: info.rawStatOutput,
                }),
            ),
            Err(errorValue) => toolError(tool, String::new(), errorValue),
        }
    }

    #[allow(non_snake_case)]
    pub fn grepCode(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let pattern = parameterValue(tool, "pattern");
        let vfs = self.vfs();
        if pattern.trim().is_empty() {
            return toolError(
                tool,
                String::new(),
                "Pattern parameter is required".to_string(),
            );
        }

        let request = GrepCodeRequest {
            path: path.clone(),
            pattern: pattern.clone(),
            filePattern: match optionalParameterValue(tool, "file_pattern") {
                Some(value) if !value.trim().is_empty() => value,
                _ => "*".to_string(),
            },
            caseInsensitive: parameterBool(tool, "case_insensitive"),
            contextLines: parameterValue(tool, "context_lines")
                .parse::<usize>()
                .unwrap_or(3),
            maxResults: parameterValue(tool, "max_results")
                .parse::<usize>()
                .unwrap_or(100),
        };
        match vfs.grepCode(request) {
            Ok(result) => successData(
                tool,
                ToolResultData::GrepResultData(grepResultData(&path, &pattern, &result)),
            ),
            Err(errorValue) => toolError(tool, String::new(), errorValue),
        }
    }

    #[allow(non_snake_case)]
    pub fn grepContext(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let intent = parameterValue(tool, "intent");
        let vfs = self.vfs();
        if intent.trim().is_empty() {
            return toolError(
                tool,
                String::new(),
                "Intent parameter is required".to_string(),
            );
        }

        let request = GrepCodeRequest {
            path: path.clone(),
            pattern: intent.clone(),
            filePattern: match optionalParameterValue(tool, "file_pattern") {
                Some(value) if !value.trim().is_empty() => value,
                _ => "*".to_string(),
            },
            caseInsensitive: true,
            contextLines: parameterValue(tool, "context_lines")
                .parse::<usize>()
                .unwrap_or(8),
            maxResults: parameterValue(tool, "max_results")
                .parse::<usize>()
                .unwrap_or(10),
        };
        match vfs.grepCode(request) {
            Ok(result) => successData(
                tool,
                ToolResultData::GrepResultData(grepResultData(&path, &intent, &result)),
            ),
            Err(errorValue) => toolError(tool, String::new(), errorValue),
        }
    }

    #[allow(non_snake_case)]
    pub fn downloadFile(&self, tool: &AITool) -> ToolResult {
        let urlParam = parameterValue(tool, "url");
        let visitKey = parameterValue(tool, "visit_key");
        let linkNumberStr = optionalParameterValue(tool, "link_number");
        let imageNumberStr = optionalParameterValue(tool, "image_number");
        let destPath = parameterValue(tool, "destination");
        let headersParam = optionalParameterValue(tool, "headers");
        let vfs = self.vfs();

        let mut resolvedUrl = urlParam;
        if resolvedUrl.trim().is_empty() {
            let linkNumber = parseIndex(linkNumberStr.as_deref());
            let imageNumber = parseIndex(imageNumberStr.as_deref());
            if visitKey.trim().is_empty() || (linkNumber.is_none() && imageNumber.is_none()) {
                return toolError(
                    tool,
                    fileOperationDataToString(
                        &format!(
                            "Download failed for {destPath}: Either url or (visit_key + link_number/image_number) is required"
                        ),
                    ),
                    "Either url or (visit_key + link_number/image_number) is required".to_string(),
                );
            }
            let Some(cached) = StandardWebVisitTool::getCachedVisitResult(&visitKey) else {
                return toolError(
                    tool,
                    fileOperationDataToString(&format!(
                        "Download failed for {destPath}: Invalid visit key."
                    )),
                    "Invalid visit key.".to_string(),
                );
            };
            resolvedUrl = if let Some(index) = linkNumber {
                cached
                    .links
                    .get(index.saturating_sub(1) as usize)
                    .map(|link| link.url.clone())
                    .unwrap_or_default()
            } else if let Some(index) = imageNumber {
                cached
                    .imageLinks
                    .get(index.saturating_sub(1) as usize)
                    .cloned()
                    .unwrap_or_default()
            } else {
                String::new()
            };
            if resolvedUrl.trim().is_empty() {
                return toolError(
                    tool,
                    fileOperationDataToString(&format!(
                        "Download failed for {destPath}: Index out of bounds."
                    )),
                    "Index out of bounds.".to_string(),
                );
            }
        }

        if resolvedUrl.trim().is_empty() || destPath.trim().is_empty() {
            return toolError(
                tool,
                fileOperationDataToString(&format!(
                    "Download failed for {destPath}: URL and destination parameters are required"
                )),
                "URL and destination parameters are required".to_string(),
            );
        }

        if !resolvedUrl.starts_with("http://") && !resolvedUrl.starts_with("https://") {
            return toolError(
                tool,
                fileOperationDataToString(&format!(
                    "Download failed for {destPath}: URL must start with http:// or https://"
                )),
                "URL must start with http:// or https://".to_string(),
            );
        }

        let headers = match parseHeaders(headersParam.as_deref()) {
            Ok(headers) => headers,
            Err(error) => {
                return toolError(
                    tool,
                    fileOperationDataToString(&format!("Download failed for {destPath}: {error}")),
                    error,
                )
            }
        };

        let response = match self.httpHost.executeHttpRequest(HttpRequestData {
            url: resolvedUrl.trim().to_string(),
            method: "GET".to_string(),
            headers,
            body: Vec::new(),
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: 15,
            readTimeoutSeconds: 30,
            followRedirects: true,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        }) {
            Ok(response) => response,
            Err(error) => {
                let message = format!("Error downloading file: {error}");
                return toolError(tool, fileOperationDataToString(&message), message);
            }
        };
        if !(200..300).contains(&response.statusCode) {
            let message = format!("Error downloading file: HTTP {}", response.statusCode);
            return toolError(tool, fileOperationDataToString(&message), message);
        }
        let bytes = response.body;

        match vfs.writeFileBytes(&destPath, bytes.as_ref()) {
            Ok(()) => successData(
                tool,
                fileOperationResult(
                    "download",
                    &destPath,
                    true,
                    format!(
                        "File downloaded successfully: {} -> {} (file size: {})",
                        resolvedUrl.trim(),
                        destPath,
                        formatSize(bytes.len() as u64)
                    ),
                ),
            ),
            Err(error) => {
                let message = format!("Error downloading file: {error}");
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn createFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let newContent = parameterValue(tool, "new");
        let mut results = self.applyFile(&AITool {
            name: "apply_file".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    value: path,
                },
                ToolParameter {
                    name: "type".to_string(),
                    value: "create".to_string(),
                },
                ToolParameter {
                    name: "new".to_string(),
                    value: newContent,
                },
            ],
        });
        results.remove(results.len() - 1)
    }

    #[allow(non_snake_case)]
    pub fn editFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let oldContent = parameterValue(tool, "old");
        let newContent = parameterValue(tool, "new");
        let mut results = self.applyFile(&AITool {
            name: "apply_file".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    value: path,
                },
                ToolParameter {
                    name: "type".to_string(),
                    value: "replace".to_string(),
                },
                ToolParameter {
                    name: "old".to_string(),
                    value: oldContent,
                },
                ToolParameter {
                    name: "new".to_string(),
                    value: newContent,
                },
            ],
        });
        results.remove(results.len() - 1)
    }

    #[allow(non_snake_case)]
    pub fn applyFile(&self, tool: &AITool) -> Vec<ToolResult> {
        let path = parameterValue(tool, "path");
        let operationType = optionalParameterValue(tool, "type")
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let oldContent = parameterValue(tool, "old");
        let newContent = parameterValue(tool, "new");
        let vfs = self.vfs();
        if path.trim().is_empty() {
            return vec![toolError(
                tool,
                fileOperationDataToString("Path parameter is required"),
                "Path parameter is required".to_string(),
            )];
        }
        if operationType.trim().is_empty() {
            return vec![toolError(
                tool,
                fileOperationDataToString("Type parameter is required (replace | delete | create)"),
                "Type parameter is required (replace | delete | create)".to_string(),
            )];
        }

        let existence = match vfs.fileExists(&path) {
            Ok(value) => value,
            Err(error) => return vec![toolError(tool, String::new(), error)],
        };
        if !existence.exists {
            if operationType != "create" {
                let message = "File does not exist. Use type=create with 'new' to create it.";
                return vec![toolError(
                    tool,
                    fileOperationDataToString(message),
                    message.to_string(),
                )];
            }
            if newContent.trim().is_empty() {
                let message = "Parameter 'new' is required for type=create";
                return vec![toolError(
                    tool,
                    fileOperationDataToString(message),
                    message.to_string(),
                )];
            }
            return vec![match vfs.writeFile(&path, &newContent, false) {
                Ok(()) => {
                    let diffContent = FileBindingService.generateUnifiedDiff("", &newContent);
                    let details = format!("Successfully created new file: {path}");
                    successData(
                        tool,
                        ToolResultData::FileApplyResultData(FileApplyResultData {
                            operation: fileOperationData("create", &path, true, details),
                            aiDiffInstructions: String::new(),
                            diffContent: Some(diffContent),
                        }),
                    )
                }
                Err(error) => {
                    let message = format!("Failed to create new file: {error}");
                    toolError(tool, fileOperationDataToString(&message), message)
                }
            }];
        }

        if operationType == "create" {
            let message = "If you need to rewrite a whole existing file: do NOT use apply_file to overwrite it. Instead, call delete_file first, then write_file.";
            return vec![toolError(
                tool,
                fileOperationDataToString(message),
                message.to_string(),
            )];
        }
        if existence.isDirectory {
            let message = format!("Path is not a file: {path}");
            return vec![toolError(
                tool,
                fileOperationDataToString(&message),
                message,
            )];
        }

        let originalContent = match vfs.readFile(&path) {
            Ok(value) => value,
            Err(error) => {
                let message = format!("Failed to read original file: {error}");
                return vec![toolError(
                    tool,
                    fileOperationDataToString(&message),
                    message,
                )];
            }
        };
        let editOperations = match operationType.as_str() {
            "replace" => {
                if oldContent.trim().is_empty() || newContent.trim().is_empty() {
                    let message = "Both 'old' and 'new' are required for type=replace";
                    return vec![toolError(
                        tool,
                        fileOperationDataToString(message),
                        message.to_string(),
                    )];
                }
                vec![StructuredEditOperation {
                    action: StructuredEditAction::REPLACE,
                    oldContent,
                    newContent,
                }]
            }
            "delete" => {
                if oldContent.trim().is_empty() {
                    let message = "Parameter 'old' is required for type=delete";
                    return vec![toolError(
                        tool,
                        fileOperationDataToString(message),
                        message.to_string(),
                    )];
                }
                vec![StructuredEditOperation {
                    action: StructuredEditAction::DELETE,
                    oldContent,
                    newContent: String::new(),
                }]
            }
            _ => {
                let message = format!(
                    "Unsupported type: {operationType} (expected replace | delete | create)"
                );
                return vec![toolError(
                    tool,
                    fileOperationDataToString(&message),
                    message,
                )];
            }
        };

        let (mergedContent, aiInstructions) =
            FileBindingService.processFileBindingOperations(&originalContent, &editOperations);
        if aiInstructions.to_ascii_lowercase().starts_with("error") {
            return vec![toolError(
                tool,
                fileOperationDataToString(&format!("File binding failed: {aiInstructions}")),
                aiInstructions,
            )];
        }
        vec![match vfs.writeFile(&path, &mergedContent, false) {
            Ok(()) => {
                let details = format!("Successfully applied AI code to file: {path}");
                let diffContent =
                    FileBindingService.generateUnifiedDiff(&originalContent, &mergedContent);
                successData(
                    tool,
                    ToolResultData::FileApplyResultData(FileApplyResultData {
                        operation: fileOperationData("apply", &path, true, details),
                        aiDiffInstructions: aiInstructions,
                        diffContent: Some(diffContent),
                    }),
                )
            }
            Err(error) => {
                let message = format!("Failed to write merged file: {error}");
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }]
    }

    #[allow(non_snake_case)]
    pub fn zipFiles(&self, tool: &AITool) -> ToolResult {
        let source = parameterValue(tool, "source");
        let destination = parameterValue(tool, "destination");
        let vfs = self.vfs();
        match vfs.zipFiles(&source, &destination) {
            Ok(()) => successData(
                tool,
                fileOperationResult(
                    "zip",
                    &source,
                    true,
                    format!("Successfully compressed {source} to {destination}"),
                ),
            ),
            Err(errorValue) => {
                let message = errorValue;
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn unzipFiles(&self, tool: &AITool) -> ToolResult {
        let source = parameterValue(tool, "source");
        let destination = parameterValue(tool, "destination");
        let vfs = self.vfs();
        match vfs.unzipFiles(&source, &destination) {
            Ok(()) => successData(
                tool,
                fileOperationResult(
                    "unzip",
                    &source,
                    true,
                    format!("Successfully extracted {source} to {destination}"),
                ),
            ),
            Err(errorValue) => {
                let message = errorValue;
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn openFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let vfs = self.vfs();
        match vfs.openFile(&path) {
            Ok(()) => successData(
                tool,
                fileOperationResult("open", &path, true, format!("Requested open for {path}")),
            ),
            Err(errorValue) => {
                let message = errorValue;
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn shareFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let title =
            optionalParameterValue(tool, "title").unwrap_or_else(|| "Share File".to_string());
        let vfs = self.vfs();
        match vfs.shareFile(&path, &title) {
            Ok(()) => successData(
                tool,
                fileOperationResult("share", &path, true, format!("Requested share for {path}")),
            ),
            Err(errorValue) => {
                let message = errorValue;
                toolError(tool, fileOperationDataToString(&message), message)
            }
        }
    }
}

pub struct FileSystemToolExecutor {
    pub tools: StandardFileSystemTools,
    pub operation: FileSystemToolOperation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileSystemToolOperation {
    ListFiles,
    ReadFile,
    ReadFilePart,
    ReadFileFull,
    ReadFileBinary,
    WriteFile,
    WriteFileBinary,
    DeleteFile,
    FileExists,
    MoveFile,
    CopyFile,
    MakeDirectory,
    FindFiles,
    FileInfo,
    GrepCode,
    GrepContext,
    DownloadFile,
    ApplyFile,
    CreateFile,
    EditFile,
    ZipFiles,
    UnzipFiles,
    OpenFile,
    ShareFile,
}

impl ToolExecutor for FileSystemToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        let names = requiredParameters(&self.operation);
        for name in names {
            if parameterValue(tool, name).trim().is_empty() {
                return ToolValidationResult {
                    valid: false,
                    errorMessage: format!("{name} parameter is required"),
                };
            }
        }
        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        let result = match self.operation {
            FileSystemToolOperation::ListFiles => self.tools.listFiles(tool),
            FileSystemToolOperation::ReadFile => self.tools.readFile(tool),
            FileSystemToolOperation::ReadFilePart => self.tools.readFilePart(tool),
            FileSystemToolOperation::ReadFileFull => self.tools.readFileFull(tool),
            FileSystemToolOperation::ReadFileBinary => self.tools.readFileBinary(tool),
            FileSystemToolOperation::WriteFile => self.tools.writeFile(tool),
            FileSystemToolOperation::WriteFileBinary => self.tools.writeFileBinary(tool),
            FileSystemToolOperation::DeleteFile => self.tools.deleteFile(tool),
            FileSystemToolOperation::FileExists => self.tools.fileExists(tool),
            FileSystemToolOperation::MoveFile => self.tools.moveFile(tool),
            FileSystemToolOperation::CopyFile => self.tools.copyFile(tool),
            FileSystemToolOperation::MakeDirectory => self.tools.makeDirectory(tool),
            FileSystemToolOperation::FindFiles => self.tools.findFiles(tool),
            FileSystemToolOperation::FileInfo => self.tools.fileInfo(tool),
            FileSystemToolOperation::GrepCode => self.tools.grepCode(tool),
            FileSystemToolOperation::GrepContext => self.tools.grepContext(tool),
            FileSystemToolOperation::DownloadFile => self.tools.downloadFile(tool),
            FileSystemToolOperation::ApplyFile => return self.tools.applyFile(tool),
            FileSystemToolOperation::CreateFile => self.tools.createFile(tool),
            FileSystemToolOperation::EditFile => self.tools.editFile(tool),
            FileSystemToolOperation::ZipFiles => self.tools.zipFiles(tool),
            FileSystemToolOperation::UnzipFiles => self.tools.unzipFiles(tool),
            FileSystemToolOperation::OpenFile => self.tools.openFile(tool),
            FileSystemToolOperation::ShareFile => self.tools.shareFile(tool),
        };
        vec![result]
    }
}

fn requiredParameters(operation: &FileSystemToolOperation) -> &'static [&'static str] {
    match operation {
        FileSystemToolOperation::ListFiles
        | FileSystemToolOperation::ReadFile
        | FileSystemToolOperation::ReadFilePart
        | FileSystemToolOperation::ReadFileFull
        | FileSystemToolOperation::ReadFileBinary
        | FileSystemToolOperation::DeleteFile
        | FileSystemToolOperation::FileExists
        | FileSystemToolOperation::MakeDirectory
        | FileSystemToolOperation::FileInfo => &["path"],
        FileSystemToolOperation::WriteFile => &["path", "content"],
        FileSystemToolOperation::WriteFileBinary => &["path", "base64Content"],
        FileSystemToolOperation::MoveFile | FileSystemToolOperation::CopyFile => {
            &["source", "destination"]
        }
        FileSystemToolOperation::FindFiles | FileSystemToolOperation::GrepCode => {
            &["path", "pattern"]
        }
        FileSystemToolOperation::GrepContext => &["path", "intent"],
        FileSystemToolOperation::DownloadFile => &["destination"],
        FileSystemToolOperation::ApplyFile => &[],
        FileSystemToolOperation::CreateFile => &["path", "new"],
        FileSystemToolOperation::EditFile => &["path", "old", "new"],
        FileSystemToolOperation::ZipFiles | FileSystemToolOperation::UnzipFiles => {
            &["source", "destination"]
        }
        FileSystemToolOperation::OpenFile | FileSystemToolOperation::ShareFile => &["path"],
    }
}

fn successData(tool: &AITool, data: ToolResultData) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: data,
        error: None,
    }
}

fn success(tool: &AITool, result: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: stringResultData(result),
        error: None,
    }
}

fn toolError(tool: &AITool, result: String, message: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(result),
        error: Some(message),
    }
}

fn toolFileEntry(entry: &FileEntry) -> ToolFileEntry {
    ToolFileEntry {
        name: entry.name.clone(),
        isDirectory: entry.isDirectory,
        size: entry.size,
        permissions: entry.permissions.clone(),
        lastModified: entry.lastModified.clone(),
    }
}

fn fileOperationResult(
    operation: &str,
    path: &str,
    successful: bool,
    details: String,
) -> ToolResultData {
    ToolResultData::FileOperationData(fileOperationData(operation, path, successful, details))
}

fn fileOperationData(
    operation: &str,
    path: &str,
    successful: bool,
    details: String,
) -> FileOperationData {
    FileOperationData {
        operation: operation.to_string(),
        path: path.to_string(),
        successful,
        details,
    }
}

fn grepResultData(searchPath: &str, pattern: &str, result: &GrepCodeResult) -> GrepResultData {
    GrepResultData {
        searchPath: searchPath.to_string(),
        pattern: pattern.to_string(),
        matches: result
            .matches
            .iter()
            .map(|fileMatch| GrepFileMatch {
                filePath: fileMatch.filePath.clone(),
                lineMatches: fileMatch
                    .lineMatches
                    .iter()
                    .map(|lineMatch| GrepLineMatch {
                        lineNumber: lineMatch.lineNumber as i32,
                        lineContent: lineMatch.lineContent.clone(),
                        matchContext: lineMatch.matchContext.clone(),
                    })
                    .collect(),
            })
            .collect(),
        totalMatches: result.totalMatches as i32,
        filesSearched: result.filesSearched as i32,
    }
}

fn fileOperationDataToString(details: &str) -> String {
    details.to_string()
}

fn parameterValue(tool: &AITool, name: &str) -> String {
    optionalParameterValue(tool, name).unwrap_or_default()
}

fn optionalParameterValue(tool: &AITool, name: &str) -> Option<String> {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.clone())
}

fn parameterBool(tool: &AITool, name: &str) -> bool {
    optionalParameterValue(tool, name)
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn parameterBoolDefaultTrue(tool: &AITool, name: &str) -> bool {
    optionalParameterValue(tool, name)
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
}

#[allow(non_snake_case)]
fn parseHeaders(headersJson: Option<&str>) -> Result<Vec<(String, String)>, String> {
    let Some(raw) = headersJson.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };
    let value = serde_json::from_str::<serde_json::Value>(raw)
        .map_err(|error| format!("Invalid headers JSON: {error}"))?;
    let Some(object) = value.as_object() else {
        return Err("headers must be a JSON object string".to_string());
    };
    let mut headers = Vec::new();
    for (key, value) in object {
        let Some(valueText) = value.as_str() else {
            return Err(format!("headers.{key} must be a string"));
        };
        if key.trim().is_empty() {
            return Err("Invalid header name: empty".to_string());
        }
        headers.push((key.clone(), valueText.to_string()));
    }
    Ok(headers)
}

#[allow(non_snake_case)]
fn parseIndex(raw: Option<&str>) -> Option<i32> {
    let value = raw.map(str::trim).unwrap_or_default();
    if value.is_empty() {
        return None;
    }
    value.parse::<i32>().ok()
}

#[allow(non_snake_case)]
fn formatSize(bytes: u64) -> String {
    if bytes > 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes > 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} bytes")
    }
}

fn addLineNumbers(content: &str, startLine: usize, totalLines: usize) -> String {
    let lines = content.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return String::new();
    }
    let maxDigits = if totalLines > 0 {
        totalLines.to_string().len()
    } else {
        lines.len().to_string().len()
    };
    let mut output = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        output.push_str(&format!(
            "{:>width$}| {line}",
            startLine + index + 1,
            width = maxDigits
        ));
    }
    output
}

#[allow(non_snake_case)]
fn fileExtension(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

#[allow(non_snake_case)]
fn isSpecialFileType(fileExtension: &str) -> bool {
    matches!(fileExtension, "jpg" | "jpeg" | "png" | "gif" | "bmp")
}
