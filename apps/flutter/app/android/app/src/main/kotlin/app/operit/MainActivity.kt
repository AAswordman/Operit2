package app.operit

import android.os.Build
import android.os.Bundle
import android.view.Display
import android.view.View
import android.graphics.Color
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.embedding.android.FlutterActivity
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.nio.charset.StandardCharsets
import java.util.concurrent.CountDownLatch
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicInteger

class MainActivity : FlutterActivity() {
    private val runtimeLock = Any()
    private val runtimeThreadIndex = AtomicInteger(0)
    private val runtimeExecutor: ExecutorService =
        Executors.newFixedThreadPool(8) { runnable ->
            Thread(runnable, "operit-runtime-${runtimeThreadIndex.incrementAndGet()}")
        }
    private var runtimeHandle: Long = 0
    private lateinit var runtimeChannel: MethodChannel

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        configureSystemBars()
        requestHighestRefreshRate()
    }

    override fun onResume() {
        super.onResume()
        configureSystemBars()
        requestHighestRefreshRate()
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        runtimeChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "operit/runtime")
        runtimeChannel
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "call" -> callRuntime(call, result, OperitRuntimeNative::call)
                    "watchSnapshot" -> callRuntime(call, result, OperitRuntimeNative::watchSnapshot)
                    "watchStream" -> callRuntime(call, result, OperitRuntimeNative::watchStream)
                    "pollWatchStream" -> pollWatchStream(call, result)
                    "pollWatchStreams" -> pollWatchStreams(call, result)
                    "closeWatchStream" -> closeWatchStream(call, result)
                    "hostDescriptor" -> runRuntime(result) {
                        OperitRuntimeNative.hostDescriptor(ensureRuntimeHandle())
                    }
                    "startWebAccessServer" -> startWebAccessServer(call, result)
                    "stopWebAccessServer" -> runRuntime(result) {
                        OperitRuntimeNative.stopWebAccessServer(ensureRuntimeHandle())
                    }
                    "remotePairStart" -> remotePairStart(call, result)
                    "remotePairFinish" -> remotePairFinish(call, result)
                    "currentPermissionRequest" -> runRuntime(result) {
                        OperitRuntimeNative.currentPermissionRequest(ensureRuntimeHandle())
                    }
                    "handlePermissionResult" -> handlePermissionResult(call, result)
                    "androidRuntimePaths" -> androidRuntimePaths(result)
                    else -> result.notImplemented()
                }
            }
    }

    override fun cleanUpFlutterEngine(flutterEngine: FlutterEngine) {
        runtimeExecutor.shutdownNow()
        synchronized(runtimeLock) {
            if (runtimeHandle != 0L) {
                OperitRuntimeNative.destroy(runtimeHandle)
                runtimeHandle = 0
            }
        }
        super.cleanUpFlutterEngine(flutterEngine)
    }

    private fun callRuntime(
        call: MethodCall,
        result: MethodChannel.Result,
        nativeCall: (Long, ByteArray) -> String,
    ) {
        val request = call.arguments as? String
        if (request == null) {
            result.error("INVALID_ARGS", "${call.method} expects a JSON string", null)
            return
        }
        runRuntime(result) {
            nativeCall(ensureRuntimeHandle(), request.toByteArray(StandardCharsets.UTF_8))
        }
    }

    private fun runRuntime(result: MethodChannel.Result, block: () -> String) {
        runtimeExecutor.execute {
            try {
                val response = block()
                runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                runOnUiThread {
                    result.error("RUNTIME_BRIDGE_ERROR", error.message, null)
                }
            }
        }
    }

    private fun pollWatchStream(call: MethodCall, result: MethodChannel.Result) {
        val subscriptionId = call.arguments as? String
        if (subscriptionId == null) {
            result.error("INVALID_ARGS", "pollWatchStream expects a subscription id", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.pollWatchStream(ensureRuntimeHandle(), subscriptionId)
        }
    }

    private fun pollWatchStreams(call: MethodCall, result: MethodChannel.Result) {
        val subscriptionIdsJson = call.arguments as? String
        if (subscriptionIdsJson == null) {
            result.error("INVALID_ARGS", "pollWatchStreams expects a JSON string array", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.pollWatchStreams(ensureRuntimeHandle(), subscriptionIdsJson)
        }
    }

    private fun closeWatchStream(call: MethodCall, result: MethodChannel.Result) {
        val subscriptionId = call.arguments as? String
        if (subscriptionId == null) {
            result.error("INVALID_ARGS", "closeWatchStream expects a subscription id", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.closeWatchStream(ensureRuntimeHandle(), subscriptionId)
        }
    }

    private fun handlePermissionResult(call: MethodCall, result: MethodChannel.Result) {
        val permissionResult = call.arguments as? String
        if (permissionResult == null) {
            result.error("INVALID_ARGS", "handlePermissionResult expects a result string", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.handlePermissionResult(ensureRuntimeHandle(), permissionResult)
        }
    }

    private fun startWebAccessServer(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val bindAddress = args?.get("bindAddress") as? String
        val token = args?.get("token") as? String
        val shutdownToken = args?.get("shutdownToken") as? String
        val webRoot = args?.get("webRoot") as? String
        val acceptedSessions = args?.get("acceptedSessions") as? String
        val acceptedSessionStorePath = args?.get("acceptedSessionStorePath") as? String
        val pairingCodePath = args?.get("pairingCodePath") as? String
        val deviceInfoJson = args?.get("deviceInfo") as? String
        if (
            bindAddress == null ||
                token == null ||
                shutdownToken == null ||
                webRoot == null ||
                acceptedSessions == null ||
                acceptedSessionStorePath == null ||
                pairingCodePath == null ||
                deviceInfoJson == null
        ) {
            result.error(
                "INVALID_ARGS",
                "startWebAccessServer expects bindAddress, token, shutdownToken, webRoot, acceptedSessions, acceptedSessionStorePath, pairingCodePath and deviceInfo",
                null,
            )
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.startWebAccessServer(
                ensureRuntimeHandle(),
                bindAddress,
                token,
                shutdownToken,
                webRoot,
                acceptedSessions,
                acceptedSessionStorePath,
                pairingCodePath,
                deviceInfoJson,
            )
        }
    }

    private fun remotePairStart(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val baseUrl = args?.get("baseUrl") as? String
        val token = args?.get("token") as? String
        val clientDeviceInfoJson = args?.get("clientDeviceInfo") as? String
        if (baseUrl == null || token == null || clientDeviceInfoJson == null) {
            result.error("INVALID_ARGS", "remotePairStart expects baseUrl, token and clientDeviceInfo", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.remotePairStart(ensureRuntimeHandle(), baseUrl, token, clientDeviceInfoJson)
        }
    }

    private fun remotePairFinish(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val pairingId = args?.get("pairingId") as? String
        val pairingCode = args?.get("pairingCode") as? String
        if (pairingId == null || pairingCode == null) {
            result.error("INVALID_ARGS", "remotePairFinish expects pairingId and pairingCode", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.remotePairFinish(ensureRuntimeHandle(), pairingId, pairingCode)
        }
    }

    private fun ensureRuntimeHandle(): Long {
        synchronized(runtimeLock) {
            if (runtimeHandle != 0L) {
                return runtimeHandle
            }
            val root = prepareAndroidRuntimePaths().storageRoot
            runtimeHandle = OperitRuntimeNative.create(root.absolutePath, this)
            if (runtimeHandle == 0L) {
                throw IllegalStateException(OperitRuntimeNative.createError())
            }
            return runtimeHandle
        }
    }

    fun handleRuntimeHostRequest(methodName: String, payloadJson: String): String {
        val latch = CountDownLatch(1)
        var response: String? = null
        var error: Throwable? = null
        runOnUiThread {
            runtimeChannel.invokeMethod(
                methodName,
                payloadJson,
                object : MethodChannel.Result {
                    override fun success(result: Any?) {
                        if (result is String) {
                            response = result
                        } else {
                            error = IllegalStateException("runtime host handler returned a non-string value")
                        }
                        latch.countDown()
                    }

                    override fun error(errorCode: String, errorMessage: String?, errorDetails: Any?) {
                        error = IllegalStateException("$errorCode: ${errorMessage.orEmpty()}")
                        latch.countDown()
                    }

                    override fun notImplemented() {
                        error = IllegalStateException("runtime host method is not implemented: $methodName")
                        latch.countDown()
                    }
                },
            )
        }
        latch.await()
        error?.let { throw it }
        return response ?: throw IllegalStateException("runtime host handler returned empty response")
    }

    private fun androidRuntimePaths(result: MethodChannel.Result) {
        Thread {
            try {
                val paths = prepareAndroidRuntimePaths()
                val response = mapOf(
                    "abi" to paths.abi,
                    "runtimeDir" to paths.runtimeDir.absolutePath,
                    "rootfsDir" to paths.rootfsDir.absolutePath,
                    "busybox" to paths.busybox.absolutePath,
                    "bash" to paths.bash.absolutePath,
                    "proot" to paths.proot.absolutePath,
                    "loader" to paths.loader.absolutePath,
                    "nativeLibraryDir" to paths.nativeLibraryDir.absolutePath,
                    "storageRoot" to paths.storageRoot.absolutePath,
                    "internalRoot" to paths.internalRoot.absolutePath,
                    "tmpDir" to paths.tmpDir.absolutePath,
                )
                runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                runOnUiThread {
                    result.error("RUNTIME_BRIDGE_ERROR", error.message, null)
                }
            }
        }.start()
    }

    private fun prepareAndroidRuntimePaths(): AndroidRuntimePaths {
        val root = applicationContext.filesDir
        root.mkdirs()
        return AndroidRuntimeAssets.prepare(applicationContext, root)
    }

    private fun requestHighestRefreshRate() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) {
            return
        }
        val display = currentDisplay() ?: return
        val currentMode = display.mode ?: return
        val preferredMode =
            display.supportedModes
                .filter {
                    it.physicalWidth == currentMode.physicalWidth &&
                        it.physicalHeight == currentMode.physicalHeight
                }
                .maxByOrNull { it.refreshRate }
                ?: return

        if (preferredMode.modeId == currentMode.modeId) {
            return
        }

        val attributes = window.attributes
        if (attributes.preferredDisplayModeId == preferredMode.modeId) {
            return
        }
        attributes.preferredDisplayModeId = preferredMode.modeId
        window.attributes = attributes
        AndroidClientLogger.i(
            applicationContext,
            "OperitMainActivity",
            "Requested display mode ${preferredMode.physicalWidth}x${preferredMode.physicalHeight}@${preferredMode.refreshRate}Hz",
        )
    }

    @Suppress("DEPRECATION")
    private fun currentDisplay(): Display? {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            display
        } else {
            windowManager.defaultDisplay
        }
    }

    private fun configureSystemBars() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            window.statusBarColor = Color.TRANSPARENT
            window.navigationBarColor = Color.TRANSPARENT
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            window.isStatusBarContrastEnforced = false
            window.isNavigationBarContrastEnforced = false
        }

        val flags =
            View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
                View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
                } else {
                    0
                }
        window.decorView.systemUiVisibility = flags
    }
}

