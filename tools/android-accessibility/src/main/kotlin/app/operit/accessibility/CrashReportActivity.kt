package app.operit.accessibility

import android.app.Activity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Button
import android.widget.TextView

/** Displays the saved report in a task and process independent of the failed service. */
class CrashReportActivity : Activity() {
    companion object {
        /** Opens the report viewer without transferring the stack trace through Binder. */
        fun open(context: Context) {
            context.startActivity(Intent(context, CrashReportActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
            })
        }
    }

    /** Shows the report and lets the user copy it or dismiss it and reopen setup. */
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val report = CrashReportStore(this)
        if (!report.exists()) {
            finishAndRemoveTask()
            return
        }
        val details = report.read()
        setContentView(R.layout.activity_crash_report)
        findViewById<View>(R.id.crash_root).apply {
            setOnApplyWindowInsetsListener { view, insets ->
                view.setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop,
                    insets.systemWindowInsetRight, insets.systemWindowInsetBottom)
                insets
            }
            requestApplyInsets()
        }
        findViewById<TextView>(R.id.crash_details).text = details
        findViewById<Button>(R.id.copy_crash_report).setOnClickListener {
            val clipboard = getSystemService(CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText(getString(R.string.crash_title), details))
        }
        findViewById<Button>(R.id.dismiss_crash_report).setOnClickListener {
            report.clear()
            finishAndRemoveTask()
        }
    }
}
