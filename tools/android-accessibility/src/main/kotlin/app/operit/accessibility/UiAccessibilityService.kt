package app.operit.accessibility

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.content.Intent
import android.graphics.Path
import android.graphics.Rect
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.json.JSONArray
import org.json.JSONObject

/** Owns Android native window discovery, complete accessibility trees and input. */
class UiAccessibilityService : AccessibilityService() {
    companion object {
        @Volatile var connected: UiAccessibilityService? = null
            private set
    }

    /** Publishes the instance after Android has authorized and connected it. */
    override fun onServiceConnected() { super.onServiceConnected(); connected = this }

    /** Reads window metadata live instead of caching accessibility events. */
    override fun onAccessibilityEvent(event: AccessibilityEvent?) = Unit

    /** Feedback interruption does not revoke the service connection. */
    override fun onInterrupt() = Unit

    /** Invalidates the endpoint when Android disconnects the service. */
    override fun onUnbind(intent: Intent?): Boolean {
        if (connected === this) connected = null
        return super.onUnbind(intent)
    }

    /** Removes a destroyed instance before another client operation can use it. */
    override fun onDestroy() { if (connected === this) connected = null; super.onDestroy() }

    /** Dispatches the structured operations shared by every Operit UI Host. */
    fun execute(request: JSONObject): JSONObject {
        val params = request.getJSONObject("parameters")
        when (val operation = request.getString("operation")) {
            "list_windows" -> {
                val result = JSONArray()
                val entries = nativeWindows()
                try { entries.forEach { result.put(describeWindow(it)) } }
                finally { entries.forEach { it.recycle() } }
                return JSONObject().put("windows", result)
            }
            "get_page_info" -> return withWindow(if (params.has("window_id")) params.getString("window_id") else null) { window ->
                val root = window.root ?: error("The selected Android window has no accessible root")
                try {
                    JSONObject().put("windowId", window.id.toString())
                        .put("packageName", root.packageName?.toString() ?: "")
                        .put("activityName", window.title?.toString() ?: "")
                        .put("scope", "desktop").put("truncated", false).put("uiElements", readNode(root))
                } finally { root.recycle() }
            }
            "focus_window" -> focusWindow(params.getString("window_id"))
            "tap", "long_press", "swipe" -> {
                val path = Path()
                val duration: Long
                if (operation == "swipe") {
                    path.moveTo(params.getDouble("start_x").toFloat(), params.getDouble("start_y").toFloat())
                    path.lineTo(params.getDouble("end_x").toFloat(), params.getDouble("end_y").toFloat())
                    duration = params.getLong("duration")
                } else {
                    val x = params.getDouble("x").toFloat()
                    val y = params.getDouble("y").toFloat()
                    path.moveTo(x, y); path.lineTo(x, y)
                    duration = if (operation == "tap") 50 else 600
                }
                require(duration in 1..60000) { "Gesture duration must be between 1 and 60000 milliseconds" }
                performGesture(path, duration)
            }
            "set_input_text" -> withWindow(null) { window ->
                val root = window.root ?: error("The active Android window has no accessible root")
                try {
                    val matches = mutableListOf<AccessibilityNodeInfo>()
                    try {
                        findInputs(root, if (params.has("resource_id")) params.getString("resource_id") else null, matches)
                        check(matches.size == 1) { "Select one unique editable input by resource_id or input focus" }
                        val arguments = Bundle().apply {
                            putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, params.getString("text"))
                        }
                        check(matches.single().performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, arguments)) { "Android rejected text input" }
                    } finally { matches.forEach { it.recycle() } }
                } finally { root.recycle() }
            }
            "press_key" -> {
                val action = when (params.getString("key_code").uppercase()) {
                    "BACK", "KEYCODE_BACK" -> GLOBAL_ACTION_BACK
                    "HOME", "KEYCODE_HOME" -> GLOBAL_ACTION_HOME
                    "RECENTS" -> GLOBAL_ACTION_RECENTS
                    "NOTIFICATIONS" -> GLOBAL_ACTION_NOTIFICATIONS
                    "QUICK_SETTINGS" -> GLOBAL_ACTION_QUICK_SETTINGS
                    else -> error("Android supports BACK, HOME, RECENTS, NOTIFICATIONS and QUICK_SETTINGS global keys")
                }
                check(performGlobalAction(action)) { "Android rejected the global key action" }
            }
            else -> error("Unknown UI operation: $operation")
        }
        return JSONObject()
    }

    /** Queries all displays on Android 11+, and the system window list on earlier releases. */
    private fun nativeWindows(): List<AccessibilityWindowInfo> {
        if (Build.VERSION.SDK_INT >= 30) {
            val displays = windowsOnAllDisplays
            return buildList { for (index in 0 until displays.size()) addAll(displays.valueAt(index)) }
        }
        return windows
    }

    /** Resolves an exact native ID or active window and releases every handle afterward. */
    private fun <T> withWindow(id: String?, action: (AccessibilityWindowInfo) -> T): T {
        val entries = nativeWindows()
        try {
            val selected = entries.singleOrNull { if (id == null) it.isActive else it.id.toString() == id }
                ?: error("The selected Android window is unavailable or ambiguous; refresh list_windows")
            return action(selected)
        } finally { entries.forEach { it.recycle() } }
    }

    /** Formats every returned rectangle using the shared absolute-screen bounds grammar. */
    private fun bounds(rect: Rect): String = "[${rect.left},${rect.top}][${rect.right},${rect.bottom}]"

    /** Describes a native window without collecting its descendants. */
    private fun describeWindow(window: AccessibilityWindowInfo): JSONObject {
        val root = window.root
        try {
            val rect = Rect().also { window.getBoundsInScreen(it) }
            return JSONObject().put("windowId", window.id.toString()).put("title", window.title?.toString() ?: "")
                .put("packageName", root?.packageName?.toString() ?: "").put("bounds", bounds(rect))
                .put("isActive", window.isActive).put("isFocused", window.isFocused).put("isMinimized", false)
        } finally { root?.recycle() }
    }

    /** Preserves all exposed descendants, non-important views and native accessibility states. */
    private fun readNode(node: AccessibilityNodeInfo): JSONObject {
        val rect = Rect().also { node.getBoundsInScreen(it) }
        val children = JSONArray()
        for (index in 0 until node.childCount) {
            val child = node.getChild(index) ?: continue
            try { children.put(readNode(child)) } finally { child.recycle() }
        }
        return JSONObject().put("className", node.className?.toString() ?: JSONObject.NULL)
            .put("text", if (node.isPassword) JSONObject.NULL else node.text?.toString() ?: JSONObject.NULL)
            .put("contentDesc", if (node.isPassword) JSONObject.NULL else node.contentDescription?.toString() ?: JSONObject.NULL)
            .put("resourceId", node.viewIdResourceName ?: JSONObject.NULL).put("bounds", bounds(rect))
            .put("isClickable", node.isClickable).put("children", children)
            .put("properties", JSONObject().put("isEnabled", node.isEnabled).put("isFocused", node.isFocused)
                .put("isFocusable", node.isFocusable).put("isEditable", node.isEditable)
                .put("isVisible", node.isVisibleToUser).put("isScrollable", node.isScrollable)
                .put("isSelected", node.isSelected).put("isCheckable", node.isCheckable)
                .put("isChecked", node.isChecked).put("isPassword", node.isPassword)
                .put("actions", JSONArray(node.actionList.map { it.id })))
    }

    /** Requests input focus from a focusable descendant without generating touch input. */
    private fun focusInput(node: AccessibilityNodeInfo): Boolean {
        if (node.isFocusable && node.performAction(AccessibilityNodeInfo.ACTION_FOCUS)) return true
        for (index in 0 until node.childCount) {
            val child = node.getChild(index) ?: continue
            try { if (focusInput(child)) return true } finally { child.recycle() }
        }
        return false
    }

    /** Focuses a selected native window and verifies the resulting window focus. */
    private fun focusWindow(id: String) {
        withWindow(id) { window ->
            if (!window.isFocused) {
                val root = window.root ?: error("The selected Android window has no accessible root")
                try { check(focusInput(root)) { "Android rejected input focus for the selected window" } }
                finally { root.recycle() }
            }
        }
        val deadline = SystemClock.uptimeMillis() + 2000
        while (!withWindow(id) { it.isFocused }) {
            check(SystemClock.uptimeMillis() < deadline) { "The selected Android window did not gain input focus" }
            SystemClock.sleep(20)
        }
    }

    /** Collects exact editable matches throughout the native window. */
    private fun findInputs(node: AccessibilityNodeInfo, resourceId: String?, result: MutableList<AccessibilityNodeInfo>) {
        val matches = if (resourceId == null) node.isFocused else node.viewIdResourceName == resourceId
        if (node.isEditable && matches) result.add(AccessibilityNodeInfo.obtain(node))
        for (index in 0 until node.childCount) {
            val child = node.getChild(index) ?: continue
            try { findInputs(child, resourceId, result) } finally { child.recycle() }
        }
    }

    /** Waits for gesture completion and reports rejection, cancellation or timeout. */
    private fun performGesture(path: Path, duration: Long) {
        val finished = CountDownLatch(1)
        var completed = false
        var accepted = false
        val gesture = GestureDescription.Builder().addStroke(GestureDescription.StrokeDescription(path, 0, duration)).build()
        check(Handler(Looper.getMainLooper()).post {
            accepted = dispatchGesture(gesture, object : GestureResultCallback() {
                /** Publishes successful completion to the operation worker. */
                override fun onCompleted(gestureDescription: GestureDescription?) { completed = true; finished.countDown() }
                /** Publishes cancellation without reporting success. */
                override fun onCancelled(gestureDescription: GestureDescription?) { finished.countDown() }
            }, null)
            if (!accepted) finished.countDown()
        }) { "Android's input thread is unavailable" }
        check(finished.await(duration + 5000, TimeUnit.MILLISECONDS)) { "Android gesture completion timed out" }
        check(accepted) { "Android rejected the gesture" }
        check(completed) { "Android cancelled the gesture" }
    }
}
