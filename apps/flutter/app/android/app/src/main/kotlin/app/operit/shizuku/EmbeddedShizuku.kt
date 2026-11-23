package app.operit.shizuku

import android.content.Context
import android.content.SharedPreferences
import android.os.Build
import android.os.DeadObjectException
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.UserManager
import android.provider.Settings
import android.util.Log
import app.operit.AndroidPrivilegeAuthorization
import app.operit.AndroidTerminalPrivileges
import app.operit.core.tools.system.AndroidRootShell
import app.operit.core.tools.system.AndroidPrivilegedCommandTarget
import moe.shizuku.manager.adb.AdbClient
import moe.shizuku.manager.adb.AdbKey
import moe.shizuku.manager.adb.AdbMdns
import moe.shizuku.manager.adb.AdbPairingClient
import moe.shizuku.manager.adb.PreferenceAdbKeyStore
import rikka.shizuku.Shizuku
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Activates one private shell service through the user's explicitly selected transport. */
object EmbeddedShizuku {
    private const val TAG = "OperitShell"
    private const val PREFS = "embedded_shizuku"
    private val main = Handler(Looper.getMainLooper())
    @Volatile private var externalBinder: IBinder? = null
    private var activationPreferences: SharedPreferences? = null

    /** Remembers external publication without replacing the explicitly selected embedded backend. */
    fun receiveExternal(context: Context, binder: IBinder) {
        externalBinder = binder
        if (!selected(context)) Shizuku.onBinderReceived(binder, context.packageName)
    }

    /** Migrates only activation settings and encrypted ADB credentials into direct-boot storage. */
    @Synchronized
    private fun preferences(context: Context): SharedPreferences {
        activationPreferences?.let { return it }
        val storage = context.createDeviceProtectedStorageContext()
        if (context.getSystemService(UserManager::class.java).isUserUnlocked) {
            check(storage.moveSharedPreferencesFrom(context.applicationContext, PREFS)) {
                "Cannot move system-service activation settings to device-protected storage"
            }
        }
        return storage.getSharedPreferences(PREFS, Context.MODE_PRIVATE).also { activationPreferences = it }
    }

    /** Retains external Shizuku until the user explicitly enables the embedded service. */
    fun selected(context: Context): Boolean = preferences(context).getBoolean("selected", false)

    /** Returns whether a previously enabled service should be restored after reboot. */
    fun restoreEnabled(context: Context): Boolean = selected(context) && preferences(context).getBoolean("restore", false)

    /** Builds argv-safe starter input from the current installed APK and native library directory. */
    fun command(context: Context): String {
        val info = context.applicationInfo
        return listOf("${info.nativeLibraryDir}/liboperit_shizuku.so", info.sourceDir, context.packageName)
            .joinToString(" ") { "'" + it.replace("'", "'\\''") + "'" }
    }

    /** Creates or reloads the encrypted upstream ADB identity instead of generating one per launch. */
    private fun key(context: Context) = AdbKey(PreferenceAdbKeyStore(preferences(context)), "operit")

    /** Selects the private backend before accepting a computer or in-app activation. */
    fun selectEmbedded(context: Context) {
        if (selected(context)) return
        externalBinder = Shizuku.getBinder()
        preferences(context).edit().putBoolean("selected", true).commit()
        Shizuku.onBinderReceived(null, context.packageName)
    }

    /** Records the computer ADB activation route before displaying its command. */
    fun selectComputerActivation(context: Context) {
        selectEmbedded(context)
        preferences(context).edit().putString("transport", "adb").apply()
    }

    /** Waits for the actual owner Binder handshake rather than declaring success after process spawn. */
    private fun awaitReady(context: Context, start: () -> Unit): Boolean {
        if (selected(context) && Shizuku.pingBinder()) {
            check(Shizuku.getUid() == 2000 && Shizuku.checkSelfPermission() == 0) { "Embedded service identity or authorization is invalid" }
            return false
        }
        val ready = CountDownLatch(1)
        val listener = Shizuku.OnBinderReceivedListener { ready.countDown() }
        Shizuku.addBinderReceivedListener(listener)
        try {
            selectEmbedded(context)
            start()
            check(ready.await(15, TimeUnit.SECONDS)) { "Embedded service did not connect within 15 seconds" }
            check(Shizuku.getUid() == 2000 && Shizuku.checkSelfPermission() == 0) { "Embedded service identity or authorization is invalid" }
            return true
        } finally {
            Shizuku.removeBinderReceivedListener(listener)
        }
    }

