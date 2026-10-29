package app.operit

import android.content.Context
import android.content.pm.PackageManager
import app.operit.core.tools.system.AndroidRootExecutionMode
import app.operit.core.tools.system.AndroidRootExecutionSettings
import app.operit.core.tools.system.AndroidRootShell
import rikka.shizuku.Shizuku

/** Represents the current Shizuku authorization state for Android host features. */
enum class ShizukuAuthorizationStatus {
    Unavailable,
    Missing,
    Authorized,
}

/** Stores and reads optional Android host privilege authorizations. */
object AndroidPrivilegeAuthorization {
    private const val PREFERENCES_NAME = "android_privilege_authorization"
    private const val ROOT_AUTHORIZED_KEY = "root_authorized"
    private const val ROOT_EXECUTION_MODE_KEY = "root_execution_mode"
    private const val ROOT_SU_COMMAND_KEY = "root_su_command"

    /** Returns the current Shizuku availability and authorization state. */
    fun shizukuAuthorizationStatus(): ShizukuAuthorizationStatus {
        if (!Shizuku.pingBinder()) {
            return ShizukuAuthorizationStatus.Unavailable
        }
        return if (Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED) {
            ShizukuAuthorizationStatus.Authorized
        } else {
            ShizukuAuthorizationStatus.Missing
        }
    }

    /** Returns whether Shizuku is active and authorized for the host. */
    fun isShizukuAuthorized(): Boolean {
        return shizukuAuthorizationStatus() == ShizukuAuthorizationStatus.Authorized
    }

    /** Requires explicit consent AND a fresh Root identity check. Call off the UI thread. */
    fun isRootAuthorized(context: Context): Boolean {
        // Merely opening onboarding must not trigger a Root manager's authorization dialog.
        if (!hasRootConsent(context)) return false
        return AndroidRootShell.checkAccess(rootExecutionSettings(context), 10_000L).granted
    }

    /** Reads consent without starting any su process or authorization prompt. */
    fun hasRootConsent(context: Context): Boolean =
        context.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
            .getBoolean(ROOT_AUTHORIZED_KEY, false)

    fun rootExecutionSettings(context: Context): AndroidRootExecutionSettings {
        val preferences = context.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
        val mode = runCatching {
            AndroidRootExecutionMode.valueOf(
                preferences.getString(ROOT_EXECUTION_MODE_KEY, AndroidRootExecutionMode.Auto.name)!!,
            )
        }.getOrDefault(AndroidRootExecutionMode.Auto)
        return AndroidRootExecutionSettings(
            mode, preferences.getString(ROOT_SU_COMMAND_KEY, "su")?.trim().orEmpty().ifEmpty { "su" },
        )
    }

    /** Configuration changes require explicit authorization of the new executable/mode. */
    fun configureRootExecution(context: Context, settings: AndroidRootExecutionSettings) {
        settings.suArguments() // Validate before committing any preferences.
        if (settings == rootExecutionSettings(context)) return
        context.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE).edit()
            .putString(ROOT_EXECUTION_MODE_KEY, settings.mode.name)
            .putString(ROOT_SU_COMMAND_KEY, settings.suCommand.trim().ifEmpty { "su" })
            .putBoolean(ROOT_AUTHORIZED_KEY, false)
            .apply()
    }

    /** Consent is not itself proof that su still grants Root access. */
    fun setRootAuthorized(context: Context, authorized: Boolean = true) {
        context.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE).edit()
            .putBoolean(ROOT_AUTHORIZED_KEY, authorized)
            .apply()
    }
}
