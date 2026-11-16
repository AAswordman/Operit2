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
import android.os.ParcelFileDescriptor
import android.os.Process
import android.provider.OpenableColumns
import android.system.Os
import android.system.OsConstants
import android.system.StructStat
import android.webkit.MimeTypeMap
import java.io.File
import java.io.FileNotFoundException
import java.util.Locale

/** Serves original files; Android manages exact-URI permissions, with no grant database or root export. */
class OperitOpenFileProvider : ContentProvider() {
    companion object {
        /** Builds a stable locator for the selected canonical file and its filesystem identity. */
        fun uriForFile(context: Context, file: File): Uri {
            val canonical = file.canonicalFile
            if (!canonical.isFile || !canonical.canRead()) throw FileNotFoundException("Selected file is unavailable")
            val stat = Os.stat(canonical.path)
            if (!OsConstants.S_ISREG(stat.st_mode)) throw FileNotFoundException("Selected file is not a regular file")
            return Uri.Builder().scheme("content").authority("${context.packageName}.openfiles")
                .appendPath(identity(stat)).appendPath(canonical.path).build()
        }

        /** Identifies the original inode while allowing its owner to update the file contents. */
        private fun identity(stat: StructStat): String = "${stat.st_dev}:${stat.st_ino}"
    }

    /** Attaches without creating storage, registrations, or an application runtime. */
    override fun onCreate(): Boolean = true

    /** Reads metadata from the same validated descriptor used for content access. */
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor {
        val file = source(uri)
        val size = openReadOnly(uri, file).use { Os.fstat(it.fileDescriptor).st_size }
        val columns = (projection ?: arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE))
            .filter { it == OpenableColumns.DISPLAY_NAME || it == OpenableColumns.SIZE }.toTypedArray()
        return MatrixCursor(columns, 1).apply {
            addRow(columns.map { if (it == OpenableColumns.DISPLAY_NAME) file.name else size }.toTypedArray())
        }
    }

    /** Resolves the original filename's MIME type only for an authorized, available file. */
    override fun getType(uri: Uri): String {
        val file = source(uri)
        openReadOnly(uri, file).use { }
        return MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase(Locale.ROOT))
            ?: "application/octet-stream"
    }

    /** Gives each reader an independent seek offset and rejects every writable mode. */
    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        if (mode != "r") throw FileNotFoundException("File access is read-only")
        return openReadOnly(uri, source(uri))
    }

    /** Rejects provider-based registration or file creation. */
    override fun insert(uri: Uri, values: ContentValues?): Uri = throw UnsupportedOperationException("Read-only provider")

    /** Rejects file modification even if a caller requests a write grant. */
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?): Int =
        throw UnsupportedOperationException("Read-only provider")

    /** Rejects deletion of the source file. */
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int =
        throw UnsupportedOperationException("Read-only provider")

    /** Requires Android's exact read grant before interpreting the private file locator. */
    private fun source(uri: Uri): File {
        val owner = checkNotNull(context)
        if (Binder.getCallingUid() != Process.myUid() && owner.checkUriPermission(
                uri, Binder.getCallingPid(), Binder.getCallingUid(), Intent.FLAG_GRANT_READ_URI_PERMISSION,
            ) != PackageManager.PERMISSION_GRANTED) throw SecurityException("File read permission is missing")
        val parts = uri.pathSegments
        if (uri.scheme != "content" || uri.authority != "${owner.packageName}.openfiles" ||
            uri.query != null || uri.fragment != null || parts.size != 2) throw FileNotFoundException("Invalid file URI")
        val file = File(parts[1])
        if (!file.isAbsolute || file.canonicalFile != file || uriForFile(owner, file) != uri) {
            throw FileNotFoundException("Selected file is unavailable or has changed")
        }
        return file
    }

    /** Rechecks the opened inode so replacing a pathname cannot redirect an existing grant. */
    private fun openReadOnly(uri: Uri, file: File): ParcelFileDescriptor {
        val descriptor = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
        try {
            val stat = Os.fstat(descriptor.fileDescriptor)
            if (!OsConstants.S_ISREG(stat.st_mode) || identity(stat) != uri.pathSegments[0] || file.canonicalFile != file) {
                throw FileNotFoundException("Selected file is unavailable or has changed")
            }
            Os.lseek(descriptor.fileDescriptor, 0, OsConstants.SEEK_CUR)
            return descriptor
        } catch (error: Exception) {
            descriptor.close()
            throw error
        }
    }
}
