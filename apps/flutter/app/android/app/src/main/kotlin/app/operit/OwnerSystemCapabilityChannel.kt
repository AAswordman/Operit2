package app.operit

import android.media.projection.MediaProjection
import android.net.Uri
import app.operit.core.tools.system.MediaProjectionCaptureManager
import app.operit.core.tools.system.MediaProjectionHolder
import app.operit.core.tools.system.ScreenCaptureActivity
import app.operit.util.OCRUtils
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.util.concurrent.CountDownLatch
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.json.JSONObject

class OwnerSystemCapabilityChannel(
    private val activity: MainActivity,
    private val runtimeHost: AndroidRuntimeHost,
) {
    private var cachedMediaProjectionCaptureManager: MediaProjectionCaptureManager? = null
    private var cachedMediaProjection: MediaProjection? = null

    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        when (call.method) {
            "ownerSystemCaptureScreenshot" -> ownerSystemCaptureScreenshot(result)
            "ownerSystemRecognizeText" -> ownerSystemRecognizeText(call, result)
            else -> return false
        }
        return true
    }

    fun release() {
        cachedMediaProjectionCaptureManager?.release()
        cachedMediaProjectionCaptureManager = null
        cachedMediaProjection = null
        MediaProjectionHolder.clear(activity.applicationContext)
    }

    fun handleRuntimeHostRequest(methodName: String, payloadJson: String): String {
        return when (methodName) {
            "systemCaptureScreenshot" -> systemCaptureScreenshot()
            "systemRecognizeText" -> systemRecognizeText(payloadJson)
            else -> throw IllegalStateException("runtime host method is not implemented: $methodName")
        }
    }

    private fun ownerSystemCaptureScreenshot(result: MethodChannel.Result) {
        try {
            result.success(systemCaptureScreenshotResult())
        } catch (error: Throwable) {
            result.error("OWNER_SYSTEM_CAPTURE_SCREENSHOT_ERROR", error.message, null)
        }
    }

    private fun ownerSystemRecognizeText(call: MethodCall, result: MethodChannel.Result) {
        val payload = call.arguments as? Map<*, *>
        if (payload == null) {
            result.error("INVALID_ARGS", "ownerSystemRecognizeText expects a map", null)
            return
        }
        runtimeHost.runBackground {
            try {
                val response = systemRecognizeTextResult(payload)
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("OWNER_SYSTEM_RECOGNIZE_TEXT_ERROR", error.message, null)
                }
            }
        }
    }

    private fun systemCaptureScreenshot(): String {
        return JSONObject(systemCaptureScreenshotResult()).toString()
    }

    private fun systemCaptureScreenshotResult(): Map<String, String> {
        return mapOf("path" to captureScreenshotToFile())
    }

    private fun systemRecognizeText(payloadJson: String): String {
        val request = JSONObject(payloadJson)
        val payload =
            mapOf(
                "imagePath" to request.getString("imagePath"),
                "language" to request.getString("language"),
                "quality" to request.getString("quality"),
            )
        return JSONObject(systemRecognizeTextResult(payload)).toString()
    }

    private fun systemRecognizeTextResult(payload: Map<*, *>): Map<String, String> {
        val imagePath =
            payload["imagePath"] as? String
                ?: throw IllegalArgumentException("imagePath is required")
        val language =
            parseOcrLanguage(
                payload["language"] as? String
                    ?: throw IllegalArgumentException("language is required"),
            )
        val quality =
            parseOcrQuality(
                payload["quality"] as? String
                    ?: throw IllegalArgumentException("quality is required"),
            )
        val text =
            runBlocking(Dispatchers.IO) {
                when (
                    val ocrResult =
                        OCRUtils.recognizeTextFromUri(
                            context = activity.applicationContext,
                            uri = Uri.fromFile(File(imagePath)),
                            language = language,
                            quality = quality,
                        )
                ) {
                    is OCRUtils.OCRResult.Success -> ocrResult.getFullText()
                    is OCRUtils.OCRResult.Error -> throw IllegalStateException(ocrResult.message)
                }
            }
        return mapOf("text" to text)
    }

    private fun parseOcrLanguage(value: String): OCRUtils.Language {
        return when (value) {
            "LATIN" -> OCRUtils.Language.LATIN
            "CHINESE" -> OCRUtils.Language.CHINESE
            "JAPANESE" -> OCRUtils.Language.JAPANESE
            "KOREAN" -> OCRUtils.Language.KOREAN
            else -> throw IllegalArgumentException("Unsupported OCR language: $value")
        }
    }

    private fun parseOcrQuality(value: String): OCRUtils.Quality {
        return when (value) {
            "LOW" -> OCRUtils.Quality.LOW
            "HIGH" -> OCRUtils.Quality.HIGH
            else -> throw IllegalArgumentException("Unsupported OCR quality: $value")
        }
    }

    private fun captureScreenshotToFile(): String {
        val screenshotDir = File(runtimeHost.prepareAndroidRuntimePaths().storageRoot, "runtime/temp/clean_on_exit")
        screenshotDir.mkdirs()

        val shortName = System.currentTimeMillis().toString().takeLast(4)
        val file = File(screenshotDir, "$shortName.png")

        val manager = ensureMediaProjectionCaptureManager()
            ?: throw IllegalStateException("Screenshot failed")

        var success = false
        var attempt = 0
        while (!success && attempt < 3) {
            success = manager.captureToFile(file)
            if (!success) {
                Thread.sleep(120)
            }
            attempt++
        }

        if (!success) {
            throw IllegalStateException("Screenshot failed")
        }
        return file.absolutePath
    }

    private fun ensureMediaProjectionCaptureManager(): MediaProjectionCaptureManager? {
        if (MediaProjectionHolder.mediaProjection == null) {
            AndroidClientLogger.d(
                activity.applicationContext,
                "OwnerSystemCapabilityChannel",
                "captureScreenshot: Requesting MediaProjection permission...",
            )
            val launchLatch = CountDownLatch(1)
            activity.runOnUiThread {
                try {
                    ScreenCaptureActivity.cleanStart(activity)
                } finally {
                    launchLatch.countDown()
                }
            }
            launchLatch.await()

            var retries = 0
            while (MediaProjectionHolder.mediaProjection == null && retries < 20) {
                Thread.sleep(500)
                retries++
            }

            if (MediaProjectionHolder.mediaProjection == null) {
                AndroidClientLogger.w(
                    activity.applicationContext,
                    "OwnerSystemCapabilityChannel",
                    "captureScreenshot: MediaProjection permission not granted or timed out",
                )
                return null
            }
        }

        return try {
            val projection = MediaProjectionHolder.mediaProjection ?: return null
            val manager =
                if (cachedMediaProjectionCaptureManager == null || cachedMediaProjection !== projection) {
                    try {
                        cachedMediaProjectionCaptureManager?.release()
                    } catch (_: Exception) {
                    }
                    cachedMediaProjection = projection
                    MediaProjectionCaptureManager(activity.applicationContext, projection).also {
                        cachedMediaProjectionCaptureManager = it
                    }
                } else {
                    cachedMediaProjectionCaptureManager!!
                }

            manager.setupDisplay()
            Thread.sleep(200)
            manager
        } catch (error: Exception) {
            AndroidClientLogger.e(
                activity.applicationContext,
                "OwnerSystemCapabilityChannel",
                "captureScreenshot: Error preparing MediaProjectionCaptureManager: ${error.message.orEmpty()}",
            )
            try {
                cachedMediaProjectionCaptureManager?.release()
            } catch (_: Exception) {
            }
            cachedMediaProjectionCaptureManager = null
            cachedMediaProjection = null
            null
        }
    }
}
