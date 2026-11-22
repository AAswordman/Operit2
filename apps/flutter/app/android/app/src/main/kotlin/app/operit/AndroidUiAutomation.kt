package app.operit

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.IBinder
import android.os.Build
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.util.Base64
import java.io.ByteArrayOutputStream
import app.operit.accessibility.IUiAutomationProvider
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.json.JSONArray
import org.json.JSONObject

/** Adapts the in-repository accessibility companion to the platform-neutral UI Host. */
internal class AndroidUiAutomation(
    private val context: Context,
    private val uiServer: AndroidUiServer,
    private val capturePrimary: () -> ByteArray,
) {
    private val lock = Any()
    private val operationLock = Any()
    private var connection: ProviderConnection? = null
    private var released = false
    private var selectedWindowId: String? = null

    /** Confirms that Android exposed the launched application on the exact hosted display. */
    fun launch(component: String) = synchronized(operationLock) {
        val provider = requireProvider()
        val window = uiServer.launch(component)
        val deadline = SystemClock.uptimeMillis() + 6000
        try {
            while (true) {
                val windows = readResponse(provider, JSONObject().put("operation", "list_windows").put("parameters", JSONObject()))
                    .getJSONArray("windows").objects()
                if (windows.any { it.getInt("displayId") == window.getInt("displayId") &&
                        it.getString("packageName") == window.getString("packageName") }) return@synchronized
                check(SystemClock.uptimeMillis() < deadline) { "Android did not expose the launched application on its requested display" }
                SystemClock.sleep(50)
            }
        } catch (error: Exception) {
            try { uiServer.closePackage(window.getString("packageName")) }
            catch (cleanupError: Exception) { error.addSuppressed(cleanupError) }
            throw error
        }
    }

    /** Associates every asynchronous service callback with its own binding. */
    private class ProviderConnection : ServiceConnection {
        val ready = CountDownLatch(1)
        @Volatile var service: IUiAutomationProvider? = null

        /** Publishes the companion's versioned Binder endpoint. */
        override fun onServiceConnected(name: ComponentName, binder: IBinder) {
            service = IUiAutomationProvider.Stub.asInterface(binder)
            ready.countDown()
        }

        /** Invalidates a disconnected endpoint. */
        override fun onServiceDisconnected(name: ComponentName) { service = null }

        /** Completes a rejected binding without waiting for the deadline. */
        override fun onNullBinding(name: ComponentName) { ready.countDown() }

        /** Invalidates a binding whose companion was replaced. */
        override fun onBindingDied(name: ComponentName) { service = null; ready.countDown() }
    }

    /** Connects to the Operit2 companion and checks its explicit wire version. */
    private fun requireProvider(): IUiAutomationProvider {
        val binding = synchronized(lock) {
            check(!released) { "UI automation owner has been released" }
            connection?.service?.let { if (it.asBinder().isBinderAlive) return it }
            val pending = ProviderConnection()
            connection?.let { context.unbindService(it) }
            connection = null
            val intent = Intent().setComponent(ComponentName("app.operit.accessibility", "app.operit.accessibility.UiAutomationBinderService"))
            check(context.bindService(intent, pending, Context.BIND_AUTO_CREATE)) {
                "请在应用权限设置中点击 UI 自动化的授予按钮，安装并启用 Operit2 Accessibility。"
            }
            connection = pending
            pending
        }
        try {
            check(binding.ready.await(5, TimeUnit.SECONDS)) { "Operit2 Accessibility connection timed out" }
            return synchronized(lock) {
                check(!released && connection === binding) { "UI automation owner has been released" }
                val service = binding.service ?: error("Operit2 Accessibility rejected the connection")
                check(service.protocolVersion == 3) { "Update Operit2 Accessibility through the UI automation grant action" }
                service
            }
        } catch (error: Exception) {
            synchronized(lock) {
                if (connection === binding) { context.unbindService(binding); connection = null }
            }
            throw error
        }
    }

    /** Reads the full streamed response and preserves native provider failures. */
    fun execute(request: JSONObject): JSONObject = synchronized(operationLock) {
        val operation = request.getString("operation")
        val parameters = JSONObject(request.getJSONObject("parameters").toString())
        if (operation == "focus_window" && !parameters.has("window_id")) {
            if (parameters.optBoolean("close_windows", false)) uiServer.closeWindows()
            selectedWindowId = null
            return@synchronized JSONObject()
        }
        if (operation == "capture_screenshot" && selectedWindowId == null) {
            return@synchronized JSONObject().put("pngBase64", Base64.encodeToString(capturePrimary(), Base64.NO_WRAP))
        }
        val provider = requireProvider()
        val hosted = uiServer.windows().objects().associateBy { it.getInt("displayId") }
        if (operation == "list_windows") {
            val result = readResponse(provider, request)
            val native = result.getJSONArray("windows").objects()
            val windows = JSONArray()
            val chromeIds = hosted.values.map { it.getString("chromeWindowId") }.toSet()
            native.filter { !hosted.containsKey(it.getInt("displayId")) && it.getString("windowId") !in chromeIds }.forEach { window ->
                window.remove("displayId")
                windows.put(window)
            }
            hosted.forEach { (displayId, overlay) ->
                val active = native.filter { it.optInt("displayId", -1) == displayId }
                overlay.put("isActive", active.any { it.getBoolean("isActive") })
                    .put("isFocused", active.any { it.getBoolean("isFocused") })
                overlay.remove("displayId")
                overlay.remove("chromeWindowId")
                windows.put(overlay)
            }
            return@synchronized JSONObject().put("windows", windows)
        }
        if (operation != "focus_window" && !parameters.has("window_id")) {
            selectedWindowId?.let { parameters.put("window_id", it) }
        }
        val targetId = parameters.optString("window_id").takeIf { it.isNotEmpty() }
        if (operation == "capture_screenshot" && Build.VERSION.SDK_INT < 30) {
            val frame = readResponse(provider, JSONObject().put("operation", "get_window_frame").put("parameters", parameters))
            val bytes = capturePrimary()
            val bitmap = checkNotNull(BitmapFactory.decodeByteArray(bytes, 0, bytes.size)) { "Android returned an invalid primary screenshot" }
            try {
                val cropped = Bitmap.createBitmap(bitmap, frame.getInt("left"), frame.getInt("top"), frame.getInt("width"), frame.getInt("height"))
                try {
                    val output = ByteArrayOutputStream()
                    check(cropped.compress(Bitmap.CompressFormat.PNG, 100, output)) { "Android could not encode the selected window screenshot" }
                    return@synchronized JSONObject().put("pngBase64", Base64.encodeToString(output.toByteArray(), Base64.NO_WRAP))
                } finally { if (cropped !== bitmap) cropped.recycle() }
            } finally { bitmap.recycle() }
        }
        val virtual = hosted.values.singleOrNull { it.getString("windowId") == targetId }
        if (virtual != null) {
            val displayId = virtual.getInt("displayId")
            if (operation == "focus_window") uiServer.control("focus", virtual.getString("windowId"))
            else if (operation != "get_page_info") {
                check(!virtual.getBoolean("isMinimized")) { "Focus the minimized Android window before sending input" }
            }
            if (operation == "press_key") return@synchronized uiServer.control("key", virtual.getString("windowId"), parameters.getString("key_code"))
            parameters.remove("window_id")
            parameters.put("display_id", displayId)
        }
        val result = readResponse(provider, JSONObject().put("operation", operation).put("parameters", parameters))
        if (operation == "focus_window") selectedWindowId = checkNotNull(targetId)
        if (virtual != null && operation == "get_page_info") result.put("windowId", targetId)
        if (operation == "get_page_info" && targetId != null) selectedWindowId = targetId
        result.remove("displayId")
        result
    }

    /** Iterates Android's JSON arrays without leaking platform metadata into shared contracts. */
    private fun JSONArray.objects(): List<JSONObject> = List(length()) { getJSONObject(it) }

    /** Reads a response from a specific binding without reconnecting mid-operation. */
    private fun readResponse(provider: IUiAutomationProvider, request: JSONObject): JSONObject {
        val descriptor = provider.execute(request.toString())
            ?: error("Operit2 Accessibility returned no response stream")
        val response = ParcelFileDescriptor.AutoCloseInputStream(descriptor).bufferedReader(Charsets.UTF_8)
            .use { JSONObject(it.readText()) }
        check(response.getBoolean("success")) { response.getString("error") }
        return response.getJSONObject("data")
    }

    /** Releases only the binding owned by this adapter. */
    fun release() = synchronized(lock) {
        released = true
        connection?.let { context.unbindService(it); it.service = null; it.ready.countDown() }
        connection = null
    }
}
