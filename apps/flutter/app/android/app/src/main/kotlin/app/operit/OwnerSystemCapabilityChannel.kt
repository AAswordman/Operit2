package app.operit

import android.media.MediaPlayer
import android.media.projection.MediaProjection
import android.net.Uri
import android.os.Bundle
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
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
            "ownerAudioPlay" -> ownerAudioPlay(call, result)
            "ownerTtsSynthesize" -> ownerTtsSynthesize(call, result)
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
            "ttsSynthesis" -> ttsSynthesize(payloadJson)
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

    private fun ownerAudioPlay(call: MethodCall, result: MethodChannel.Result) {
        val payload = call.arguments as? Map<*, *>
        if (payload == null) {
            result.error("INVALID_ARGS", "ownerAudioPlay expects a map", null)
            return
        }
        runtimeHost.runBackground {
            try {
                val response = audioPlayResult(payload)
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("OWNER_AUDIO_PLAY_ERROR", error.message, null)
                }
            }
        }
    }

    private fun ownerTtsSynthesize(call: MethodCall, result: MethodChannel.Result) {
        val payload = call.arguments as? Map<*, *>
        if (payload == null) {
            result.error("INVALID_ARGS", "ownerTtsSynthesize expects a map", null)
            return
        }
        runtimeHost.runBackground {
            try {
                val response = ttsSynthesizeResult(payload)
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("OWNER_TTS_SYNTHESIZE_ERROR", error.message, null)
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

    private fun audioPlayResult(payload: Map<*, *>): Map<String, Any> {
        val path = payload["path"] as? String ?: throw IllegalArgumentException("path is required")
        val file = File(path)
        if (!file.exists()) {
            throw IllegalStateException("audio file not found: ${file.absolutePath}")
        }
        val mediaPlayer = MediaPlayer()
        try {
            mediaPlayer.setDataSource(file.absolutePath)
            mediaPlayer.prepare()
            mediaPlayer.setOnCompletionListener {
                it.release()
            }
            mediaPlayer.setOnErrorListener { player, _, _ ->
                player.release()
                true
            }
            mediaPlayer.start()
        } catch (error: Throwable) {
            mediaPlayer.release()
            throw error
        }
        return mapOf(
            "path" to file.absolutePath,
            "started" to true,
            "details" to "media_player_started",
        )
    }

    private fun ttsSynthesize(payloadJson: String): String {
        val request = JSONObject(payloadJson)
        val payload =
            mapOf(
                "text" to request.getString("text"),
                "voice" to request.optString("voice", ""),
                "locale" to request.optString("locale", ""),
                "speed" to request.getDouble("speed"),
                "pitch" to request.getDouble("pitch"),
                "outputFormat" to request.getString("outputFormat"),
            )
        return JSONObject(ttsSynthesizeResult(payload)).toString()
    }

    private fun ttsSynthesizeResult(payload: Map<*, *>): Map<String, Any> {
        val text = payload["text"] as? String ?: throw IllegalArgumentException("text is required")
        if (text.isBlank()) {
            throw IllegalArgumentException("text is empty")
        }
        val outputFormat =
            payload["outputFormat"] as? String ?: throw IllegalArgumentException("outputFormat is required")
        if (outputFormat != "wav") {
            throw IllegalArgumentException("android system tts only supports wav output")
        }
        val voice = (payload["voice"] as? String).orEmpty().trim()
        val localeTag = (payload["locale"] as? String).orEmpty().trim()
        val speed = (payload["speed"] as? Number)?.toFloat() ?: throw IllegalArgumentException("speed is required")
        val pitch = (payload["pitch"] as? Number)?.toFloat() ?: throw IllegalArgumentException("pitch is required")
        val outputDir = File(runtimeHost.prepareAndroidRuntimePaths().storageRoot, "runtime/temp/tts")
        outputDir.mkdirs()
        val outputFile = File(outputDir, "${java.util.UUID.randomUUID()}.wav")
        val initLatch = CountDownLatch(1)
        var initStatus = TextToSpeech.ERROR
        val tts = TextToSpeech(activity.applicationContext) { status ->
            initStatus = status
            initLatch.countDown()
        }
        initLatch.await()
        if (initStatus != TextToSpeech.SUCCESS) {
            tts.shutdown()
            throw IllegalStateException("android system tts init failed")
        }
        if (localeTag.isNotEmpty()) {
            val locale = java.util.Locale.forLanguageTag(localeTag)
            val languageResult = tts.setLanguage(locale)
            if (languageResult == TextToSpeech.LANG_MISSING_DATA || languageResult == TextToSpeech.LANG_NOT_SUPPORTED) {
                tts.shutdown()
                throw IllegalStateException("android system tts language not supported: $localeTag")
            }
        }
        if (voice.isNotEmpty()) {
            val selectedVoice = tts.voices.firstOrNull { it.name == voice }
            if (selectedVoice == null) {
                tts.shutdown()
                throw IllegalStateException("android system tts voice not found: $voice")
            }
            tts.voice = selectedVoice
        }
        tts.setSpeechRate(speed)
        tts.setPitch(pitch)
        val completionLatch = CountDownLatch(1)
        var synthesisError: String? = null
        tts.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
            override fun onStart(utteranceId: String?) {}

            override fun onDone(utteranceId: String?) {
                completionLatch.countDown()
            }

            override fun onError(utteranceId: String?) {
                synthesisError = "android system tts synthesis failed"
                completionLatch.countDown()
            }

            override fun onError(utteranceId: String?, errorCode: Int) {
                synthesisError = "android system tts synthesis failed: $errorCode"
                completionLatch.countDown()
            }
        })
        val utteranceId = java.util.UUID.randomUUID().toString()
        val params = Bundle()
        val synthStatus = tts.synthesizeToFile(text, params, outputFile, utteranceId)
        if (synthStatus != TextToSpeech.SUCCESS) {
            tts.shutdown()
            throw IllegalStateException("android system tts synthesizeToFile failed")
        }
        completionLatch.await()
        tts.shutdown()
        synthesisError?.let { throw IllegalStateException(it) }
        if (!outputFile.exists()) {
            throw IllegalStateException("android system tts output missing: ${outputFile.absolutePath}")
        }
        return mapOf(
            "audioPath" to outputFile.absolutePath,
            "details" to "android TextToSpeech synthesis completed",
        )
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
