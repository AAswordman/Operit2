package app.operit

import android.app.Activity
import android.content.ClipData
import android.content.Intent
import android.database.Cursor
import android.net.Uri
import android.os.Build
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.util.Log
import androidx.core.content.FileProvider
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.util.LinkedHashSet
import java.util.UUID

/** Owns Android file, photo-picker, directory, and camera requests from Compose DSL. */
class ComposeDslFilePickerChannel(private val activity: MainActivity) {
    companion object {
        private const val TAG = "ComposeDslFilePicker"
        private const val CHANNEL_NAME = "operit/compose_dsl_file_picker"
        private const val DOCUMENT_REQUEST_CODE = 46110
        private const val VISUAL_MEDIA_REQUEST_CODE = 46111
        private const val DIRECTORY_REQUEST_CODE = 46112
        private const val CAMERA_REQUEST_CODE = 46113
    }

    /** Enumerates the normalized picker modes emitted by the Rust host boundary. */
    private enum class PickerMode(val wireName: String) {
        DOCUMENT("document"),
        IMAGE("image"),
        VIDEO("video"),
        MEDIA("media"),
        DIRECTORY("directory"),
        CAMERA("camera"),
    }

    /** Stores one picker request validated by the Compose DSL host boundary. */
    private data class FilePickerRequest(
        val picker: PickerMode,
        val mimeTypes: List<String>,
        val allowMultiple: Boolean,
        val persistPermission: Boolean,
    )

    /** Retains the active platform request until its result is delivered to Flutter. */
    private data class PendingRequest(
        val request: FilePickerRequest,
        val result: MethodChannel.Result,
        val cameraOutput: File? = null,
    )

    /** Stores metadata exposed for one selected content URI. */
    private data class ContentDescriptor(
        val name: String?,
        val mimeType: String?,
    )

    private var pendingRequest: PendingRequest? = null

    /** Registers the picker channel on Flutter's binary messenger. */
    fun attach(messenger: BinaryMessenger) {
        MethodChannel(messenger, CHANNEL_NAME).setMethodCallHandler(::handle)
    }

    /** Releases the active request reference when Flutter's engine is detached. */
    fun clear() {
        pendingRequest = null
    }

