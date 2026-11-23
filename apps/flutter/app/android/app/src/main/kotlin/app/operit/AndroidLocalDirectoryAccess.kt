package app.operit

import android.content.Context
import android.os.Build
import android.os.Environment
import android.os.storage.StorageManager
import java.io.File
import java.io.IOException

/** Browses real shared-storage directories without resolving SAF document URIs. */
internal class AndroidLocalDirectoryAccess(
    private val context: Context,
    private val isGranted: () -> Boolean,
) {
    /** Lists mounted shared-storage volumes using the Android storage API. */
    @Suppress("DEPRECATION")
    fun roots(): List<Map<String, String>> {
        requirePermission()
        val roots = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val storage = requireNotNull(context.getSystemService(StorageManager::class.java)) {
                "Android storage service is unavailable"
            }
            storage.storageVolumes.mapNotNull { volume ->
                volume.directory?.takeIf { it.isDirectory }?.let {
                    location(it.canonicalFile, volume.getDescription(context))
                }
            }
        } else {
            listOf(location(Environment.getExternalStorageDirectory().canonicalFile,
                context.getString(R.string.operit_shared_storage)))
        }
        check(roots.isNotEmpty()) { "No shared storage volume is available" }
        return roots
    }

    /** Lists folders and breadcrumbs within one authorized shared-storage volume. */
    fun list(path: String): Map<String, Any> {
        val (directory, root) = resolve(path)
        val children = directory.listFiles() ?: throw IOException("Cannot read directory: $path")
        val breadcrumbs = generateSequence(directory) { current ->
            if (current.path == root.getValue("path")) null else current.parentFile
        }.toList().asReversed().mapIndexed { index, file ->
            location(file, if (index == 0) root.getValue("name") else file.name)
        }
        return mapOf(
            "path" to directory.path,
            "name" to breadcrumbs.last().getValue("name"),
            "writable" to directory.canWrite(),
            "breadcrumbs" to breadcrumbs,
            "directories" to children.filter { it.isDirectory && !it.isHidden }
                .sortedWith(compareBy(String.CASE_INSENSITIVE_ORDER) { it.name })
                .map { location(it, it.name) },
        )
    }

    /** Creates one direct child, rejecting traversal, existing files and inaccessible locations. */
    fun create(path: String, name: String): Map<String, Any> {
        require(name.isNotBlank() && name == name.trim() && name !in setOf(".", "..") &&
            name.none { it == '/' || it == '\\' || it == '\u0000' }) { "Invalid folder name" }
        val (parent, _) = resolve(path)
        val directory = File(parent, name)
        check(directory.mkdir()) { "Cannot create folder: $name" }
        return list(directory.path)
    }

    /** Validates native paths against mounted volumes after checking the actual permission. */
    private fun resolve(path: String): Pair<File, Map<String, String>> {
        require(File(path).isAbsolute) { "Directory path must be absolute" }
        val directory = File(path).canonicalFile
        val root = roots().firstOrNull {
            directory.path == it.getValue("path") || directory.path.startsWith("${it.getValue("path")}/")
        } ?: throw SecurityException("Directory is outside shared storage")
        check(directory.isDirectory) { "Directory no longer exists: $path" }
        return directory to root
    }

    /** Rejects revoked grants on every operation instead of relying on an earlier snapshot. */
    private fun requirePermission() {
        if (!isGranted()) throw SecurityException("Shared storage permission is required")
    }

    /** Encodes a directory without converting it into a document-provider URI. */
    private fun location(directory: File, name: String): Map<String, String> =
        mapOf("path" to directory.path, "name" to name)
}
