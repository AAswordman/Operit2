/* METADATA
{
    "name": "extended_file_tools",
    "display_name": {"zh":"增强文件工具","en":"Extended File Tools"},
    "description": {
        "zh": "基于 v2 VFS 的增强文件工具：检查存在、压缩、解压、打开和分享。移动、复制和文件信息由内置文件工具提供。",
        "en": "Extended file tools using v2 VFS: existence checks, zip, unzip, open, and share. Move, copy, and file info are provided by the built-in file tools."
    },
    "category": "File",
    "enabledByDefault": true,
    "tools": [
        {
            "name": "file_exists",
            "description": {"zh":"检查文件或目录是否存在。","en":"Check if a file or directory exists."},
            "parameters": [
                {"name":"path","description":{"zh":"目标路径（VFS 路径）","en":"Target path (VFS path)"},"type":"string","required":true}
            ]
        },
        {
            "name": "zip_files",
            "description": {"zh":"压缩文件/目录。","en":"Zip files/directories."},
            "parameters": [
                {"name":"source","description":{"zh":"源路径（VFS 路径）","en":"Source path (VFS path)"},"type":"string","required":true},
                {"name":"destination","description":{"zh":"输出 zip 文件路径（VFS 路径）","en":"Destination zip path (VFS path)"},"type":"string","required":true},
                {"name":"include_root_directory","description":{"zh":"压缩目录时是否保留源目录名作为顶层目录，默认 true","en":"When zipping a directory, keep the source directory name as the top-level folder, default true"},"type":"boolean","required":false}
            ]
        },
        {
            "name": "unzip_files",
            "description": {"zh":"解压 zip 文件。","en":"Unzip an archive."},
            "parameters": [
                {"name":"source","description":{"zh":"zip 文件路径（VFS 路径）","en":"Zip file path (VFS path)"},"type":"string","required":true},
                {"name":"destination","description":{"zh":"解压目录（VFS 路径）","en":"Destination directory (VFS path)"},"type":"string","required":true}
            ]
        },
        {
            "name": "open_file",
            "description": {"zh":"用系统默认应用打开文件。","en":"Open a file with system default app."},
            "parameters": [
                {"name":"path","description":{"zh":"文件路径（VFS 路径）","en":"File path (VFS path)"},"type":"string","required":true}
            ]
        },
        {
            "name": "share_file",
            "description": {"zh":"分享文件给其他应用。","en":"Share a file with other apps."},
            "parameters": [
                {"name":"path","description":{"zh":"文件路径（VFS 路径）","en":"File path (VFS path)"},"type":"string","required":true},
                {"name":"title","description":{"zh":"可选：分享标题","en":"Optional: share title"},"type":"string","required":false}
            ]
        }
    ]
}*/

/** Implements extended file tools through the shared v2 VFS host API. */
const ExtendedFileTools = (function () {
    interface ToolResponse<T> {
        success: boolean;
        message: string;
        data: T;
    }

    interface PathParams {
        path: string;
    }

    interface TransferParams {
        source: string;
        destination: string;
    }

    interface ZipParams extends TransferParams {
        include_root_directory?: boolean;
    }

    interface ShareParams extends PathParams {
        title?: string;
    }

    /** Checks whether a VFS path exists without treating absence as a tool failure. */
    async function file_exists(params: PathParams): Promise<ToolResponse<FileExistsData>> {
        const result = await Tools.Files.exists(params.path);
        return { success: true, message: '检查完成', data: result };
    }

    /** Creates a ZIP archive with the requested root directory layout. */
    async function zip_files(params: ZipParams): Promise<ToolResponse<FileOperationData>> {
        const result = await Tools.Files.zip(params.source, params.destination, params.include_root_directory);
        return { success: result.successful, message: result.details, data: result };
    }

    /** Extracts a ZIP archive to a VFS directory. */
    async function unzip_files(params: TransferParams): Promise<ToolResponse<FileOperationData>> {
        const result = await Tools.Files.unzip(params.source, params.destination);
        return { success: result.successful, message: result.details, data: result };
    }

    /** Opens a VFS file using the host's system handler. */
    async function open_file(params: PathParams): Promise<ToolResponse<FileOperationData>> {
        const result = await Tools.Files.open(params.path);
        return { success: result.successful, message: result.details, data: result };
    }

    /** Shares a VFS file through the host with an optional title. */
    async function share_file(params: ShareParams): Promise<ToolResponse<FileOperationData>> {
        const result = await Tools.Files.share(params.path, params.title);
        return { success: result.successful, message: result.details, data: result };
    }

    /** Completes a tool invocation and reports host errors to the caller. */
    async function completeTool<P, T>(func: (params: P) => Promise<ToolResponse<T>>, params: P): Promise<void> {
        try {
            complete(await func(params));
        } catch (error: unknown) {
            complete({ success: false, message: String(error) });
        }
    }

    /** Reports package availability without performing file operations. */
    async function main(): Promise<void> {
        complete({ success: true, message: '增强文件工具包已加载' });
    }

    return {
        /** Completes a VFS existence check. */
        file_exists: (params: PathParams) => completeTool(file_exists, params),
        /** Completes a VFS archive operation. */
        zip_files: (params: ZipParams) => completeTool(zip_files, params),
        /** Completes a VFS extraction operation. */
        unzip_files: (params: TransferParams) => completeTool(unzip_files, params),
        /** Completes a host file open operation. */
        open_file: (params: PathParams) => completeTool(open_file, params),
        /** Completes a host file share operation. */
        share_file: (params: ShareParams) => completeTool(share_file, params),
        main,
    };
})();

exports.file_exists = ExtendedFileTools.file_exists;
exports.zip_files = ExtendedFileTools.zip_files;
exports.unzip_files = ExtendedFileTools.unzip_files;
exports.open_file = ExtendedFileTools.open_file;
exports.share_file = ExtendedFileTools.share_file;
exports.main = ExtendedFileTools.main;
