package app.operit

import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.SystemClock
import android.provider.DocumentsContract
import android.util.Log
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import java.io.File
import java.io.FileNotFoundException
import java.io.IOException
import java.util.UUID

/** Opens a single readable file; never exposes file:// or a whole private directory. */
internal class AndroidFileOpener(context: Context) {
    companion object {
        private val presentationRequestId = ThreadLocal<String?>()

        /** Associates only the file-presentation worker with its Core request, not file paths. */
        fun <T> withPresentationTrace(requestId: String, action: () -> T): T {
            val previous = presentationRequestId.get()
            presentationRequestId.set(requestId)
            try {
                return action()
            } finally {
                if (previous == null) presentationRequestId.remove() else presentationRequestId.set(previous)
            }
        }

        /** Native exception messages can contain an Intent URI/token or the physical path. */
        fun safeOpenFailureMessage(error: Exception): String = when (error) {
            is ActivityNotFoundException -> "No application is available to open this workspace file"
            is SecurityException -> "Workspace file permission is missing or was revoked"
            is WorkspaceOpenFileGrantLimitException -> "Too many active workspace file permissions; retry after they expire"
            is FileNotFoundException -> "Selected workspace file is unavailable or has changed"
            is IllegalArgumentException -> if (error.message == "Cannot open a directory as a file") {
                "Cannot open a directory as a file"
            } else {
                "Workspace file could not be opened"
            }
            else -> "Workspace file could not be opened"
        }

        private fun safeOpenFailure(error: Exception): Exception {
            val message = safeOpenFailureMessage(error)
            // Never attach the original cause: its Intent/path could reach logs via a stack trace.
            return when (error) {
                is ActivityNotFoundException -> ActivityNotFoundException(message)
                is SecurityException -> SecurityException(message)
                is WorkspaceOpenFileGrantLimitException -> WorkspaceOpenFileGrantLimitException()
                is FileNotFoundException -> FileNotFoundException(message)
                is IllegalArgumentException -> IllegalArgumentException(message)
                else -> IOException(message)
            }
        }
    }

    private val context = context.applicationContext

    /** Workspace opens grant the original file; legacy open/share retain their staged snapshots. */
    fun openWorkspaceFile(file: File?, tree: String?, documentId: String?) {
        val startedAt = SystemClock.elapsedRealtime()
        val backend = if (file != null) "local" else "saf"
        var registeredUri: Uri? = null
        var dispatched = false
        try {
            val (uri, mimeType) = if (file != null) {
                val type = MimeTypeMap.getSingleton()
                    .getMimeTypeFromExtension(file.extension.lowercase(java.util.Locale.ROOT))
                    ?: "application/octet-stream"
                val localUri = WorkspaceOpenFileProvider.register(context, file, type)
                registeredUri = localUri
                Pair(localUri, type)
            } else {
                readableUri(null, tree, documentId)
            }
            trace("uri_prepared", backend, startedAt, workspace = true)
            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, mimeType)
                clipData = ClipData.newRawUri("Opened workspace file", uri)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            context.startActivity(intent)
            dispatched = true
            trace("activity_dispatched", backend, startedAt, workspace = true)
        } catch (error: Exception) {
            trace("failed", backend, startedAt, failureType = error.javaClass.simpleName, workspace = true)
            throw safeOpenFailure(error)
        } finally {
            if (!dispatched) registeredUri?.let { uri ->
                runCatching { context.revokeUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION) }
                val cleanup = runCatching { WorkspaceOpenFileProvider.revoke(context, uri) }
                if (cleanup.isFailure) {
                    trace("grant_cleanup_failed", backend, startedAt, workspace = true)
                }
            }
        }
    }

    fun open(file: File?, tree: String?, documentId: String?) {
        val (uri, mimeType) = readableUri(file, tree, documentId)
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, mimeType)
            clipData = ClipData.newRawUri("Opened file", uri)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(intent)
    }

    fun share(file: File?, tree: String?, documentId: String?, title: String) {
        val (uri, mimeType) = readableUri(file, tree, documentId)
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = mimeType
            putExtra(Intent.EXTRA_STREAM, uri)
            clipData = ClipData.newRawUri("Shared file", uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(Intent.createChooser(intent, title.takeIf { it.isNotBlank() }).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
        })
    }

    private fun readableUri(file: File?, tree: String?, documentId: String?): Pair<Uri, String> {
        val uri: Uri
        val mimeType: String
        if (file != null) {
            require(file.isFile && file.canRead()) { "File is not readable: $file" }
            val root = File(context.cacheDir, "open_files")
            check(root.isDirectory || root.mkdirs()) { "Cannot create open-file staging directory" }
            // Keep recent files alive for recipient apps, but bound stale staging storage.
            val cutoff = System.currentTimeMillis() - 24 * 60 * 60 * 1000L
            root.listFiles()?.filter { it.isDirectory && it.lastModified() < cutoff }
                ?.forEach { it.deleteRecursively() }
            val directory = File(root, UUID.randomUUID().toString())
            check(directory.mkdir()) { "Cannot stage file for opening" }
            val staged = File(directory, file.name)
            try {
                val startedAt = SystemClock.elapsedRealtime()
                trace("legacy_staging_started", "local", startedAt, bytes = file.length())
                val copied = file.inputStream().use { input -> staged.outputStream().use { input.copyTo(it) } }
                trace("legacy_staging_finished", "local", startedAt, bytes = copied)
                uri = FileProvider.getUriForFile(context, "${context.packageName}.openfiles", staged)
                mimeType = MimeTypeMap.getSingleton()
                    .getMimeTypeFromExtension(file.extension.lowercase(java.util.Locale.ROOT))
                    ?: "application/octet-stream"
            } catch (error: Exception) {
                directory.deleteRecursively()
                throw error
            }
        } else {
            require(tree != null && documentId != null) { "Missing document capability" }
            uri = DocumentsContract.buildDocumentUriUsingTree(Uri.parse(tree), documentId)
            mimeType = context.contentResolver.getType(uri) ?: "application/octet-stream"
        }
        return Pair(uri, mimeType)
    }

    private fun trace(
        phase: String,
        backend: String,
        startedAt: Long,
        bytes: Long? = null,
        failureType: String? = null,
        workspace: Boolean = false,
    ) {
        val requestId = presentationRequestId.get()
        if (requestId == null && !workspace) return
        Log.i("WorkspaceFileOpen", "requestId=${requestId ?: "native"} phase=$phase backend=$backend " +
            "elapsedMs=${SystemClock.elapsedRealtime() - startedAt} thread=${Thread.currentThread().name}" +
            (bytes?.let { " bytes=$it" } ?: "") + (failureType?.let { " failureType=$it" } ?: ""))
    }
}

/** A concrete provider avoids device-specific issues with registering the base class directly. */
class OperitOpenFileProvider : FileProvider()
