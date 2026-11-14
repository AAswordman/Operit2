package app.operit.accessibility

import android.app.Activity
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.provider.Settings
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.FrameLayout
import android.widget.TextView

/** Presents the companion's connection state and the system authorization entry point. */
class SetupActivity : Activity() {
    private lateinit var status: TextView
    private lateinit var badge: TextView
    private lateinit var description: TextView
    private lateinit var settingsButton: Button
    private val riskConsent = AccessibilityRiskConsent(this)

    /** Presents the branded, responsive service setup screen with native Android controls. */
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = Color.TRANSPARENT
        window.navigationBarColor = Color.TRANSPARENT
        val dark = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
        window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
            View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION or
            if (dark) 0 else View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR or View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
        setContentView(R.layout.activity_setup)
        findViewById<View>(R.id.setup_root).apply {
            setOnApplyWindowInsetsListener { view, insets ->
                view.setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop,
                    insets.systemWindowInsetRight, insets.systemWindowInsetBottom)
                insets
            }
            requestApplyInsets()
        }
        val content = findViewById<View>(R.id.setup_content)
        content.parent.let { it as View }.addOnLayoutChangeListener { parent, left, _, right, _, _, _, _, _ ->
            val width = minOf(right - left, (608 * resources.displayMetrics.density).toInt())
            if (content.layoutParams.width != width) content.layoutParams = FrameLayout.LayoutParams(
                width, ViewGroup.LayoutParams.WRAP_CONTENT, android.view.Gravity.CENTER_HORIZONTAL)
        }
        status = findViewById(R.id.service_status)
        badge = findViewById(R.id.status_badge)
        description = findViewById(R.id.status_description)
        settingsButton = findViewById<Button>(R.id.open_settings).apply {
            setOnClickListener {
                if (UiAccessibilityService.connected != null) openSettings()
                else riskConsent.request { accepted -> if (accepted) openSettings() }
            }
        }
        findViewById<TextView>(R.id.provider_version).text = getString(R.string.version_label,
            packageManager.getPackageInfo(packageName, 0).versionName)
    }

    /** Refreshes status after the user returns from system settings. */
    override fun onResume() {
        super.onResume()
        refreshStatus()
    }

    /** Refreshes the card using the service's actual connection state. */
    private fun refreshStatus() {
        val connected = UiAccessibilityService.connected != null
        status.setText(if (connected) R.string.status_enabled else R.string.status_disabled)
        badge.setText(if (connected) R.string.badge_enabled else R.string.badge_disabled)
        badge.setTextColor(getColor(if (connected) R.color.provider_success else R.color.provider_pending))
        badge.background = GradientDrawable().apply {
            cornerRadius = 24 * resources.displayMetrics.density
            setColor(getColor(if (connected) R.color.provider_success_soft else R.color.provider_pending_soft))
        }
        description.setText(if (connected) R.string.description_enabled else R.string.description_disabled)
        settingsButton.setText(if (connected) R.string.manage_settings else R.string.open_settings)
    }

    /** Opens the Android-owned accessibility authorization screen. */
    private fun openSettings() = startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))

    /** Dismisses the acknowledgment when its screen owner is destroyed. */
    override fun onDestroy() {
        riskConsent.release()
        super.onDestroy()
    }
}
