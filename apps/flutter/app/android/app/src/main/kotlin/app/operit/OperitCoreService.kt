package app.operit

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import java.util.concurrent.atomic.AtomicBoolean

class OperitCoreService : Service() {
    companion object {
        private const val notificationChannelId = "operit_core_runtime"
        private const val notificationId = 2401
        private const val actionStart = "app.operit.action.START_CORE_RUNTIME"
        private const val actionBootCompleted = "app.operit.action.START_CORE_RUNTIME_AFTER_BOOT"

        /** Starts the process-level Core foreground service. */
        fun start(context: Context) {
            val applicationContext = context.applicationContext
            val intent = Intent(applicationContext, OperitCoreService::class.java).apply {
                action = actionStart
            }
            startService(applicationContext, intent)
        }

        /** Starts Core and requests one boot-completed event after Runtime initialization. */
        fun startAfterBoot(context: Context) {
            val applicationContext = context.applicationContext
            val intent = Intent(applicationContext, OperitCoreService::class.java).apply {
                action = actionBootCompleted
            }
            startService(applicationContext, intent)
        }

        /** Starts the Android service using the required foreground-service API. */
        private fun startService(applicationContext: Context, intent: Intent) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                applicationContext.startForegroundService(intent)
            } else {
                applicationContext.startService(intent)
            }
        }
    }

    private val mainHandler = Handler(Looper.getMainLooper())
    private val runtimeStartRequested = AtomicBoolean(false)
    private val runtimeReady = AtomicBoolean(false)
    private val pendingBootCompleted = AtomicBoolean(false)
    private val destroyed = AtomicBoolean(false)
    private lateinit var runtimeHost: AndroidRuntimeHost

    /** Creates the foreground service and starts the persistent Runtime. */
    override fun onCreate() {
        super.onCreate()
        runtimeHost = AndroidCoreRuntime.get(applicationContext)
        startForeground(notificationId, createNotification())
        ensureRuntimeStarted()
    }

    /** Keeps the Core service active across task removal and process recreation. */
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == actionBootCompleted) {
            pendingBootCompleted.set(true)
        }
        ensureRuntimeStarted()
        dispatchPendingBootCompleted()
        return START_STICKY
    }

    /** Exposes no binder because Flutter communicates through its platform channel. */
    override fun onBind(intent: Intent?): IBinder? = null

    /** Releases Android event receivers owned by the foreground service. */
    override fun onDestroy() {
        destroyed.set(true)
        HostEventBridge.clear(applicationContext)
        super.onDestroy()
    }

    /** Starts Runtime initialization once and installs system event receivers. */
    private fun ensureRuntimeStarted() {
        if (!runtimeStartRequested.compareAndSet(false, true)) {
            return
        }
        runtimeHost.runBackground {
            try {
                runtimeHost.ensureRuntimeHandle()
                mainHandler.post {
                    if (!destroyed.get()) {
                        HostEventBridge.startHostEventReceivers(
                            applicationContext,
                            runtimeHost::ensureRuntimeHandle,
                        )
                        runtimeReady.set(true)
                        dispatchPendingBootCompleted()
                    }
                }
            } catch (error: Throwable) {
                AndroidClientLogger.e(
                    applicationContext,
                    "OperitCoreService",
                    "Core Runtime startup failed: ${error.stackTraceToString()}",
                )
            }
        }
    }

    /** Emits the manifest boot event exactly once after the native Runtime becomes ready. */
    private fun dispatchPendingBootCompleted() {
        if (!runtimeReady.get() || !pendingBootCompleted.compareAndSet(true, false)) {
            return
        }
        runtimeHost.runBackground {
            val event = RuntimeEvents.androidBroadcast(
                RuntimeEvents.Topic.SYSTEM_BOOT_COMPLETED,
                org.json.JSONObject().put("bootCompleted", true),
            )
            RuntimeEvents.emit(runtimeHost.ensureRuntimeHandle(), event)
        }
    }

    /** Creates the persistent Core service notification. */
    private fun createNotification(): Notification {
        val notificationManager =
            getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            notificationManager.createNotificationChannel(
                NotificationChannel(
                    notificationChannelId,
                    getString(R.string.operit_core_service_name),
                    NotificationManager.IMPORTANCE_LOW,
                ),
            )
        }
        val launchIntent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val pendingIntent =
            PendingIntent.getActivity(
                this,
                0,
                launchIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
        val builder =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                Notification.Builder(this, notificationChannelId)
            } else {
                Notification.Builder(this)
            }
        return builder
            .setContentTitle(getString(R.string.operit_core_service_name))
            .setContentText(getString(R.string.operit_core_service_status))
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()
    }
}
