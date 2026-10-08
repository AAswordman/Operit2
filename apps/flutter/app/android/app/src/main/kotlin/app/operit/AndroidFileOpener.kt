package app.operit

import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import java.io.File
import java.util.UUID

/** Opens a single readable file; never exposes file:// or a whole private directory. */
internal class AndroidFileOpener(context: Context) {
    private val context = context.applicationContext

    fun open(file: File?, tree: String?, documentId: String?) {
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
                file.inputStream().use { input -> staged.outputStream().use { input.copyTo(it) } }
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
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, mimeType)
            clipData = ClipData.newRawUri("Opened file", uri)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        // ActivityNotFoundException / SecurityException propagate to the tool result.
        context.startActivity(intent)
    }
}

/** A concrete provider avoids device-specific issues with registering the base class directly. */
class OperitOpenFileProvider : FileProvider()
