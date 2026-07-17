package app.operit

import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.view.Display
import android.view.View
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine

class MainActivity : FlutterActivity() {
    companion object {
        @Volatile private var activeActivity: MainActivity? = null

        fun currentActivity(): MainActivity? = activeActivity
    }

    private lateinit var runtimeHost: AndroidRuntimeHost
    private lateinit var ownerSystem: OwnerSystemCapabilityChannel
    private lateinit var runtimeRouter: RuntimeMethodChannelRouter
    private lateinit var nativeCrashChannel: NativeCrashChannel

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        activeActivity = this
        configureSystemBars()
        requestHighestRefreshRate()
    }

    override fun onResume() {
        super.onResume()
        activeActivity = this
        configureSystemBars()
        requestHighestRefreshRate()
    }

    override fun onDestroy() {
        if (activeActivity === this) {
            activeActivity = null
        }
        super.onDestroy()
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        val runtimeHost = ensureRuntimeHost()
        val ownerSystem = ensureOwnerSystem(runtimeHost)
        runtimeRouter = RuntimeMethodChannelRouter(this, runtimeHost, ownerSystem)
        runtimeRouter.configure(flutterEngine.dartExecutor.binaryMessenger)
        nativeCrashChannel = NativeCrashChannel(this)
        nativeCrashChannel.configure(flutterEngine.dartExecutor.binaryMessenger)
    }

    override fun cleanUpFlutterEngine(flutterEngine: FlutterEngine) {
        try {
            if (::ownerSystem.isInitialized) {
                ownerSystem.release()
            }
            if (::runtimeRouter.isInitialized) {
                runtimeRouter.clear()
            }
        } catch (_: Exception) {
        }
        super.cleanUpFlutterEngine(flutterEngine)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        if (::runtimeRouter.isInitialized &&
            runtimeRouter.onRequestPermissionsResult(requestCode, permissions, grantResults)
        ) {
            return
        }
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
    }

    fun ensureHostEventRuntimeHandle(): Long = ensureRuntimeHost().ensureRuntimeHandle()

    fun handleRuntimeHostRequest(methodName: String, payloadJson: String): String {
        return ensureOwnerSystem(ensureRuntimeHost()).handleRuntimeHostRequest(methodName, payloadJson)
    }

    /** Returns the process-level Runtime host shared with the Core service. */
    private fun ensureRuntimeHost(): AndroidRuntimeHost {
        if (!::runtimeHost.isInitialized) {
            runtimeHost = AndroidCoreRuntime.get(applicationContext)
        }
        return runtimeHost
    }

    private fun ensureOwnerSystem(runtimeHost: AndroidRuntimeHost): OwnerSystemCapabilityChannel {
        if (!::ownerSystem.isInitialized) {
            ownerSystem = OwnerSystemCapabilityChannel(this, runtimeHost)
        }
        return ownerSystem
    }

    private fun requestHighestRefreshRate() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) {
            return
        }
        val display = currentDisplay() ?: return
        val currentMode = display.mode ?: return
        val preferredMode =
            display.supportedModes
                .filter {
                    it.physicalWidth == currentMode.physicalWidth &&
                        it.physicalHeight == currentMode.physicalHeight
                }
                .maxByOrNull { it.refreshRate }
                ?: return

        if (preferredMode.modeId == currentMode.modeId) {
            return
        }

        val attributes = window.attributes
        if (attributes.preferredDisplayModeId == preferredMode.modeId) {
            return
        }
        attributes.preferredDisplayModeId = preferredMode.modeId
        window.attributes = attributes
        AndroidClientLogger.i(
            applicationContext,
            "OperitMainActivity",
            "Requested display mode ${preferredMode.physicalWidth}x${preferredMode.physicalHeight}@${preferredMode.refreshRate}Hz",
        )
    }

    @Suppress("DEPRECATION")
    private fun currentDisplay(): Display? {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            display
        } else {
            windowManager.defaultDisplay
        }
    }

    private fun configureSystemBars() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            window.statusBarColor = Color.TRANSPARENT
            window.navigationBarColor = Color.TRANSPARENT
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            window.isStatusBarContrastEnforced = false
            window.isNavigationBarContrastEnforced = false
        }

        val flags =
            View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
                View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
                } else {
                    0
                }
        window.decorView.systemUiVisibility = flags
    }
}
