// Generated from operit-plugin-sdk Rust declarations.

import type { BinaryFileContentData, DirectoryListingData, FileApplyResultData, FileContentData, FileExistsData, FileInfoData, FileOperationData, FilePartContentData, FindFilesResultData, GrepResultData } from "./results";

/**
 * Reads, searches, mutates, archives, and downloads virtual file-system content.
 */
export namespace Files {
  /**
   * Configures regular-expression searches across virtual file-system content.
   */
  export interface HostGrepOptions {
    /**
     * Restricts the search to file names matching this glob pattern.
     */
    file_pattern?: string;
    /**
     * Enables case-insensitive pattern matching.
     */
    case_insensitive?: boolean;
    /**
     * Includes this many surrounding lines before and after each match.
     */
    context_lines?: number;
    /**
     * Limits the total number of matches returned.
     */
    max_results?: number;
  }

  /**
   * Configures intent-based content searches in the virtual file system.
   */
  export interface HostGrepContextOptions {
    /**
     * Restricts directory searches to file names matching this glob pattern.
     */
    file_pattern?: string;
    /**
     * Limits the number of relevant content regions returned.
     */
    max_results?: number;
  }

  /**
   * Identifies a remote resource and its destination for a file download.
   */
  export interface HostDownloadOptions {
    /**
     * Provides a direct URL to download.
     */
    url?: string;
    /**
     * References the stored context produced by an earlier web visit.
     */
    visit_key?: string;
    /**
     * Selects a numbered link from the referenced web visit.
     */
    link_number?: number;
    /**
     * Selects a numbered image from the referenced web visit.
     */
    image_number?: number;
    /**
     * Sets the virtual file-system path where the resource is written.
     */
    destination: string;
    /**
     * Adds HTTP request headers used to retrieve the resource.
     */
    headers?: Record<string, string>;
  }

