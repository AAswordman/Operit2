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
}
