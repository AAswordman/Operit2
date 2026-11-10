package app.operit

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.OpenableColumns
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.io.InputStream
import java.util.UUID

/** Owns files received through Android sharing and document-open intents. */
class IncomingFileChannel(private val activity: MainActivity, messenger: BinaryMessenger) {
    private data class Input(val file: File, var stream: InputStream? = null)
    private val channel = MethodChannel(messenger, "operit/incoming_files")
    private val worker = HostTaskExecutor(serial = true)
    private val inputs = mutableMapOf<String, Input>()
    private val pending = mutableListOf<List<Map<String, Any?>>>()
    private val errors = mutableListOf<String>()
    private var ready = false
    @Volatile private var closed = false

    /** Installs metadata, bounded-read and release operations on the main engine. */
    init {
        channel.setMethodCallHandler(::handle)
    }

    /** Captures each URI once while the sender's temporary read grant is valid. */
    fun receive(intent: Intent) {
        val uris = linkedSetOf<Uri>()
        when (intent.action) {
            Intent.ACTION_SEND -> stream(intent)?.let(uris::add)
            Intent.ACTION_SEND_MULTIPLE -> streams(intent).forEach(uris::add)
            Intent.ACTION_VIEW -> intent.data?.let(uris::add)
            else -> return
        }
        intent.clipData?.let { clip ->
            for (index in 0 until clip.itemCount) {
                clip.getItemAt(index).uri?.let(uris::add)
            }
        }
        // Activity recreation must not import the same launch intent again.
        intent.action = null
        if (uris.isEmpty()) return
        val mimeType = intent.type
        worker.execute {
            val files = mutableListOf<Map<String, Any?>>()
            for (uri in uris) {
                try {
                    require(uri.scheme == "content" || uri.scheme == "file") {
                        "Unsupported incoming file URI scheme: ${uri.scheme}"
                    }
                    val name = displayName(uri)
                    val token = UUID.randomUUID().toString()
                    val file = File(activity.cacheDir, "incoming-files/$token")
                    check(file.parentFile!!.mkdirs() || file.parentFile!!.isDirectory)
                    try {
                        val source = activity.contentResolver.openInputStream(uri)
                            ?: error("Unable to open shared file: $name")
                        source.use { input -> file.outputStream().use { input.copyTo(it, 1024 * 1024) } }
                        check(!closed) { "Incoming file receiver closed" }
                        inputs[token] = Input(file)
                        files.add(mapOf("token" to token, "name" to name,
                            "byteLength" to file.length(), "mimeType" to mimeType))
                    } catch (error: Exception) {
                        file.delete()
                        throw error
                    }
                } catch (error: Exception) {
                    activity.runOnUiThread { emitError(error.message ?: error.javaClass.simpleName) }
                }
            }
            activity.runOnUiThread {
                if (!closed && files.isNotEmpty()) {
                    if (ready) channel.invokeMethod("files", files) else pending.add(files)
                }
            }
        }
    }

    /** Reads authoritative names from content providers and file URLs. */
    private fun displayName(uri: Uri): String {
        if (uri.scheme == "file") return File(requireNotNull(uri.path)).name
        activity.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null).use { cursor ->
            check(cursor != null && cursor.moveToFirst()) { "Shared file metadata is unavailable" }
            val name = cursor.getString(cursor.getColumnIndexOrThrow(OpenableColumns.DISPLAY_NAME))
            require(!name.isNullOrBlank()) { "Shared file has no filename" }
            return name
        }
    }

    /** Uses the typed parcelable API on Android versions that provide it. */
    @Suppress("DEPRECATION")
    private fun stream(intent: Intent): Uri? =
        if (Build.VERSION.SDK_INT >= 33) intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
        else intent.getParcelableExtra(Intent.EXTRA_STREAM)

    /** Reads all stream attachments in one multi-file share. */
    @Suppress("DEPRECATION")
    private fun streams(intent: Intent): List<Uri> =
        if (Build.VERSION.SDK_INT >= 33) intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java).orEmpty()
        else intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM).orEmpty()

    /** Atomically transfers startup batches and serves bounded file reads off-main. */
    private fun handle(call: MethodCall, result: MethodChannel.Result) {
        if (call.method == "ready") {
            ready = true
            result.success(pending.toList())
            pending.clear()
            errors.forEach { channel.invokeMethod("error", it) }
            errors.clear()
            return
        }
        if (call.method != "readChunk" && call.method != "close") {
            result.notImplemented()
            return
        }
        worker.execute {
            try {
                val token = requireNotNull(call.argument<String>("token"))
                val value = when (call.method) {
                    "readChunk" -> {
                        val input = inputs[token] ?: error("Incoming file token is not open")
                        val maxBytes = requireNotNull(call.argument<Int>("maxBytes"))
                        require(maxBytes > 0)
                        val stream = input.stream ?: input.file.inputStream().also { input.stream = it }
                        val buffer = ByteArray(maxBytes.coerceAtMost(1024 * 1024))
                        val count = stream.read(buffer)
                        if (count < 0) ByteArray(0) else buffer.copyOf(count)
                    }
                    else -> {
                        inputs.remove(token)?.let { input ->
                            try { input.stream?.close() } finally { check(input.file.delete()) }
                        }
                        null
                    }
                }
                activity.runOnUiThread { result.success(value) }
            } catch (error: Exception) {
                activity.runOnUiThread { result.error("INCOMING_FILE_FAILED", error.message, null) }
            }
        }
    }

    /** Queues startup errors until the Dart receiver can report them. */
    private fun emitError(message: String) {
        if (closed) return
        if (ready) channel.invokeMethod("error", message) else errors.add(message)
    }

    /** Releases temporary files and handles when the owning Flutter engine closes. */
    fun clear() {
        closed = true
        channel.setMethodCallHandler(null)
        pending.clear()
        worker.execute {
            for (input in inputs.values) {
                try { input.stream?.close() } finally { input.file.delete() }
            }
            inputs.clear()
        }
        worker.shutdown()
    }
}
