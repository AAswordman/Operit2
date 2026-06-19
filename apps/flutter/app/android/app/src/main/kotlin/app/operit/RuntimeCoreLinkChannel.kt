package app.operit

import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.nio.charset.StandardCharsets

class RuntimeCoreLinkChannel(private val runtimeHost: AndroidRuntimeHost) {
    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        when (call.method) {
            "call" -> callRuntime(call, result, OperitRuntimeNative::call)
            "watchSnapshot" -> callRuntime(call, result, OperitRuntimeNative::watchSnapshot)
            "watchStream" -> callRuntime(call, result, OperitRuntimeNative::watchStream)
            "pollWatchStream" -> pollWatchStream(call, result)
            "pollWatchStreams" -> pollWatchStreams(call, result)
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

    private fun pollWatchStream(call: MethodCall, result: MethodChannel.Result) {
        val subscriptionId = call.arguments as? String
        if (subscriptionId == null) {
            result.error("INVALID_ARGS", "pollWatchStream expects a subscription id", null)
            return
        }
        runtimeHost.runRuntime(result) {
            OperitRuntimeNative.pollWatchStream(runtimeHost.ensureRuntimeHandle(), subscriptionId)
        }
    }

    private fun pollWatchStreams(call: MethodCall, result: MethodChannel.Result) {
        val subscriptionIdsJson = call.arguments as? String
        if (subscriptionIdsJson == null) {
            result.error("INVALID_ARGS", "pollWatchStreams expects a JSON string array", null)
            return
        }
        runtimeHost.runRuntime(result) {
            OperitRuntimeNative.pollWatchStreams(runtimeHost.ensureRuntimeHandle(), subscriptionIdsJson)
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
}
