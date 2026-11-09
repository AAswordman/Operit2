package app.operit

import java.io.File

/** Defines the directories that may be mounted before shared-storage authorization. */
internal object StorageRootAccess {
    /** Requires permission for every path outside the application's canonical private roots. */
    fun requiresPermission(path: String, ownedRoots: List<File>): Boolean {
        val candidate = File(path).canonicalFile
        return ownedRoots.none { root ->
            val owned = root.canonicalFile
            candidate == owned || candidate.path.startsWith(owned.path + File.separator)
        }
    }
}