object AndroidClientLogger {
    fun i(context: android.content.Context, tag: String, message: String) {
        write(context, "I", tag, message)
    }

    @Synchronized
    private fun write(context: android.content.Context, level: String, tag: String, message: String) {
        val logsDir = File(context.filesDir, "client/logs")
        logsDir.mkdirs()
        val logFile = File(logsDir, "client.log")
        logFile.appendText("${System.currentTimeMillis()} $level/$tag: $message\n", Charsets.UTF_8)
    }
}

object OperitRuntimeNative {
    init {
        System.loadLibrary("operit_flutter_bridge")
    }

    @JvmStatic external fun create(storageRoot: String, host: MainActivity): Long
    @JvmStatic external fun createError(): String
    @JvmStatic external fun destroy(handle: Long)
    @JvmStatic external fun call(handle: Long, request: ByteArray): String
    @JvmStatic external fun watchSnapshot(handle: Long, request: ByteArray): String
    @JvmStatic external fun watchStream(handle: Long, request: ByteArray): String
    @JvmStatic external fun pollWatchStream(handle: Long, subscriptionId: String): String
    @JvmStatic external fun pollWatchStreams(handle: Long, subscriptionIdsJson: String): String
    @JvmStatic external fun closeWatchStream(handle: Long, subscriptionId: String): String
    @JvmStatic external fun hostDescriptor(handle: Long): String
    @JvmStatic
    external fun startWebAccessServer(
        handle: Long,
        bindAddress: String,
        token: String,
        shutdownToken: String,
        webRoot: String,
        acceptedSessions: String,
        acceptedSessionStorePath: String,
        pairingCodePath: String,
        deviceInfoJson: String,
    ): String
    @JvmStatic external fun stopWebAccessServer(handle: Long): String
    @JvmStatic external fun remotePairStart(handle: Long, baseUrl: String, token: String, clientDeviceInfoJson: String): String
    @JvmStatic external fun remotePairFinish(handle: Long, pairingId: String, pairingCode: String): String
    @JvmStatic external fun currentPermissionRequest(handle: Long): String
    @JvmStatic external fun handlePermissionResult(handle: Long, permissionResult: String): String
}
