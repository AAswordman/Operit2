package app.operit

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.InputStream
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

/** Owns document metadata and lazy bounded input streams for snapshots and chat attachments. */
class DocumentInputChannel(private val activity: MainActivity) {
    companion object {
        private const val SNAPSHOT_CHANNEL_NAME = "operit/snapshot_import_input"
        private const val FILE_CHANNEL_NAME = "operit/file_input"
        private const val PICK_DOCUMENT_REQUEST_CODE = 46091
        private const val MAX_CHUNK_SIZE = 1024 * 1024
    }

    private data class OpenDocumentInput(val uri: Uri, var stream: InputStream? = null)
    private data class PendingPick(
        val result: MethodChannel.Result,
        val multiple: Boolean,
        val requireLength: Boolean,
    )

    // Only this worker touches input handles. Reads, closes and teardown are ordered, off-main.
    private val worker = Executors.newSingleThreadExecutor()
    private val closed = AtomicBoolean(false)
    private val openInputs = mutableMapOf<String, OpenDocumentInput>()
    private var pendingPick: PendingPick? = null

    fun attach(messenger: BinaryMessenger) {
        MethodChannel(messenger, SNAPSHOT_CHANNEL_NAME).setMethodCallHandler(::handle)
        MethodChannel(messenger, FILE_CHANNEL_NAME).setMethodCallHandler(::handle)
    }

    fun clear() {
        if (!closed.compareAndSet(false, true)) return
        pendingPick?.result?.error("CHANNEL_CLOSED", "Document input channel closed", null)
        pendingPick = null
        worker.execute {
            for (input in openInputs.values) {
                try { input.stream?.close() } catch (_: Exception) { }
            }
            openInputs.clear()
        }
        worker.shutdown()
    }

    private fun handle(call: MethodCall, result: MethodChannel.Result) {
        if (closed.get()) {
            result.error("CHANNEL_CLOSED", "Document input channel closed", null)
            return
        }
        when (call.method) {
            "pick" -> pickDocuments(result, false, true, listOf(
                "application/zip", "application/x-zip-compressed", "application/octet-stream",
            ))
            "pickImages" -> pickDocuments(result, true, false, listOf("image/*"), imagesOnly = true)
            "pickFiles" -> pickDocuments(result, true, false, call.argument<List<String>>("mimeTypes") ?: emptyList())
            "readChunk" -> readChunk(call, result)
            "close" -> closeInput(call, result)
            else -> result.notImplemented()
        }
    }

    /** Selection returns only metadata and tokens; content is opened on the first read. */
    private fun pickDocuments(
        result: MethodChannel.Result,
        multiple: Boolean,
        requireLength: Boolean,
        mimeTypes: List<String>,
        imagesOnly: Boolean = false,
    ) {
        if (pendingPick != null) {
            result.error("PICK_IN_PROGRESS", "A document picker is already open", null)
            return
        }
        pendingPick = PendingPick(result, multiple, requireLength)
        try {
            val intent = if (imagesOnly) {
                // AndroidX chooses the system/backported photo picker and falls back to
                // ACTION_OPEN_DOCUMENT only when no photo picker is available.
                ActivityResultContracts.PickMultipleVisualMedia().createIntent(
                    activity,
                    PickVisualMediaRequest.Builder()
                        .setMediaType(ActivityResultContracts.PickVisualMedia.ImageOnly)
                        .build(),
                )
            } else {
                Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type = if (mimeTypes.size == 1) mimeTypes.first() else "*/*"
                    if (mimeTypes.isNotEmpty()) putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes.toTypedArray())
                    putExtra(Intent.EXTRA_ALLOW_MULTIPLE, multiple)
                }
            }
            activity.startActivityForResult(intent, PICK_DOCUMENT_REQUEST_CODE)
        } catch (error: Exception) {
            pendingPick = null
            result.error("PICK_FAILED", error.message, null)
        }
    }

    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        if (requestCode != PICK_DOCUMENT_REQUEST_CODE) return false
        val pick = pendingPick ?: return true
        pendingPick = null
        if (resultCode != Activity.RESULT_OK) {
            pick.result.success(if (pick.multiple) emptyList<Map<String, Any?>>() else null)
            return true
        }
        val uris = linkedSetOf<Uri>()
        data?.data?.let(uris::add)
        data?.clipData?.let { clip ->
            for (i in 0 until clip.itemCount) uris.add(clip.getItemAt(i).uri)
        }
        if (uris.isEmpty()) {
            pick.result.error("MISSING_DOCUMENT", "Document picker returned no document", null)
            return true
        }
        background(pick.result, "OPEN_FAILED") {
            val descriptors = uris.map { uri ->
                val info = describe(uri)
                if (pick.requireLength && info["byteLength"] == null) {
                    throw IllegalStateException("Selected snapshot does not report its byte length")
                }
                Pair(uri, info)
            }
            val files = descriptors.map { (uri, info) ->
                val token = UUID.randomUUID().toString()
                openInputs[token] = OpenDocumentInput(uri)
                info + ("token" to token)
            }
            if (pick.multiple) files else files.first()
        }
        return true
    }

    private fun readChunk(call: MethodCall, result: MethodChannel.Result) {
        val token = call.argument<String>("token")
        val requestedLength = call.argument<Int>("maxBytes")
        if (token == null || requestedLength == null || requestedLength <= 0) {
            result.error("INVALID_ARGS", "readChunk expects a token and positive maxBytes", null)
            return
        }
        background(result, "READ_FAILED") {
            val input = openInputs[token] ?: throw IllegalStateException("Document input token is not open")
            val stream = input.stream ?: (activity.contentResolver.openInputStream(input.uri)
                ?: throw IllegalStateException("Unable to open selected document")).also { input.stream = it }
            val buffer = ByteArray(requestedLength.coerceAtMost(MAX_CHUNK_SIZE))
            var count: Int
            do { count = stream.read(buffer) } while (count == 0)
            when {
                count < 0 -> ByteArray(0)
                count == buffer.size -> buffer
                else -> buffer.copyOf(count)
            }
        }
    }

    private fun closeInput(call: MethodCall, result: MethodChannel.Result) {
        val token = call.argument<String>("token")
        if (token == null) {
            result.error("INVALID_ARGS", "close expects an input token", null)
            return
        }
        background(result, "CLOSE_FAILED") {
            openInputs.remove(token)?.stream?.close()
            null
        }
    }

    private fun background(result: MethodChannel.Result, errorCode: String, operation: () -> Any?) {
        worker.execute {
            try {
                if (closed.get()) throw IllegalStateException("Document input channel closed")
                val value = operation()
                activity.runOnUiThread { result.success(value) }
            } catch (error: Exception) {
                activity.runOnUiThread { result.error(errorCode, error.message, null) }
            }
        }
    }

    /** Unknown file sizes are valid for normal attachments; snapshots still require a size. */
    private fun describe(uri: Uri): Map<String, Any?> {
        activity.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null).use { cursor ->
            if (cursor == null || !cursor.moveToFirst()) {
                throw IllegalStateException("Selected document metadata is unavailable")
            }
            val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            val sizeIndex = cursor.getColumnIndex(OpenableColumns.SIZE)
            val name = if (nameIndex >= 0) cursor.getString(nameIndex) else null
            if (name.isNullOrBlank()) throw IllegalStateException("Selected document has no filename")
            val size = if (sizeIndex >= 0 && !cursor.isNull(sizeIndex)) cursor.getLong(sizeIndex).takeIf { it >= 0 } else null
            return mapOf("name" to name, "byteLength" to size, "mimeType" to activity.contentResolver.getType(uri))
        }
    }
}
