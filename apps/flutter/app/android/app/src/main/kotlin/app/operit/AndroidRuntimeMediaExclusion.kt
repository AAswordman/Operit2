package app.operit

import java.io.File
import java.io.IOException

internal object AndroidRuntimeMediaExclusion {
    private val identityId = Regex("^identity-[a-z0-9-]+$")

    fun ensureMarker(root: File, sharedStorageRoots: List<File>) {
        val identities = root.parentFile ?: return
        val base = identities.parentFile ?: return
        if (identities.name != "identities" || !identityId.matches(root.name)) return

        val directory = root.canonicalFile
        // Canonicalize storage aliases, but don't let a linked identity directory hide another tree.
        if (directory != File(base.canonicalFile, "identities/${root.name}")) return
        if (sharedStorageRoots.none { volume ->
                directory.path.startsWith(volume.canonicalPath + File.separator)
            }
        ) return

        val marker = File(directory, ".nomedia")
        if (!marker.createNewFile() && !marker.isFile) {
            throw IOException("Not a file: ${marker.path}")
        }
    }
}
