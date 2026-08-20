package app.operit

import android.os.SystemClock
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import app.operit.util.AppLogger

class RuntimeCoreLinkChannel(
    private val activity: MainActivity,
    private val runtimeHost: AndroidRuntimeHost,
) {
    private val watchPumpLock = Any()
    @Volatile
    private var watchPumpRunning = false
    @Volatile
    private var runtimeChannel: MethodChannel? = null
    private var watchPumpFrameIndex = 0L

    fun attach(channel: MethodChannel) {
        runtimeChannel = channel
        AppLogger.d("RuntimeCoreLink", "runtime channel attached")
    }

    fun clear() {
        runtimeChannel = null
        AppLogger.d("RuntimeCoreLink", "runtime channel cleared")
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
        AppLogger.d(
            "RuntimeCoreLink",
            "watch stream open requested bytes=${request.size}",
        )
        runtimeHost.runRuntime(result) {
            val response = OperitRuntimeNative.watchStream(
                runtimeHost.ensureRuntimeHandle(),
                request,
            )
            AppLogger.d(
                "RuntimeCoreLink",
                "watch stream native open returned bytes=${response.size}",
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
        AppLogger.d(
            "RuntimeCoreLink",
            "watch stream close requested subscription=$subscriptionId",
        )
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
                AppLogger.d("RuntimeCoreLink", "watch pump already running")
                return
            }
            watchPumpRunning = true
        }
        AppLogger.d("RuntimeCoreLink", "watch pump started")
        runtimeHost.runBackground {
            try {
                while (watchPumpRunning) {
                    val frame = OperitRuntimeNative.nextWatchChannelEvent(
                        runtimeHost.ensureRuntimeHandle(),
                    )
                    if (frame == null) {
                        AppLogger.d(
                            "RuntimeCoreLink",
                            "watch pump stopped reason=native_channel_closed",
                        )
                        synchronized(watchPumpLock) { watchPumpRunning = false }
                        return@runBackground
                    }
                    val frameIndex = synchronized(watchPumpLock) {
                        val index = watchPumpFrameIndex
                        watchPumpFrameIndex += 1L
                        index
                    }
                    val dequeuedAt = SystemClock.elapsedRealtime()
                    val sampled = frameIndex < 20L || frameIndex % 50L == 0L
                    if (sampled) {
                        AppLogger.d(
                            "RuntimeCoreLink",
                            "watch frame dequeued index=$frameIndex bytes=${frame.size}",
                        )
                    }
                    val channel = runtimeChannel
                    if (channel != null) {
                        activity.runOnUiThread {
                            if (sampled) {
                                AppLogger.d(
                                    "RuntimeCoreLink",
                                    "watch frame delivered index=$frameIndex uiQueueMs=${SystemClock.elapsedRealtime() - dequeuedAt}",
                                )
                            }
                            channel.invokeMethod("watchChannelEvent", frame)
                        }
                    } else if (sampled) {
                        AppLogger.d(
                            "RuntimeCoreLink",
                            "watch frame dropped index=$frameIndex reason=channel_unattached",
                        )
                    }
                }
            } catch (error: Throwable) {
                AppLogger.e(
                    "RuntimeCoreLink",
                    "watch pump failed running=$watchPumpRunning",
                    error,
                )
                synchronized(watchPumpLock) {
                    watchPumpRunning = false
                }
            }
        }
    }
}
