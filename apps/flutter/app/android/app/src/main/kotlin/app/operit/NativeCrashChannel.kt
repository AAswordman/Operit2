package app.operit

import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodChannel

/** Connects Flutter fatal reports to the Android-owned crash Activity. */
class NativeCrashChannel(private val activity: MainActivity) {
    /** Registers the native crash presentation method channel. */
    fun configure(messenger: BinaryMessenger) {
        MethodChannel(messenger, "operit/crash").setMethodCallHandler { call, result ->
            if (call.method != "present") {
                result.notImplemented()
                return@setMethodCallHandler
            }
            val details = call.argument<String>("details")
            if (details == null) {
                result.error("INVALID_ARGS", "present requires crash details", null)
                return@setMethodCallHandler
            }
            NativeCrashActivity.start(activity.applicationContext, details)
            result.success(null)
        }
    }
}
