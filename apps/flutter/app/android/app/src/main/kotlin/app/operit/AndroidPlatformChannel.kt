package app.operit

import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

class AndroidPlatformChannel(
    private val activity: MainActivity,
    private val runtimeHost: AndroidRuntimeHost,
) {
    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        when (call.method) {
            "androidRuntimePaths" -> androidRuntimePaths(result)
            else -> return false
        }
        return true
    }

    private fun androidRuntimePaths(result: MethodChannel.Result) {
        Thread {
            try {
                val response = runtimeHost.androidRuntimePathsMap()
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("RUNTIME_BRIDGE_ERROR", error.message, null)
                }
            }
        }.start()
    }
}