    /** Uses the explicit Root activation action without enabling Root for ordinary tools. */
    fun startRoot(context: Context) {
        val started = awaitReady(context) {
            val result = AndroidRootShell.execute(AndroidPrivilegedCommandTarget.RootAuto, command(context), 15000L,
                AndroidPrivilegeAuthorization.rootExecutionSettings(context))
            check(result.exitCode == 0) { "Embedded service activation failed: ${result.stderrText()}" }
        }
        if (started) preferences(context).edit().putString("transport", "root").apply()
        enableAutomaticRestore(context)
    }

    /** Pairs the saved local ADB identity using the system-provided pairing code and port. */
    fun pair(context: Context, port: Int, code: String) {
        check(Build.VERSION.SDK_INT >= 30) { "Wireless pairing requires Android 11 or later" }
        require(port in 1..65535 && code.matches(Regex("[0-9]{6}"))) { "Invalid pairing port or code" }
        AdbPairingClient("127.0.0.1", port, code, key(context)).use { check(it.start()) { "ADB pairing failed" } }
    }

    /** Starts over local ADB and detaches before the activation connection closes. */
    fun startAdb(context: Context, port: Int) {
        require(port in 1..65535) { "Invalid ADB connection port" }
        val started = awaitReady(context) { launchAdb(context, port) }
        if (started) preferences(context).edit().putString("transport", "adb").apply()
        enableAutomaticRestore(context)
    }

    /** Sends the detached activation command using the same paired key in either boot phase. */
    private fun launchAdb(context: Context, port: Int) {
        AdbClient("127.0.0.1", port, key(context)).use { adb ->
            adb.connect()
            adb.shellCommand(command(context)) { Log.d(TAG, it.toString(Charsets.UTF_8)) }
        }
    }

    /** Discovers the current local TLS ADB port rather than persisting a stale ephemeral port. */
    fun discoverAndStart(context: Context) {
        startAdb(context, discoverPort(context))
    }

    /** Bounds local discovery so Android owns retries instead of an application polling loop. */
    private fun discoverPort(context: Context): Int {
        check(Build.VERSION.SDK_INT >= 30) { "Wireless activation requires Android 11 or later" }
        val found = CountDownLatch(1)
        var port = -1
        val mdns = AdbMdns(context, AdbMdns.TLS_CONNECT) { value -> if (value > 0) { port = value; found.countDown() } }
        main.post { mdns.start() }
        try {
            check(found.await(15, TimeUnit.SECONDS)) { "No local wireless debugging service found; enable it in developer settings" }
            return port
        } finally { main.post { mdns.stop() } }
    }

    /** Prepares reboot recovery as part of activation without a separate user-facing switch. */
    fun enableAutomaticRestore(context: Context) {
        if (!restoreEnabled(context)) setRestore(context, true)
    }

    /** Retains recovery settings while active and restores them after an explicit stop. */
    private fun setRestore(context: Context, enabled: Boolean) {
        if (enabled) {
            check(selected(context) && preferences(context).contains("transport")) { "Start the embedded service before enabling recovery" }
            if (preferences(context).getString("transport", null) == "adb") {
                check(Shizuku.pingBinder() && Shizuku.getUid() == 2000) { "Start the embedded service before enabling wireless recovery" }
                val grant = AndroidTerminalPrivileges.shared(context).executePiped(
                    "/system/bin/pm grant ${context.packageName} android.permission.WRITE_SECURE_SETTINGS", 10_000L)
                check(grant.exitCode == 0) { "Wireless recovery permission rejected: ${grant.stderrText()}" }
                check(context.checkSelfPermission(android.Manifest.permission.WRITE_SECURE_SETTINGS) == 0) {
                    "Wireless recovery requires WRITE_SECURE_SETTINGS"
                }
                val settings = preferences(context)
                if (!settings.getBoolean("adb_timeout_saved", false)) {
                    check(settings.edit().putBoolean("adb_timeout_saved", true)
                        .putString("adb_timeout_before_restore", Settings.Global.getString(context.contentResolver, "adb_allowed_connection_time"))
                        .commit()) { "Cannot save the original ADB authorization timeout" }
                }
                check(Settings.Global.putLong(context.contentResolver, "adb_allowed_connection_time", 0L)) {
                    "Cannot retain ADB pairing authorization"
                }
            }
        } else {
            val settings = preferences(context)
            if (settings.getBoolean("adb_timeout_saved", false)) {
                if (Settings.Global.getString(context.contentResolver, "adb_allowed_connection_time") == "0") {
                    check(Settings.Global.putString(context.contentResolver, "adb_allowed_connection_time",
                        settings.getString("adb_timeout_before_restore", null))) { "Cannot restore the ADB authorization timeout" }
                }
                settings.edit().remove("adb_timeout_saved").remove("adb_timeout_before_restore").apply()
            }
        }
        preferences(context).edit().putBoolean("restore", enabled).apply()
        if (enabled) scheduleRestore(context) else EmbeddedShizukuRestoreService.cancel(context)
    }

