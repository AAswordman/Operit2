package app.operit

object OperitRuntimeNative {
    init {
        System.loadLibrary("operit_flutter_bridge")
    }

    @JvmStatic external fun create(
        runtimeRoot: String,
        workspaceRoot: String,
        host: AndroidRuntimeHost,
    ): Long
    @JvmStatic external fun createError(): String
    @JvmStatic external fun destroy(handle: Long)
    @JvmStatic external fun call(handle: Long, request: ByteArray): String
    @JvmStatic external fun watchSnapshot(handle: Long, request: ByteArray): String
    @JvmStatic external fun watchStream(handle: Long, request: ByteArray): String
    @JvmStatic external fun nextWatchChannelEvent(handle: Long): String
    @JvmStatic external fun closeWatchStream(handle: Long, subscriptionId: String): String
    @JvmStatic
    external fun startWebAccessServer(
        handle: Long,
        bindAddress: String,
        token: String,
        shutdownToken: String,
        webRoot: String,
        deviceId: String,
        acceptedSessions: String,
        acceptedSessionStorePath: String,
        pairingCodePath: String,
        deviceInfoJson: String,
        enableWebAccess: String,
        enableDiscovery: String,
    ): String

    @JvmStatic external fun stopWebAccessServer(handle: Long): String

    @JvmStatic
    external fun discoverDevices(
        handle: Long,
        timeoutMs: Long,
    ): String

    @JvmStatic external fun remotePairStart(
        handle: Long,
        baseUrl: String,
        tokenHash: String,
        clientDeviceInfoJson: String,
    ): String

    @JvmStatic external fun remotePairFinish(
        handle: Long,
        pairingId: String,
        pairingCode: String,
    ): String

    @JvmStatic external fun emitRuntimeEvent(handle: Long, eventJson: String): String
}
