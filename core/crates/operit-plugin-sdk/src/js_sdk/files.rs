//! Virtual file-system operations and their plugin-facing request types.
use super::results::*;
use super::{JsDate, JsFuture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures regular-expression searches across virtual file-system content.
pub struct FilesHostGrepOptions {
    /// Restricts the search to file names matching this glob pattern.
    pub file_pattern: Option<String>,
    /// Enables case-insensitive pattern matching.
    pub case_insensitive: Option<bool>,
    /// Includes this many surrounding lines before and after each match.
    pub context_lines: Option<f64>,
    /// Limits the total number of matches returned.
    pub max_results: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures intent-based content searches in the virtual file system.
pub struct FilesHostGrepContextOptions {
    /// Restricts directory searches to file names matching this glob pattern.
    pub file_pattern: Option<String>,
    /// Limits the number of relevant content regions returned.
    pub max_results: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies a remote resource and its destination for a file download.
pub struct FilesHostDownloadOptions {
    /// Provides a direct URL to download.
    pub url: Option<String>,
    /// References the stored context produced by an earlier web visit.
    pub visit_key: Option<String>,
    /// Selects a numbered link from the referenced web visit.
    pub link_number: Option<f64>,
    /// Selects a numbered image from the referenced web visit.
    pub image_number: Option<f64>,
    /// Sets the virtual file-system path where the resource is written.
    pub destination: String,
    /// Adds HTTP request headers used to retrieve the resource.
    pub headers: Option<BTreeMap<String, String>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the mutation performed by an intelligent file-apply operation.
pub enum ApplyFileType {
    #[serde(rename = "replace")]
    Replace,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "create")]
    Create,
}
/// Reads, searches, mutates, archives, and downloads virtual file-system content.
pub trait FilesHost: Send + Sync {
    ///
    ///List files in a directory
    ///@param path - VFS directory path
    ///
    fn list(&self, path: String) -> JsFuture<DirectoryListingData>;
    ///
    ///Read file contents
    ///@param path - VFS file path
    ///
    fn read_overload_1(&self, path: String) -> JsFuture<FileContentData>;
    /// Reads a file using options that can include semantic intent and direct image handling.
    fn read_overload_2(&self, options: FilesReadFileOptions) -> JsFuture<FileContentData>;
    ///
    ///Read file content by line range
    ///@param path - VFS file path
    ///@param startLine - Starting line number (1-indexed, default 1)
    ///@param endLine - Ending line number (1-indexed, inclusive, optional)
    ///
    fn readPart(
        &self,
        path: String,
        startLine: Option<f64>,
        endLine: Option<f64>,
    ) -> JsFuture<FilePartContentData>;
    ///
    ///Write content to file
    ///@param path - VFS file path
    ///@param content - Content to write
    ///@param append - Whether to append to file
    ///
    fn write(
        &self,
        path: String,
        content: String,
        append: Option<bool>,
    ) -> JsFuture<FileOperationData>;
    ///
    ///Write base64 encoded content to a binary file
    ///@param path - VFS file path
    ///@param base64Content - Base64 encoded content to write
    ///
    fn writeBinary(&self, path: String, base64Content: String) -> JsFuture<FileOperationData>;
    ///
    ///Read binary file content as a structured result with Base64 data
    ///@param path - VFS file path
    ///
    fn readBinary(&self, path: String) -> JsFuture<BinaryFileContentData>;
    ///
    ///Delete a file or directory
    ///@param path - VFS file or directory path
    ///@param recursive - Delete recursively
    ///
    fn deleteFile(&self, path: String, recursive: Option<bool>) -> JsFuture<FileOperationData>;
    ///
    ///Check if file exists
    ///@param path - VFS path to check
    ///
    fn exists(&self, path: String) -> JsFuture<FileExistsData>;
    ///
    ///Move file from source to destination
    ///@param source - Source VFS path
    ///@param destination - Destination VFS path
    ///
    fn r#move(&self, source: String, destination: String) -> JsFuture<FileOperationData>;
    ///
    ///Copy file from source to destination
    ///@param source - Source VFS path
    ///@param destination - Destination VFS path
    ///@param recursive - Copy recursively
    ///
    fn copy(
        &self,
        source: String,
        destination: String,
        recursive: Option<bool>,
    ) -> JsFuture<FileOperationData>;
    ///
    ///Create a directory
    ///@param path - VFS directory path
    ///@param create_parents - Create parent directories
    ///
    fn mkdir(&self, path: String, create_parents: Option<bool>) -> JsFuture<FileOperationData>;
    ///
    ///Find files matching a pattern
    ///@param path - VFS base directory
    ///@param pattern - Search pattern
    ///@param options - Search options
    ///
    fn find(
        &self,
        path: String,
        pattern: String,
        options: Option<BTreeMap<String, serde_json::Value>>,
    ) -> JsFuture<FindFilesResultData>;
    ///
    ///Search code content matching a regex pattern in files
    ///@param path - VFS base directory to search
    ///@param pattern - Regex pattern to search for
    ///@param options - Search options
    ///@param options.file_pattern - File filter pattern (e.g., "*.kt"), default "*"
    ///@param options.case_insensitive - Ignore case in pattern matching, default false
    ///@param options.context_lines - Number of context lines before/after each match, default 3
    ///@param options.max_results - Maximum number of matches to return, default 100
    ///
    fn grep(
        &self,
        path: String,
        pattern: String,
        options: Option<FilesHostGrepOptions>,
    ) -> JsFuture<GrepResultData>;
    ///
    ///Search for relevant content based on intent/context understanding
    ///@param path - VFS directory or file path
    ///@param intent - Intent or context description string
    ///@param options - Search options
    ///@param options.file_pattern - File filter pattern for directory mode (e.g., "*.kt"), default "*"
    ///@param options.max_results - Maximum number of items to return, default 10
    ///
    fn grepContext(
        &self,
        path: String,
        intent: String,
        options: Option<FilesHostGrepContextOptions>,
    ) -> JsFuture<GrepResultData>;
    ///
    ///Get information about a file
    ///@param path - VFS file path
    ///
    fn info(&self, path: String) -> JsFuture<FileInfoData>;
    ///
    ///Apply AI-generated content to a file with intelligent merging
    ///@param path - VFS file path
    ///@param type - Operation type: replace | delete | create
    ///@param old - Exact content to match (required for replace/delete)
    ///@param newContent - New content to insert (required for replace/create)
    ///
    fn apply(
        &self,
        path: String,
        r#type: ApplyFileType,
        old: Option<String>,
        newContent: Option<String>,
    ) -> JsFuture<FileApplyResultData>;
    ///
    ///Create a new file. Internally delegates to apply_file with type=create.
    ///@param path - VFS file path
    ///@param newContent - Full file content
    ///
    fn create(&self, path: String, newContent: String) -> JsFuture<FileApplyResultData>;
    ///
    ///Edit an existing file. Internally delegates to apply_file with type=replace.
    ///@param path - VFS file path
    ///@param oldContent - Exact content to match
    ///@param newContent - New content to insert
    ///
    fn edit(
        &self,
        path: String,
        oldContent: String,
        newContent: String,
    ) -> JsFuture<FileApplyResultData>;
    ///
    ///Zip files/directories
    ///@param source - Source VFS path
    ///@param destination - Destination VFS path
    ///@param include_root_directory - When zipping a directory, whether to keep the source directory itself as the top-level folder, default true
    ///
    fn zip(
        &self,
        source: String,
        destination: String,
        include_root_directory: Option<bool>,
    ) -> JsFuture<FileOperationData>;
    ///
    ///Unzip an archive
    ///@param source - Source archive VFS path
    ///@param destination - Target directory VFS path
    ///
    fn unzip(&self, source: String, destination: String) -> JsFuture<FileOperationData>;
    ///
    ///Open a file with system handler
    ///@param path - VFS file path
    ///
    fn open(&self, path: String) -> JsFuture<FileOperationData>;
    ///
    ///Share a file with other apps
    ///@param path - VFS file path
    ///@param title - Share title
    ///
    fn share(&self, path: String, title: Option<String>) -> JsFuture<FileOperationData>;
    ///
    ///Download a file from URL
    ///@param url - Source URL
    ///@param destination - Destination VFS path
    ///@param headers - Optional headers for the request
    ///
    fn download_overload_1(
        &self,
        url: String,
        destination: String,
        headers: Option<BTreeMap<String, String>>,
    ) -> JsFuture<FileOperationData>;
    /// Downloads a direct URL or a resource selected from stored web-visit context.
    fn download_overload_2(&self, options: FilesHostDownloadOptions)
        -> JsFuture<FileOperationData>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures content extraction when reading a virtual file.
pub struct FilesReadFileOptions {
    /// Identifies the virtual file-system path to read.
    #[serde(rename = "path")]
    pub path: String,
    /// Describes the information to prioritize when extracting large-file content.
    #[serde(rename = "intent")]
    pub intent: Option<String>,
    /// Requests direct image registration instead of textual file decoding.
    #[serde(rename = "direct_image")]
    pub direct_image: Option<bool>,
}
