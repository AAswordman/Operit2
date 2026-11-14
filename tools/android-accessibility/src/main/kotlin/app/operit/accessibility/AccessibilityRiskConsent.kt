package app.operit.accessibility

import android.app.Activity
import android.app.AlertDialog
import android.content.Context
import android.graphics.Typeface
import android.text.Editable
import android.text.TextWatcher
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

/** Records the provider's one-time, exact-text risk acknowledgment. */
class AccessibilityRiskConsent(private val activity: Activity) {
    private var dialog: AlertDialog? = null
    private val preferences = activity.getSharedPreferences("accessibility_risk_consent", Context.MODE_PRIVATE)

    /** Requests the exact acknowledgment only until the user has confirmed it once. */
    fun request(complete: (Boolean) -> Unit) {
        if (preferences.getBoolean("accepted", false)) {
            complete(true)
            return
        }
        check(dialog == null) { "Accessibility risk acknowledgment is already open" }
        val chinese = activity.resources.configuration.locales[0].language == "zh"
        val phrase = if (chinese) CHINESE_ACKNOWLEDGMENT else ENGLISH_ACKNOWLEDGMENT
        val density = activity.resources.displayMetrics.density
        val padding = (24 * density).toInt()
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(padding, (8 * density).toInt(), padding, 0)
        }
        content.addView(TextView(activity).apply {
            text = if (chinese) "无障碍服务能够读取界面内容并执行操作。部分应用可能检测到此服务，并限制账号功能或封禁账号。请先理解这些风险，再决定是否授权。"
                else "Accessibility can read screen content and perform actions. Some apps may detect this service and restrict or ban your account. Understand these risks before granting access."
            textSize = 14f
            setLineSpacing(4 * density, 1f)
        })
        content.addView(TextView(activity).apply {
            text = if (chinese) "请逐字输入下方文字，空格和标点也必须一致：" else "Type the statement exactly, including spaces and punctuation:"
            textSize = 13f
            setPadding(0, (20 * density).toInt(), 0, (10 * density).toInt())
        })
        content.addView(TextView(activity).apply {
            text = phrase
            textSize = 15f
            setTypeface(typeface, Typeface.BOLD)
            setTextIsSelectable(false)
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
        })
        val input = EditText(activity).apply {
            hint = if (chinese) "在此输入完整确认文字" else "Enter the complete acknowledgment"
            textSize = 14f
            minLines = 2
            maxLines = 5
            setPadding(0, (12 * density).toInt(), 0, (12 * density).toInt())
        }
        content.addView(input)
        var accepted = false
        val prompt = AlertDialog.Builder(activity)
            .setTitle(if (chinese) "授权前，请确认风险" else "Understand the risks before granting access")
            .setView(ScrollView(activity).apply { addView(content) })
            .setPositiveButton(if (chinese) "我已知晓，继续授权" else "Acknowledge and continue", null)
            .setNegativeButton(if (chinese) "暂不开启" else "Not now", null)
            .create()
        prompt.setOnDismissListener {
            dialog = null
            complete(accepted)
        }
        prompt.setOnShowListener {
            val confirm = prompt.getButton(AlertDialog.BUTTON_POSITIVE)
            confirm.isEnabled = false
            input.addTextChangedListener(object : TextWatcher {
                /** Updates the action only when every character matches the displayed statement. */
                override fun onTextChanged(text: CharSequence?, start: Int, before: Int, count: Int) {
                    confirm.isEnabled = matches(input.text.toString(), phrase)
                }
                /** Leaves text input unchanged before edits. */
                override fun beforeTextChanged(text: CharSequence?, start: Int, count: Int, after: Int) = Unit
                /** Leaves text input unchanged after edits. */
                override fun afterTextChanged(text: Editable?) = Unit
            })
            confirm.setOnClickListener {
                accepted = matches(input.text.toString(), phrase)
                if (accepted) {
                    preferences.edit().putBoolean("accepted", true).apply()
                    prompt.dismiss()
                }
            }
        }
        dialog = prompt
        prompt.show()
    }

    /** Dismisses an outstanding acknowledgment when its Activity owner is released. */
    fun release() {
        dialog?.dismiss()
    }

    companion object {
        const val CHINESE_ACKNOWLEDGMENT = "我已明确并知晓无障碍权限导致的其他应用封号、限制账号功能风险，并后果自己承担"
        const val ENGLISH_ACKNOWLEDGMENT = "I understand and acknowledge the risks of account bans and restricted account functions caused by accessibility permissions, and I bear the consequences myself"

        /** Compares the complete input without trimming, normalizing or ignoring case. */
        fun matches(input: String, expected: String): Boolean = input == expected
    }
}
