package app.operit

import android.Manifest
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Binder
import android.os.Build
import android.os.Process
import android.util.Base64
import app.operit.core.tools.system.AndroidPrivilegedCommandExecutor
import app.operit.core.tools.system.AndroidPrivilegedCommandTarget
import app.operit.windowserver.IUiServer
import java.util.UUID
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import org.json.JSONArray
import org.json.JSONObject

/** Deploys the uninstalled Android window payload through the existing privileged command Host. */
internal class AndroidUiServer(private val context: Context) {
    private val owner = Binder()
    @Volatile private var server: IUiServer? = null
    @Volatile private var released = false

    /** Selects the already-authorized platform capability without requesting permissions. */
    fun isAuthorized(): Boolean = server != null || AndroidPrivilegeAuthorization.isShizukuAuthorized() ||
        AndroidPrivilegeAuthorization.hasRootConsent(context)

    /** Launches an ordinary application in the Host's independently rendered native window. */
    @Synchronized
    fun launch(component: String): JSONObject = call(requireServer(), JSONObject().put("operation", "launch").put("component", component))

    /** Queries live overlay geometry; a dead server never silently becomes a new session. */
    @Synchronized
    fun windows(): JSONArray {
        check(!released) { "Android window owner has been released" }
        val endpoint = server ?: return JSONArray()
        return call(endpoint, JSONObject().put("operation", "windows")).getJSONArray("windows")
    }

    /** Sends control to one exact native display belonging to this Host. */
    @Synchronized
    fun control(operation: String, windowId: String, key: String? = null): JSONObject {
        check(!released) { "Android window owner has been released" }
        val request = JSONObject().put("operation", operation).put("windowId", windowId)
        if (key != null) request.put("key", key)
        return call(checkNotNull(server) { "Android window server has not started" }, request)
    }

    /** Releases only this package's native windows after an explicit stop-app request. */
    @Synchronized
    fun closePackage(packageName: String) {
        check(!released) { "Android window owner has been released" }
        server?.let { call(it, JSONObject().put("operation", "close_package").put("packageName", packageName)) }
    }

    /** Disposes windows through Host scheduling without blocking Android's lifecycle thread. */
    fun release() {
        released = true
        HostTaskScheduler.execute(Runnable {
            synchronized(this) {
                server?.let { endpoint ->
                    try { call(endpoint, JSONObject().put("operation", "shutdown")) }
                    catch (error: Exception) { android.util.Log.w("OperitUiServer", "Window server shutdown failed", error) }
                }
                server = null
            }
        })
    }

    /** Validates transport errors without launching a replacement for a lost session. */
    private fun call(endpoint: IUiServer, request: JSONObject): JSONObject {
        check(endpoint.asBinder().isBinderAlive) { "Android window server disconnected; its windows are no longer available" }
        val response = JSONObject(endpoint.execute(request.toString()))
        check(response.getBoolean("success")) { response.getString("error") }
        return response.getJSONObject("data")
    }

