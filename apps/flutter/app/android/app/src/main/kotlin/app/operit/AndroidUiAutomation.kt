package app.operit

import android.accessibilityservice.AccessibilityService
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.IBinder
import android.os.SystemClock
import com.ai.assistance.operit.provider.IAccessibilityProvider
import java.io.StringReader
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.json.JSONArray
import org.json.JSONObject
import org.xmlpull.v1.XmlPullParser
import org.xmlpull.v1.XmlPullParserFactory

/** Owns only the client connection to the independently installed accessibility APK. */
class AndroidUiAutomation(private val context: Context) {
    private val lock = Any()
    private val operationLock = Any()
    private var connection: ProviderConnection? = null
    private var released = false

    /** Keeps late callbacks attached to their own binding rather than a newer client. */
    private class ProviderConnection : ServiceConnection {
        val ready = CountDownLatch(1)
        @Volatile var service: IAccessibilityProvider? = null

        /** Publishes the binder returned by this external provider connection. */
        override fun onServiceConnected(name: ComponentName, binder: IBinder) {
            service = IAccessibilityProvider.Stub.asInterface(binder)
            ready.countDown()
        }

        /** Invalidates a disconnected provider. */
        override fun onServiceDisconnected(name: ComponentName) { service = null }

        /** Completes a rejected bind immediately. */
        override fun onNullBinding(name: ComponentName) { ready.countDown() }

        /** Invalidates a binding whose provider was replaced or stopped. */
        override fun onBindingDied(name: ComponentName) { service = null; ready.countDown() }
    }

    /** Binds the exact legacy service and waits only on the runtime worker. */
    private fun requireProvider(): IAccessibilityProvider {
        val binding = synchronized(lock) {
            check(!released) { "UI automation owner has been released" }
            connection?.service?.let { if (it.asBinder().isBinderAlive) return it }
            val pending = ProviderConnection()
            connection?.let { context.unbindService(it) }
            connection = null
            val intent = Intent("com.ai.assistance.operit.provider.IAccessibilityProvider")
                .setComponent(ComponentName("com.ai.assistance.operit.provider", "com.ai.assistance.operit.provider.RemoteBinderService"))
            check(context.bindService(intent, pending, Context.BIND_AUTO_CREATE)) {
                "Install the separate Operit accessibility provider APK before using UI automation"
            }
            connection = pending
            pending
        }
        try {
            check(binding.ready.await(5, TimeUnit.SECONDS)) { "Accessibility provider binding timed out" }
            return synchronized(lock) {
                check(!released && connection === binding) { "UI automation owner has been released" }
                binding.service ?: error("Accessibility provider rejected the connection")
            }
        } catch (error: Exception) {
            synchronized(lock) {
                if (connection === binding) {
                    context.unbindService(binding)
                    connection = null
                }
            }
            throw error
        }
    }

    /** Dispatches UI operations exclusively through the legacy AIDL interface. */
    fun execute(request: JSONObject): JSONObject = synchronized(operationLock) {
        val service = requireProvider()
        check(service.isAccessibilityServiceEnabled) { "Enable the separate Operit provider in Android accessibility settings" }
        val operation = request.getString("operation")
        val params = request.getJSONObject("parameters")
        if (operation == "get_page_info") return@synchronized readPage(service)
        val success = when (operation) {
            "tap" -> service.performClick(params.getInt("x"), params.getInt("y"))
            "long_press" -> service.performLongPress(params.getInt("x"), params.getInt("y"))
            "swipe" -> service.performSwipe(params.getInt("start_x"), params.getInt("start_y"), params.getInt("end_x"), params.getInt("end_y"), params.getLong("duration"))
            "set_input_text" -> {
                val nodeId = if (params.has("resource_id")) findBounds(service.uiHierarchy, params.getString("resource_id")) else service.findFocusedNodeId()
                check(!nodeId.isNullOrEmpty()) { "No accessible input element is focused" }
                service.setTextOnNode(nodeId, params.getString("text"))
            }
            "press_key" -> {
                val action = when (params.getString("key_code").uppercase()) {
                    "BACK", "KEYCODE_BACK" -> AccessibilityService.GLOBAL_ACTION_BACK
                    "HOME", "KEYCODE_HOME" -> AccessibilityService.GLOBAL_ACTION_HOME
                    "RECENTS" -> AccessibilityService.GLOBAL_ACTION_RECENTS
                    "NOTIFICATIONS" -> AccessibilityService.GLOBAL_ACTION_NOTIFICATIONS
                    "QUICK_SETTINGS" -> AccessibilityService.GLOBAL_ACTION_QUICK_SETTINGS
                    else -> error("The legacy accessibility APK supports BACK, HOME, RECENTS, NOTIFICATIONS and QUICK_SETTINGS global keys")
                }
                service.performGlobalAction(action)
            }
            else -> error("Unknown UI operation: $operation")
        }
        check(success) { "Accessibility provider rejected $operation" }
        // The legacy binder acknowledges dispatch before the gesture finishes.
        val gestureDuration = when (operation) {
            "tap" -> 50L
            "long_press" -> 600L
            "swipe" -> params.getLong("duration")
            else -> 0L
        }
        if (gestureDuration > 0) SystemClock.sleep(gestureDuration)
        JSONObject()
    }

