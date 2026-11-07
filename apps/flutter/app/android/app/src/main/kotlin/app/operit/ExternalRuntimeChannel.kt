package app.operit

import android.os.Process
import java.io.File
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import org.json.JSONObject

/** Connects a module window to Root-owned state without starting an application Core. */
class ExternalRuntimeChannel(private val activity: MainActivity, private val host: AndroidRuntimeHost) {
    private var deployment: JSONObject? = null

    /** Reads the endpoint published by the module into this application's private sandbox. */
    @Synchronized
    private fun endpoint(): JSONObject {
        val existing = deployment
        if (existing != null) return existing
        val file = File(activity.filesDir, "host-runtime/client.json")
        check(file.isFile) { "Module is not ready: the private client endpoint is missing" }
        val value = JSONObject(file.readText())
        check(value.getInt("protocol") == 1) { "Incompatible module protocol" }
        deployment = value
        return value
    }

    /** Wraps the platform storage response expected by the shared Flutter bootstrap code. */
    private fun success(value: Any?): String = JSONObject().put("ok", true)
        .put("value", value ?: JSONObject.NULL).toString()

    /** Validates the exact roots owned by the currently running module identity. */
    private fun paths(call: MethodCall): Map<String, String> {
        val bootstrap = endpoint().getJSONObject("bootstrap")
        val identity = bootstrap.getString("activeIdentityId")
        val runtime = "${bootstrap.getString("runtimeRoot")}/identities/$identity"
        val workspace = "${bootstrap.getString("workspaceRoot")}/identities/$identity"
        val suppliedRuntime = call.argument<String>("runtimeRoot")
        val suppliedWorkspace = call.argument<String>("workspaceRoot")
        val active = suppliedRuntime == runtime && suppliedWorkspace == workspace
        val base = suppliedRuntime == bootstrap.getString("runtimeRoot") && suppliedWorkspace == bootstrap.getString("workspaceRoot")
        check(active || base) {
            "The running module has fixed runtime and workspace roots"
        }
        return mapOf("runtimeRoot" to suppliedRuntime!!, "workspaceRoot" to suppliedWorkspace!!)
    }

    /** Intercepts only lifecycle and storage operations that would otherwise create a local Core. */
    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        if (call.method !in setOf("connectCoreFfi", "dataRescue", "runtimeBootstrapRead", "runtimeBootstrapWrite",
                "localRuntimeStorageDefaults", "localRuntimeStoragePaths", "setLocalRuntimeStorage",
                "startLocalCoreService", "localRuntimeStartupStatus", "restartApplication", "terminateApplication",
                "hostEventScheduleFailures")) return false
        host.runBackground {
            try {
                val response: Any? = when (call.method) {
                    "connectCoreFfi" -> OperitRuntimeNative.connectModuleCoreFfi(endpoint().toString())
                    "dataRescue" -> when (JSONObject(call.arguments as String).getString("operation")) {
                        "launchMode" -> success(JSONObject().put("rescue", false).put("clientOnly", true))
                        "runtimeLease" -> success(null)
                        else -> error("Module data maintenance is performed by the module, not the App client")
                    }
                    "runtimeBootstrapRead" -> success(endpoint().getJSONObject("bootstrap").toString())
                    "runtimeBootstrapWrite" -> {
                        val content = call.arguments as String
                        OperitRuntimeNative.writeModuleBootstrap(endpoint().toString(), content)
                        endpoint().put("bootstrap", JSONObject(content))
                        success(null)
                    }
                    "localRuntimeStorageDefaults" -> {
                        val config = endpoint().getJSONObject("bootstrap")
                        mapOf("runtimeRoot" to config.getString("runtimeRoot"), "workspaceRoot" to config.getString("workspaceRoot"))
                    }
                    "localRuntimeStoragePaths" -> paths(call)
                    "setLocalRuntimeStorage" -> { paths(call); null }
                    "startLocalCoreService" -> { endpoint(); null }
                    "localRuntimeStartupStatus" -> mapOf("state" to "connecting", "message" to "正在连接模块核心")
                    "hostEventScheduleFailures" -> emptyList<Any>()
                    "restartApplication", "terminateApplication" -> {
                        activity.runOnUiThread { activity.finishAndRemoveTask(); Process.killProcess(Process.myPid()) }
                        null
                    }
                    else -> error("Unknown module lifecycle operation")
                }
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread { result.error("MODULE_RUNTIME_ERROR", error.message, null) }
            }
        }
        return true
    }
}
