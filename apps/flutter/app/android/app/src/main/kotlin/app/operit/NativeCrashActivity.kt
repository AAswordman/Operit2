package app.operit

import android.app.Activity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

/** Displays a process-independent native crash screen for fatal application failures. */
class NativeCrashActivity : Activity() {
    companion object {
        private const val extraDetails = "app.operit.extra.CRASH_DETAILS"

        /** Starts the dedicated crash process with a complete diagnostic report. */
        fun start(context: Context, details: String) {
            context.startActivity(
                Intent(context, NativeCrashActivity::class.java)
                    .putExtra(extraDetails, details)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK),
            )
        }
    }

    /** Creates the native crash layout without requiring a running Flutter engine. */
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val details = requireNotNull(intent.getStringExtra(extraDetails))
        title = "Operit2 has stopped"
        val padding = (24 * resources.displayMetrics.density).toInt()
        val content = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(padding, padding, padding, padding)
            setBackgroundColor(Color.rgb(250, 250, 250))
        }
        content.addView(TextView(this).apply {
            text = "Operit2 has stopped"
            textSize = 26f
            setTextColor(Color.rgb(32, 32, 32))
            setTypeface(typeface, Typeface.BOLD)
        })
        content.addView(TextView(this).apply {
            text = "A fatal error prevented this session from continuing."
            textSize = 16f
            setTextColor(Color.rgb(70, 70, 70))
            setPadding(0, padding / 2, 0, padding)
        })
        val detailsView = TextView(this).apply {
            text = details
            textSize = 12f
            typeface = Typeface.MONOSPACE
            setTextColor(Color.rgb(32, 32, 32))
            setTextIsSelectable(true)
        }
        content.addView(ScrollView(this).apply {
            addView(detailsView)
        }, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            0,
            1f,
        ))
        content.addView(Button(this).apply {
            text = "Copy details"
            setOnClickListener {
                val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                clipboard.setPrimaryClip(ClipData.newPlainText("Operit2 crash", details))
            }
        }, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
        ).apply { gravity = Gravity.END })
        content.addView(Button(this).apply {
            text = "Close"
            setOnClickListener { finishAndRemoveTask() }
        }, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
        ).apply { gravity = Gravity.END })
        setContentView(content)
    }
}
