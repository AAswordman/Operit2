package app.operit

import android.app.Activity
import android.content.Intent
import android.database.Cursor
import android.net.Uri
import android.provider.OpenableColumns
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.InputStream
import java.util.UUID

/** Streams user-selected snapshot files to Flutter without materializing their full contents. */
class SnapshotImportInputChannel(private val activity: MainActivity) {
    companion object {
        private const val CHANNEL_NAME = "operit/snapshot_import_input"
        private const val PICK_SNAPSHOT_REQUEST_CODE = 46091
        private const val DEFAULT_CHUNK_SIZE = 64 * 1024
    }

    private data class OpenSnapshotInput(
        val stream: InputStream,
        val uri: Uri,
    )

    private val openInputs = mutableMapOf<String, OpenSnapshotInput>()
    private var pickResult: MethodChannel.Result? = null

    /** Registers snapshot input methods on the Flutter binary messenger. */
    fun attach(messenger: BinaryMessenger) {
        MethodChannel(messenger, CHANNEL_NAME).setMethodCallHandler(::handle)
    }

    /** Releases every opened content stream owned by this channel. */
    fun clear() {
        for (input in openInputs.values) {
            input.stream.close()
        }
        openInputs.clear()
        pickResult = null
    }

    /** Handles one snapshot selection or bounded-read MethodChannel operation. */
    private fun handle(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "pick" -> pickSnapshot(result)
            "readChunk" -> readChunk(call, result)
            "close" -> closeInput(call, result)
            else -> result.notImplemented()
        }
    }

    /** Launches Android's document picker for one ZIP-based snapshot. */
    private fun pickSnapshot(result: MethodChannel.Result) {
        if (pickResult != null) {
            result.error("PICK_IN_PROGRESS", "A snapshot picker is already open", null)
            return
        }
        pickResult = result
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "application/zip"
            putExtra(
                Intent.EXTRA_MIME_TYPES,
                arrayOf(
                    "application/zip",
                    "application/x-zip-compressed",
                    "application/octet-stream",
                ),
            )
        }
        activity.startActivityForResult(intent, PICK_SNAPSHOT_REQUEST_CODE)
    }

    /** Receives the single document-picker result and opens its content stream. */
    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        if (requestCode != PICK_SNAPSHOT_REQUEST_CODE) {
            return false
        }
        val result = pickResult
        pickResult = null
        if (result == null) {
            return true
        }
        if (resultCode != Activity.RESULT_OK) {
            result.success(null)
            return true
        }
        val uri = data?.data
        if (uri == null) {
            result.error("MISSING_DOCUMENT", "Snapshot picker returned no document", null)
            return true
        }
        try {
            val descriptor = describe(uri)
            val stream = activity.contentResolver.openInputStream(uri)
                ?: throw IllegalStateException("Unable to open selected snapshot")
            val token = UUID.randomUUID().toString()
            openInputs[token] = OpenSnapshotInput(stream, uri)
            result.success(
                mapOf(
                    "token" to token,
                    "name" to descriptor.name,
                    "byteLength" to descriptor.byteLength,
                ),
            )
        } catch (error: Throwable) {
            result.error("OPEN_FAILED", error.message, null)
        }
        return true
    }

    /** Reads one requested bounded byte chunk from an opened snapshot stream. */
    private fun readChunk(call: MethodCall, result: MethodChannel.Result) {
        val arguments = call.arguments as? Map<*, *>
        val token = arguments?.get("token") as? String
        val requestedLength = arguments?.get("maxBytes") as? Int
        if (token == null || requestedLength == null || requestedLength <= 0) {
            result.error("INVALID_ARGS", "readChunk expects a token and positive maxBytes", null)
            return
        }
        val input = openInputs[token]
        if (input == null) {
            result.error("UNKNOWN_INPUT", "Snapshot input token is not open", null)
            return
        }
        val buffer = ByteArray(requestedLength.coerceAtMost(DEFAULT_CHUNK_SIZE))
        try {
            val count = input.stream.read(buffer)
            result.success(if (count <= 0) ByteArray(0) else buffer.copyOf(count))
        } catch (error: Throwable) {
            result.error("READ_FAILED", error.message, null)
        }
    }

    /** Closes one opened snapshot input and releases its document stream. */
    private fun closeInput(call: MethodCall, result: MethodChannel.Result) {
        val token = (call.arguments as? Map<*, *>)?.get("token") as? String
        if (token == null) {
            result.error("INVALID_ARGS", "close expects a token", null)
            return
        }
        val input = openInputs.remove(token)
        if (input == null) {
            result.error("UNKNOWN_INPUT", "Snapshot input token is not open", null)
            return
        }
        input.stream.close()
        result.success(null)
    }

    /** Reads the selected document's display name and authoritative byte length. */
    private fun describe(uri: Uri): SnapshotDescriptor {
        var cursor: Cursor? = null
        try {
            cursor = activity.contentResolver.query(
                uri,
                arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE),
                null,
                null,
                null,
            )
            if (cursor == null || !cursor.moveToFirst()) {
                throw IllegalStateException("Selected snapshot metadata is unavailable")
            }
            val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            val sizeIndex = cursor.getColumnIndex(OpenableColumns.SIZE)
            if (nameIndex < 0 || sizeIndex < 0 || cursor.isNull(sizeIndex)) {
                throw IllegalStateException("Selected snapshot does not report its byte length")
            }
            val name = cursor.getString(nameIndex)
            val byteLength = cursor.getLong(sizeIndex)
            if (name.isNullOrBlank() || byteLength < 0) {
                throw IllegalStateException("Selected snapshot metadata is invalid")
            }
            return SnapshotDescriptor(name, byteLength)
        } finally {
            cursor?.close()
        }
    }

    /** Stores the metadata Flutter needs before starting the Core Link upload. */
    private data class SnapshotDescriptor(
        val name: String,
        val byteLength: Long,
    )
}