    /** Translates a resource ID to the bounds token required by the legacy APK. */
    private fun findBounds(xml: String, resourceId: String): String {
        val parser = XmlPullParserFactory.newInstance().newPullParser()
        parser.setInput(StringReader(xml))
        val matches = mutableListOf<String>()
        while (parser.eventType != XmlPullParser.END_DOCUMENT) {
            if (parser.eventType == XmlPullParser.START_TAG && parser.name == "node" && parser.getAttributeValue(null, "resource-id") == resourceId) {
                matches.add(parser.getAttributeValue(null, "bounds") ?: error("Selected input has no bounds"))
            }
            parser.next()
        }
        check(matches.size == 1) { "Select one unique input resource ID" }
        return matches.single()
    }

    /** Parses one XML snapshot into the common node format with traversal limits. */
    private fun readPage(service: IAccessibilityProvider): JSONObject {
        val xml = service.uiHierarchy
        check(!xml.isNullOrBlank()) { "Accessibility provider returned no hierarchy" }
        val parser = XmlPullParserFactory.newInstance().newPullParser()
        parser.setInput(StringReader(xml))
        val roots = JSONArray()
        val stack = ArrayDeque<JSONObject?>()
        var packageName = ""
        var count = 0
        var truncated = false
        while (parser.eventType != XmlPullParser.END_DOCUMENT) {
            if (parser.eventType == XmlPullParser.START_TAG && parser.name == "node") {
                if (count >= 4000 || stack.size >= 40 || (stack.isNotEmpty() && stack.last() == null)) {
                    truncated = true
                    stack.addLast(null)
                } else {
                    count++
                    val node = JSONObject()
                        .put("className", parser.getAttributeValue(null, "class") ?: JSONObject.NULL)
                        .put("text", parser.getAttributeValue(null, "text") ?: JSONObject.NULL)
                        .put("contentDesc", parser.getAttributeValue(null, "content-desc") ?: JSONObject.NULL)
                        .put("resourceId", parser.getAttributeValue(null, "resource-id") ?: JSONObject.NULL)
                        .put("bounds", parser.getAttributeValue(null, "bounds") ?: JSONObject.NULL)
                        .put("isClickable", parser.getAttributeValue(null, "clickable") == "true")
                        .put("children", JSONArray())
                    if (stack.isEmpty()) roots.put(node) else stack.last()!!.getJSONArray("children").put(node)
                    if (packageName.isEmpty()) packageName = parser.getAttributeValue(null, "package") ?: ""
                    stack.addLast(node)
                }
            } else if (parser.eventType == XmlPullParser.END_TAG && parser.name == "node") {
                check(stack.isNotEmpty()) { "Invalid accessibility hierarchy nesting" }
                stack.removeLast()
            }
            parser.next()
        }
        check(roots.length() > 0) { "Accessibility hierarchy contains no nodes" }
        val root = JSONObject().put("className", "surface").put("text", JSONObject.NULL)
            .put("contentDesc", JSONObject.NULL).put("resourceId", JSONObject.NULL).put("bounds", JSONObject.NULL)
            .put("isClickable", false).put("children", roots)
        return JSONObject().put("packageName", packageName).put("activityName", service.currentActivityName)
            .put("uiElements", root).put("scope", "desktop").put("truncated", truncated)
    }

    /** Releases the client binding when the application owner shuts down. */
    fun release() = synchronized(lock) {
        released = true
        connection?.let {
            context.unbindService(it)
            it.service = null
            it.ready.countDown()
        }
        connection = null
    }
}
