use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::{
    FileEntry, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest, GrepCodeResult,
    HttpHost, HttpRequestData,
};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::{AITool, ToolExecutor, ToolValidationResult};
use crate::core::tools::ToolExecutionLimits::ToolExecutionLimits;

use super::super::PathValidator::PathValidator;
use super::StandardWebVisitTool::StandardWebVisitTool;

#[derive(Clone)]
pub struct StandardFileSystemTools {
    pub host: Arc<dyn FileSystemHost>,
    pub httpHost: Arc<dyn HttpHost>,
}

impl StandardFileSystemTools {
    pub fn new(host: Arc<dyn FileSystemHost>, httpHost: Arc<dyn HttpHost>) -> Self {
        Self { host, httpHost }
    }

    #[allow(non_snake_case)]
    pub fn listFiles(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.listFiles(&path) {
            Ok(entries) => success(
                tool,
                directoryListingDataToString(self.envLabel(), &path, &entries),
            ),
            Err(error) => toolError(tool, String::new(), error.message),
        }
    }

    #[allow(non_snake_case)]
    pub fn readFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.fileExists(&path) {
            Ok(existence) if existence.exists && !existence.isDirectory => {
                match self
                    .host
                    .readFileWithLimit(&path, ToolExecutionLimits::MAX_FILE_READ_BYTES)
                {
                    Ok(content) => {
                        let mut finalContent = addLineNumbers(&content, 0, 0);
                        if existence.size > ToolExecutionLimits::MAX_FILE_READ_BYTES as i64 {
                            finalContent.push_str("\n\n... (file content truncated) ...");
                        }
                        success(
                            tool,
                            fileContentDataToString(self.envLabel(), &path, &finalContent),
                        )
                    }
                    Err(error) => toolError(
                        tool,
                        String::new(),
                        format!("Error reading file: {}", error.message),
                    ),
                }
            }
            Ok(_) => toolError(tool, String::new(), format!("Path is not a file: {path}")),
            Err(error) => toolError(tool, String::new(), error.message),
        }
    }

    #[allow(non_snake_case)]
    pub fn readFileFull(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.fileExists(&path) {
            Ok(existence) if existence.exists && !existence.isDirectory => {
                match self.host.readFile(&path) {
                    Ok(content) => success(
                        tool,
                        fileContentDataToString(self.envLabel(), &path, &content),
                    ),
                    Err(error) => toolError(
                        tool,
                        String::new(),
                        format!("Error reading file: {}", error.message),
                    ),
                }
            }
            Ok(existence) if !existence.exists => {
                toolError(tool, String::new(), format!("File does not exist: {path}"))
            }
            Ok(_) => toolError(tool, String::new(), format!("Path is not a file: {path}")),
            Err(error) => toolError(tool, String::new(), error.message),
        }
    }

    #[allow(non_snake_case)]
    pub fn readFilePart(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let startLineParam = parameterValue(tool, "start_line")
            .parse::<usize>()
            .unwrap_or(1);
        let endLineParam =
            optionalParameterValue(tool, "end_line").and_then(|value| value.parse::<usize>().ok());
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        let content = match self.host.readFile(&path) {
            Ok(value) => value,
            Err(error) => {
                return toolError(
                    tool,
                    String::new(),
                    format!("Error reading file part: {}", error.message),
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
        success(
            tool,
            filePartContentDataToString(
                self.envLabel(),
                &numbered,
                0,
                1,
                startIndex,
                endIndex,
                totalLines,
            ),
        )
    }

    #[allow(non_snake_case)]
    pub fn readFileBinary(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.readFileBytes(&path) {
            Ok(bytes) => {
                let base64Content = STANDARD.encode(&bytes);
                success(
                    tool,
                    binaryFileContentDataToString(
                        self.envLabel(),
                        &path,
                        bytes.len(),
                        base64Content.len(),
                    ),
                )
            }
            Err(error) => toolError(
                tool,
                String::new(),
                format!("Error reading binary file: {}", error.message),
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn writeFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let content = parameterValue(tool, "content");
        let append = parameterBool(tool, "append");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.writeFile(&path, &content, append) {
            Ok(()) => {
                let operation = if append { "append" } else { "write" };
                let details = if append {
                    format!("Content appended to {path}")
                } else {
                    format!("Content written to {path}")
                };
                success(tool, fileOperationDataToString(self.envLabel(), &details))
            }
            Err(errorValue) => {
                let message = format!("Error writing to file: {}", errorValue.message);
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn writeFileBinary(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let base64Content = parameterValue(tool, "base64Content");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        let decoded = match STANDARD.decode(base64Content.as_bytes()) {
            Ok(value) => value,
            Err(errorValue) => {
                let message = format!("Invalid base64 content: {errorValue}");
                return toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                );
            }
        };
        match self.host.writeFileBytes(&path, &decoded) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(
                    self.envLabel(),
                    &format!("Binary content written to {path} ({} bytes)", decoded.len()),
                ),
            ),
            Err(errorValue) => {
                let message = format!("Error writing binary file: {}", errorValue.message);
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn deleteFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let recursive = parameterBool(tool, "recursive");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.deleteFile(&path, recursive) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(self.envLabel(), &format!("Successfully deleted {path}")),
            ),
            Err(errorValue) => {
                let message = errorValue.message;
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn fileExists(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.fileExists(&path) {
            Ok(existence) => success(
                tool,
                fileExistsDataToString(
                    self.envLabel(),
                    &path,
                    existence.exists,
                    existence.isDirectory,
                    existence.size,
                ),
            ),
            Err(error) => toolError(tool, String::new(), error.message),
        }
    }

    #[allow(non_snake_case)]
    pub fn moveFile(&self, tool: &AITool) -> ToolResult {
        let sourcePath = parameterValue(tool, "source");
        let destPath = parameterValue(tool, "destination");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &sourcePath, &tool.name, "source")
        {
            return result;
        }
        if let Some(result) = PathValidator::validateHostPath(
            self.host.as_ref(),
            &destPath,
            &tool.name,
            "destination",
        ) {
            return result;
        }

        match self.host.moveFile(&sourcePath, &destPath) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(
                    self.envLabel(),
                    &format!("Successfully moved {sourcePath} to {destPath}"),
                ),
            ),
            Err(errorValue) => {
                let message = errorValue.message;
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn copyFile(&self, tool: &AITool) -> ToolResult {
        let sourcePath = parameterValue(tool, "source");
        let destPath = parameterValue(tool, "destination");
        let recursive = parameterBoolDefaultTrue(tool, "recursive");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &sourcePath, &tool.name, "source")
        {
            return result;
        }
        if let Some(result) = PathValidator::validateHostPath(
            self.host.as_ref(),
            &destPath,
            &tool.name,
            "destination",
        ) {
            return result;
        }

        match self.host.copyFile(&sourcePath, &destPath, recursive) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(
                    self.envLabel(),
                    &format!("Successfully copied {sourcePath} to {destPath}"),
                ),
            ),
            Err(errorValue) => {
                let message = errorValue.message;
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn makeDirectory(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let createParents = parameterBool(tool, "create_parents");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.makeDirectory(&path, createParents) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(self.envLabel(), &format!("Directory created: {path}")),
            ),
            Err(errorValue) => {
                let message = format!("Error creating directory: {}", errorValue.message);
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn findFiles(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let pattern = parameterValue(tool, "pattern");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }
        if pattern.trim().is_empty() {
            return toolError(
                tool,
                findFilesResultDataToString(self.envLabel(), &path, &pattern, &[]),
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
        match self.host.findFiles(request) {
            Ok(files) => success(
                tool,
                findFilesResultDataToString(self.envLabel(), &path, &pattern, &files),
            ),
            Err(errorValue) => toolError(
                tool,
                findFilesResultDataToString(self.envLabel(), &path, &pattern, &[]),
                errorValue.message,
            ),
        }
    }

    #[allow(non_snake_case)]
    pub fn fileInfo(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.fileInfo(&path) {
            Ok(info) => success(tool, fileInfoDataToString(self.envLabel(), &info)),
            Err(errorValue) => toolError(tool, String::new(), errorValue.message),
        }
    }

    #[allow(non_snake_case)]
    pub fn grepCode(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let pattern = parameterValue(tool, "pattern");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }
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
        match self.host.grepCode(request) {
            Ok(result) => success(
                tool,
                grepResultDataToString(self.envLabel(), &path, &pattern, &result),
            ),
            Err(errorValue) => toolError(tool, String::new(), errorValue.message),
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

        if let Some(result) = PathValidator::validateHostPath(
            self.host.as_ref(),
            &destPath,
            &tool.name,
            "destination",
        ) {
            return result;
        }

        let mut resolvedUrl = urlParam;
        if resolvedUrl.trim().is_empty() {
            let linkNumber = parseIndex(linkNumberStr.as_deref());
            let imageNumber = parseIndex(imageNumberStr.as_deref());
            if visitKey.trim().is_empty() || (linkNumber.is_none() && imageNumber.is_none()) {
                return toolError(
                    tool,
                    fileOperationDataToString(
                        self.envLabel(),
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
                    fileOperationDataToString(
                        self.envLabel(),
                        &format!("Download failed for {destPath}: Invalid visit key."),
                    ),
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
                    fileOperationDataToString(
                        self.envLabel(),
                        &format!("Download failed for {destPath}: Index out of bounds."),
                    ),
                    "Index out of bounds.".to_string(),
                );
            }
        }

        if resolvedUrl.trim().is_empty() || destPath.trim().is_empty() {
            return toolError(
                tool,
                fileOperationDataToString(
                    self.envLabel(),
                    &format!(
                        "Download failed for {destPath}: URL and destination parameters are required"
                    ),
                ),
                "URL and destination parameters are required".to_string(),
            );
        }

        if !resolvedUrl.starts_with("http://") && !resolvedUrl.starts_with("https://") {
            return toolError(
                tool,
                fileOperationDataToString(
                    self.envLabel(),
                    &format!(
                        "Download failed for {destPath}: URL must start with http:// or https://"
                    ),
                ),
                "URL must start with http:// or https://".to_string(),
            );
        }

        let headers = match parseHeaders(headersParam.as_deref()) {
            Ok(headers) => headers,
            Err(error) => {
                return toolError(
                    tool,
                    fileOperationDataToString(
                        self.envLabel(),
                        &format!("Download failed for {destPath}: {error}"),
                    ),
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
                return toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                );
            }
        };
        if !(200..300).contains(&response.statusCode) {
            let message = format!("Error downloading file: HTTP {}", response.statusCode);
            return toolError(
                tool,
                fileOperationDataToString(self.envLabel(), &message),
                message,
            );
        }
        let bytes = response.body;

        match self.host.writeFileBytes(&destPath, bytes.as_ref()) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(
                    self.envLabel(),
                    &format!(
                        "File downloaded successfully: {} -> {} (file size: {})",
                        resolvedUrl.trim(),
                        destPath,
                        formatSize(bytes.len() as u64)
                    ),
                ),
            ),
            Err(error) => {
                let message = format!("Error downloading file: {}", error.message);
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn createFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let content = parameterValue(tool, "new");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        match self.host.writeFile(&path, &content, false) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(self.envLabel(), &format!("Content written to {path}")),
            ),
            Err(errorValue) => {
                let message = format!("Error writing to file: {}", errorValue.message);
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn editFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let oldContent = parameterValue(tool, "old");
        let newContent = parameterValue(tool, "new");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }

        let current = match self.host.readFile(&path) {
            Ok(value) => value,
            Err(errorValue) => {
                let message = format!("Error reading file: {}", errorValue.message);
                return toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                );
            }
        };
        if !current.contains(&oldContent) {
            let message = "Old content was not found in file.".to_string();
            return toolError(
                tool,
                fileOperationDataToString(self.envLabel(), &message),
                message,
            );
        }
        let updated = current.replacen(&oldContent, &newContent, 1);
        match self.host.writeFile(&path, &updated, false) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(self.envLabel(), &format!("Content written to {path}")),
            ),
            Err(errorValue) => {
                let message = format!("Error writing to file: {}", errorValue.message);
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn zipFiles(&self, tool: &AITool) -> ToolResult {
        let source = parameterValue(tool, "source");
        let destination = parameterValue(tool, "destination");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &source, &tool.name, "source")
        {
            return result;
        }
        if let Some(result) = PathValidator::validateHostPath(
            self.host.as_ref(),
            &destination,
            &tool.name,
            "destination",
        ) {
            return result;
        }
        match self.host.zipFiles(&source, &destination) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(
                    self.envLabel(),
                    &format!("Successfully compressed {source} to {destination}"),
                ),
            ),
            Err(errorValue) => {
                let message = errorValue.message;
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn unzipFiles(&self, tool: &AITool) -> ToolResult {
        let source = parameterValue(tool, "source");
        let destination = parameterValue(tool, "destination");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &source, &tool.name, "source")
        {
            return result;
        }
        if let Some(result) = PathValidator::validateHostPath(
            self.host.as_ref(),
            &destination,
            &tool.name,
            "destination",
        ) {
            return result;
        }
        match self.host.unzipFiles(&source, &destination) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(
                    self.envLabel(),
                    &format!("Successfully extracted {source} to {destination}"),
                ),
            ),
            Err(errorValue) => {
                let message = errorValue.message;
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn openFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }
        match self.host.openFile(&path) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(self.envLabel(), &format!("Requested open for {path}")),
            ),
            Err(errorValue) => {
                let message = errorValue.message;
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn shareFile(&self, tool: &AITool) -> ToolResult {
        let path = parameterValue(tool, "path");
        let title =
            optionalParameterValue(tool, "title").unwrap_or_else(|| "Share File".to_string());
        if let Some(result) =
            PathValidator::validateHostPath(self.host.as_ref(), &path, &tool.name, "path")
        {
            return result;
        }
        match self.host.shareFile(&path, &title) {
            Ok(()) => success(
                tool,
                fileOperationDataToString(self.envLabel(), &format!("Requested share for {path}")),
            ),
            Err(errorValue) => {
                let message = errorValue.message;
                toolError(
                    tool,
                    fileOperationDataToString(self.envLabel(), &message),
                    message,
                )
            }
        }
    }

    fn envLabel(&self) -> &str {
        self.host.envLabel()
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
    DownloadFile,
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
            FileSystemToolOperation::DownloadFile => self.tools.downloadFile(tool),
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
        FileSystemToolOperation::DownloadFile => &["destination"],
        FileSystemToolOperation::CreateFile => &["path", "new"],
        FileSystemToolOperation::EditFile => &["path", "old", "new"],
        FileSystemToolOperation::ZipFiles | FileSystemToolOperation::UnzipFiles => {
            &["source", "destination"]
        }
        FileSystemToolOperation::OpenFile | FileSystemToolOperation::ShareFile => &["path"],
    }
}

fn success(tool: &AITool, result: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result,
        error: None,
    }
}

fn toolError(tool: &AITool, result: String, message: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result,
        error: Some(message),
    }
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

fn directoryListingDataToString(env: &str, path: &str, entries: &[FileEntry]) -> String {
    let mut output = format!("[{env}] Directory listing for {path}:\n");
    for entry in entries {
        let typeIndicator = if entry.isDirectory { "d" } else { "-" };
        output.push_str(&format!(
            "{typeIndicator}{} {:>8} {} {}\n",
            entry.permissions, entry.size, entry.lastModified, entry.name
        ));
    }
    output
}

fn fileContentDataToString(env: &str, path: &str, content: &str) -> String {
    format!("[{env}] Content of {path}:\n{content}")
}

fn binaryFileContentDataToString(
    env: &str,
    path: &str,
    size: usize,
    base64Length: usize,
) -> String {
    format!("[{env}] Binary content of {path} ({size} bytes, base64 length={base64Length})")
}

fn fileExistsDataToString(
    env: &str,
    path: &str,
    exists: bool,
    isDirectory: bool,
    size: i64,
) -> String {
    if exists {
        let fileType = if isDirectory { "Directory" } else { "File" };
        format!("[{env}] {fileType} exists at path: {path} (size: {size} bytes)")
    } else {
        format!("[{env}] No file or directory exists at path: {path}")
    }
}

fn fileInfoDataToString(env: &str, info: &FileInfo) -> String {
    if !info.exists {
        return format!(
            "[{env}] File or directory does not exist at path: {}",
            info.path
        );
    }
    format!(
        "[{env}] File information for {}:\nType: {}\nSize: {} bytes\nPermissions: {}\nOwner: {}\nGroup: {}\nLast modified: {}\n",
        info.path,
        info.fileType,
        info.size,
        info.permissions,
        info.owner,
        info.group,
        info.lastModified
    )
}

fn fileOperationDataToString(env: &str, details: &str) -> String {
    format!("[{env}] {details}")
}

fn filePartContentDataToString(
    env: &str,
    content: &str,
    partIndex: usize,
    totalParts: usize,
    startLine: usize,
    endLine: usize,
    totalLines: usize,
) -> String {
    format!(
        "[{env}] Part {} of {totalParts} (Lines {}-{endLine} of {totalLines})\n\n{content}",
        partIndex + 1,
        startLine + 1,
    )
}

fn findFilesResultDataToString(env: &str, path: &str, pattern: &str, files: &[String]) -> String {
    let mut output = String::new();
    output.push_str(&format!("[{env}] File Search Result:\n"));
    output.push_str(&format!("Search Path: {path}\n"));
    output.push_str(&format!("Pattern: {pattern}\n"));
    output.push_str(&format!("Found {} files:\n", files.len()));
    for (index, file) in files.iter().enumerate() {
        if index < 10 || files.len() <= 20 {
            output.push_str(&format!("- {file}\n"));
        } else if index == 10 && files.len() > 20 {
            output.push_str(&format!("... and {} other files\n", files.len() - 10));
        }
    }
    output
}

fn grepResultDataToString(
    env: &str,
    searchPath: &str,
    pattern: &str,
    result: &GrepCodeResult,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("[{env}] Grep Search Result:\n"));
    output.push_str(&format!("Search Path: {searchPath}\n"));
    output.push_str(&format!("Pattern: {pattern}\n"));
    output.push_str(&format!(
        "Total Matches: {} (in {} files)\n",
        result.totalMatches,
        result.matches.len()
    ));
    output.push_str(&format!("Files Searched: {}\n\n", result.filesSearched));

    if result.matches.is_empty() {
        output.push_str("No matches found\n");
        return output;
    }

    let maxDisplayMatches = 30usize;
    let mut displayedMatches = 0usize;
    let mut collapsedMatches = 0usize;
    for fileMatch in &result.matches {
        let remainingSlots = maxDisplayMatches.saturating_sub(displayedMatches);
        if remainingSlots == 0 {
            collapsedMatches += fileMatch.lineMatches.len();
            continue;
        }

        output.push_str(&format!("File: {}\n", fileMatch.filePath));
        let matchesToShow = fileMatch
            .lineMatches
            .iter()
            .take(remainingSlots)
            .collect::<Vec<_>>();
        let matchesCollapsedInFile = fileMatch
            .lineMatches
            .len()
            .saturating_sub(matchesToShow.len());

        for lineMatch in matchesToShow {
            match &lineMatch.matchContext {
                Some(context) if !context.trim().is_empty() => {
                    let contextLines = context.lines().collect::<Vec<_>>();
                    let centerIndex = contextLines.len() / 2;
                    for (index, contextLine) in contextLines.iter().enumerate() {
                        let actualLineNumber = lineMatch
                            .lineNumber
                            .saturating_sub(centerIndex)
                            .saturating_add(index);
                        if index == centerIndex {
                            output.push_str(&format!("{actualLineNumber:>6}|>{contextLine}\n"));
                        } else {
                            output.push_str(&format!("{actualLineNumber:>6}| {contextLine}\n"));
                        }
                    }
                    output.push('\n');
                }
                _ => {
                    output.push_str(&format!(
                        "{:>6}| {}\n",
                        lineMatch.lineNumber, lineMatch.lineContent
                    ));
                }
            }
            displayedMatches += 1;
        }

        if matchesCollapsedInFile > 0 {
            output.push_str(&format!(
                "  ... ({matchesCollapsedInFile} more match groups collapsed in this file)\n"
            ));
            collapsedMatches += matchesCollapsedInFile;
        }
        output.push('\n');
    }

    if collapsedMatches > 0 {
        output.push_str(&format!(
            "{}\nTo save space, {collapsedMatches} match groups were collapsed\nDisplayed {displayedMatches} match groups, total {} matches\n",
            "=".repeat(60),
            result.totalMatches
        ));
    }

    output
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
