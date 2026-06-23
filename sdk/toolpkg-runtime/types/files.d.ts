/**
 * File operation type definitions for Assistance Package Tools
 */

import {
    DirectoryListingData,
    FileContentData,
    BinaryFileContentData,
    FileOperationData,
    FileExistsData,
    FindFilesResultData,
    FileInfoData,
    FilePartContentData,
    FileApplyResultData,
    GrepResultData
} from './results';

export type ApplyFileType = "replace" | "delete" | "create";

/**
 * File operations namespace
 */
export namespace Files {
    /**
     * List files in a directory
     * @param path - VFS directory path
     */
    function list(path: string): Promise<DirectoryListingData>;

    /**
     * Read file contents
     * @param path - VFS file path
     */
    function read(path: string): Promise<FileContentData>;

    interface ReadFileOptions {
        path: string;
        intent?: string;
        direct_image?: boolean;
    }

    function read(options: ReadFileOptions): Promise<FileContentData>;

    /**
     * Read file content by line range
     * @param path - VFS file path
     * @param startLine - Starting line number (1-indexed, default 1)
     * @param endLine - Ending line number (1-indexed, inclusive, optional)
     */
    function readPart(path: string, startLine?: number, endLine?: number): Promise<FilePartContentData>;

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
     * Read binary file content as a structured result with Base64 data
     * @param path - VFS file path
     */
    function readBinary(path: string): Promise<BinaryFileContentData>;

    /**
     * Delete a file or directory
     * @param path - VFS file or directory path
     * @param recursive - Delete recursively
     */
    function deleteFile(path: string, recursive?: boolean): Promise<FileOperationData>;

    /**
     * Check if file exists
     * @param path - VFS path to check
     */
    function exists(path: string): Promise<FileExistsData>;

    /**
     * Move file from source to destination
     * @param source - Source VFS path
     * @param destination - Destination VFS path
     */
    function move(source: string, destination: string): Promise<FileOperationData>;

    /**
     * Copy file from source to destination
     * @param source - Source VFS path
     * @param destination - Destination VFS path
     * @param recursive - Copy recursively
     */
    function copy(source: string, destination: string, recursive?: boolean): Promise<FileOperationData>;

    /**
     * Create a directory
     * @param path - VFS directory path
     * @param create_parents - Create parent directories
     */
    function mkdir(path: string, create_parents?: boolean): Promise<FileOperationData>;

    /**
     * Find files matching a pattern
     * @param path - VFS base directory
     * @param pattern - Search pattern
     * @param options - Search options
     */
    function find(path: string, pattern: string, options?: Record<string, any>): Promise<FindFilesResultData>;

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
    function grep(path: string, pattern: string, options?: {
        file_pattern?: string;
        case_insensitive?: boolean;
        context_lines?: number;
        max_results?: number;
    }): Promise<GrepResultData>;

    /**
     * Search for relevant content based on intent/context understanding
     * @param path - VFS directory or file path
     * @param intent - Intent or context description string
     * @param options - Search options
     * @param options.file_pattern - File filter pattern for directory mode (e.g., "*.kt"), default "*"
     * @param options.max_results - Maximum number of items to return, default 10
     */
    function grepContext(path: string, intent: string, options?: {
        file_pattern?: string;
        max_results?: number;
    }): Promise<GrepResultData>;

    /**
     * Get information about a file
     * @param path - VFS file path
     */
    function info(path: string): Promise<FileInfoData>;

    /**
     * Apply AI-generated content to a file with intelligent merging
     * @param path - VFS file path
     * @param type - Operation type: replace | delete | create
     * @param old - Exact content to match (required for replace/delete)
     * @param newContent - New content to insert (required for replace/create)
     */
    function apply(path: string, type: ApplyFileType, old?: string, newContent?: string): Promise<FileApplyResultData>;

    /**
     * Create a new file. Internally delegates to apply_file with type=create.
     * @param path - VFS file path
     * @param newContent - Full file content
     */
    function create(path: string, newContent: string): Promise<FileApplyResultData>;

    /**
     * Edit an existing file. Internally delegates to apply_file with type=replace.
     * @param path - VFS file path
     * @param oldContent - Exact content to match
     * @param newContent - New content to insert
     */
    function edit(path: string, oldContent: string, newContent: string): Promise<FileApplyResultData>;

    /**
     * Zip files/directories
     * @param source - Source VFS path
     * @param destination - Destination VFS path
     * @param include_root_directory - When zipping a directory, whether to keep the source directory itself as the top-level folder, default true
     */
    function zip(source: string, destination: string, include_root_directory?: boolean): Promise<FileOperationData>;

    /**
     * Unzip an archive
     * @param source - Source archive VFS path
     * @param destination - Target directory VFS path
     */
    function unzip(source: string, destination: string): Promise<FileOperationData>;

    /**
     * Open a file with system handler
     * @param path - VFS file path
     */
    function open(path: string): Promise<FileOperationData>;

    /**
     * Share a file with other apps
     * @param path - VFS file path
     * @param title - Share title
     */
    function share(path: string, title?: string): Promise<FileOperationData>;

    /**
     * Download a file from URL
     * @param url - Source URL
     * @param destination - Destination VFS path
     * @param headers - Optional headers for the request
     */
    function download(url: string, destination: string, headers?: Record<string, string>): Promise<FileOperationData>;

    function download(options: {
        url?: string;
        visit_key?: string;
        link_number?: number;
        image_number?: number;
        destination: string;
        headers?: Record<string, string>;
    }): Promise<FileOperationData>;
}
