package app.operit

import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodChannel

class RuntimeMethodChannelRouter(
    activity: MainActivity,
    runtimeHost: AndroidRuntimeHost,
    ownerSystem: OwnerSystemCapabilityChannel,
) {
    private val coreLinkChannel = RuntimeCoreLinkChannel(activity, runtimeHost)
    private val linkHostChannel = RuntimeLinkHostChannel(runtimeHost)
    private val ownerSystemChannel = ownerSystem
    private val androidPlatformChannel = AndroidPlatformChannel(activity, runtimeHost)
    private var runtimeChannel: MethodChannel? = null

    fun configure(messenger: BinaryMessenger) {
        runtimeChannel = MethodChannel(messenger, "operit/runtime").also { channel ->
            coreLinkChannel.attach(channel)
            channel.setMethodCallHandler { call, result ->
                when {
                    call.method == "notificationActivationInitial" ->
                        result.success(activity.takeNotificationActivation())
                    call.method == "notificationActivationReady" -> {
                        activity.markNotificationActivationReceiverReady()
                        result.success(null)
                    }
                    coreLinkChannel.handle(call, result) -> Unit
                    linkHostChannel.handle(call, result) -> Unit
                    ownerSystemChannel.handle(call, result) -> Unit
                    androidPlatformChannel.handle(call, result) -> Unit
                    else -> result.notImplemented()
                }
            }
        }
    }

    fun clear() {
        coreLinkChannel.clear()
        runtimeChannel?.setMethodCallHandler(null)
        runtimeChannel = null
    }

    /** Emits one notification activation after Dart has installed its receiver. */
    fun emitNotificationActivation(activation: Map<String, String>) {
        runtimeChannel?.invokeMethod("notificationActivation", activation)
    }

    fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ): Boolean {
        return androidPlatformChannel.onRequestPermissionsResult(requestCode, permissions, grantResults)
    }
}