    /** Routes one Compose DSL picker request from Flutter. */
    private fun handle(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "open" -> open(call, result)
            else -> result.notImplemented()
        }
    }

    /** Parses and launches one platform picker request. */
    private fun open(call: MethodCall, result: MethodChannel.Result) {
        if (pendingRequest != null) {
            result.error("PICKER_IN_PROGRESS", "A Compose DSL picker is already open", null)
            return
        }
        val arguments = call.arguments as? Map<*, *>
        if (arguments == null) {
            result.error("INVALID_ARGS", "Compose DSL picker expects a map", null)
            return
        }
        try {
            val request = parseRequest(arguments)
            val launch = createLaunch(request)
            val pending = PendingRequest(
                request = request,
                result = result,
                cameraOutput = launch.cameraOutput,
            )
            pendingRequest = pending
            activity.startActivityForResult(launch.intent, launch.requestCode)
        } catch (error: Throwable) {
            Log.e(TAG, "Compose DSL picker launch failed", error)
            pendingRequest = null
            result.error("PICKER_LAUNCH_FAILED", error.message, null)
        }
    }

    /** Decodes the normalized request map provided by ComposeDslFilePickerService. */
    private fun parseRequest(arguments: Map<*, *>): FilePickerRequest {
        val pickerToken = arguments["picker"] as? String
            ?: throw IllegalArgumentException("Compose DSL picker mode is required")
        val picker = PickerMode.entries.firstOrNull { it.wireName == pickerToken }
            ?: throw IllegalArgumentException("Unsupported Compose DSL picker mode: $pickerToken")
        val rawMimeTypes = arguments["mimeTypes"] as? List<*>
            ?: throw IllegalArgumentException("Compose DSL picker mimeTypes are required")
        val mimeTypes = rawMimeTypes.mapIndexed { index, value ->
            value as? String
                ?: throw IllegalArgumentException("Compose DSL picker mimeTypes[$index] must be a string")
        }
        val allowMultiple = arguments["allowMultiple"] as? Boolean
            ?: throw IllegalArgumentException("Compose DSL picker allowMultiple is required")
        val persistPermission = arguments["persistPermission"] as? Boolean
            ?: throw IllegalArgumentException("Compose DSL picker persistPermission is required")
        return FilePickerRequest(picker, mimeTypes, allowMultiple, persistPermission)
    }

    /** Builds the Android intent required by one picker mode. */
    private fun createLaunch(request: FilePickerRequest): PickerLaunch {
        return when (request.picker) {
            PickerMode.DOCUMENT -> PickerLaunch(DOCUMENT_REQUEST_CODE, documentIntent(request))
            PickerMode.IMAGE,
            PickerMode.VIDEO,
            PickerMode.MEDIA -> PickerLaunch(VISUAL_MEDIA_REQUEST_CODE, visualMediaIntent(request))
            PickerMode.DIRECTORY -> PickerLaunch(DIRECTORY_REQUEST_CODE, directoryIntent(request))
            PickerMode.CAMERA -> {
                val output = createCameraOutput()
                PickerLaunch(CAMERA_REQUEST_CODE, cameraIntent(output), output)
            }
        }
    }

    /** Creates a document-picker intent with the requested MIME filtering and URI flags. */
    private fun documentIntent(request: FilePickerRequest): Intent {
        val mimeTypes = request.mimeTypes
        if (mimeTypes.isEmpty()) {
            throw IllegalArgumentException("Document picker requires MIME types")
        }
        return Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = mimeTypes.singleOrNull() ?: "*/*"
            if (mimeTypes.size > 1) {
                putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes.toTypedArray())
            }
            if (request.allowMultiple) {
                putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
            }
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            if (request.persistPermission) {
                addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
            }
        }
    }

    /** Creates an Android 13+ system Photo Picker intent for image, video, or combined media. */
    private fun visualMediaIntent(request: FilePickerRequest): Intent {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
            throw UnsupportedOperationException(
                "Compose DSL visual media picking requires Android 13 or newer",
            )
        }
        val mimeType = when (request.picker) {
            PickerMode.IMAGE -> "image/*"
            PickerMode.VIDEO -> "video/*"
            PickerMode.MEDIA -> null
            PickerMode.DOCUMENT,
            PickerMode.DIRECTORY,
            PickerMode.CAMERA -> throw IllegalArgumentException("Picker mode is not visual media")
        }
        return Intent(MediaStore.ACTION_PICK_IMAGES).apply {
            if (mimeType != null) {
                type = mimeType
            }
            if (request.allowMultiple) {
                val maxItems = MediaStore.getPickImagesMaxLimit()
                if (maxItems <= 0) {
                    throw IllegalStateException("Android Photo Picker returned an invalid selection limit")
                }
                putExtra(MediaStore.EXTRA_PICK_IMAGES_MAX, maxItems)
            }
        }
    }

    /** Creates the persisted document-tree picker intent. */
    private fun directoryIntent(request: FilePickerRequest): Intent {
        return Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            if (request.persistPermission) {
                addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
            }
        }
    }

    /** Allocates one cache-owned JPEG output file for an external camera activity. */
    private fun createCameraOutput(): File {
        val directory = File(activity.cacheDir, "compose_dsl_file_picker/camera")
        if (!directory.exists() && !directory.mkdirs()) {
            throw IllegalStateException("Unable to create Compose DSL camera directory")
        }
        return File(directory, "${UUID.randomUUID()}.jpg")
    }

    /** Creates an external-camera intent with a FileProvider URI owned by this application. */
    private fun cameraIntent(output: File): Intent {
        val uri = FileProvider.getUriForFile(
            activity,
            "${activity.packageName}.compose_dsl_file_picker",
            output,
        )
        return Intent(MediaStore.ACTION_IMAGE_CAPTURE).apply {
            putExtra(MediaStore.EXTRA_OUTPUT, uri)
            clipData = ClipData.newRawUri("compose_dsl_camera_output", uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        }
    }

    /** Delivers one MainActivity result to its owning Compose DSL picker operation. */
    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        val pending = pendingRequest ?: return false
        if (
            requestCode != DOCUMENT_REQUEST_CODE &&
            requestCode != VISUAL_MEDIA_REQUEST_CODE &&
            requestCode != DIRECTORY_REQUEST_CODE &&
            requestCode != CAMERA_REQUEST_CODE
        ) {
            return false
        }
        if (resultCode != Activity.RESULT_OK) {
            pending.cameraOutput?.delete()
            completeSuccess(pending, buildResult(cancelled = true, files = emptyList()))
            return true
        }
        try {
            when (pending.request.picker) {
                PickerMode.DIRECTORY -> completeDirectory(pending, data)
                PickerMode.CAMERA -> completeCamera(pending)
                PickerMode.DOCUMENT,
                PickerMode.IMAGE,
                PickerMode.VIDEO,
                PickerMode.MEDIA -> completeFilesAsync(pending, data, selectedUris(data))
            }
        } catch (error: Throwable) {
            completeError(pending, error)
        }
        return true
    }

    /** Returns all distinct content URIs delivered by a document or Photo Picker result. */
    private fun selectedUris(data: Intent?): List<Uri> {
        val uris = LinkedHashSet<Uri>()
        data?.data?.let(uris::add)
        data?.clipData?.let { clipData ->
            for (index in 0 until clipData.itemCount) {
                clipData.getItemAt(index).uri?.let(uris::add)
            }
        }
        if (uris.isEmpty()) {
            throw IllegalStateException("Compose DSL picker returned no selected content")
        }
        return uris.toList()
    }

    /** Persists a directory URI when requested and returns the directory-only public result. */
    private fun completeDirectory(pending: PendingRequest, data: Intent?) {
        val uri = data?.data
            ?: throw IllegalStateException("Compose DSL directory picker returned no directory")
        if (pending.request.persistPermission) {
            persistUriPermissions(data, listOf(uri))
        }
        completeSuccess(
            pending,
            buildResult(false, listOf(JSONObject().put("uri", uri.toString()))),
        )
    }

    /** Validates one camera output and returns it without an extra content copy. */
    private fun completeCamera(pending: PendingRequest) {
        val output = pending.cameraOutput
            ?: throw IllegalStateException("Compose DSL camera output is missing")
        if (!output.isFile || output.length() <= 0L) {
            throw IllegalStateException("Compose DSL camera returned no image")
        }
        completeSuccess(
            pending,
            buildResult(
                false,
                listOf(
                    JSONObject()
                        .put("uri", Uri.fromFile(output).toString())
                        .put("path", output.absolutePath)
                        .put("name", output.name)
                        .put("mimeType", "image/jpeg")
                        .put("size", output.length()),
                ),
            ),
        )
    }

    /** Copies picked content away from provider access before responding to Flutter. */
    private fun completeFilesAsync(pending: PendingRequest, data: Intent?, uris: List<Uri>) {
        if (!pending.request.allowMultiple && uris.size > 1) {
            throw IllegalStateException("Compose DSL picker returned multiple files for a single selection")
        }
        if (pending.request.persistPermission) {
            persistUriPermissions(data, uris)
        }
        Thread {
            try {
                val files = uris.map(::stagePickedFile)
                activity.runOnUiThread {
                    completeSuccess(pending, buildResult(false, files))
                }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    completeError(pending, error)
                }
            }
        }.start()
    }

    /** Retains document URI grants granted by Android's document provider. */
    private fun persistUriPermissions(data: Intent?, uris: List<Uri>) {
        val flags = (data?.flags ?: Intent.FLAG_GRANT_READ_URI_PERMISSION) and
            (Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        if ((flags and Intent.FLAG_GRANT_READ_URI_PERMISSION) == 0) {
            throw SecurityException("Compose DSL picker did not grant readable persistable permission")
        }
        uris.forEach { uri ->
            activity.contentResolver.takePersistableUriPermission(uri, flags)
        }
    }

    /** Copies one content URI into cache storage and returns its public Compose DSL metadata. */
    private fun stagePickedFile(uri: Uri): JSONObject {
        val descriptor = describe(uri)
        val output = stagedOutputFile(descriptor.name)
        try {
            val input = activity.contentResolver.openInputStream(uri)
                ?: throw IllegalStateException("Unable to open selected Compose DSL content")
            input.use { source ->
                FileOutputStream(output).use { destination ->
                    source.copyTo(destination)
                }
            }
            return JSONObject()
                .put("uri", uri.toString())
                .put("path", output.absolutePath)
                .put("name", descriptor.name ?: output.name)
                .put("mimeType", descriptor.mimeType)
                .put("size", output.length())
        } catch (error: Throwable) {
            output.delete()
            throw error
        }
    }

    /** Creates a collision-free cache destination for one selected content item. */
    private fun stagedOutputFile(name: String?): File {
        val directory = File(activity.cacheDir, "compose_dsl_file_picker/files")
        if (!directory.exists() && !directory.mkdirs()) {
            throw IllegalStateException("Unable to create Compose DSL picker cache directory")
        }
        val suffix = name?.let(::safeFileName)?.takeIf(String::isNotEmpty) ?: "selected"
        return File(directory, "${UUID.randomUUID()}-$suffix")
    }

    /** Converts a provider display name into one cache-file name segment. */
    private fun safeFileName(name: String): String {
        return name.replace(Regex("[\\\\/:*?\"<>|]"), "_")
    }

    /** Reads provider metadata when it is available without requiring a filesystem path. */
    private fun describe(uri: Uri): ContentDescriptor {
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
                return ContentDescriptor(null, activity.contentResolver.getType(uri))
            }
            val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            val name = if (nameIndex >= 0 && !cursor.isNull(nameIndex)) cursor.getString(nameIndex) else null
            return ContentDescriptor(name, activity.contentResolver.getType(uri))
        } finally {
            cursor?.close()
        }
    }

    /** Builds the public JSON result consumed by Compose DSL JavaScript. */
    private fun buildResult(cancelled: Boolean, files: List<JSONObject>): String {
        val encodedFiles = JSONArray()
        files.forEach(encodedFiles::put)
        return JSONObject()
            .put("cancelled", cancelled)
            .put("files", encodedFiles)
            .toString()
    }

    /** Completes one pending request on Flutter's platform thread. */
    private fun completeSuccess(pending: PendingRequest, resultJson: String) {
        if (pendingRequest !== pending) {
            return
        }
        pendingRequest = null
        pending.result.success(resultJson)
    }

    /** Completes one pending request with a logged platform error. */
    private fun completeError(pending: PendingRequest, error: Throwable) {
        if (pendingRequest !== pending) {
            return
        }
        pendingRequest = null
        pending.cameraOutput?.delete()
        Log.e(TAG, "Compose DSL picker failed", error)
        pending.result.error("PICKER_RESULT_FAILED", error.message, null)
    }

    /** Groups an Android intent and its matching request code. */
    private data class PickerLaunch(
        val requestCode: Int,
        val intent: Intent,
        val cameraOutput: File? = null,
    )
}
