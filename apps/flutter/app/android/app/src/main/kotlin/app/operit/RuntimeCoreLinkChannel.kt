package app.operit

import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import org.json.JSONObject
import java.nio.charset.StandardCharsets

class RuntimeCoreLinkChannel(
    private val activity: MainActivity,
    private val runtimeHost: AndroidRuntimeHost,
) {
    private val watchPumpLock = Any()
    @Volatile
    private var watchPumpRunning = false
    @Volatile
    private var runtimeChannel: MethodChannel? = null

    fun attach(channel: MethodChannel) {
        runtimeChannel = channel
    }

    fun clear() {
        runtimeChannel = null
    }

    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        when (call.method) {
            "call" -> callRuntime(call, result, OperitRuntimeNative::call)
            "watchSnapshot" -> callRuntime(call, result, OperitRuntimeNative::watchSnapshot)
            "watchStream" -> watchStream(call, result)
            "closeWatchStream" -> closeWatchStream(call, result)
            else -> return false
        }
        return true
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
        runtimeHost.runRuntime(result) {
            nativeCall(runtimeHost.ensureRuntimeHandle(), request.toByteArray(StandardCharsets.UTF_8))
        }
    }

    private fun watchStream(call: MethodCall, result: MethodChannel.Result) {
        val request = call.arguments as? String
        if (request == null) {
            result.error("INVALID_ARGS", "watchStream expects a JSON string", null)
            return
        }
        runtimeHost.runRuntime(result) {
            val response = OperitRuntimeNative.watchStream(
                runtimeHost.ensureRuntimeHandle(),
                request.toByteArray(StandardCharsets.UTF_8),
            )
            if (JSONObject(response).has("subscriptionId")) {
                ensureWatchPump()
            }
            response
        }
    }

    private fun closeWatchStream(call: MethodCall, result: MethodChannel.Result) {
        val subscriptionId = call.arguments as? String
        if (subscriptionId == null) {
            result.error("INVALID_ARGS", "closeWatchStream expects a subscription id", null)
            return
        }
        runtimeHost.runRuntime(result) {
            OperitRuntimeNative.closeWatchStream(runtimeHost.ensureRuntimeHandle(), subscriptionId)
        }
    }

    private fun ensureWatchPump() {
        synchronized(watchPumpLock) {
            if (watchPumpRunning) {
                return
            }
            watchPumpRunning = true
        }
        runtimeHost.runBackground {
            try {
                while (watchPumpRunning) {
                    val frame = OperitRuntimeNative.nextWatchChannelEvent(
                        runtimeHost.ensureRuntimeHandle(),
                    )
                    val frameJson = JSONObject(frame)
                    if (frameJson.has("code") && frameJson.has("message")) {
                        synchronized(watchPumpLock) {
                            watchPumpRunning = false
                        }
                        return@runBackground
                    }
                    val channel = runtimeChannel
                    if (channel != null) {
                        activity.runOnUiThread {
                            channel.invokeMethod("watchChannelEvent", frame)
                        }
                    }
                }
            } catch (_: Throwable) {
                synchronized(watchPumpLock) {
                    watchPumpRunning = false
                }
            }
        }
    }
}
