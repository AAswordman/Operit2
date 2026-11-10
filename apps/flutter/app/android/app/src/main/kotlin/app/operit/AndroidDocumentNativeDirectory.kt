package app.operit

import java.io.File

/** Only the platform external-storage provider defines IDs that can be mapped to local volumes. */
internal object AndroidDocumentNativeDirectory {
    fun resolve(
        authority: String?,
        treeDocumentId: String,
        documentId: String,
        volumes: Map<String, File>,
    ): String {
        require(authority == "com.android.externalstorage.documents") {
            "Selected document provider does not expose a native terminal directory; use a local storage workspace"
        }
        fun localFile(id: String): Pair<File, File> {
            val parts = id.split(':', limit = 2)
            require(parts.size == 2) { "Invalid external-storage document ID" }
            val volume = volumes.entries.firstOrNull { it.key.equals(parts[0], ignoreCase = true) }?.value
                ?.canonicalFile ?: throw IllegalArgumentException("Selected storage volume is unavailable for terminal access")
            val relative = parts[1]
            require(!relative.startsWith('/') && !relative.contains('\\') && !relative.contains('\u0000') &&
                (relative.isEmpty() || relative.split('/').all { it.isNotEmpty() && it != "." && it != ".." })) {
                "Invalid external-storage document path"
            }
            val directory = if (relative.isEmpty()) volume else File(volume, relative).canonicalFile
            require(contains(volume, directory)) { "Document path escapes its storage volume" }
            return volume to directory
        }
        val (treeVolume, root) = localFile(treeDocumentId)
        val (documentVolume, directory) = localFile(documentId)
        require(treeVolume == documentVolume && contains(root, directory)) {
            "Terminal working directory is outside the selected document tree"
        }
        require(directory.isDirectory) { "Terminal working directory does not exist or is not a directory" }
        require(directory.canRead() && directory.canExecute()) {
            "Terminal requires direct filesystem access to this directory; enable file management permission or use an app workspace"
        }
        return directory.path
    }

    private fun contains(parent: File, child: File): Boolean =
        parent == child || child.path.startsWith(parent.path.trimEnd(File.separatorChar) + File.separator)
}
