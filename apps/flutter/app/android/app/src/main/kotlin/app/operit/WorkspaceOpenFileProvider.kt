package app.operit

import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.Binder
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.os.Process
import android.provider.OpenableColumns
import android.system.Os
import android.system.OsConstants
import android.system.StructStat
import java.io.File
import java.io.FileNotFoundException

/**
 * Opens only app-registered, individually granted workspace files without staging their bytes.
 * A live read-only FD is not a snapshot: the source app may modify its bytes, and URI revocation
 * or expiry cannot recall an FD already delivered. Identity checks protect subsequent opens.
 */
class WorkspaceOpenFileProvider : ContentProvider() {
    internal companion object {
        fun authority(context: Context): String = "${context.packageName}.workspaceopenfiles"

        private fun store(context: Context): WorkspaceOpenFileGrantStore =
            WorkspaceOpenFileGrantStore(File(context.noBackupFilesDir, "workspace_open_grants"))

        /** App-internal registration; no ContentProvider RPC can register an arbitrary path. */
        fun register(context: Context, file: File, mimeType: String): Uri {
            val grant = try {
                val canonical = file.canonicalFile
                if (!canonical.isFile || !canonical.canRead()) throw FileNotFoundException()
                // The registration records the identity of an opened file, not just a pathname stat.
                ParcelFileDescriptor.open(canonical, ParcelFileDescriptor.MODE_READ_ONLY).use { descriptor ->
                    val stat = Os.fstat(descriptor.fileDescriptor)
                    if (!OsConstants.S_ISREG(stat.st_mode) || canonical.canonicalPath != canonical.path) {
                        throw FileNotFoundException()
                    }
                    Os.lseek(descriptor.fileDescriptor, 0, OsConstants.SEEK_CUR)
                    store(context).register(canonical, file.name, mimeType, identity(stat))
                }
            } catch (error: WorkspaceOpenFileGrantLimitException) {
                // The bounded-store error is safe for local UI feedback and has no paths/tokens.
                throw error
            } catch (_: Exception) {
                throw FileNotFoundException("Workspace file could not be granted for reading")
            }
            return Uri.Builder().scheme("content").authority(authority(context)).appendPath(grant.token).build()
        }

        /** Removes only authorization metadata, without touching the original workspace file. */
        fun revoke(context: Context, uri: Uri) {
            store(context).revoke(token(context, uri))
        }

        private fun token(context: Context, uri: Uri): String {
            val value = uri.encodedPath?.removePrefix("/") ?: ""
            if (uri.scheme != "content" || uri.authority != authority(context) ||
                uri.encodedQuery != null || uri.encodedFragment != null ||
                uri.encodedPath != "/$value" || !WorkspaceOpenFileGrantStore.isValidToken(value)) {
                throw FileNotFoundException("Invalid workspace file URI")
            }
            return value
        }

        private fun identity(stat: StructStat): WorkspaceOpenFileIdentity = WorkspaceOpenFileIdentity(
            stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime, stat.st_ctime,
        )
    }

    override fun onCreate(): Boolean = true

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?,
    ): Cursor {
        val grant = requireGrant(uri)
        openReadOnly(grant).use { }
        val columns = (projection ?: arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE))
            .filter { it == OpenableColumns.DISPLAY_NAME || it == OpenableColumns.SIZE }.toTypedArray()
        return MatrixCursor(columns, 1).apply {
            addRow(columns.map { column ->
                if (column == OpenableColumns.DISPLAY_NAME) grant.displayName else grant.identity.size
            }.toTypedArray())
        }
    }

    override fun getType(uri: Uri): String {
        val grant = requireGrant(uri)
        openReadOnly(grant).use { }
        return grant.mimeType
    }

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        if (mode != "r") throw FileNotFoundException("Workspace file permissions are read-only")
        return openReadOnly(requireGrant(uri))
    }

    override fun insert(uri: Uri, values: ContentValues?): Uri =
        throw UnsupportedOperationException("Workspace file provider is read-only")

    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?): Int =
        throw UnsupportedOperationException("Workspace file provider is read-only")

    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int =
        throw UnsupportedOperationException("Workspace file provider is read-only")

    override fun call(method: String, arg: String?, extras: Bundle?): Bundle =
        throw UnsupportedOperationException("Workspace file provider has no registration RPC")

    private fun requireGrant(uri: Uri): WorkspaceOpenFileGrant {
        val context = this.context ?: throw FileNotFoundException("Workspace file provider is unavailable")
        val token = token(context, uri)
        if (Binder.getCallingUid() != Process.myUid() && context.checkUriPermission(
                uri, Binder.getCallingPid(), Binder.getCallingUid(), Intent.FLAG_GRANT_READ_URI_PERMISSION,
            ) != PackageManager.PERMISSION_GRANTED) {
            throw SecurityException("Workspace file URI has no read grant")
        }
        return store(context).find(token)
            ?: throw FileNotFoundException("Workspace file permission is unavailable or expired")
    }

    private fun openReadOnly(grant: WorkspaceOpenFileGrant): ParcelFileDescriptor {
        var descriptor: ParcelFileDescriptor? = null
        var delivered = false
        try {
            val file = File(grant.canonicalPath)
            if (file.canonicalPath != grant.canonicalPath) throw FileNotFoundException()
            val before = Os.lstat(file.path)
            if (!OsConstants.S_ISREG(before.st_mode) || identity(before) != grant.identity) {
                throw FileNotFoundException()
            }
            // Fresh open on every call gives each recipient its own seek offset, not a shared dup.
            val openedDescriptor = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
            descriptor = openedDescriptor
            val opened = Os.fstat(openedDescriptor.fileDescriptor)
            if (!OsConstants.S_ISREG(opened.st_mode) || identity(opened) != grant.identity ||
                file.canonicalPath != grant.canonicalPath) {
                throw FileNotFoundException()
            }
            // A regular-file stat alone is insufficient for special filesystem backends.
            Os.lseek(openedDescriptor.fileDescriptor, 0, OsConstants.SEEK_CUR)
            delivered = true
            return openedDescriptor
        } catch (_: Exception) {
            // Do not expose the physical pathname or token through host/UI/log errors.
            throw FileNotFoundException("Selected workspace file is unavailable or has changed")
        } finally {
            // Includes unexpected failures after open; a successfully delivered FD belongs to its caller.
            if (!delivered) runCatching { descriptor?.close() }
        }
    }
}