    /** Stages a dedicated DEX archive in shell-owned tmp and receives its nonce-bound Binder. */
    private fun requireServer(): IUiServer {
        check(!released) { "Android window owner has been released" }
        server?.let { return it }
        check(Build.VERSION.SDK_INT >= 30) { "Native Android windows require Android 11 or newer" }
        check(android.provider.Settings.canDrawOverlays(context)) { "Enable the existing Android overlay permission for native windows" }
        val target = when {
            AndroidPrivilegeAuthorization.isShizukuAuthorized() -> AndroidPrivilegedCommandTarget.Shizuku
            AndroidPrivilegeAuthorization.hasRootConsent(context) -> AndroidPrivilegedCommandTarget.RootAuto
            else -> throw SecurityException("Native Android windows require authorized Shizuku or Root")
        }
        val nonce = UUID.randomUUID().toString()
        val directory = "/data/local/tmp/operit-ui-$nonce"
        val ready = CompletableFuture<IUiServer>()
        var launched = false
        val action = context.packageName + ".UI_SERVER_READY"
        val receiver = object : BroadcastReceiver() {
            /** Accepts only the shell's targeted handoff for this exact deployment. */
            override fun onReceive(context: Context, intent: Intent) {
                if (intent.action != action || intent.getStringExtra("nonce") != nonce) return
                val binder = intent.getBundleExtra("endpoint")?.getBinder("server") ?: return
                ready.complete(IUiServer.Stub.asInterface(binder))
            }
        }
        if (Build.VERSION.SDK_INT >= 33) {
            context.registerReceiver(receiver, IntentFilter(action), Manifest.permission.DUMP, null, Context.RECEIVER_EXPORTED)
        } else {
            @Suppress("DEPRECATION")
            context.registerReceiver(receiver, IntentFilter(action), Manifest.permission.DUMP, null)
        }
        try {
            command(target, "mkdir -m 700 ${quote(directory)}")
            val archive = context.assets.open("companions/operit-ui-server.jar").use { it.readBytes() }
            val encoded = Base64.encodeToString(archive, Base64.NO_WRAP)
            for (chunk in encoded.chunked(24000)) {
                command(target, "printf %s ${quote(chunk)} >> ${quote("$directory/server.b64")}")
            }
            command(target, "base64 -d ${quote("$directory/server.b64")} > ${quote("$directory/server.jar")} && " +
                "chmod 444 ${quote("$directory/server.jar")} && rm ${quote("$directory/server.b64")}")
            if (target != AndroidPrivilegedCommandTarget.Shizuku) {
                command(target, "chown 2000:2000 ${quote(directory)} ${quote("$directory/server.jar")}")
            }
            val libraryDirectory = context.applicationInfo.nativeLibraryDir
            val nativeLibrary = "$libraryDirectory/liboperit_flutter_bridge.so"
            val executable = if (Process.is64Bit()) "/system/bin/app_process64" else "/system/bin/app_process32"
            command(target, "CLASSPATH=${quote("$directory/server.jar")} LD_LIBRARY_PATH=${quote(libraryDirectory)} " +
                "$executable / app.operit.uiserver.UiServerMain ${Process.myUid()} ${quote(context.packageName)} " +
                "${quote(nonce)} ${quote(nativeLibrary)} > ${quote("$directory/server.log")} 2>&1 < /dev/null &")
            launched = true
            val endpoint = ready.get(12, TimeUnit.SECONDS)
            check(endpoint.protocolVersion == 1) { "Unsupported Android window server protocol" }
            val windowSession = Class.forName("android.view.WindowManagerGlobal")
                .getMethod("getWindowSession").invoke(null) as android.os.IInterface
            endpoint.attach(owner, windowSession.asBinder())
            server = endpoint
            return endpoint
        } catch (error: Exception) {
            if (error is InterruptedException) Thread.currentThread().interrupt()
            var diagnostic = ""
            if (launched) {
                try { diagnostic = command(target, "cat ${quote("$directory/server.log")}").trim() }
                catch (diagnosticError: Exception) { error.addSuppressed(diagnosticError) }
            }
            try {
                command(target, "rm -f ${quote("$directory/server.jar")} ${quote("$directory/server.b64")} ${quote("$directory/server.log")} && rmdir ${quote(directory)}")
            } catch (cleanupError: Exception) { error.addSuppressed(cleanupError) }
            throw IllegalStateException("Android window server startup failed: $error\n$diagnostic".trim(), error)
        } finally { context.unregisterReceiver(receiver) }
    }

    /** Uses existing Root settings and Shizuku shell-identity enforcement for every deployment step. */
    private fun command(target: AndroidPrivilegedCommandTarget, text: String): String {
        val result = AndroidPrivilegedCommandExecutor.execute(context, target, text, timeoutMillis = 15_000,
            rootSettings = AndroidPrivilegeAuthorization.rootExecutionSettings(context))
        check(result.exitCode == 0) { "Android window server command failed: ${result.stderrText()}" }
        return result.stdout.toString(Charsets.UTF_8)
    }

    /** Quotes one opaque argument for the Android system shell. */
    private fun quote(value: String): String = "'" + value.replace("'", "'\\''") + "'"
}
