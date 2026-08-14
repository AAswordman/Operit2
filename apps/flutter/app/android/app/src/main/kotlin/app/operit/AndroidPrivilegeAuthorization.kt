package app.operit

import android.content.Context
import android.content.pm.PackageManager
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

    /** Returns whether the user has explicitly approved Root for the host. */
    fun isRootAuthorized(context: Context): Boolean {
        return context
            .getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
            .getBoolean(ROOT_AUTHORIZED_KEY, false)
    }

    /** Persists the user's explicit Root approval for the host. */
    fun setRootAuthorized(context: Context) {
        context
            .getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(ROOT_AUTHORIZED_KEY, true)
            .apply()
    }
}
