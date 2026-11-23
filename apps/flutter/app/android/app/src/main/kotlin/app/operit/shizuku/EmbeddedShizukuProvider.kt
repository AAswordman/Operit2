package app.operit.shizuku

import android.os.Binder
import android.os.Bundle
import moe.shizuku.api.BinderContainer
import rikka.shizuku.Shizuku
import rikka.shizuku.ShizukuProvider
import rikka.shizuku.ShizukuApiConstants.USER_SERVICE_ARG_TOKEN

/** Keeps external and embedded Binder publication explicitly separated. */
class EmbeddedShizukuProvider : ShizukuProvider() {
    /** Leaves Sui initialization to an explicitly selected external backend. */
    override fun onCreate(): Boolean {
        val app = checkNotNull(context).applicationContext
        if (EmbeddedShizuku.selected(app)) disableAutomaticSuiInitialization()
        Shizuku.addBinderDeadListener {
            if (EmbeddedShizuku.restoreEnabled(app)) EmbeddedShizuku.scheduleRestore(app)
        }
        return super.onCreate()
    }

    /** Accepts the private service only from shell and attaches its owned UserService. */
    override fun call(method: String, arg: String?, extras: Bundle?): Bundle? {
        val app = checkNotNull(context)
        val embedded = EmbeddedShizuku.selected(app)
        if (method == "sendEmbeddedBinder") {
            check(Binder.getCallingUid() == 2000) { "Embedded Binder publisher must be shell" }
            if (!embedded || extras == null) return null
            extras.classLoader = BinderContainer::class.java.classLoader
            val binder = extras.getParcelable<BinderContainer>(EXTRA_BINDER)?.binder ?: return null
            Shizuku.onBinderReceived(binder, app.packageName)
            return Bundle()
        }
        if (method == "sendUserService") {
            check(embedded && Binder.getCallingUid() == 2000) { "Invalid embedded UserService publisher" }
            checkNotNull(extras).classLoader = BinderContainer::class.java.classLoader
            val binder = extras.getParcelable<BinderContainer>(EXTRA_BINDER)?.binder ?: return null
            val token = extras.getString(USER_SERVICE_ARG_TOKEN) ?: return null
            check(Shizuku.pingBinder()) { "Embedded service disconnected" }
            Shizuku.attachUserService(binder, Bundle().apply { putString(USER_SERVICE_ARG_TOKEN, token) })
            return Bundle().apply { putParcelable(EXTRA_BINDER, BinderContainer(Shizuku.getBinder())) }
        }
        if (method == "sendBinder") {
            check(Binder.getCallingUid() == 0 || Binder.getCallingUid() == 2000) { "Invalid external Shizuku publisher" }
            checkNotNull(extras).classLoader = BinderContainer::class.java.classLoader
            val binder = extras.getParcelable<BinderContainer>(EXTRA_BINDER)?.binder ?: return null
            EmbeddedShizuku.receiveExternal(app, binder)
            return Bundle()
        }
        return super.call(method, arg, extras)
    }

    companion object {
        private const val EXTRA_BINDER = "moe.shizuku.privileged.api.intent.extra.BINDER"
    }
}
