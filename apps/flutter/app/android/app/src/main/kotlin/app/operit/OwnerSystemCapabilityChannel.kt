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
    }

    private var cachedMediaProjectionCaptureManager: MediaProjectionCaptureManager? = null
    private var cachedMediaProjection: MediaProjection? = null
    private val ttsPlaybackLock = Any()
    private val ttsPlaybackQueue = ArrayDeque<TtsPlaybackUtterance>()
    private var ttsPlaybackEngine: TextToSpeech? = null
    private var ttsPlaybackCurrentUtteranceId: String? = null
    private var ttsPlaybackCurrentText: String = ""
    private var ttsPlaybackRangeStart: Int = 0
    private var ttsPlaybackPausedSegments: List<String> = emptyList()
    private var ttsPlaybackPaused: Boolean = false
    private var ttsPlaybackPath: String = ""
    private var ttsPlaybackIndex: Long = 0
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

    private data class TtsPlaybackUtterance(
        val utteranceId: String,
        val text: String,
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
        ttsPlaybackEngine?.shutdown()
        ttsPlaybackEngine = null
        synchronized(ttsPlaybackLock) {
            ttsPlaybackQueue.clear()
            ttsPlaybackPausedSegments = emptyList()
            ttsPlaybackPaused = false
            ttsPlaybackCurrentUtteranceId = null
            ttsPlaybackCurrentText = ""
            ttsPlaybackRangeStart = 0
        }
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

    private fun ownerTtsPlayback(call: MethodCall, result: MethodChannel.Result) {
        val payload = call.arguments as? Map<*, *>
        if (payload == null) {
            result.error("INVALID_ARGS", "ownerTtsPlayback expects a map", null)
            return
        }
        runtimeHost.runBackground {
            try {
                val response = ttsPlaybackResult(payload)
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

    private fun ttsPlayback(payloadJson: String): String {
        val request = JSONObject(payloadJson)
        val payload =
            mapOf(
                "command" to request.getString("command"),
                "text" to request.optString("text", ""),
                "voice" to request.optString("voice", ""),
                "locale" to request.optString("locale", ""),
                "speed" to request.optDouble("speed", 1.0),
                "pitch" to request.optDouble("pitch", 1.0),
                "interrupt" to request.optBoolean("interrupt", false),
            )
        return JSONObject(ttsPlaybackResult(payload)).toString()
    }

    private fun ttsPlaybackResult(payload: Map<*, *>): Map<String, Any> {
        return when (val command = payload["command"] as? String ?: throw IllegalArgumentException("command is required")) {
            "speak" -> ttsPlaybackSpeak(payload)
            "pause" -> ttsPlaybackPause()
            "resume" -> ttsPlaybackResume()
            "stop" -> ttsPlaybackStop()
            "state" -> ttsPlaybackStatus("android system tts playback state")
            else -> throw IllegalArgumentException("unsupported tts playback command: $command")
        }
    }

    private fun ttsPlaybackSpeak(payload: Map<*, *>): Map<String, Any> {
        val text = payload["text"] as? String ?: throw IllegalArgumentException("text is required")
        if (text.isBlank()) {
            throw IllegalArgumentException("text is empty")
        }
        val voice = (payload["voice"] as? String).orEmpty().trim()
        val localeTag = (payload["locale"] as? String).orEmpty().trim()
        val speed = (payload["speed"] as? Number)?.toFloat() ?: throw IllegalArgumentException("speed is required")
        val pitch = (payload["pitch"] as? Number)?.toFloat() ?: throw IllegalArgumentException("pitch is required")
        val interrupt = payload["interrupt"] as? Boolean ?: throw IllegalArgumentException("interrupt is required")
        val tts = ensureTtsPlaybackEngine()
        configureTtsPlaybackVoice(tts, voice, localeTag, speed, pitch)
        val utterance = TtsPlaybackUtterance(UUID.randomUUID().toString(), text)
        synchronized(ttsPlaybackLock) {
            if (interrupt) {
                ttsPlaybackQueue.clear()
                ttsPlaybackPausedSegments = emptyList()
                ttsPlaybackPaused = false
                ttsPlaybackCurrentUtteranceId = null
                ttsPlaybackCurrentText = ""
                ttsPlaybackRangeStart = 0
                tts.stop()
            }
            if (ttsPlaybackPaused && !interrupt) {
                ttsPlaybackPausedSegments = ttsPlaybackPausedSegments + text
                return ttsPlaybackStatusLocked("android system tts playback buffered while paused")
            }
            ttsPlaybackQueue.addLast(utterance)
            ttsPlaybackPath = "android-tts://${++ttsPlaybackIndex}"
        }
        val queueMode = if (interrupt) TextToSpeech.QUEUE_FLUSH else TextToSpeech.QUEUE_ADD
        val speakStatus = tts.speak(utterance.text, queueMode, null, utterance.utteranceId)
        if (speakStatus != TextToSpeech.SUCCESS) {
            synchronized(ttsPlaybackLock) {
                removeTtsPlaybackUtteranceLocked(utterance.utteranceId)
            }
            throw IllegalStateException("android system tts speak failed")
        }
        return ttsPlaybackStatus("android system tts playback started")
    }

    private fun ttsPlaybackPause(): Map<String, Any> {
        val tts = ttsPlaybackEngine ?: return ttsPlaybackStatus("android system tts playback is not initialized")
        synchronized(ttsPlaybackLock) {
            val segments = buildTtsPlaybackPausedSegmentsLocked()
            if (segments.isEmpty()) {
                return ttsPlaybackStatusLocked("android system tts playback is not active")
            }
            ttsPlaybackPausedSegments = segments
            ttsPlaybackPaused = true
            ttsPlaybackQueue.clear()
            ttsPlaybackCurrentUtteranceId = null
            ttsPlaybackCurrentText = ""
            ttsPlaybackRangeStart = 0
        }
        tts.stop()
        return ttsPlaybackStatus("android system tts playback paused")
    }

    private fun ttsPlaybackResume(): Map<String, Any> {
        val tts = ttsPlaybackEngine ?: throw IllegalStateException("android system tts playback is not initialized")
        val segments =
            synchronized(ttsPlaybackLock) {
                if (ttsPlaybackPausedSegments.isEmpty()) {
                    return ttsPlaybackStatusLocked("android system tts playback is not paused")
                }
                val captured = ttsPlaybackPausedSegments
                ttsPlaybackPausedSegments = emptyList()
                ttsPlaybackPaused = false
                captured
            }
        segments.forEachIndexed { index, segment ->
            val utterance = TtsPlaybackUtterance(UUID.randomUUID().toString(), segment)
            synchronized(ttsPlaybackLock) {
                ttsPlaybackQueue.addLast(utterance)
                if (ttsPlaybackPath.isEmpty()) {
                    ttsPlaybackPath = "android-tts://${++ttsPlaybackIndex}"
                }
            }
            val queueMode = if (index == 0) TextToSpeech.QUEUE_FLUSH else TextToSpeech.QUEUE_ADD
            val speakStatus = tts.speak(utterance.text, queueMode, null, utterance.utteranceId)
            if (speakStatus != TextToSpeech.SUCCESS) {
                synchronized(ttsPlaybackLock) {
                    removeTtsPlaybackUtteranceLocked(utterance.utteranceId)
                }
                throw IllegalStateException("android system tts resume failed")
            }
        }
        return ttsPlaybackStatus("android system tts playback resumed")
    }

    private fun ttsPlaybackStop(): Map<String, Any> {
        ttsPlaybackEngine?.stop()
        synchronized(ttsPlaybackLock) {
            ttsPlaybackQueue.clear()
            ttsPlaybackPausedSegments = emptyList()
            ttsPlaybackPaused = false
            ttsPlaybackCurrentUtteranceId = null
            ttsPlaybackCurrentText = ""
            ttsPlaybackRangeStart = 0
        }
        return ttsPlaybackStatus("android system tts playback stopped")
    }

    private fun ensureTtsPlaybackEngine(): TextToSpeech {
        ttsPlaybackEngine?.let { return it }
        val initLatch = CountDownLatch(1)
        var initStatus = TextToSpeech.ERROR
        val tts = TextToSpeech(activity.applicationContext) { status ->
            initStatus = status
            initLatch.countDown()
        }
        initLatch.await()
        if (initStatus != TextToSpeech.SUCCESS) {
            tts.shutdown()
            throw IllegalStateException("android system tts playback init failed")
        }
        tts.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
            override fun onStart(utteranceId: String?) {
                if (utteranceId == null) {
                    return
                }
                synchronized(ttsPlaybackLock) {
                    val entry = ttsPlaybackQueue.firstOrNull { it.utteranceId == utteranceId }
                    if (entry != null) {
                        ttsPlaybackCurrentUtteranceId = utteranceId
                        ttsPlaybackCurrentText = entry.text
                        ttsPlaybackRangeStart = 0
                    }
                }
            }

            override fun onDone(utteranceId: String?) {
                finishTtsPlaybackUtterance(utteranceId)
            }

            override fun onError(utteranceId: String?) {
                finishTtsPlaybackUtterance(utteranceId)
            }

            override fun onError(utteranceId: String?, errorCode: Int) {
                finishTtsPlaybackUtterance(utteranceId)
            }

            override fun onRangeStart(utteranceId: String?, start: Int, end: Int, frame: Int) {
                synchronized(ttsPlaybackLock) {
                    if (utteranceId == ttsPlaybackCurrentUtteranceId) {
                        ttsPlaybackRangeStart = start.coerceIn(0, ttsPlaybackCurrentText.length)
                    }
                }
            }
        })
        ttsPlaybackEngine = tts
        return tts
    }

    private fun configureTtsPlaybackVoice(
        tts: TextToSpeech,
        voice: String,
        localeTag: String,
        speed: Float,
        pitch: Float,
    ) {
        if (localeTag.isNotEmpty()) {
            val locale = java.util.Locale.forLanguageTag(localeTag)
            val languageResult = tts.setLanguage(locale)
            if (languageResult == TextToSpeech.LANG_MISSING_DATA || languageResult == TextToSpeech.LANG_NOT_SUPPORTED) {
                throw IllegalStateException("android system tts language not supported: $localeTag")
            }
        }
        if (voice.isNotEmpty()) {
            val selectedVoice = tts.voices.firstOrNull { it.name == voice }
            if (selectedVoice == null) {
                throw IllegalStateException("android system tts voice not found: $voice")
            }
            tts.voice = selectedVoice
        }
        tts.setSpeechRate(speed)
        tts.setPitch(pitch)
    }

    private fun finishTtsPlaybackUtterance(utteranceId: String?) {
        if (utteranceId == null) {
            return
        }
        synchronized(ttsPlaybackLock) {
            removeTtsPlaybackUtteranceLocked(utteranceId)
            if (ttsPlaybackCurrentUtteranceId == utteranceId) {
                ttsPlaybackCurrentUtteranceId = null
                ttsPlaybackCurrentText = ""
                ttsPlaybackRangeStart = 0
            }
        }
    }

    private fun buildTtsPlaybackPausedSegmentsLocked(): List<String> {
        if (ttsPlaybackQueue.isEmpty()) {
            return emptyList()
        }
        val currentId = ttsPlaybackCurrentUtteranceId
        if (currentId == null) {
            return ttsPlaybackQueue.map { it.text }.filter { it.isNotBlank() }
        }
        val result = mutableListOf<String>()
        var foundCurrent = false
        ttsPlaybackQueue.forEach { entry ->
            if (!foundCurrent && entry.utteranceId == currentId) {
                foundCurrent = true
                val safeStart = ttsPlaybackRangeStart.coerceIn(0, entry.text.length)
                val remaining = entry.text.substring(safeStart).trimStart()
                if (remaining.isNotBlank()) {
                    result += remaining
                }
            } else if (foundCurrent && entry.text.isNotBlank()) {
                result += entry.text
            }
        }
        if (!foundCurrent) {
            return ttsPlaybackQueue.map { it.text }.filter { it.isNotBlank() }
        }
        return result
    }

    private fun removeTtsPlaybackUtteranceLocked(utteranceId: String) {
        val iterator = ttsPlaybackQueue.iterator()
        while (iterator.hasNext()) {
            if (iterator.next().utteranceId == utteranceId) {
                iterator.remove()
                return
            }
        }
    }

    private fun ttsPlaybackStatus(details: String): Map<String, Any> {
        synchronized(ttsPlaybackLock) {
            return ttsPlaybackStatusLocked(details)
        }
    }

    private fun ttsPlaybackStatusLocked(details: String): Map<String, Any> {
        val active = ttsPlaybackQueue.isNotEmpty() || ttsPlaybackPaused
        return mapOf(
            "path" to ttsPlaybackPath,
            "active" to active,
            "paused" to ttsPlaybackPaused,
            "details" to details,
        )
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
        val outputDir = File(runtimeHost.prepareAndroidRuntimePaths().runtimeRoot, "temp/tts")
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
