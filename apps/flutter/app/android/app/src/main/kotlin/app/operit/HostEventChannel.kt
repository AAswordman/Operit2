package app.operit

import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

class HostEventChannel(
    private val runtimeHost: AndroidRuntimeHost,
) {
    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        when (call.method) {
            "dispatchHostEvent" -> dispatchHostEvent(call, result)
            else -> return false
        }
        return true
    }

    private fun dispatchHostEvent(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val source = args?.get("source") as? String
        val payload = args?.get("payload") as? String
        if (source == null || payload == null) {
            result.error("INVALID_ARGS", "dispatchHostEvent expects source and payload", null)
            return
        }
        runtimeHost.runRuntime(result) {
            OperitRuntimeNative.dispatchHostEvent(runtimeHost.ensureRuntimeHandle(), source, payload)
        }
    }

}