  /**
   * Apply AI-generated content to a file with intelligent merging
   * @param path - VFS file path
   * @param type - Operation type: replace | delete | create
   * @param old - Exact content to match (required for replace/delete)
   * @param newContent - New content to insert (required for replace/create)
   */
  function apply(path: string, type: ApplyFileType, old?: string, newContent?: string): Promise<FileApplyResultData>;
  /**
   * Copy file from source to destination
   * @param source - Source VFS path
   * @param destination - Destination VFS path
   * @param recursive - Copy recursively
   */
  function copy(source: string, destination: string, recursive?: boolean): Promise<FileOperationData>;
  /**
   * Create a new file. Internally delegates to apply_file with type=create.
   * @param path - VFS file path
   * @param newContent - Full file content
   */
  function create(path: string, newContent: string): Promise<FileApplyResultData>;
  /**
   * Delete a file or directory
   * @param path - VFS file or directory path
   * @param recursive - Delete recursively
   */
  function deleteFile(path: string, recursive?: boolean): Promise<FileOperationData>;
  /**
   * Download a file from URL
   * @param url - Source URL
   * @param destination - Destination VFS path
   * @param headers - Optional headers for the request
   */
  function download(url: string, destination: string, headers?: Record<string, string>): Promise<FileOperationData>;
  /**
   * Downloads a direct URL or a resource selected from stored web-visit context.
   */
  function download(options: HostDownloadOptions): Promise<FileOperationData>;
  /**
   * Edit an existing file. Internally delegates to apply_file with type=replace.
   * @param path - VFS file path
   * @param oldContent - Exact content to match
   * @param newContent - New content to insert
   */
  function edit(path: string, oldContent: string, newContent: string): Promise<FileApplyResultData>;
  /**
   * Check if file exists
   * @param path - VFS path to check
   */
  function exists(path: string): Promise<FileExistsData>;
  /**
   * Find files matching a pattern
   * @param path - VFS base directory
   * @param pattern - Search pattern
   * @param options - Search options
   */
  function find(path: string, pattern: string, options?: Record<string, unknown>): Promise<FindFilesResultData>;
  /**
   * Search code content matching a regex pattern in files
   * @param path - VFS base directory to search
   * @param pattern - Regex pattern to search for
   * @param options - Search options
   * @param options.file_pattern - File filter pattern (e.g., "*.kt"), default "*"
   * @param options.case_insensitive - Ignore case in pattern matching, default false
   * @param options.context_lines - Number of context lines before/after each match, default 3
   * @param options.max_results - Maximum number of matches to return, default 100
   */
  function grep(path: string, pattern: string, options?: HostGrepOptions): Promise<GrepResultData>;
  /**
   * Search for relevant content based on intent/context understanding
   * @param path - VFS directory or file path
   * @param intent - Intent or context description string
   * @param options - Search options
   * @param options.file_pattern - File filter pattern for directory mode (e.g., "*.kt"), default "*"
   * @param options.max_results - Maximum number of items to return, default 10
   */
  function grepContext(path: string, intent: string, options?: HostGrepContextOptions): Promise<GrepResultData>;
  /**
   * Get information about a file
   * @param path - VFS file path
   */
  function info(path: string): Promise<FileInfoData>;
  /**
   * List files in a directory
   * @param path - VFS directory path
   */
  function list(path: string): Promise<DirectoryListingData>;
  /**
   * Create a directory
   * @param path - VFS directory path
   * @param create_parents - Create parent directories
   */
  function mkdir(path: string, create_parents?: boolean): Promise<FileOperationData>;
  /**
   * Open a file with system handler
   * @param path - VFS file path
   */
  function open(path: string): Promise<FileOperationData>;
  /**
   * Move file from source to destination
   * @param source - Source VFS path
   * @param destination - Destination VFS path
   */
  function move(source: string, destination: string): Promise<FileOperationData>;
  /**
   * Read binary file content as a structured result with Base64 data
   * @param path - VFS file path
   */
  function readBinary(path: string): Promise<BinaryFileContentData>;
  /**
   * Read file content by line range
   * @param path - VFS file path
   * @param startLine - Starting line number (1-indexed, default 1)
   * @param endLine - Ending line number (1-indexed, inclusive, optional)
   */
  function readPart(path: string, startLine?: number, endLine?: number): Promise<FilePartContentData>;
  /**
   * Read file contents
   * @param path - VFS file path
   */
  function read(path: string): Promise<FileContentData>;
  /**
   * Reads a file using options that can include semantic intent and direct image handling.
   */
  function read(options: ReadFileOptions): Promise<FileContentData>;
  /**
   * Share a file with other apps
   * @param path - VFS file path
   * @param title - Share title
   */
  function share(path: string, title?: string): Promise<FileOperationData>;
  /**
   * Unzip an archive
   * @param source - Source archive VFS path
   * @param destination - Target directory VFS path
   */
  function unzip(source: string, destination: string): Promise<FileOperationData>;
  /**
   * Write content to file
   * @param path - VFS file path
   * @param content - Content to write
   * @param append - Whether to append to file
   */
  function write(path: string, content: string, append?: boolean): Promise<FileOperationData>;
  /**
   * Write base64 encoded content to a binary file
   * @param path - VFS file path
   * @param base64Content - Base64 encoded content to write
   */
  function writeBinary(path: string, base64Content: string): Promise<FileOperationData>;
  /**
   * Zip files/directories
   * @param source - Source VFS path
   * @param destination - Destination VFS path
   * @param include_root_directory - When zipping a directory, whether to keep the source directory itself as the top-level folder, default true
   */
  function zip(source: string, destination: string, include_root_directory?: boolean): Promise<FileOperationData>;
  /**
   * Configures content extraction when reading a virtual file.
   */
  export interface ReadFileOptions {
    /**
     * Identifies the virtual file-system path to read.
     */
    path: string;
    /**
     * Describes the information to prioritize when extracting large-file content.
     */
    intent?: string;
    /**
     * Requests direct image registration instead of textual file decoding.
     */
    direct_image?: boolean;
  }

}

/**
 * Selects the mutation performed by an intelligent file-apply operation.
 */
export type ApplyFileType = "replace" | "delete" | "create";
