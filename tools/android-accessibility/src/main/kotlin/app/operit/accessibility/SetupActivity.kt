package app.operit.accessibility

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

/** Presents the companion's connection state and the system authorization entry point. */
class SetupActivity : Activity() {
    private lateinit var status: TextView

    /** Builds a small native setup screen without a UI framework dependency. */
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val content = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val padding = (24 * resources.displayMetrics.density).toInt()
            setPadding(padding, padding, padding, padding)
        }
        content.addView(TextView(this).apply { setText(R.string.setup_description); textSize = 18f })
        status = TextView(this).also { content.addView(it) }
        content.addView(Button(this).apply {
            setText(R.string.open_settings)
            setOnClickListener { startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)) }
        })
        setContentView(content)
    }

    /** Refreshes status after the user returns from system settings. */
    override fun onResume() {
        super.onResume()
        status.setText(if (UiAccessibilityService.connected != null) R.string.status_enabled else R.string.status_disabled)
    }
}