    /** Schedules recovery with network constraints only for the saved wireless transport. */
    fun scheduleRestore(context: Context) {
        EmbeddedShizukuRestoreService.schedule(context, preferences(context).getString("transport", null) == "adb")
    }

    /** Restores the same selected activation mode without switching identities or replaying commands. */
    fun restore(context: Context) {
        if (!selected(context)) return
        if (Shizuku.pingBinder()) {
            enableAutomaticRestore(context)
            return
        }
        if (!restoreEnabled(context)) return
        when (preferences(context).getString("transport", null)) {
            "root" -> startRoot(context)
            "adb" -> {
                armWirelessDebugging(context)
                discoverAndStart(context)
            }
            else -> error("No saved activation method")
        }
    }

    /** Restores developer options and both ADB switches using the retained non-Root permission. */
    private fun armWirelessDebugging(context: Context) {
        check(context.checkSelfPermission(android.Manifest.permission.WRITE_SECURE_SETTINGS) == 0) {
            "Wireless recovery requires WRITE_SECURE_SETTINGS"
        }
        val resolver = context.contentResolver
        check(Settings.Global.putInt(resolver, Settings.Global.DEVELOPMENT_SETTINGS_ENABLED, 1)) { "Cannot enable developer options" }
        check(Settings.Global.putInt(resolver, Settings.Global.ADB_ENABLED, 1)) { "Cannot enable ADB" }
        check(Settings.Global.putLong(resolver, "adb_allowed_connection_time", 0L)) { "Cannot retain ADB pairing authorization" }
        check(Settings.Global.putInt(resolver, "adb_wifi_enabled", 1)) { "Cannot enable wireless debugging" }
    }

    /** Attempts paired ADB activation before unlock without loading the credential-protected Core. */
    fun restoreBeforeUnlock(context: Context) {
        if (!restoreEnabled(context) || preferences(context).getString("transport", null) != "adb") return
        armWirelessDebugging(context)
        val port = discoverPort(context)
        if (restoreEnabled(context)) launchAdb(context, port)
        // The owner Provider is available after unlock; that phase verifies the Binder handshake.
    }

    /** Stops only the selected embedded backend and honors deliberate stops during reboot recovery. */
    fun stop(context: Context) {
        check(selected(context)) { "Embedded service is not selected" }
        setRestore(context, false)
        if (Shizuku.pingBinder()) {
            try { Shizuku.exit() }
            catch (error: RuntimeException) {
                // A successful remote exit can kill Binder before its synchronous reply arrives.
                if (error.cause !is DeadObjectException) throw error
            }
        }
    }

    /** Switches the explicit selection without killing or modifying an external Shizuku server. */
    fun selectExternal(context: Context) {
        if (selected(context)) stop(context)
        preferences(context).edit().putBoolean("selected", false).apply()
        Shizuku.onBinderReceived(externalBinder?.takeIf { it.isBinderAlive }, context.packageName)
    }

    /** Connects Sui only after the user explicitly selects that external backend. */
    fun selectSui(context: Context) {
        selectExternal(context)
        check(rikka.sui.Sui.init(context.packageName)) { "Sui is not available" }
        externalBinder = Shizuku.getBinder()
    }
}
