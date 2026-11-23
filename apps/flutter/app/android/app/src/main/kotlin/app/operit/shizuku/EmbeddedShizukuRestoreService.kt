package app.operit.shizuku

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.job.JobInfo
import android.app.job.JobParameters
import android.app.job.JobScheduler
import android.app.job.JobService
import android.content.BroadcastReceiver
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat

/** Lets Android schedule explicit reboot recovery and retry when a network becomes available. */
class EmbeddedShizukuRestoreService : JobService() {
    private var worker: Thread? = null

    /** Performs one bounded activation attempt; retry timing belongs to JobScheduler. */
    override fun onStartJob(params: JobParameters): Boolean {
        worker = Thread({
            var retry = false
            try { EmbeddedShizuku.restore(this) }
            catch (error: Exception) {
                if (error is InterruptedException) Thread.currentThread().interrupt()
                retry = EmbeddedShizuku.restoreEnabled(this) && !Thread.currentThread().isInterrupted
                Log.w("OperitShell", "Service recovery failed", error)
                val manager = getSystemService(NotificationManager::class.java)
                if (Build.VERSION.SDK_INT >= 26) {
                    manager.createNotificationChannel(NotificationChannel("operit_shell_restore", "系统服务恢复", NotificationManager.IMPORTANCE_LOW))
                }
                manager.notify(4318, NotificationCompat.Builder(this, "operit_shell_restore")
                    .setSmallIcon(android.R.drawable.stat_notify_error).setContentTitle("Operit 系统服务等待恢复")
                    .setContentText(error.message).setContentIntent(PendingIntent.getActivity(this, 0,
                        Intent(this, EmbeddedShizukuActivity::class.java), PendingIntent.FLAG_IMMUTABLE)).build())
            } finally {
                if (!Thread.currentThread().isInterrupted) jobFinished(params, retry)
            }
        }, "operit-shell-restore").apply { start() }
        return true
    }

    /** Cancels an interrupted attempt while preserving the explicit recovery choice. */
    override fun onStopJob(params: JobParameters): Boolean {
        worker?.interrupt()
        return EmbeddedShizuku.restoreEnabled(this)
    }

    companion object {
        private const val JOB_ID = 4317

        /** Replaces one persisted recovery job rather than keeping an application process alive. */
        fun schedule(context: Context, wireless: Boolean) {
            val job = JobInfo.Builder(JOB_ID, ComponentName(context, EmbeddedShizukuRestoreService::class.java))
                .setPersisted(true)
                .setMinimumLatency(5_000)
                .setBackoffCriteria(30_000, JobInfo.BACKOFF_POLICY_EXPONENTIAL)
            if (wireless) {
                if (Build.VERSION.SDK_INT >= 28) {
                    job.setRequiredNetwork(NetworkRequest.Builder()
                        .addTransportType(NetworkCapabilities.TRANSPORT_WIFI).build())
                } else job.setRequiredNetworkType(JobInfo.NETWORK_TYPE_UNMETERED)
            }
            check(context.getSystemService(JobScheduler::class.java).schedule(job.build()) == JobScheduler.RESULT_SUCCESS) {
                "Android rejected service recovery scheduling"
            }
        }

        /** Removes pending restoration after a deliberate stop or backend switch. */
        fun cancel(context: Context) {
            context.getSystemService(JobScheduler::class.java).cancel(JOB_ID)
        }
    }
}

/** Starts paired ADB before unlock and schedules verified recovery after unlock. */
class EmbeddedShizukuBootReceiver : BroadcastReceiver() {
    /** Restores only the explicit user choice on reboot or package replacement. */
    override fun onReceive(context: Context, intent: Intent) {
        if (!EmbeddedShizuku.restoreEnabled(context)) return
        when (intent.action) {
            Intent.ACTION_LOCKED_BOOT_COMPLETED -> EmbeddedShizukuLockedBootService.start(context)
            Intent.ACTION_BOOT_COMPLETED, Intent.ACTION_USER_UNLOCKED, Intent.ACTION_MY_PACKAGE_REPLACED ->
                EmbeddedShizuku.scheduleRestore(context)
        }
    }
}
