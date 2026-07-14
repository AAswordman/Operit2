package app.operit

import android.Manifest
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.BluetoothServerSocket
import android.bluetooth.BluetoothSocket
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.media.MediaPlayer
import android.media.projection.MediaProjection
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import android.util.Base64
import app.operit.core.tools.system.MediaProjectionCaptureManager
import app.operit.core.tools.system.MediaProjectionHolder
import app.operit.core.tools.system.ScreenCaptureActivity
import app.operit.util.OCRUtils
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.nio.charset.StandardCharsets
import java.util.ArrayDeque
import java.util.Locale
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.json.JSONArray
import org.json.JSONObject

class OwnerSystemCapabilityChannel(
    private val activity: MainActivity,
    private val runtimeHost: AndroidRuntimeHost,
) {
    private companion object {
        private const val DEFAULT_CLASSIC_UUID = "00001101-0000-1000-8000-00805f9b34fb"
        private const val TTS_ENGINE_INIT_TIMEOUT_MS = 15_000L
        private const val TTS_SYNTHESIS_TIMEOUT_MS = 120_000L
    }

    private var cachedMediaProjectionCaptureManager: MediaProjectionCaptureManager? = null
    private var cachedMediaProjection: MediaProjection? = null
    private val ttsPlaybackLock = Any()
    private val ttsPlaybackSynthesisLock = Any()
    private val ttsPlaybackAudioQueue = ArrayDeque<TtsPlaybackAudio>()
    private var ttsPlaybackEngine: TextToSpeech? = null
    private var ttsPlaybackPreparing: Boolean = false
    private var ttsPlaybackPaused: Boolean = false
    private var ttsPlaybackPath: String = ""
    private var ttsPlaybackIndex: Long = 0
    private var ttsPlaybackGeneration: Long = 0
    private var ttsPlaybackDetails: String = "android system tts playback idle"
    private var ttsPlaybackError: String? = null
    private var ttsPlaybackReleased: Boolean = false
    private var ttsPlaybackLastOwnerSequence: Long = 0
    private var ttsPlaybackCommandEpoch: Long = 0
    private val musicLock = Any()
    private var musicPlayer: MediaPlayer? = null
    private var musicState: String = "idle"
    private var musicSource: String? = null
    private var musicSourceType: String? = null
    private var musicTitle: String? = null
    private var musicArtist: String? = null
    private var musicDurationMs: Long? = null
    private var musicVolume: Double = 1.0
    private var musicLoopPlayback: Boolean = false
    private var musicMessage: String = "android music player idle"
    private val bluetoothClassicSockets = ConcurrentHashMap<String, BluetoothSocket>()
    private val bluetoothClassicServers = ConcurrentHashMap<String, BluetoothServerSocket>()
    private val bluetoothBleSessions = ConcurrentHashMap<String, BluetoothBleSession>()

    private data class TtsPlaybackAudio(
        val player: MediaPlayer,
        val file: File,
        val deleteOnRelease: Boolean,
    )

    private data class BluetoothBleSession(
        val sessionId: String,
        val address: String,
        val lock: Object = Object(),
        val notifications: ArrayDeque<Map<String, Any?>> = ArrayDeque(),
        var gatt: BluetoothGatt? = null,
        var connected: Boolean = false,
        var servicesReady: Boolean = false,
        var operationDone: Boolean = false,
        var operationStatus: Int = BluetoothGatt.GATT_SUCCESS,
        var operationBytes: ByteArray? = null,
    )

    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        when (call.method) {
            "ownerSystemCaptureScreenshot" -> ownerSystemCaptureScreenshot(result)
            "ownerSystemLanguageCode" -> ownerSystemLanguageCode(result)
            "ownerSystemRecognizeText" -> ownerSystemRecognizeText(call, result)
            "ownerAudioPlay" -> ownerAudioPlay(call, result)
            "ownerMusicPlayback" -> ownerMusicPlayback(call, result)
            "ownerBluetooth" -> ownerBluetooth(call, result)
            "ownerTtsSynthesize" -> ownerTtsSynthesize(call, result)
            "ownerTtsPlayback" -> ownerTtsPlayback(call, result)
            else -> return false
        }
        return true
    }

    /** Releases owner-host resources and invalidates every active TTS generation. */
    fun release() {
        cachedMediaProjectionCaptureManager?.release()
        cachedMediaProjectionCaptureManager = null
        cachedMediaProjection = null
        releaseMusicPlayer()
        bluetoothClassicSockets.values.forEach { socket ->
            try {
                socket.close()
            } catch (_: Exception) {
            }
        }
        bluetoothClassicSockets.clear()
        bluetoothClassicServers.values.forEach { server ->
            try {
                server.close()
            } catch (_: Exception) {
            }
        }
        bluetoothClassicServers.clear()
        bluetoothBleSessions.values.forEach { session ->
            try {
                session.gatt?.close()
            } catch (_: Exception) {
            }
        }
        bluetoothBleSessions.clear()
        val playbackResources =
            synchronized(ttsPlaybackLock) {
                ttsPlaybackReleased = true
                ttsPlaybackGeneration += 1
                ttsPlaybackEngine?.shutdown()
                ttsPlaybackEngine = null
                detachTtsPlaybackAudioLocked()
            }
        releaseTtsPlaybackAudio(playbackResources)
        MediaProjectionHolder.clear(activity.applicationContext)
    }

    fun handleRuntimeHostRequest(methodName: String, payloadJson: String): String {
        return when (methodName) {
            "systemCaptureScreenshot" -> systemCaptureScreenshot()
            "systemLanguageCode" -> systemLanguageCode()
            "systemRecognizeText" -> systemRecognizeText(payloadJson)
            "musicPlayback" -> musicPlayback(payloadJson)
            "bluetooth" -> bluetooth(payloadJson)
            "ttsSynthesis" -> ttsSynthesize(payloadJson)
            "ttsPlayback" -> ttsPlayback(payloadJson)
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

    private fun ownerSystemLanguageCode(result: MethodChannel.Result) {
        try {
            result.success(systemLanguageCodeResult())
        } catch (error: Throwable) {
            result.error("OWNER_SYSTEM_LANGUAGE_CODE_ERROR", error.message, null)
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

    private fun ownerMusicPlayback(call: MethodCall, result: MethodChannel.Result) {
        val payload = call.arguments as? Map<*, *>
        if (payload == null) {
            result.error("INVALID_ARGS", "ownerMusicPlayback expects a map", null)
            return
        }
        runtimeHost.runBackground {
            try {
                val response = musicPlaybackResult(payload)
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("OWNER_MUSIC_PLAYBACK_ERROR", error.message, null)
                }
            }
        }
    }

    private fun ownerBluetooth(call: MethodCall, result: MethodChannel.Result) {
        val payload = call.arguments as? Map<*, *>
        if (payload == null) {
            result.error("INVALID_ARGS", "ownerBluetooth expects a map", null)
            return
        }
        runtimeHost.runBackground {
            try {
                val response = bluetoothResult(payload)
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("OWNER_BLUETOOTH_ERROR", error.message, null)
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

    /** Executes an insertion-ordered owner TTS command on a runtime worker. */
    private fun ownerTtsPlayback(call: MethodCall, result: MethodChannel.Result) {
        val payload = call.arguments as? Map<*, *>
        if (payload == null) {
            result.error("INVALID_ARGS", "ownerTtsPlayback expects a map", null)
            return
        }
        runtimeHost.runBackground {
            try {
                val ownerSequence =
                    (payload["ownerSequence"] as? Number)?.toLong()
                        ?: throw IllegalArgumentException("ownerSequence is required")
                val response = ttsPlaybackResult(payload, ownerSequence)
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("OWNER_TTS_PLAYBACK_ERROR", error.message, null)
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

    private fun systemLanguageCode(): String {
        return JSONObject(systemLanguageCodeResult()).toString()
    }

    private fun systemLanguageCodeResult(): Map<String, String> {
        return mapOf("languageCode" to Locale.getDefault().toLanguageTag())
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

    private fun musicPlayback(payloadJson: String): String {
        val request = JSONObject(payloadJson)
        val payload =
            mapOf(
                "command" to request.getString("command"),
                "source" to request.optString("source", ""),
                "sourceType" to request.optString("sourceType", ""),
                "title" to request.optString("title", ""),
                "artist" to request.optString("artist", ""),
                "loopPlayback" to request.optBoolean("loopPlayback", false),
                "volume" to request.optDouble("volume", 1.0),
                "positionMs" to request.optLong("positionMs", 0L),
            )
        return JSONObject(musicPlaybackResult(payload)).toString()
    }

    private fun musicPlaybackResult(payload: Map<*, *>): Map<String, Any?> {
        return when (val command = payload["command"] as? String ?: throw IllegalArgumentException("command is required")) {
            "play" -> musicPlay(payload)
            "pause" -> musicPause()
            "resume" -> musicResume()
            "stop" -> musicStop()
            "seek" -> musicSeek((payload["positionMs"] as? Number)?.toLong() ?: 0L)
            "set_volume" -> musicSetVolume((payload["volume"] as? Number)?.toDouble() ?: 1.0)
            "status" -> musicStatus("android music player status")
            else -> throw IllegalArgumentException("unsupported music command: $command")
        }
    }

    private fun musicPlay(payload: Map<*, *>): Map<String, Any?> {
        val source = payload["source"] as? String ?: throw IllegalArgumentException("source is required")
        val sourceType = payload["sourceType"] as? String ?: throw IllegalArgumentException("sourceType is required")
        val loopPlayback = payload["loopPlayback"] as? Boolean ?: false
        val volume = ((payload["volume"] as? Number)?.toDouble() ?: 1.0).coerceIn(0.0, 1.0)
        val startPositionMs = ((payload["positionMs"] as? Number)?.toLong() ?: 0L).coerceAtLeast(0L)
        val player = MediaPlayer()
        try {
            when (sourceType) {
                "path" -> player.setDataSource(File(source).absolutePath)
                "uri", "url" -> player.setDataSource(activity.applicationContext, Uri.parse(source))
                else -> throw IllegalArgumentException("unsupported music sourceType: $sourceType")
            }
            player.isLooping = loopPlayback
            player.setVolume(volume.toFloat(), volume.toFloat())
            player.setOnCompletionListener {
                synchronized(musicLock) {
                    if (musicPlayer === it) {
                        musicState = "completed"
                        musicMessage = "android music playback completed"
                    }
                }
            }
            player.setOnErrorListener { activePlayer, _, _ ->
                synchronized(musicLock) {
                    if (musicPlayer === activePlayer) {
                        musicState = "error"
                        musicMessage = "android music playback error"
                    }
                }
                true
            }
            player.prepare()
            if (startPositionMs > 0) {
                player.seekTo(startPositionMs.toInt())
            }
            synchronized(musicLock) {
                releaseMusicPlayerLocked()
                musicPlayer = player
                musicState = "playing"
                musicSource = source
                musicSourceType = sourceType
                musicTitle = (payload["title"] as? String)?.takeIf { it.isNotBlank() }
                musicArtist = (payload["artist"] as? String)?.takeIf { it.isNotBlank() }
                musicDurationMs = player.duration.toLong().takeIf { it >= 0 }
                musicVolume = volume
                musicLoopPlayback = loopPlayback
                musicMessage = "android music playback started"
            }
            player.start()
            return musicStatus("android music playback started")
        } catch (error: Throwable) {
            player.release()
            throw error
        }
    }

    private fun musicPause(): Map<String, Any?> {
        synchronized(musicLock) {
            val player = musicPlayer ?: throw IllegalStateException("android music player is not initialized")
            if (player.isPlaying) {
                player.pause()
                musicState = "paused"
                musicMessage = "android music playback paused"
            }
        }
        return musicStatus("android music playback paused")
    }

    private fun musicResume(): Map<String, Any?> {
        synchronized(musicLock) {
            val player = musicPlayer ?: throw IllegalStateException("android music player is not initialized")
            player.start()
            musicState = "playing"
            musicMessage = "android music playback resumed"
        }
        return musicStatus("android music playback resumed")
    }

    private fun musicStop(): Map<String, Any?> {
        synchronized(musicLock) {
            releaseMusicPlayerLocked()
            musicState = "stopped"
            musicMessage = "android music playback stopped"
        }
        return musicStatus("android music playback stopped")
    }

    private fun musicSeek(positionMs: Long): Map<String, Any?> {
        synchronized(musicLock) {
            val player = musicPlayer ?: throw IllegalStateException("android music player is not initialized")
            player.seekTo(positionMs.coerceAtLeast(0L).toInt())
            musicMessage = "android music playback seeked"
        }
        return musicStatus("android music playback seeked")
    }

    private fun musicSetVolume(volume: Double): Map<String, Any?> {
        val normalizedVolume = volume.coerceIn(0.0, 1.0)
        synchronized(musicLock) {
            val player = musicPlayer ?: throw IllegalStateException("android music player is not initialized")
            player.setVolume(normalizedVolume.toFloat(), normalizedVolume.toFloat())
            musicVolume = normalizedVolume
            musicMessage = "android music playback volume changed"
        }
        return musicStatus("android music playback volume changed")
    }

    private fun musicStatus(message: String): Map<String, Any?> {
        synchronized(musicLock) {
            val player = musicPlayer
            val state =
                if (player != null && player.isPlaying) {
                    "playing"
                } else {
                    musicState
                }
            return mapOf(
                "state" to state,
                "source" to musicSource,
                "sourceType" to musicSourceType,
                "title" to musicTitle,
                "artist" to musicArtist,
                "durationMs" to musicDurationMs,
                "positionMs" to (player?.currentPosition?.toLong() ?: 0L),
                "bufferedPositionMs" to (player?.currentPosition?.toLong() ?: 0L),
                "volume" to musicVolume,
                "loopPlayback" to musicLoopPlayback,
                "message" to message,
            )
        }
    }

    private fun releaseMusicPlayer() {
        synchronized(musicLock) {
            releaseMusicPlayerLocked()
        }
    }

    private fun releaseMusicPlayerLocked() {
        musicPlayer?.release()
        musicPlayer = null
    }

    private fun bluetooth(payloadJson: String): String {
        val request = JSONObject(payloadJson)
        val payload =
            mapOf(
                "command" to request.getString("command"),
                "paramsJson" to request.getString("paramsJson"),
            )
        return JSONObject(bluetoothResult(payload)).toString()
    }

    private fun bluetoothResult(payload: Map<*, *>): Map<String, String> {
        val command = payload["command"] as? String ?: throw IllegalArgumentException("command is required")
        val paramsJson = payload["paramsJson"] as? String ?: throw IllegalArgumentException("paramsJson is required")
        val params = JSONObject(paramsJson)
        val value =
            when (command) {
                "request_permission" -> "android_bluetooth_permission_required:${missingBluetoothPermissions().joinToString(",")}"
                "state" -> bluetoothState()
                "request_enable" -> requestEnableBluetooth()
                "bonded_devices" -> bluetoothBondedDevices()
                "scan" -> bluetoothScan(params)
                "classic_connect" -> bluetoothClassicConnect(params)
                "classic_listen" -> bluetoothClassicListen(params)
                "classic_accept" -> bluetoothClassicAccept(params)
                "classic_send" -> bluetoothClassicSend(params)
                "classic_read" -> bluetoothClassicRead(params)
                "classic_send_and_read" -> bluetoothClassicSendAndRead(params)
                "close" -> bluetoothClose(params.getString("sessionId"))
                "ble_connect" -> bluetoothBleConnect(params)
                "ble_discover_services" -> bluetoothBleDiscoverServices(params)
                "ble_read_characteristic" -> bluetoothBleReadCharacteristic(params)
                "ble_write_characteristic" -> bluetoothBleWriteCharacteristic(params)
                "ble_write_and_read_characteristic" -> bluetoothBleWriteAndReadCharacteristic(params)
                "ble_subscribe_characteristic" -> bluetoothBleSubscribeCharacteristic(params)
                "ble_read_notifications" -> bluetoothBleReadNotifications(params)
                else -> throw IllegalArgumentException("unsupported bluetooth command: $command")
            }
        return mapOf("resultJson" to jsonString(value))
    }

    private fun bluetoothAdapter(): BluetoothAdapter {
        val manager = activity.applicationContext.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
        return manager.adapter ?: throw IllegalStateException("Bluetooth adapter is not available")
    }

    private fun requireBluetoothConnectPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
            activity.checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) != PackageManager.PERMISSION_GRANTED
        ) {
            throw SecurityException("BLUETOOTH_CONNECT permission is required")
        }
    }

    private fun requireBluetoothScanPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
            activity.checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) != PackageManager.PERMISSION_GRANTED
        ) {
            throw SecurityException("BLUETOOTH_SCAN permission is required")
        }
    }

    private fun missingBluetoothPermissions(): List<String> {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
            return emptyList()
        }
        return listOf(
            Manifest.permission.BLUETOOTH_CONNECT,
            Manifest.permission.BLUETOOTH_SCAN,
        ).filter { activity.checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }
    }

    private fun bluetoothState(): Map<String, Any> {
        val adapter = bluetoothAdapter()
        val enabled = adapter.isEnabled
        return mapOf(
            "supported" to true,
            "enabled" to enabled,
            "state" to if (enabled) "enabled" else "disabled",
        )
    }

    private fun requestEnableBluetooth(): String {
        requireBluetoothConnectPermission()
        val adapter = bluetoothAdapter()
        if (adapter.isEnabled) {
            return "android_bluetooth_enabled"
        }
        activity.runOnUiThread {
            activity.startActivity(Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE))
        }
        return "android_bluetooth_enable_requested"
    }

    private fun bluetoothBondedDevices(): Map<String, Any> {
        requireBluetoothConnectPermission()
        return mapOf(
            "devices" to bluetoothAdapter().bondedDevices.map { bluetoothDeviceMap(it, "unknown", null) },
        )
    }

    private fun bluetoothScan(params: JSONObject): Map<String, Any> {
        requireBluetoothScanPermission()
        requireBluetoothConnectPermission()
        val adapter = bluetoothAdapter()
        val durationMs = params.optLong("durationMs", 10000L).coerceAtLeast(0L)
        val devices = ConcurrentHashMap<String, Map<String, Any?>>()
        val receiver =
            object : BroadcastReceiver() {
                override fun onReceive(context: Context?, intent: Intent) {
                    val device =
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                            intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE, BluetoothDevice::class.java)
                        } else {
                            @Suppress("DEPRECATION")
                            intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE)
                        }
                    if (device != null) {
                        devices[device.address] =
                            bluetoothDeviceMap(device, "classic", intent.getShortExtra(BluetoothDevice.EXTRA_RSSI, Short.MIN_VALUE).toInt())
                    }
                }
            }
        activity.applicationContext.registerReceiver(receiver, IntentFilter(BluetoothDevice.ACTION_FOUND))
        try {
            adapter.cancelDiscovery()
            adapter.startDiscovery()
            if (durationMs > 0L) {
                Thread.sleep(durationMs)
            }
            adapter.cancelDiscovery()
        } finally {
            activity.applicationContext.unregisterReceiver(receiver)
        }
        return mapOf(
            "devices" to devices.values.toList(),
            "durationMs" to durationMs,
            "includesBle" to params.optBoolean("includeBle", true),
        )
    }

    private fun bluetoothClassicConnect(params: JSONObject): Map<String, Any> {
        requireBluetoothConnectPermission()
        val address = params.getString("address")
        val uuid = UUID.fromString(params.optString("uuid", DEFAULT_CLASSIC_UUID))
        val socket = bluetoothAdapter().getRemoteDevice(address).createRfcommSocketToServiceRecord(uuid)
        bluetoothAdapter().cancelDiscovery()
        socket.connect()
        val sessionId = "android-classic-${UUID.randomUUID()}"
        bluetoothClassicSockets[sessionId] = socket
        return mapOf("sessionId" to sessionId, "address" to address, "mode" to "classic")
    }

    private fun bluetoothClassicListen(params: JSONObject): Map<String, Any> {
        requireBluetoothConnectPermission()
        val name = params.optString("name", "OperitBluetooth")
        val uuid = UUID.fromString(params.optString("uuid", DEFAULT_CLASSIC_UUID))
        val server = bluetoothAdapter().listenUsingRfcommWithServiceRecord(name, uuid)
        val sessionId = "android-classic-listener-${UUID.randomUUID()}"
        bluetoothClassicServers[sessionId] = server
        return mapOf("sessionId" to sessionId, "address" to bluetoothAdapter().address, "mode" to "classic_listener")
    }

    private fun bluetoothClassicAccept(params: JSONObject): Map<String, Any> {
        requireBluetoothConnectPermission()
        val listenerSessionId = params.getString("listenerSessionId")
        val timeoutMs = params.optLong("timeoutMs", 30000L).coerceAtLeast(0L)
        val server = bluetoothClassicServers[listenerSessionId]
            ?: throw IllegalStateException("Bluetooth listener session not found: $listenerSessionId")
        val socket = server.accept(timeoutMs.toInt())
            ?: throw IllegalStateException("Bluetooth accept timed out: $listenerSessionId")
        val sessionId = "android-classic-${UUID.randomUUID()}"
        bluetoothClassicSockets[sessionId] = socket
        return mapOf("sessionId" to sessionId, "address" to socket.remoteDevice.address, "mode" to "classic")
    }

    private fun bluetoothClassicSend(params: JSONObject): Map<String, Any> {
        val sessionId = params.getString("sessionId")
        val bytes = bluetoothPayloadBytes(params)
        val socket = bluetoothClassicSockets[sessionId]
            ?: throw IllegalStateException("Bluetooth classic session not found: $sessionId")
        socket.outputStream.write(bytes)
        socket.outputStream.flush()
        return mapOf("sessionId" to sessionId, "bytesWritten" to bytes.size.toLong())
    }

    private fun bluetoothClassicRead(params: JSONObject): Map<String, Any?> {
        val sessionId = params.getString("sessionId")
        val maxBytes = params.optLong("maxBytes", 4096L).coerceAtLeast(1L).toInt()
        val timeoutMs = params.optLong("timeoutMs", 30000L).coerceAtLeast(1L)
        val socket = bluetoothClassicSockets[sessionId]
            ?: throw IllegalStateException("Bluetooth classic session not found: $sessionId")
        val buffer = ByteArray(maxBytes)
        val latch = CountDownLatch(1)
        var bytesRead = -1
        var readError: Throwable? = null
        Thread {
            try {
                bytesRead = socket.inputStream.read(buffer)
            } catch (error: Throwable) {
                readError = error
            } finally {
                latch.countDown()
            }
        }.start()
        if (!latch.await(timeoutMs, TimeUnit.MILLISECONDS)) {
            throw IllegalStateException("Bluetooth classic read timed out: $sessionId")
        }
        readError?.let { throw it }
        if (bytesRead < 0) {
            return mapOf("sessionId" to sessionId, "bytesRead" to 0L, "text" to null, "dataBase64" to "")
        }
        return bluetoothReadMap(sessionId, buffer.copyOf(bytesRead))
    }

    private fun bluetoothClassicSendAndRead(params: JSONObject): Map<String, Any?> {
        bluetoothClassicSend(params)
        return bluetoothClassicRead(params)
    }

    private fun bluetoothClose(sessionId: String): String {
        bluetoothClassicSockets.remove(sessionId)?.close()
        bluetoothClassicServers.remove(sessionId)?.close()
        bluetoothBleSessions.remove(sessionId)?.gatt?.close()
        return "android_bluetooth_session_closed:$sessionId"
    }

    private fun bluetoothBleConnect(params: JSONObject): Map<String, Any> {
        requireBluetoothConnectPermission()
        val address = params.getString("address")
        val device = bluetoothAdapter().getRemoteDevice(address)
        val sessionId = "android-ble-${UUID.randomUUID()}"
        val session = BluetoothBleSession(sessionId = sessionId, address = address)
        val callback =
            object : BluetoothGattCallback() {
                override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
                    synchronized(session.lock) {
                        session.operationStatus = status
                        session.connected = status == BluetoothGatt.GATT_SUCCESS && newState == BluetoothProfile.STATE_CONNECTED
                        session.operationDone = true
                        session.lock.notifyAll()
                    }
                }

                override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
                    synchronized(session.lock) {
                        session.operationStatus = status
                        session.servicesReady = status == BluetoothGatt.GATT_SUCCESS
                        session.operationDone = true
                        session.lock.notifyAll()
                    }
                }

                override fun onCharacteristicRead(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, status: Int) {
                    synchronized(session.lock) {
                        session.operationStatus = status
                        @Suppress("DEPRECATION")
                        session.operationBytes = characteristic.value
                        session.operationDone = true
                        session.lock.notifyAll()
                    }
                }

                override fun onCharacteristicRead(
                    gatt: BluetoothGatt,
                    characteristic: BluetoothGattCharacteristic,
                    value: ByteArray,
                    status: Int,
                ) {
                    synchronized(session.lock) {
                        session.operationStatus = status
                        session.operationBytes = value
                        session.operationDone = true
                        session.lock.notifyAll()
                    }
                }

                override fun onCharacteristicWrite(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, status: Int) {
                    synchronized(session.lock) {
                        session.operationStatus = status
                        session.operationDone = true
                        session.lock.notifyAll()
                    }
                }

                override fun onDescriptorWrite(gatt: BluetoothGatt, descriptor: BluetoothGattDescriptor, status: Int) {
                    synchronized(session.lock) {
                        session.operationStatus = status
                        session.operationDone = true
                        session.lock.notifyAll()
                    }
                }

                override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic) {
                    @Suppress("DEPRECATION")
                    val bytes = characteristic.value ?: ByteArray(0)
                    enqueueBleNotification(session, characteristic.uuid.toString(), bytes)
                }

                override fun onCharacteristicChanged(
                    gatt: BluetoothGatt,
                    characteristic: BluetoothGattCharacteristic,
                    value: ByteArray,
                ) {
                    enqueueBleNotification(session, characteristic.uuid.toString(), value)
                }
            }
        val gatt =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                device.connectGatt(activity.applicationContext, params.optBoolean("autoConnect", false), callback, BluetoothDevice.TRANSPORT_LE)
            } else {
                @Suppress("DEPRECATION")
                device.connectGatt(activity.applicationContext, params.optBoolean("autoConnect", false), callback)
            }
        session.gatt = gatt
        bluetoothBleSessions[sessionId] = session
        waitBleOperation(session, 30000L, "BLE connect")
        if (!session.connected) {
            bluetoothBleSessions.remove(sessionId)
            gatt.close()
            throw IllegalStateException("BLE connect failed: ${session.operationStatus}")
        }
        return mapOf("sessionId" to sessionId, "address" to address, "mode" to "ble")
    }

    private fun enqueueBleNotification(session: BluetoothBleSession, characteristicUuid: String, bytes: ByteArray) {
        synchronized(session.lock) {
            session.notifications.addLast(
                mapOf(
                    "characteristicUuid" to characteristicUuid,
                    "bytesRead" to bytes.size.toLong(),
                    "text" to String(bytes, StandardCharsets.UTF_8),
                    "dataBase64" to Base64.encodeToString(bytes, Base64.NO_WRAP),
                    "timestamp" to System.currentTimeMillis(),
                ),
            )
        }
    }

    private fun bluetoothBleDiscoverServices(params: JSONObject): Map<String, Any> {
        val session = requireBleSession(params.getString("sessionId"))
        val gatt = session.gatt ?: throw IllegalStateException("BLE GATT is not available: ${session.sessionId}")
        synchronized(session.lock) {
            session.operationDone = false
            session.operationStatus = BluetoothGatt.GATT_SUCCESS
        }
        if (!gatt.discoverServices()) {
            throw IllegalStateException("BLE discover services did not start: ${session.sessionId}")
        }
        waitBleOperation(session, params.optLong("timeoutMs", 30000L), "BLE discover services")
        if (!session.servicesReady) {
            throw IllegalStateException("BLE discover services failed: ${session.operationStatus}")
        }
        return mapOf(
            "sessionId" to session.sessionId,
            "services" to gatt.services.map { service ->
                mapOf(
                    "uuid" to service.uuid.toString(),
                    "characteristics" to service.characteristics.map { characteristic ->
                        mapOf(
                            "uuid" to characteristic.uuid.toString(),
                            "properties" to characteristicProperties(characteristic),
                        )
                    },
                )
            },
        )
    }

    private fun bluetoothBleReadCharacteristic(params: JSONObject): Map<String, Any?> {
        val session = requireBleSession(params.getString("sessionId"))
        val gatt = session.gatt ?: throw IllegalStateException("BLE GATT is not available: ${session.sessionId}")
        val characteristic = bleCharacteristic(gatt, params.getString("serviceUuid"), params.getString("characteristicUuid"))
        synchronized(session.lock) {
            session.operationDone = false
            session.operationBytes = null
            session.operationStatus = BluetoothGatt.GATT_SUCCESS
        }
        if (!gatt.readCharacteristic(characteristic)) {
            throw IllegalStateException("BLE characteristic read did not start: ${characteristic.uuid}")
        }
        waitBleOperation(session, params.optLong("timeoutMs", 30000L), "BLE characteristic read")
        if (session.operationStatus != BluetoothGatt.GATT_SUCCESS) {
            throw IllegalStateException("BLE characteristic read failed: ${session.operationStatus}")
        }
        return bluetoothReadMap(session.sessionId, session.operationBytes ?: ByteArray(0))
    }

    private fun bluetoothBleWriteCharacteristic(params: JSONObject): Map<String, Any> {
        val bytes = bluetoothPayloadBytes(params)
        writeBleCharacteristic(
            params.getString("sessionId"),
            params.getString("serviceUuid"),
            params.getString("characteristicUuid"),
            bytes,
            params.optLong("timeoutMs", 30000L),
        )
        return mapOf("sessionId" to params.getString("sessionId"), "bytesWritten" to bytes.size.toLong())
    }

    private fun bluetoothBleWriteAndReadCharacteristic(params: JSONObject): Map<String, Any?> {
        val sessionId = params.getString("sessionId")
        writeBleCharacteristic(
            sessionId,
            params.getString("writeServiceUuid"),
            params.getString("writeCharacteristicUuid"),
            bluetoothPayloadBytes(params),
            params.optLong("timeoutMs", 30000L),
        )
        val readParams =
            JSONObject()
                .put("sessionId", sessionId)
                .put("serviceUuid", params.getString("readServiceUuid"))
                .put("characteristicUuid", params.getString("readCharacteristicUuid"))
                .put("timeoutMs", params.optLong("timeoutMs", 30000L))
        return bluetoothBleReadCharacteristic(readParams)
    }

    private fun bluetoothBleSubscribeCharacteristic(params: JSONObject): Map<String, Any> {
        val session = requireBleSession(params.getString("sessionId"))
        val gatt = session.gatt ?: throw IllegalStateException("BLE GATT is not available: ${session.sessionId}")
        val characteristic = bleCharacteristic(gatt, params.getString("serviceUuid"), params.getString("characteristicUuid"))
        val enable = params.optBoolean("enable", true)
        if (!gatt.setCharacteristicNotification(characteristic, enable)) {
            throw IllegalStateException("BLE characteristic notification change failed: ${characteristic.uuid}")
        }
        val descriptor = characteristic.descriptors.firstOrNull()
        if (descriptor != null) {
            synchronized(session.lock) {
                session.operationDone = false
                session.operationStatus = BluetoothGatt.GATT_SUCCESS
            }
            @Suppress("DEPRECATION")
            descriptor.value =
                if (enable) BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE else BluetoothGattDescriptor.DISABLE_NOTIFICATION_VALUE
            if (!gatt.writeDescriptor(descriptor)) {
                throw IllegalStateException("BLE descriptor write did not start: ${descriptor.uuid}")
            }
            waitBleOperation(session, params.optLong("timeoutMs", 30000L), "BLE descriptor write")
            if (session.operationStatus != BluetoothGatt.GATT_SUCCESS) {
                throw IllegalStateException("BLE descriptor write failed: ${session.operationStatus}")
            }
        }
        return mapOf("sessionId" to session.sessionId, "bytesWritten" to 0L)
    }

    private fun bluetoothBleReadNotifications(params: JSONObject): Map<String, Any> {
        val session = requireBleSession(params.getString("sessionId"))
        val limit = params.optLong("limit", 50L).coerceAtLeast(0L).toInt()
        val notifications = mutableListOf<Map<String, Any?>>()
        synchronized(session.lock) {
            while (notifications.size < limit && session.notifications.isNotEmpty()) {
                notifications.add(session.notifications.removeFirst())
            }
        }
        return mapOf("sessionId" to session.sessionId, "notifications" to notifications)
    }

    private fun writeBleCharacteristic(
        sessionId: String,
        serviceUuid: String,
        characteristicUuid: String,
        bytes: ByteArray,
        timeoutMs: Long,
    ) {
        val session = requireBleSession(sessionId)
        val gatt = session.gatt ?: throw IllegalStateException("BLE GATT is not available: $sessionId")
        val characteristic = bleCharacteristic(gatt, serviceUuid, characteristicUuid)
        @Suppress("DEPRECATION")
        characteristic.value = bytes
        synchronized(session.lock) {
            session.operationDone = false
            session.operationStatus = BluetoothGatt.GATT_SUCCESS
        }
        if (!gatt.writeCharacteristic(characteristic)) {
            throw IllegalStateException("BLE characteristic write did not start: $characteristicUuid")
        }
        waitBleOperation(session, timeoutMs, "BLE characteristic write")
        if (session.operationStatus != BluetoothGatt.GATT_SUCCESS) {
            throw IllegalStateException("BLE characteristic write failed: ${session.operationStatus}")
        }
    }

    private fun waitBleOperation(session: BluetoothBleSession, timeoutMs: Long, label: String) {
        val deadline = System.currentTimeMillis() + timeoutMs.coerceAtLeast(1L)
        synchronized(session.lock) {
            while (!session.operationDone) {
                val remaining = deadline - System.currentTimeMillis()
                if (remaining <= 0L) {
                    throw IllegalStateException("$label timed out: ${session.sessionId}")
                }
                session.lock.wait(remaining)
            }
        }
    }

    private fun requireBleSession(sessionId: String): BluetoothBleSession {
        return bluetoothBleSessions[sessionId] ?: throw IllegalStateException("BLE session not found: $sessionId")
    }

    private fun bleCharacteristic(
        gatt: BluetoothGatt,
        serviceUuid: String,
        characteristicUuid: String,
    ): BluetoothGattCharacteristic {
        val service = gatt.getService(UUID.fromString(serviceUuid))
            ?: throw IllegalStateException("BLE service not found: $serviceUuid")
        return service.getCharacteristic(UUID.fromString(characteristicUuid))
            ?: throw IllegalStateException("BLE characteristic not found: $characteristicUuid")
    }

    private fun bluetoothDeviceMap(device: BluetoothDevice, source: String, rssi: Int?): Map<String, Any?> {
        requireBluetoothConnectPermission()
        return mapOf(
            "name" to device.name,
            "address" to device.address,
            "type" to bluetoothDeviceType(device.type),
            "bondState" to bluetoothBondState(device.bondState),
            "source" to source,
            "rssi" to rssi?.takeIf { it != Short.MIN_VALUE.toInt() },
        )
    }

    private fun bluetoothDeviceType(type: Int): String {
        return when (type) {
            BluetoothDevice.DEVICE_TYPE_CLASSIC -> "classic"
            BluetoothDevice.DEVICE_TYPE_LE -> "ble"
            BluetoothDevice.DEVICE_TYPE_DUAL -> "dual"
            BluetoothDevice.DEVICE_TYPE_UNKNOWN -> "unknown"
            else -> "unknown"
        }
    }

    private fun bluetoothBondState(state: Int): String {
        return when (state) {
            BluetoothDevice.BOND_BONDED -> "bonded"
            BluetoothDevice.BOND_BONDING -> "bonding"
            BluetoothDevice.BOND_NONE -> "none"
            else -> "unknown"
        }
    }

    private fun bluetoothPayloadBytes(params: JSONObject): ByteArray {
        val text = params.optString("text", "")
        val dataBase64 = params.optString("dataBase64", "")
        if (text.isNotEmpty() == dataBase64.isNotEmpty()) {
            throw IllegalArgumentException("Provide exactly one of text or dataBase64")
        }
        return if (text.isNotEmpty()) {
            text.toByteArray(StandardCharsets.UTF_8)
        } else {
            Base64.decode(dataBase64, Base64.DEFAULT)
        }
    }

    private fun bluetoothReadMap(sessionId: String, bytes: ByteArray): Map<String, Any?> {
        return mapOf(
            "sessionId" to sessionId,
            "bytesRead" to bytes.size.toLong(),
            "text" to String(bytes, StandardCharsets.UTF_8),
            "dataBase64" to Base64.encodeToString(bytes, Base64.NO_WRAP),
        )
    }

    private fun characteristicProperties(characteristic: BluetoothGattCharacteristic): List<String> {
        val properties = mutableListOf<String>()
        val value = characteristic.properties
        if (value and BluetoothGattCharacteristic.PROPERTY_READ != 0) properties += "read"
        if (value and BluetoothGattCharacteristic.PROPERTY_WRITE != 0) properties += "write"
        if (value and BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE != 0) properties += "write_without_response"
        if (value and BluetoothGattCharacteristic.PROPERTY_NOTIFY != 0) properties += "notify"
        if (value and BluetoothGattCharacteristic.PROPERTY_INDICATE != 0) properties += "indicate"
        return properties
    }

    private fun jsonString(value: Any?): String {
        return when (value) {
            is String -> JSONObject.quote(value)
            is Map<*, *> -> jsonObject(value).toString()
            is Iterable<*> -> jsonArray(value).toString()
            null -> "null"
            else -> JSONObject.wrap(value).toString()
        }
    }

    private fun jsonObject(value: Map<*, *>): JSONObject {
        val output = JSONObject()
        for ((key, item) in value) {
            output.put(key.toString(), jsonValue(item))
        }
        return output
    }

    private fun jsonArray(value: Iterable<*>): JSONArray {
        val output = JSONArray()
        for (item in value) {
            output.put(jsonValue(item))
        }
        return output
    }

    private fun jsonValue(value: Any?): Any? {
        return when (value) {
            is Map<*, *> -> jsonObject(value)
            is Iterable<*> -> jsonArray(value)
            null -> JSONObject.NULL
            else -> value
        }
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

    /** Decodes a runtime JSON payload and executes one TTS command. */
    private fun ttsPlayback(payloadJson: String): String {
        val request = JSONObject(payloadJson)
        val payload =
            mapOf(
                "command" to request.getString("command"),
                "audioPath" to request.optString("audioPath", ""),
                "text" to request.optString("text", ""),
                "voice" to request.optString("voice", ""),
                "locale" to request.optString("locale", ""),
                "speed" to request.optDouble("speed", 1.0),
                "pitch" to request.optDouble("pitch", 1.0),
                "interrupt" to request.optBoolean("interrupt", false),
            )
        return JSONObject(ttsPlaybackResult(payload, null)).toString()
    }

    /** Executes one Android TTS playback command. */
    private fun ttsPlaybackResult(payload: Map<*, *>, ownerSequence: Long?): Map<String, Any> {
        val command = payload["command"] as? String ?: throw IllegalArgumentException("command is required")
        if (command == "state") {
            return ttsPlaybackState()
        }
        val commandEpoch = registerTtsPlaybackCommand(ownerSequence)
            ?: return ttsPlaybackStatus("android system tts ignored an outdated owner command")
        return when (command) {
            "play" -> ttsPlaybackPlayAudio(payload, commandEpoch)
            "speak" -> ttsPlaybackSpeak(payload, commandEpoch)
            "pause" -> ttsPlaybackPause(commandEpoch)
            "resume" -> ttsPlaybackResume(commandEpoch)
            "stop" -> ttsPlaybackStop(commandEpoch)
            else -> throw IllegalArgumentException("unsupported tts playback command: $command")
        }
    }

    /** Starts one generated speech audio file in the Android TTS session. */
    private fun ttsPlaybackPlayAudio(payload: Map<*, *>, commandEpoch: Long): Map<String, Any> {
        val path = payload["audioPath"] as? String ?: throw IllegalArgumentException("audioPath is required")
        if (path.isBlank()) {
            throw IllegalArgumentException("audioPath is empty")
        }
        val file = File(path)
        if (!file.isFile) {
            throw IllegalArgumentException("TTS audio file does not exist: $path")
        }
        val previousAudio: List<TtsPlaybackAudio>
        val engine: TextToSpeech?
        val generation: Long
        synchronized(ttsPlaybackLock) {
            if (!isTtsPlaybackCommandCurrentLocked(commandEpoch)) {
                return ttsPlaybackStatusLocked("android TTS ignored an outdated play command")
            }
            if (ttsPlaybackReleased) {
                throw IllegalStateException("android TTS playback host is released")
            }
            ttsPlaybackGeneration += 1
            generation = ttsPlaybackGeneration
            previousAudio = detachTtsPlaybackAudioLocked()
            engine = ttsPlaybackEngine
            ttsPlaybackPreparing = true
            ttsPlaybackPaused = false
            ttsPlaybackPath = path
            ttsPlaybackDetails = "android generated TTS playback preparing"
            ttsPlaybackError = null
        }
        engine?.stop()
        releaseTtsPlaybackAudio(previousAudio)
        val audio = prepareTtsPlaybackAudio(listOf(file), generation, false)
        synchronized(ttsPlaybackLock) {
            if (generation != ttsPlaybackGeneration || !ttsPlaybackPreparing) {
                releaseTtsPlaybackAudio(audio)
                return ttsPlaybackStatusLocked("android TTS playback request cancelled")
            }
            audio.forEach(ttsPlaybackAudioQueue::addLast)
            ttsPlaybackPreparing = false
            ttsPlaybackDetails = "android generated TTS playback started"
            try {
                ttsPlaybackAudioQueue.first().player.start()
            } catch (error: Throwable) {
                val failedAudio = detachTtsPlaybackAudioLocked()
                releaseTtsPlaybackAudio(failedAudio)
                ttsPlaybackPath = ""
                ttsPlaybackDetails = "android generated TTS playback start failed"
                ttsPlaybackError = error.message ?: error.toString()
                throw error
            }
            return ttsPlaybackStatusLocked(ttsPlaybackDetails)
        }
    }

    /** Synthesizes and starts one generation-checked Android speech request. */
    private fun ttsPlaybackSpeak(payload: Map<*, *>, commandEpoch: Long): Map<String, Any> {
        val text = payload["text"] as? String ?: throw IllegalArgumentException("text is required")
        if (text.isBlank()) {
            throw IllegalArgumentException("text is empty")
        }
        val voice = (payload["voice"] as? String).orEmpty().trim()
        val localeTag = (payload["locale"] as? String).orEmpty().trim()
        val speed = (payload["speed"] as? Number)?.toFloat() ?: throw IllegalArgumentException("speed is required")
        val pitch = (payload["pitch"] as? Number)?.toFloat() ?: throw IllegalArgumentException("pitch is required")
        val interrupt = payload["interrupt"] as? Boolean ?: throw IllegalArgumentException("interrupt is required")
        val previousAudio: List<TtsPlaybackAudio>
        val generation: Long
        synchronized(ttsPlaybackLock) {
            if (!isTtsPlaybackCommandCurrentLocked(commandEpoch)) {
                return ttsPlaybackStatusLocked("android system tts ignored an outdated speak command")
            }
            if (ttsPlaybackReleased) {
                throw IllegalStateException("android system tts playback host is released")
            }
            if (!interrupt && (ttsPlaybackPreparing || ttsPlaybackAudioQueue.isNotEmpty())) {
                throw IllegalStateException("android system tts playback is busy")
            }
            ttsPlaybackGeneration += 1
            generation = ttsPlaybackGeneration
            previousAudio = detachTtsPlaybackAudioLocked()
            ttsPlaybackPreparing = true
            ttsPlaybackPaused = false
            ttsPlaybackPath = "android-tts://${++ttsPlaybackIndex}"
            ttsPlaybackDetails = "android system tts synthesis started"
            ttsPlaybackError = null
        }
        releaseTtsPlaybackAudio(previousAudio)
        if (interrupt) {
            synchronized(ttsPlaybackLock) { ttsPlaybackEngine }?.stop()
        }
        val outputFiles =
            try {
                synthesizeTtsPlaybackFiles(text, voice, localeTag, speed, pitch, generation)
            } catch (error: Throwable) {
                synchronized(ttsPlaybackLock) {
                    if (generation == ttsPlaybackGeneration) {
                        ttsPlaybackPreparing = false
                        ttsPlaybackPath = ""
                        ttsPlaybackDetails = "android system tts synthesis failed"
                        ttsPlaybackError = error.message ?: error.toString()
                    }
                }
                throw error
            }
        if (!isTtsPlaybackGenerationCurrent(generation)) {
            deleteTtsPlaybackFiles(outputFiles)
            return ttsPlaybackStatus("android system tts playback request cancelled")
        }
        val audio = prepareTtsPlaybackAudio(outputFiles, generation, true)
        synchronized(ttsPlaybackLock) {
            if (generation != ttsPlaybackGeneration || !ttsPlaybackPreparing) {
                releaseTtsPlaybackAudio(audio)
                return ttsPlaybackStatusLocked("android system tts playback request cancelled")
            }
            audio.forEach(ttsPlaybackAudioQueue::addLast)
            ttsPlaybackPreparing = false
            ttsPlaybackPaused = false
            ttsPlaybackDetails = "android system tts playback started"
            try {
                ttsPlaybackAudioQueue.first().player.start()
            } catch (error: Throwable) {
                val failedAudio = detachTtsPlaybackAudioLocked()
                releaseTtsPlaybackAudio(failedAudio)
                ttsPlaybackPath = ""
                ttsPlaybackDetails = "android system tts playback start failed"
                ttsPlaybackError = error.message ?: error.toString()
                throw error
            }
            return ttsPlaybackStatusLocked(ttsPlaybackDetails)
        }
    }

    /** Pauses Android speech at the exact MediaPlayer position. */
    private fun ttsPlaybackPause(commandEpoch: Long): Map<String, Any> {
        synchronized(ttsPlaybackLock) {
            if (!isTtsPlaybackCommandCurrentLocked(commandEpoch)) {
                return ttsPlaybackStatusLocked("android system tts ignored an outdated pause command")
            }
            val current = ttsPlaybackAudioQueue.firstOrNull()
            if (current == null) {
                return ttsPlaybackStatusLocked("android system tts playback is not active")
            }
            if (!ttsPlaybackPaused) {
                current.player.pause()
                ttsPlaybackPaused = true
                ttsPlaybackDetails = "android system tts playback paused"
            }
            return ttsPlaybackStatusLocked(ttsPlaybackDetails)
        }
    }

    /** Resumes Android speech from the exact paused MediaPlayer position. */
    private fun ttsPlaybackResume(commandEpoch: Long): Map<String, Any> {
        synchronized(ttsPlaybackLock) {
            if (!isTtsPlaybackCommandCurrentLocked(commandEpoch)) {
                return ttsPlaybackStatusLocked("android system tts ignored an outdated resume command")
            }
            val current = ttsPlaybackAudioQueue.firstOrNull()
            if (current == null || !ttsPlaybackPaused) {
                return ttsPlaybackStatusLocked("android system tts playback is not paused")
            }
            current.player.start()
            ttsPlaybackPaused = false
            ttsPlaybackDetails = "android system tts playback resumed"
            return ttsPlaybackStatusLocked(ttsPlaybackDetails)
        }
    }

    /** Invalidates preparation and stops all Android speech audio. */
    private fun ttsPlaybackStop(commandEpoch: Long): Map<String, Any> {
        val audio: List<TtsPlaybackAudio>
        val engine: TextToSpeech?
        synchronized(ttsPlaybackLock) {
            if (!isTtsPlaybackCommandCurrentLocked(commandEpoch)) {
                return ttsPlaybackStatusLocked("android system tts ignored an outdated stop command")
            }
            ttsPlaybackGeneration += 1
            ttsPlaybackPreparing = false
            ttsPlaybackPaused = false
            ttsPlaybackPath = ""
            ttsPlaybackDetails = "android system tts playback stopped"
            ttsPlaybackError = null
            audio = detachTtsPlaybackAudioLocked()
            engine = ttsPlaybackEngine
        }
        engine?.stop()
        releaseTtsPlaybackAudio(audio)
        return ttsPlaybackStatus("android system tts playback stopped")
    }

    /** Synthesizes every bounded Android playback segment into a WAV file. */
    private fun synthesizeTtsPlaybackFiles(
        text: String,
        voice: String,
        localeTag: String,
        speed: Float,
        pitch: Float,
        generation: Long,
    ): List<File> {
        synchronized(ttsPlaybackSynthesisLock) {
            val tts = ensureTtsPlaybackEngine()
            configureTtsPlaybackVoice(tts, voice, localeTag, speed, pitch)
            val outputDir = File(runtimeHost.prepareAndroidRuntimePaths().runtimeRoot, "temp/tts-playback")
            if (!outputDir.mkdirs() && !outputDir.isDirectory) {
                throw IllegalStateException("failed to create Android TTS playback directory: ${outputDir.absolutePath}")
            }
            val outputFiles = mutableListOf<File>()
            try {
                for (segment in splitTtsPlaybackText(text)) {
                    if (!isTtsPlaybackGenerationCurrent(generation)) {
                        break
                    }
                    val outputFile = File(outputDir, "${UUID.randomUUID()}.wav")
                    synthesizeTtsFile(
                        tts,
                        segment,
                        outputFile,
                        "android system tts playback",
                        cancelled = { !isTtsPlaybackGenerationCurrent(generation) },
                    )
                    outputFiles += outputFile
                }
            } catch (error: Throwable) {
                deleteTtsPlaybackFiles(outputFiles)
                throw error
            }
            return outputFiles
        }
    }

    /** Creates the shared Android TextToSpeech engine with a bounded wait. */
    private fun ensureTtsPlaybackEngine(): TextToSpeech {
        synchronized(ttsPlaybackLock) {
            ttsPlaybackEngine?.let { return it }
            if (ttsPlaybackReleased) {
                throw IllegalStateException("android system tts playback host is released")
            }
        }
        val tts = createInitializedTtsEngine("android system tts playback")
        synchronized(ttsPlaybackLock) {
            if (ttsPlaybackReleased) {
                tts.shutdown()
                throw IllegalStateException("android system tts playback host is released")
            }
            val current = ttsPlaybackEngine
            if (current != null) {
                tts.shutdown()
                return current
            }
            ttsPlaybackEngine = tts
            return tts
        }
    }

    /** Applies explicit voice, locale, rate, and pitch configuration. */
    private fun configureTtsPlaybackVoice(
        tts: TextToSpeech,
        voice: String,
        localeTag: String,
        speed: Float,
        pitch: Float,
    ) {
        if (!speed.isFinite() || speed <= 0.0f) {
            throw IllegalArgumentException("tts speed must be positive and finite")
        }
        if (!pitch.isFinite() || pitch <= 0.0f) {
            throw IllegalArgumentException("tts pitch must be positive and finite")
        }
        val locale = if (localeTag.isEmpty()) Locale.getDefault() else Locale.forLanguageTag(localeTag)
        val languageResult = tts.setLanguage(locale)
        if (languageResult == TextToSpeech.LANG_MISSING_DATA || languageResult == TextToSpeech.LANG_NOT_SUPPORTED) {
            throw IllegalStateException("android system tts language not supported: ${locale.toLanguageTag()}")
        }
        if (voice.isNotEmpty()) {
            val selectedVoice = tts.voices.firstOrNull { it.name == voice }
                ?: throw IllegalStateException("android system tts voice not found: $voice")
            tts.voice = selectedVoice
        }
        if (tts.setSpeechRate(speed) != TextToSpeech.SUCCESS) {
            throw IllegalStateException("android system tts rejected speech rate: $speed")
        }
        if (tts.setPitch(pitch) != TextToSpeech.SUCCESS) {
            throw IllegalStateException("android system tts rejected pitch: $pitch")
        }
    }

    /** Prepares and chains every synthesized playback file. */
    private fun prepareTtsPlaybackAudio(
        files: List<File>,
        generation: Long,
        deleteOnRelease: Boolean,
    ): List<TtsPlaybackAudio> {
        if (files.isEmpty()) {
            throw IllegalStateException("android system tts playback produced no audio files")
        }
        val audio = mutableListOf<TtsPlaybackAudio>()
        try {
            files.forEach { file ->
                val player = MediaPlayer()
                try {
                    player.setDataSource(file.absolutePath)
                    player.prepare()
                } catch (error: Throwable) {
                    player.release()
                    throw error
                }
                audio += TtsPlaybackAudio(player, file, deleteOnRelease)
            }
            for (index in 0 until audio.lastIndex) {
                audio[index].player.setNextMediaPlayer(audio[index + 1].player)
            }
            audio.forEach { entry ->
                entry.player.setOnCompletionListener { player ->
                    finishTtsPlaybackAudio(generation, player, null)
                }
                entry.player.setOnErrorListener { player, what, extra ->
                    finishTtsPlaybackAudio(generation, player, "android system tts playback error: $what/$extra")
                    true
                }
            }
            return audio
        } catch (error: Throwable) {
            releaseTtsPlaybackAudio(audio)
            if (deleteOnRelease) {
                deleteTtsPlaybackFiles(files.drop(audio.size))
            }
            throw error
        }
    }

    /** Advances or terminates the Android playback queue after a player event. */
    private fun finishTtsPlaybackAudio(generation: Long, player: MediaPlayer, error: String?) {
        val completed: TtsPlaybackAudio?
        val remaining: List<TtsPlaybackAudio>
        synchronized(ttsPlaybackLock) {
            if (generation != ttsPlaybackGeneration || ttsPlaybackAudioQueue.firstOrNull()?.player !== player) {
                return
            }
            completed = ttsPlaybackAudioQueue.removeFirst()
            if (error != null) {
                ttsPlaybackGeneration += 1
                remaining = detachTtsPlaybackAudioLocked()
                ttsPlaybackPaused = false
                ttsPlaybackPath = ""
                ttsPlaybackDetails = error
                ttsPlaybackError = error
            } else {
                remaining = emptyList()
                if (ttsPlaybackAudioQueue.isEmpty()) {
                    ttsPlaybackPaused = false
                    ttsPlaybackPath = ""
                    ttsPlaybackDetails = "android system tts playback completed"
                }
            }
        }
        releaseTtsPlaybackAudio(listOfNotNull(completed) + remaining)
    }

    /** Detaches all prepared Android playback resources while holding the state lock. */
    private fun detachTtsPlaybackAudioLocked(): List<TtsPlaybackAudio> {
        val audio = ttsPlaybackAudioQueue.toList()
        ttsPlaybackAudioQueue.clear()
        return audio
    }

    /** Stops, releases, and deletes detached Android playback resources. */
    private fun releaseTtsPlaybackAudio(audio: List<TtsPlaybackAudio>) {
        audio.forEach { entry ->
            entry.player.setOnCompletionListener(null)
            entry.player.setOnErrorListener(null)
            entry.player.release()
            if (entry.deleteOnRelease) {
                entry.file.delete()
            }
        }
    }

    /** Deletes synthesized Android playback files that have no player. */
    private fun deleteTtsPlaybackFiles(files: List<File>) {
        files.forEach { file ->
            file.delete()
        }
    }

    /** Returns whether one Android playback generation is still current. */
    private fun isTtsPlaybackGenerationCurrent(generation: Long): Boolean {
        synchronized(ttsPlaybackLock) {
            return generation == ttsPlaybackGeneration && ttsPlaybackPreparing && !ttsPlaybackReleased
        }
    }

    /** Registers a mutating command and returns its cross-origin command epoch. */
    private fun registerTtsPlaybackCommand(ownerSequence: Long?): Long? {
        synchronized(ttsPlaybackLock) {
            if (ownerSequence != null) {
                if (ownerSequence <= 0L) {
                    throw IllegalArgumentException("ownerSequence must be positive")
                }
                if (ownerSequence <= ttsPlaybackLastOwnerSequence) {
                    return null
                }
                ttsPlaybackLastOwnerSequence = ownerSequence
            }
            ttsPlaybackCommandEpoch += 1
            return ttsPlaybackCommandEpoch
        }
    }

    /** Checks whether a registered command remains the newest mutating command. */
    private fun isTtsPlaybackCommandCurrentLocked(commandEpoch: Long): Boolean {
        return commandEpoch == ttsPlaybackCommandEpoch
    }

    /** Returns playback state or propagates an asynchronous MediaPlayer failure. */
    private fun ttsPlaybackState(): Map<String, Any> {
        synchronized(ttsPlaybackLock) {
            ttsPlaybackError?.let { throw IllegalStateException(it) }
            return ttsPlaybackStatusLocked("android system tts playback state")
        }
    }

    /** Returns a synchronized Android playback status snapshot. */
    private fun ttsPlaybackStatus(details: String): Map<String, Any> {
        synchronized(ttsPlaybackLock) {
            return ttsPlaybackStatusLocked(details)
        }
    }

    /** Returns an Android playback status while the playback lock is held. */
    private fun ttsPlaybackStatusLocked(details: String): Map<String, Any> {
        val active = ttsPlaybackPreparing || ttsPlaybackAudioQueue.isNotEmpty()
        return mapOf(
            "path" to ttsPlaybackPath,
            "active" to active,
            "paused" to (active && ttsPlaybackPaused),
            "details" to details,
        )
    }

    /** Splits speech without exceeding the Android engine input limit. */
    private fun splitTtsPlaybackText(text: String): List<String> {
        val maxLength = TextToSpeech.getMaxSpeechInputLength()
        val segments = mutableListOf<String>()
        var start = 0
        while (start < text.length) {
            var end = (start + maxLength).coerceAtMost(text.length)
            if (end < text.length && Character.isHighSurrogate(text[end - 1])) {
                end -= 1
            }
            segments += text.substring(start, end)
            start = end
        }
        return segments
    }

    /** Synthesizes one bounded Android system TTS file request. */
    private fun ttsSynthesizeResult(payload: Map<*, *>): Map<String, Any> {
        val text = payload["text"] as? String ?: throw IllegalArgumentException("text is required")
        if (text.isBlank()) {
            throw IllegalArgumentException("text is empty")
        }
        if (text.length > TextToSpeech.getMaxSpeechInputLength()) {
            throw IllegalArgumentException("android system tts text exceeds the engine input limit")
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
        val outputDir = File(runtimeHost.prepareAndroidRuntimePaths().runtimeRoot, "temp/tts")
        if (!outputDir.mkdirs() && !outputDir.isDirectory) {
            throw IllegalStateException("failed to create Android TTS directory: ${outputDir.absolutePath}")
        }
        val outputFile = File(outputDir, "${UUID.randomUUID()}.wav")
        val tts = createInitializedTtsEngine("android system tts")
        try {
            configureTtsPlaybackVoice(tts, voice, localeTag, speed, pitch)
            synthesizeTtsFile(
                tts,
                text,
                outputFile,
                "android system tts",
                cancelled = { false },
            )
        } catch (error: Throwable) {
            outputFile.delete()
            throw error
        } finally {
            tts.shutdown()
        }
        return mapOf(
            "audioPath" to outputFile.absolutePath,
            "details" to "android TextToSpeech synthesis completed",
        )
    }

    /** Creates an Android TextToSpeech engine without allowing an unbounded initialization wait. */
    private fun createInitializedTtsEngine(label: String): TextToSpeech {
        val initLatch = CountDownLatch(1)
        var initStatus = TextToSpeech.ERROR
        val tts = TextToSpeech(activity.applicationContext) { status ->
            initStatus = status
            initLatch.countDown()
        }
        if (!initLatch.await(TTS_ENGINE_INIT_TIMEOUT_MS, TimeUnit.MILLISECONDS)) {
            tts.shutdown()
            throw IllegalStateException("$label initialization timed out")
        }
        if (initStatus != TextToSpeech.SUCCESS) {
            tts.shutdown()
            throw IllegalStateException("$label initialization failed: $initStatus")
        }
        return tts
    }

    /** Synthesizes one utterance and validates its exact completion callback. */
    private fun synthesizeTtsFile(
        tts: TextToSpeech,
        text: String,
        outputFile: File,
        label: String,
        cancelled: () -> Boolean,
    ) {
        val utteranceId = UUID.randomUUID().toString()
        val completionLatch = CountDownLatch(1)
        var synthesisError: String? = null
        tts.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
            override fun onStart(utteranceId: String?) {}

            override fun onDone(completedUtteranceId: String?) {
                if (completedUtteranceId == utteranceId) {
                    completionLatch.countDown()
                }
            }

            override fun onError(failedUtteranceId: String?) {
                if (failedUtteranceId == utteranceId) {
                    synthesisError = "$label synthesis failed"
                    completionLatch.countDown()
                }
            }

            override fun onError(failedUtteranceId: String?, errorCode: Int) {
                if (failedUtteranceId == utteranceId) {
                    synthesisError = "$label synthesis failed: $errorCode"
                    completionLatch.countDown()
                }
            }

            override fun onStop(stoppedUtteranceId: String?, interrupted: Boolean) {
                if (stoppedUtteranceId == utteranceId) {
                    synthesisError = "$label synthesis stopped"
                    completionLatch.countDown()
                }
            }
        })
        val params = Bundle()
        val synthStatus = tts.synthesizeToFile(text, params, outputFile, utteranceId)
        if (synthStatus != TextToSpeech.SUCCESS) {
            outputFile.delete()
            throw IllegalStateException("$label synthesizeToFile failed")
        }
        val deadlineNanos = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(TTS_SYNTHESIS_TIMEOUT_MS)
        while (true) {
            if (cancelled()) {
                tts.stop()
                outputFile.delete()
                throw IllegalStateException("$label synthesis cancelled")
            }
            val remainingNanos = deadlineNanos - System.nanoTime()
            if (remainingNanos <= 0L) {
                tts.stop()
                outputFile.delete()
                throw IllegalStateException("$label synthesis timed out")
            }
            val waitNanos = remainingNanos.coerceAtMost(TimeUnit.MILLISECONDS.toNanos(100L))
            if (completionLatch.await(waitNanos, TimeUnit.NANOSECONDS)) {
                break
            }
        }
        synthesisError?.let { throw IllegalStateException(it) }
        if (!outputFile.isFile || outputFile.length() == 0L) {
            outputFile.delete()
            throw IllegalStateException("$label output missing: ${outputFile.absolutePath}")
        }
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
        val screenshotDir = File(runtimeHost.prepareAndroidRuntimePaths().runtimeRoot, "temp/clean_on_exit")
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
