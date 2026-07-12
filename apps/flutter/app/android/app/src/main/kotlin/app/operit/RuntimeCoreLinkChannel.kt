package app.operit

import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

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
            "pushOpen" -> callRuntime(call, result, OperitRuntimeNative::pushOpen)
            "pushItem" -> callRuntime(call, result, OperitRuntimeNative::pushItem)
            "pushClose" -> pushClose(call, result)
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
        nativeCall: (Long, ByteArray) -> ByteArray,
    ) {
        val request = call.arguments as? ByteArray
        if (request == null) {
            result.error("INVALID_ARGS", "${call.method} expects MessagePack bytes", null)
            return
        }
        runtimeHost.runRuntime(result) {
            nativeCall(runtimeHost.ensureRuntimeHandle(), request)
        }
    }

    private fun watchStream(call: MethodCall, result: MethodChannel.Result) {
        val request = call.arguments as? ByteArray
        if (request == null) {
            result.error("INVALID_ARGS", "watchStream expects MessagePack bytes", null)
            return
        }
        runtimeHost.runRuntime(result) {
            val response = OperitRuntimeNative.watchStream(
                runtimeHost.ensureRuntimeHandle(),
                request,
            )
            ensureWatchPump()
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

    /** Closes one local Link push stream. */
    private fun pushClose(call: MethodCall, result: MethodChannel.Result) {
        val pushId = call.arguments as? String
        if (pushId == null) {
            result.error("INVALID_ARGS", "pushClose expects a push id", null)
            return
        }
        runtimeHost.runRuntime(result) {
            OperitRuntimeNative.pushClose(runtimeHost.ensureRuntimeHandle(), pushId)
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
                    if (frame == null) {
                        synchronized(watchPumpLock) { watchPumpRunning = false }
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
