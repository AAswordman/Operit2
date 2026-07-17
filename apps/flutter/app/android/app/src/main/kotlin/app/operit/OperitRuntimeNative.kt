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
    @JvmStatic external fun call(handle: Long, request: ByteArray): ByteArray
    @JvmStatic external fun pushOpen(handle: Long, request: ByteArray): ByteArray
    @JvmStatic external fun pushItem(handle: Long, item: ByteArray): ByteArray
    @JvmStatic external fun pushClose(handle: Long, pushId: String): ByteArray
    @JvmStatic external fun watchSnapshot(handle: Long, request: ByteArray): ByteArray
    @JvmStatic external fun watchStream(handle: Long, request: ByteArray): ByteArray
    @JvmStatic external fun nextWatchChannelEvent(handle: Long): ByteArray?
    @JvmStatic external fun closeWatchStream(handle: Long, subscriptionId: String): ByteArray
    @JvmStatic
    external fun startWebAccessServer(
        handle: Long,
        bindAddress: String,
        token: String,
        shutdownToken: String,
        webRoot: String,
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

    @JvmStatic
    external fun emitHostRuntimeEventSchedule(
        handle: Long,
        scheduleId: String,
        scheduledAtMillis: Long,
        firedAtMillis: Long,
    ): String
}
