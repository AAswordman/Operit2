package app.operit

import android.os.Bundle
import android.os.Process
import android.content.Intent
import android.util.Log
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import dev.flutter.packages.file_selector_android.FileSelectorAndroidPlugin
import org.json.JSONObject

/** Hosts storage maintenance in a separate process without normal owner-system initialization. */
class DataRescueActivity : FlutterActivity() {
    private lateinit var storageHost: AndroidRuntimeHost
    private val rescueRequests = HostTaskExecutor(serial = true)

    /** Selects the maintenance route before Dart initializes any business service. */
    override fun getInitialRoute(): String = "/data-rescue"

    /** Registers only the offline Host ABI alongside ordinary file-picker plugins. */
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        flutterEngine.plugins.add(FileSelectorAndroidPlugin())
        storageHost = AndroidRuntimeHost(applicationContext)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "operit/runtime").setMethodCallHandler { call, result ->
            if (call.method != "dataRescue") {
                result.notImplemented()
            } else {
                val encoded = call.arguments as? String
                if (encoded == null) {
                    result.error("INVALID_ARGS", "dataRescue expects JSON text", null)
                } else {
                    try {
                        val command = JSONObject(encoded)
                        when (command.getString("operation")) {
                            "launchMode" -> result.success("{\"ok\":true,\"value\":{\"rescue\":true,\"canStopNormalProcess\":true}}")
                            "stopNormalProcess" -> {
                                stopService(Intent(applicationContext, OperitCoreService::class.java))
                                val manager = getSystemService(ACTIVITY_SERVICE) as android.app.ActivityManager
                                val processes = manager.runningAppProcesses
                                if (processes != null) {
                                    for (process in processes) {
                                        if (process.uid == Process.myUid() && process.processName == packageName) Process.killProcess(process.pid)
                                    }
                                }
                                result.success("{\"ok\":true,\"value\":null}")
                            }
                            else -> rescueRequests.execute {
                                try {
                                    val response = OperitRuntimeNative.dataRescue(storageHost.defaultRuntimeRootPath(), encoded, storageHost)
                                    runOnUiThread { result.success(response) }
                                } catch (error: Throwable) {
                                    runOnUiThread { result.error("DATA_RESCUE_ERROR", error.message, null) }
                                }
                            }
                        }
                    } catch (error: Throwable) { result.error("DATA_RESCUE_ERROR", error.message, null) }
                }
            }
        }
    }

    /** Closes queued maintenance work and releases its lease when the owning window is destroyed. */
    override fun onDestroy() {
        if (::storageHost.isInitialized) {
            rescueRequests.execute {
                try {
                    val response = JSONObject(OperitRuntimeNative.dataRescue(
                        storageHost.defaultRuntimeRootPath(),
                        "{\"operation\":\"close\"}",
                        storageHost,
                    ))
                    if (!response.getBoolean("ok")) {
                        Log.e("DataRescueActivity", "Cannot close maintenance session: " + response.getString("error"))
                    }
                } finally {
                    storageHost.destroy()
                }
            }
        }
        rescueRequests.shutdown()
        super.onDestroy()
    }

}
