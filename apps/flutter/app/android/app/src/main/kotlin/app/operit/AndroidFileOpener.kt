package app.operit

import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import java.io.File
import java.io.FileNotFoundException
import java.io.IOException

/** Presents original files through Android's individual read-only URI grants. */
internal class AndroidFileOpener(context: Context) {
    private val context = context.applicationContext

    companion object {
        /** Keeps native paths and Intent contents out of owner error responses. */
        fun safeFailureMessage(error: Exception): String = when (error) {
            is ActivityNotFoundException -> "No application is available to open this file"
            is SecurityException -> "File permission is missing or was revoked"
            is FileNotFoundException -> "Selected file is unavailable or has changed"
            else -> "File could not be presented"
        }
    }

    /** Opens one local or SAF file without copying its content. */
    fun open(file: File?, tree: String?, documentId: String?) {
        present(file, tree, documentId, null)
    }

    /** Shares the same read-only source capability used by file opening. */
    fun share(file: File?, tree: String?, documentId: String?, title: String) {
        present(file, tree, documentId, title)
    }

    /** Builds a single-file Intent and lets Android own the recipient grant lifetime. */
    private fun present(file: File?, tree: String?, documentId: String?, title: String?) {
        try {
            val uri = if (file != null) {
                OperitOpenFileProvider.uriForFile(context, file)
            } else {
                require(tree != null && documentId != null) { "Missing document capability" }
                DocumentsContract.buildDocumentUriUsingTree(Uri.parse(tree), documentId)
            }
            val mimeType = context.contentResolver.getType(uri) ?: "application/octet-stream"
            val intent = Intent(if (title == null) Intent.ACTION_VIEW else Intent.ACTION_SEND).apply {
                if (title == null) setDataAndType(uri, mimeType) else {
                    type = mimeType
                    putExtra(Intent.EXTRA_STREAM, uri)
                }
                clipData = ClipData.newRawUri(file?.name ?: "Selected file", uri)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            val launch = if (title == null) intent else Intent.createChooser(intent, title.takeIf { it.isNotBlank() }).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            context.startActivity(launch)
        } catch (error: Exception) {
            // A failed launch creates no app-owned registration or file to clean up.
            throw IOException(safeFailureMessage(error))
        }
    }
}
