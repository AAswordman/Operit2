package app.operit

import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

class RuntimeLinkHostChannel(private val runtimeHost: AndroidRuntimeHost) {
    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        when (call.method) {
            "startWebAccessServer" -> startWebAccessServer(call, result)
            "stopWebAccessServer" -> runtimeHost.runRuntime(result) {
                OperitRuntimeNative.stopWebAccessServer(runtimeHost.ensureRuntimeHandle())
            }
            else -> return false
        }
        return true
    }

    private fun startWebAccessServer(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val bindAddress = args?.get("bindAddress") as? String
        val token = args?.get("token") as? String
        val shutdownToken = args?.get("shutdownToken") as? String
        val webRoot = args?.get("webRoot") as? String
        val deviceInfoJson = args?.get("deviceInfo") as? String
        val enableWebAccess = args?.get("enableWebAccess") as? String
        val enableDiscovery = args?.get("enableDiscovery") as? String
        if (
            bindAddress == null ||
                token == null ||
                shutdownToken == null ||
                webRoot == null ||
                deviceInfoJson == null ||
                enableWebAccess == null ||
                enableDiscovery == null
        ) {
            result.error(
                "INVALID_ARGS",
                "startWebAccessServer expects bindAddress, token, shutdownToken, webRoot, deviceInfo, enableWebAccess and enableDiscovery",
                null,
            )
            return
        }
        runtimeHost.runRuntime(result) {
            OperitRuntimeNative.startWebAccessServer(
                runtimeHost.ensureRuntimeHandle(),
                bindAddress,
                token,
                shutdownToken,
                webRoot,
                deviceInfoJson,
                enableWebAccess,
                enableDiscovery,
            )
        }
    }

}
