package app.operit

import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodChannel

class RuntimeMethodChannelRouter(
    private val activity: MainActivity,
    runtimeHost: AndroidRuntimeHost,
    ownerSystem: OwnerSystemCapabilityChannel,
) {
    private val coreLinkChannel = RuntimeCoreLinkChannel(runtimeHost)
    private val externalRuntimeChannel = if (activity.isExternalRuntime) ExternalRuntimeChannel(activity, runtimeHost) else null
    private val ownerSystemChannel = ownerSystem
    private val androidPlatformChannel = AndroidPlatformChannel(activity, runtimeHost)
    private val documentInputChannel = DocumentInputChannel(activity)
    private var runtimeChannel: MethodChannel? = null

    fun configure(messenger: BinaryMessenger) {
        runtimeChannel = MethodChannel(messenger, "operit/runtime").also { channel ->
            channel.setMethodCallHandler { call, result ->
                when {
                    externalRuntimeChannel?.handle(call, result) == true -> Unit
                    call.method == "notificationActivationInitial" ->
                        result.success(activity.takeNotificationActivation())
                    call.method == "notificationActivationReady" -> {
                        activity.markNotificationActivationReceiverReady()
                        result.success(null)
                    }
                    coreLinkChannel.handle(call, result) -> Unit
                    ownerSystemChannel.handle(call, result) -> Unit
                    androidPlatformChannel.handle(call, result) -> Unit
                    else -> result.notImplemented()
                }
            }
        }
        documentInputChannel.attach(messenger)
    }

    fun clear() {
        documentInputChannel.clear()
        runtimeChannel?.setMethodCallHandler(null)
        runtimeChannel = null
    }

    /** Emits one notification activation after Dart has installed its receiver. */
    fun emitNotificationActivation(activation: Map<String, String>) {
        runtimeChannel?.invokeMethod("notificationActivation", activation)
    }

    /** Routes Android permission results to the channel that owns the request. */
    fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ): Boolean {
        if (ownerSystemChannel.onRequestPermissionsResult(requestCode)) return true
        return androidPlatformChannel.onRequestPermissionsResult(requestCode, permissions, grantResults)
    }

    /** Delivers one Android document picker result to the document input channel. */
    fun onActivityResult(requestCode: Int, resultCode: Int, data: android.content.Intent?): Boolean {
        if (androidPlatformChannel.onActivityResult(requestCode)) return true
        return documentInputChannel.onActivityResult(requestCode, resultCode, data)
    }
}
