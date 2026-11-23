package app.operit

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import org.json.JSONObject

internal object TermuxTerminalLauncher {
    private const val PACKAGE = "com.termux"
    private const val PERMISSION = "com.termux.permission.RUN_COMMAND"
    private const val EXTRA = "com.termux.RUN_COMMAND_"
    private const val FILES = "/data/data/com.termux/files"

    /** Starts one background helper through Termux's documented Android service. */
    fun start(context: Context, request: JSONObject) {
        val intent = Intent("com.termux.RUN_COMMAND").apply {
            component = ComponentName(PACKAGE, "$PACKAGE.app.RunCommandService")
            putExtra("${EXTRA}PATH", "$FILES/usr/bin/python")
            putExtra("${EXTRA}WORKDIR", "$FILES/home")
            putExtra("${EXTRA}BACKGROUND", true)
            putExtra("${EXTRA}COMMAND_LABEL", "Operit terminal")
            putExtra("${EXTRA}ARGUMENTS", arrayOf(
                "-I", "-S", "-c", request.getString("script"),
                request.getInt("port").toString(), request.getString("token"),
                request.getInt("rows").toString(), request.getInt("cols").toString(),
                request.getString("prompt"), request.getString("directory"),
            ))
        }
        check(context.packageManager.resolveService(intent, 0) != null) {
            "Install a Termux build that supports RUN_COMMAND, then open it once."
        }
        check(context.checkSelfPermission(PERMISSION) == PackageManager.PERMISSION_GRANTED) {
            "Grant Operit the Termux permission (Run commands in Termux) in Android app settings."
        }
        val service = if (Build.VERSION.SDK_INT >= 26) context.startForegroundService(intent) else context.startService(intent)
        checkNotNull(service) { "Android could not start the Termux command service." }
    }
}
