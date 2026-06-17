package app.operit.core.tools.system

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import app.operit.AndroidClientLogger
import app.operit.R

class ScreenCaptureService : Service() {
    companion object {
        private const val TAG = "ScreenCaptureService"
        private const val CHANNEL_ID = "ScreenCaptureChannel"
        private const val NOTIFICATION_ID = 2001
        private const val ACTION_START = "app.operit.action.SCREEN_CAPTURE_FGS_START"

        @Volatile
        var isMediaProjectionForegroundReady: Boolean = false
            private set

        fun start(context: Context) {
            isMediaProjectionForegroundReady = false
            val intent =
                Intent(context, ScreenCaptureService::class.java).apply { action = ACTION_START }
            try {
                context.startService(intent)
            } catch (_: IllegalStateException) {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    context.startForegroundService(intent)
                } else {
                    context.startService(intent)
                }
            }
        }

        fun stop(context: Context) {
            isMediaProjectionForegroundReady = false
            val intent = Intent(context, ScreenCaptureService::class.java)
            context.stopService(intent)
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        AndroidClientLogger.d(applicationContext, TAG, "ScreenCaptureService created")
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        AndroidClientLogger.d(applicationContext, TAG, "ScreenCaptureService started")
        if (intent?.action == ACTION_START) {
            startForegroundService()
            return START_NOT_STICKY
        }

        stopSelf()
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        super.onDestroy()
        isMediaProjectionForegroundReady = false
        AndroidClientLogger.d(applicationContext, TAG, "ScreenCaptureService destroyed")
    }

    private fun startForegroundService() {
        try {
            val notification = createNotification()
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(
                    NOTIFICATION_ID,
                    notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION,
                )
            } else {
                startForeground(NOTIFICATION_ID, notification)
            }
            isMediaProjectionForegroundReady = true
        } catch (error: Exception) {
            AndroidClientLogger.e(
                applicationContext,
                TAG,
                "Error starting foreground service: ${error.message.orEmpty()}",
            )
        }
    }

    private fun createNotification(): Notification {
        val builder =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                Notification.Builder(this, CHANNEL_ID)
            } else {
                @Suppress("DEPRECATION")
                Notification.Builder(this)
            }
        return builder
            .setContentTitle("Screen Capture Active")
            .setContentText("Operit is capturing screen content")
            .setSmallIcon(R.drawable.ic_launcher_simple_foreground)
            .setOngoing(true)
            .build()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val serviceChannel =
                NotificationChannel(
                    CHANNEL_ID,
                    "Screen Capture Service",
                    NotificationManager.IMPORTANCE_LOW,
                )
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(serviceChannel)
        }
    }
}
