package app.operit.accessibility

import android.app.Activity
import android.app.Dialog
import android.content.Context
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextWatcher
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView

/**
 * Records the provider's one-time, exact-text risk acknowledgment.
 *
 * The panel is inflated from `dialog_risk_consent.xml` and hosted in a plain [Dialog] so the window
 * background can take the 24dp radius used by `provider_card`. The companion module intentionally
 * carries no Material Components dependency and [SetupActivity] is a plain [Activity], so a
 * platform `AlertDialog` would fall back to a square panel with stock grey buttons.
 */
class AccessibilityRiskConsent(private val activity: Activity) {
    private var dialog: Dialog? = null
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

        val prompt = Dialog(activity, R.style.ProviderDialog)
        prompt.setContentView(R.layout.dialog_risk_consent)
        prompt.window?.setBackgroundDrawableResource(R.drawable.provider_dialog_background)
        prompt.setCanceledOnTouchOutside(false)

        val root = prompt.findViewById<ViewGroup>(R.id.consent_root)
        prompt.findViewById<TextView>(R.id.consent_title).setText(R.string.consent_title)
        prompt.findViewById<TextView>(R.id.consent_body).setText(R.string.consent_body)
        prompt.findViewById<TextView>(R.id.consent_statement_label).setText(R.string.consent_statement_label)
        prompt.findViewById<TextView>(R.id.consent_statement).text = phrase
        val input = prompt.findViewById<EditText>(R.id.consent_input)

        // The actions live inside the rounded panel; a platform button bar would render outside it.
        val actions = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            layoutParams = LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT,
            ).apply { topMargin = (24 * density).toInt() }
        }
        val dismiss = dialogButton(R.string.consent_not_now, secondary = true)
        val confirm = dialogButton(R.string.consent_continue, secondary = false)
        actions.addView(dismiss, LinearLayout.LayoutParams(0, (52 * density).toInt(), 1f))
        actions.addView(confirm, LinearLayout.LayoutParams(0, (52 * density).toInt(), 1f).apply {
            marginStart = (12 * density).toInt()
        })
        root.addView(actions)

        var accepted = false
        // The statement is the whole point of this screen, so the action stays disabled until the
        // input matches it character for character.
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
        dismiss.setOnClickListener { prompt.dismiss() }
        confirm.setOnClickListener {
            accepted = matches(input.text.toString(), phrase)
            if (accepted) {
                preferences.edit().putBoolean("accepted", true).apply()
                prompt.dismiss()
            }
        }

        prompt.setOnDismissListener {
            dialog = null
            complete(accepted)
        }
        dialog = prompt
        prompt.show()
    }

    /** Builds one action that follows the provider button language instead of the platform bar. */
    private fun dialogButton(textRes: Int, secondary: Boolean): Button {
        val density = activity.resources.displayMetrics.density
        return Button(activity).apply {
            setText(textRes)
            isAllCaps = false
            textSize = 15f
            typeface = android.graphics.Typeface.create("sans-serif-medium", android.graphics.Typeface.NORMAL)
            setTextColor(
                if (secondary) activity.getColor(R.color.provider_text)
                else activity.getColor(android.R.color.white),
            )
            stateListAnimator = null
            background = if (secondary) secondaryButtonBackground() else primaryButtonBackground(density)
        }
    }

    /** Filled primary action, matching `provider_button`. */
    private fun primaryButtonBackground(density: Float) = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = 16 * density
        setColor(activity.getColor(R.color.provider_primary))
    }

    /** Outlined secondary action so it stays subordinate to the acknowledgment. */
    private fun secondaryButtonBackground() = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = 16 * activity.resources.displayMetrics.density
        setColor(activity.getColor(R.color.provider_soft))
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