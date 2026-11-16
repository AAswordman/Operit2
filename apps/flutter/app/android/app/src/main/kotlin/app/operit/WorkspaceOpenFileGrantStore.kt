package app.operit

import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.security.SecureRandom
import java.util.concurrent.ConcurrentHashMap
import org.json.JSONObject

/** Identity captured from a readable regular file, without reading its content. */
internal data class WorkspaceOpenFileIdentity(
    val device: Long,
    val inode: Long,
    val size: Long,
    val modifiedAtSeconds: Long,
    val changedAtSeconds: Long,
)

internal data class WorkspaceOpenFileGrant(
    val token: String,
    val canonicalPath: String,
    val displayName: String,
    val mimeType: String,
    val identity: WorkspaceOpenFileIdentity,
    val issuedAt: Long,
    val expiresAt: Long,
)

/** A fixed, path-free error that can be shown by the requesting workspace action. */
internal class WorkspaceOpenFileGrantLimitException : IllegalStateException(
    "Too many active workspace file permissions; retry after they expire",
)

/**
 * Persists bounded single-file capabilities in the app's private no-backup directory.
 * Only metadata is removed on expiry/revocation; the selected file is never deleted.
 * Separate instances share a process lock, including the opener and a restarted provider.
 */
internal class WorkspaceOpenFileGrantStore(
    directory: File,
    private val now: () -> Long = System::currentTimeMillis,
    private val random: SecureRandom = SecureRandom(),
) {
    companion object {
        const val TTL_MILLIS = 24L * 60 * 60 * 1000
        const val MAX_GRANTS = 128
        private const val MAX_METADATA_BYTES = 16 * 1024
        private val tokenPattern = Regex("[0-9a-f]{32}")
        private val locks = ConcurrentHashMap<String, Any>()

        fun isValidToken(token: String): Boolean = tokenPattern.matches(token)
    }

    private val directory = directory.canonicalFile
    private val lock = locks.getOrPut(this.directory.path) { Any() }

    /** Registers only an already validated canonical file and its opened-FD identity. */
    fun register(
        file: File,
        displayName: String,
        mimeType: String,
        identity: WorkspaceOpenFileIdentity,
    ): WorkspaceOpenFileGrant = synchronized(lock) {
        require(file.isAbsolute && file.canonicalPath == file.path && file.isFile && file.canRead()) {
            "Workspace file is not a readable regular file"
        }
        require(!file.path.startsWith(directory.path + File.separator)) {
            "Workspace permission metadata cannot be granted"
        }
        require(displayName.isNotEmpty() && !displayName.contains('\u0000')) { "Invalid file display name" }
        require(mimeType.isNotBlank() && identity.size >= 0) { "Invalid file metadata" }
        val issuedAt = now()
        require(issuedAt >= 0 && issuedAt <= Long.MAX_VALUE - TTL_MILLIS) { "Invalid grant clock" }
        check(directory.isDirectory || directory.mkdirs()) { "Cannot create workspace file permissions" }
        val active = directory.listFiles()
            ?: throw IOException("Cannot read workspace file permissions")
        var count = 0
        for (metadata in active) {
            // Interrupted atomic writes cannot accumulate unbounded private staging metadata.
            if (metadata.name.startsWith("grant-") && metadata.name.endsWith(".tmp") && !metadata.isDirectory) {
                if (!metadata.delete() && metadata.exists()) throw IOException("Cannot clear permission metadata")
                continue
            }
            val token = metadata.name.removeSuffix(".json")
            if (metadata.name == "$token.json" && isValidToken(token)) {
                if (readLocked(token, issuedAt) != null) count++
            }
        }
        if (count >= MAX_GRANTS) throw WorkspaceOpenFileGrantLimitException()
        val token = (0 until 16).asSequence().map {
            val bytes = ByteArray(16)
            random.nextBytes(bytes)
            bytes.joinToString("") { (it.toInt() and 0xff).toString(16).padStart(2, '0') }
        }.firstOrNull { !metadataFile(it).exists() }
            ?: throw IOException("Cannot allocate workspace file permission")
        val grant = WorkspaceOpenFileGrant(
            token, file.path, displayName, mimeType, identity, issuedAt, issuedAt + TTL_MILLIS,
        )
        writeLocked(grant)
        grant
    }

    /** Reads one exact token; clock rollback also expires it instead of extending its life. */
    fun find(token: String): WorkspaceOpenFileGrant? = synchronized(lock) {
        if (!isValidToken(token) || !directory.isDirectory) null else readLocked(token, now())
    }

    /** Revokes future opens; an FD already delivered to another process remains readable. */
    fun revoke(token: String) = synchronized(lock) {
        require(isValidToken(token)) { "Invalid workspace file permission" }
        val metadata = metadataFile(token)
        if (metadata.exists() && !metadata.delete()) throw IOException("Cannot revoke workspace file permission")
        Unit
    }

    private fun metadataFile(token: String): File = File(directory, "$token.json")

    private fun readLocked(token: String, clock: Long): WorkspaceOpenFileGrant? {
        val metadata = metadataFile(token)
        if (!metadata.exists()) return null
        val grant = try {
            // Never follow a metadata symlink, or read an unbounded/corrupt metadata file.
            if (!metadata.isFile || metadata.canonicalPath != metadata.path ||
                metadata.length() !in 2L..MAX_METADATA_BYTES.toLong()) {
                null
            } else {
                val json = JSONObject(metadata.readText(Charsets.UTF_8))
                val identity = WorkspaceOpenFileIdentity(
                    json.getLong("device"), json.getLong("inode"), json.getLong("size"),
                    json.getLong("modifiedAtSeconds"), json.getLong("changedAtSeconds"),
                )
                WorkspaceOpenFileGrant(
                    json.getString("token"), json.getString("canonicalPath"),
                    json.getString("displayName"), json.getString("mimeType"), identity,
                    json.getLong("issuedAt"), json.getLong("expiresAt"),
                ).takeIf {
                    it.token == token && it.issuedAt >= 0 && it.expiresAt > it.issuedAt &&
                        it.expiresAt - it.issuedAt == TTL_MILLIS && clock >= it.issuedAt && clock < it.expiresAt &&
                        it.identity.size >= 0 && it.displayName.isNotEmpty() && it.mimeType.isNotBlank() &&
                        File(it.canonicalPath).isAbsolute && File(it.canonicalPath).canonicalPath == it.canonicalPath
                }
            }
        } catch (_: Exception) {
            null
        }
        if (grant == null && !metadata.delete() && metadata.exists()) {
            throw IOException("Cannot remove expired workspace file permission")
        }
        return grant
    }

    private fun writeLocked(grant: WorkspaceOpenFileGrant) {
        val json = JSONObject()
            .put("token", grant.token).put("canonicalPath", grant.canonicalPath)
            .put("displayName", grant.displayName).put("mimeType", grant.mimeType)
            .put("device", grant.identity.device).put("inode", grant.identity.inode)
            .put("size", grant.identity.size).put("modifiedAtSeconds", grant.identity.modifiedAtSeconds)
            .put("changedAtSeconds", grant.identity.changedAtSeconds)
            .put("issuedAt", grant.issuedAt).put("expiresAt", grant.expiresAt)
        val bytes = json.toString().toByteArray(Charsets.UTF_8)
        require(bytes.size <= MAX_METADATA_BYTES) { "Workspace file metadata is too large" }
        val temporary = File.createTempFile("grant-", ".tmp", directory)
        try {
            FileOutputStream(temporary).use { output ->
                output.write(bytes)
                output.flush()
                output.fd.sync()
            }
            if (!temporary.renameTo(metadataFile(grant.token))) {
                throw IOException("Cannot save workspace file permission")
            }
        } finally {
            temporary.delete()
        }
    }
}
