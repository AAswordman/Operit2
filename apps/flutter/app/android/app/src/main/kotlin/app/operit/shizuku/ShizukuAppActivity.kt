// Copied from Shizuku AppActivity.kt (Apache-2.0); adapted to the owner's theme selection.
package app.operit.shizuku

import android.content.res.Resources
import android.content.res.Resources.Theme
import android.graphics.Color
import android.os.Build
import androidx.annotation.RequiresApi
import app.operit.embeddedshizuku.R
import rikka.core.res.isNight
import rikka.core.res.resolveColor
import rikka.material.app.MaterialActivity

abstract class ShizukuAppActivity : MaterialActivity() {

    override fun computeUserThemeKey(): String {
        return resources.configuration.isNight().toString()
    }

    override fun onApplyUserThemeResource(theme: Theme, isDecorView: Boolean) {
        theme.applyStyle(if (resources.configuration.isNight()) R.style.Theme else R.style.Theme_Light, true)
        theme.applyStyle(if (resources.configuration.isNight())
            R.style.ThemeOverlay_DynamicColors_Dark else R.style.ThemeOverlay_DynamicColors_Light, true)
    }

    @RequiresApi(Build.VERSION_CODES.M)
    override fun onApplyTranslucentSystemBars() {
        super.onApplyTranslucentSystemBars()

        val window = window
        val theme = theme

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            window?.decorView?.post {
                if (window.decorView.rootWindowInsets?.systemWindowInsetBottom ?: 0 >= Resources.getSystem().displayMetrics.density * 40) {
                    window.navigationBarColor =
                        theme.resolveColor(android.R.attr.navigationBarColor) and 0x00ffffff or -0x20000000
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                        window.isNavigationBarContrastEnforced = false
                    }
                } else {
                    window.navigationBarColor = Color.TRANSPARENT
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                        window.isNavigationBarContrastEnforced = true
                    }
                }
            }
        }
    }

    override fun onSupportNavigateUp(): Boolean {
        if (!super.onSupportNavigateUp()) {
            finish()
        }
        return true
    }
}
