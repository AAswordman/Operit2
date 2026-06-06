package com.ai.assistance.operit2

import android.os.Build
import android.os.Bundle
import android.util.Log
import android.view.Display
import android.view.View
import android.graphics.Color
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.embedding.android.FlutterActivity
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.nio.charset.StandardCharsets
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

class MainActivity : FlutterActivity() {
    private val runtimeLock = Any()
    private val runtimeExecutor: ExecutorService = Executors.newCachedThreadPool()
    private var runtimeHandle: Long = 0

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
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "operit/runtime")
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "call" -> callRuntime(call, result, OperitRuntimeNative::call)
                    "watchSnapshot" -> callRuntime(call, result, OperitRuntimeNative::watchSnapshot)
                    "watchStream" -> callRuntime(call, result, OperitRuntimeNative::watchStream)
                    "pollWatchStream" -> pollWatchStream(call, result)
                    "closeWatchStream" -> closeWatchStream(call, result)
                    "hostDescriptor" -> runRuntime(result) {
                        OperitRuntimeNative.hostDescriptor(ensureRuntimeHandle())
                    }
                    "currentPermissionRequest" -> runRuntime(result) {
                        OperitRuntimeNative.currentPermissionRequest(ensureRuntimeHandle())
                    }
                    "handlePermissionResult" -> handlePermissionResult(call, result)
                    "nextBrowserAutomationRequest" -> runRuntime(result) {
                        OperitRuntimeNative.nextBrowserAutomationRequest(ensureRuntimeHandle())
                    }
                    "handleBrowserAutomationResult" -> handleBrowserAutomationResult(call, result)
                    "nextWebVisitRequest" -> runRuntime(result) {
                        OperitRuntimeNative.nextWebVisitRequest(ensureRuntimeHandle())
                    }
                    "handleWebVisitResult" -> handleWebVisitResult(call, result)
                    "androidRuntimePaths" -> androidRuntimePaths(result)
                    "listTerminalSessions" -> runRuntime(result) {
                        OperitRuntimeNative.listTerminalSessions(ensureRuntimeHandle())
                    }
                    "startTerminalPty" -> startTerminalPty(call, result)
                    "readTerminalPty" -> terminalPtySessionCall(call, result, OperitRuntimeNative::readTerminalPty)
                    "pollTerminalPtyExit" -> terminalPtySessionCall(call, result, OperitRuntimeNative::pollTerminalPtyExit)
                    "closeTerminalPty" -> terminalPtySessionCall(call, result, OperitRuntimeNative::closeTerminalPty)
                    "getTerminalSessionScreen" -> terminalPtySessionCall(call, result, OperitRuntimeNative::getTerminalSessionScreen)
                    "inputTerminalSession" -> inputTerminalSession(call, result)
                    "terminalDebugInfo" -> terminalDebugInfo(call, result)
                    "writeTerminalPty" -> writeTerminalPty(call, result)
                    "resizeTerminalPty" -> resizeTerminalPty(call, result)
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

    private fun handleBrowserAutomationResult(call: MethodCall, result: MethodChannel.Result) {
        val browserResult = call.arguments as? String
        if (browserResult == null) {
            result.error("INVALID_ARGS", "handleBrowserAutomationResult expects a JSON string", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.handleBrowserAutomationResult(ensureRuntimeHandle(), browserResult)
        }
    }

    private fun handleWebVisitResult(call: MethodCall, result: MethodChannel.Result) {
        val webVisitResult = call.arguments as? String
        if (webVisitResult == null) {
            result.error("INVALID_ARGS", "handleWebVisitResult expects a JSON string", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.handleWebVisitResult(ensureRuntimeHandle(), webVisitResult)
        }
    }

    private fun ensureRuntimeHandle(): Long {
        synchronized(runtimeLock) {
            if (runtimeHandle != 0L) {
                return runtimeHandle
            }
            val root = prepareAndroidRuntimePaths().storageRoot
            runtimeHandle = OperitRuntimeNative.create(root.absolutePath)
            if (runtimeHandle == 0L) {
                throw IllegalStateException(OperitRuntimeNative.createError())
            }
            return runtimeHandle
        }
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
        val root = File(applicationContext.filesDir, "operit-runtime")
        root.mkdirs()
        return AndroidRuntimeAssets.prepare(applicationContext, root)
    }

    private fun startTerminalPty(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val sessionName = args?.get("sessionName") as? String
        val workingDirectory = args?.get("workingDirectory") as? String
        val rows = args?.get("rows") as? Int
        val columns = args?.get("columns") as? Int
        if (sessionName == null || workingDirectory == null || rows == null || columns == null) {
            result.error("INVALID_ARGS", "startTerminalPty expects sessionName, workingDirectory, rows, columns", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.startTerminalPty(
                ensureRuntimeHandle(),
                sessionName,
                workingDirectory,
                rows,
                columns,
            )
        }
    }

    private fun terminalPtySessionCall(
        call: MethodCall,
        result: MethodChannel.Result,
        nativeCall: (Long, String) -> String,
    ) {
        val sessionId = call.arguments as? String
        if (sessionId == null) {
            result.error("INVALID_ARGS", "${call.method} expects a session id", null)
            return
        }
        runRuntime(result) {
            nativeCall(ensureRuntimeHandle(), sessionId)
        }
    }

    private fun writeTerminalPty(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val sessionId = args?.get("sessionId") as? String
        val data = args?.get("data") as? ByteArray
        if (sessionId == null || data == null) {
            result.error("INVALID_ARGS", "writeTerminalPty expects sessionId and data", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.writeTerminalPty(ensureRuntimeHandle(), sessionId, data)
        }
    }

    private fun resizeTerminalPty(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val sessionId = args?.get("sessionId") as? String
        val rows = args?.get("rows") as? Int
        val columns = args?.get("columns") as? Int
        if (sessionId == null || rows == null || columns == null) {
            result.error("INVALID_ARGS", "resizeTerminalPty expects sessionId, rows, columns", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.resizeTerminalPty(ensureRuntimeHandle(), sessionId, rows, columns)
        }
    }

    private fun inputTerminalSession(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val sessionId = args?.get("sessionId") as? String
        val input = args?.get("input") as? String
        if (sessionId == null || input == null) {
            result.error("INVALID_ARGS", "inputTerminalSession expects sessionId and input", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.inputTerminalSession(ensureRuntimeHandle(), sessionId, input)
        }
    }

    private fun terminalDebugInfo(call: MethodCall, result: MethodChannel.Result) {
        val workingDirectory = call.arguments as? String
        if (workingDirectory == null) {
            result.error("INVALID_ARGS", "terminalDebugInfo expects a working directory", null)
            return
        }
        runRuntime(result) {
            OperitRuntimeNative.terminalDebugInfo(ensureRuntimeHandle(), workingDirectory)
        }
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
        Log.i(
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

object OperitRuntimeNative {
    init {
        System.loadLibrary("operit_flutter_bridge")
    }

    @JvmStatic external fun create(storageRoot: String): Long
    @JvmStatic external fun createError(): String
    @JvmStatic external fun destroy(handle: Long)
    @JvmStatic external fun call(handle: Long, request: ByteArray): String
    @JvmStatic external fun watchSnapshot(handle: Long, request: ByteArray): String
    @JvmStatic external fun watchStream(handle: Long, request: ByteArray): String
    @JvmStatic external fun pollWatchStream(handle: Long, subscriptionId: String): String
    @JvmStatic external fun closeWatchStream(handle: Long, subscriptionId: String): String
    @JvmStatic external fun hostDescriptor(handle: Long): String
    @JvmStatic external fun currentPermissionRequest(handle: Long): String
    @JvmStatic external fun handlePermissionResult(handle: Long, permissionResult: String): String
    @JvmStatic external fun nextBrowserAutomationRequest(handle: Long): String
    @JvmStatic external fun handleBrowserAutomationResult(handle: Long, resultJson: String): String
    @JvmStatic external fun nextWebVisitRequest(handle: Long): String
    @JvmStatic external fun handleWebVisitResult(handle: Long, resultJson: String): String
    @JvmStatic external fun startTerminalPty(handle: Long, sessionName: String, workingDirectory: String, rows: Int, columns: Int): String
    @JvmStatic external fun listTerminalSessions(handle: Long): String
    @JvmStatic external fun readTerminalPty(handle: Long, sessionId: String): String
    @JvmStatic external fun writeTerminalPty(handle: Long, sessionId: String, data: ByteArray): String
    @JvmStatic external fun resizeTerminalPty(handle: Long, sessionId: String, rows: Int, columns: Int): String
    @JvmStatic external fun pollTerminalPtyExit(handle: Long, sessionId: String): String
    @JvmStatic external fun closeTerminalPty(handle: Long, sessionId: String): String
    @JvmStatic external fun getTerminalSessionScreen(handle: Long, sessionId: String): String
    @JvmStatic external fun inputTerminalSession(handle: Long, sessionId: String, input: String): String
    @JvmStatic external fun terminalDebugInfo(handle: Long, workingDirectory: String): String
}
