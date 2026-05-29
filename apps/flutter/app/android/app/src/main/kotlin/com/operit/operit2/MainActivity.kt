package com.operit.operit2

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

class MainActivity : FlutterActivity() {
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
                    else -> result.notImplemented()
                }
            }
    }

    override fun cleanUpFlutterEngine(flutterEngine: FlutterEngine) {
        if (runtimeHandle != 0L) {
            OperitRuntimeNative.destroy(runtimeHandle)
            runtimeHandle = 0
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
        Thread {
            try {
                val response = block()
                runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                runOnUiThread {
                    result.error("RUNTIME_BRIDGE_ERROR", error.message, null)
                }
            }
        }.start()
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

    private fun ensureRuntimeHandle(): Long {
        if (runtimeHandle != 0L) {
            return runtimeHandle
        }
        val root = File(applicationContext.filesDir, "operit-runtime")
        root.mkdirs()
        runtimeHandle = OperitRuntimeNative.create(root.absolutePath)
        if (runtimeHandle == 0L) {
            throw IllegalStateException(OperitRuntimeNative.createError())
        }
        return runtimeHandle
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
}
