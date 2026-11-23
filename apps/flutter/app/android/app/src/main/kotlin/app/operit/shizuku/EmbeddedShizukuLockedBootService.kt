package app.operit.shizuku

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat

/** Runs one paired ADB attempt in the existing isolated maintenance process before unlock. */
class EmbeddedShizukuLockedBootService : Service() {
    private var worker: Thread? = null

    /** Exposes no Binder interface to applications. */
    override fun onBind(intent: Intent?): IBinder? = null

    /** Meets the foreground deadline before starting bounded discovery and ADB activation. */
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val manager = getSystemService(NotificationManager::class.java)
        if (Build.VERSION.SDK_INT >= 26) manager.createNotificationChannel(
            NotificationChannel("operit_shell_restore", "系统服务恢复", NotificationManager.IMPORTANCE_LOW))
        val notification = NotificationCompat.Builder(this, "operit_shell_restore")
            .setSmallIcon(android.R.drawable.stat_notify_sync)
            .setContentTitle("Operit 正在恢复内置系统服务")
            .setContentText("通过已配对的 ADB 自动激活，无需 Root").setOngoing(true).build()
        if (Build.VERSION.SDK_INT >= 34) startForeground(4319, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        else startForeground(4319, notification)
        if (worker?.isAlive == true) return START_NOT_STICKY
        worker = Thread({
            try { EmbeddedShizuku.restoreBeforeUnlock(this) }
            catch (error: Exception) {
                if (error is InterruptedException) Thread.currentThread().interrupt()
                Log.w("OperitShell", "Activation before unlock did not complete; unlock recovery remains scheduled", error)
            } finally { stopSelf() }
        }, "operit-shell-direct-boot").apply { start() }
        return START_NOT_STICKY
    }

    /** Cancels a pending wait when Android stops this bounded boot attempt. */
    override fun onDestroy() {
        worker?.interrupt()
        super.onDestroy()
    }

    companion object {
        /** Requests direct-boot execution without holding a broadcast open during network IO. */
        fun start(context: Context) {
            val intent = Intent(context, EmbeddedShizukuLockedBootService::class.java)
            try {
                if (Build.VERSION.SDK_INT >= 26) context.startForegroundService(intent) else context.startService(intent)
            } catch (error: Exception) {
                Log.w("OperitShell", "Android rejected activation before unlock", error)
            }
        }
    }
}
