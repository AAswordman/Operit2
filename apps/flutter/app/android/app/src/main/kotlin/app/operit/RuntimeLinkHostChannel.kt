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
            "discoverDevices" -> discoverDevices(call, result)
            "remotePairStart" -> remotePairStart(call, result)
            "remotePairFinish" -> remotePairFinish(call, result)
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

    private fun discoverDevices(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val timeoutMs = args?.get("timeoutMs") as? Number
        if (timeoutMs == null) {
            result.error("INVALID_ARGS", "discoverDevices expects timeoutMs", null)
            return
        }
        runtimeHost.runRuntime(result) {
            OperitRuntimeNative.discoverDevices(runtimeHost.ensureRuntimeHandle(), timeoutMs.toLong())
        }
    }

    private fun remotePairStart(call: MethodCall, result: MethodChannel.Result) {
        val args = call.arguments as? Map<*, *>
        val baseUrl = args?.get("baseUrl") as? String
        val tokenHash = args?.get("tokenHash") as? String
        val clientDeviceInfoJson = args?.get("clientDeviceInfo") as? String
        if (baseUrl == null || tokenHash == null || clientDeviceInfoJson == null) {
            result.error("INVALID_ARGS", "remotePairStart expects baseUrl, tokenHash and clientDeviceInfo", null)
            return
        }
        runtimeHost.runRuntime(result) {
            OperitRuntimeNative.remotePairStart(
                runtimeHost.ensureRuntimeHandle(),
                baseUrl,
                tokenHash,
                clientDeviceInfoJson,
            )
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
        runtimeHost.runRuntime(result) {
            OperitRuntimeNative.remotePairFinish(runtimeHost.ensureRuntimeHandle(), pairingId, pairingCode)
        }
    }
}
