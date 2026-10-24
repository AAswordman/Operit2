package app.operit

import android.app.Service
import app.operit.keep_alive.KeepAliveServiceHost
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import java.util.concurrent.atomic.AtomicBoolean

class OperitCoreService : Service() {
    companion object {
        private const val TAG = "OperitCoreService"
        private const val notificationChannelId = "operit_core_runtime"
        private const val notificationId = 2401
        private const val actionStart = "app.operit.action.START_CORE_RUNTIME"

        /** Starts the process-level Core foreground service. */
        fun start(context: Context) {
            val applicationContext = context.applicationContext
            val intent = Intent(applicationContext, OperitCoreService::class.java).apply {
                action = actionStart
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
    private val destroyed = AtomicBoolean(false)
    private lateinit var runtimeHost: AndroidRuntimeHost

    /** Creates the foreground service and starts the persistent Runtime. */
    override fun onCreate() {
        super.onCreate()
        runtimeHost = AndroidCoreRuntime.get(applicationContext)
        KeepAliveServiceHost.attach(
            this,
            notificationChannelId,
            notificationId,
            getString(R.string.operit_core_service_name),
            getString(R.string.operit_core_service_status),
            R.drawable.ic_stat_operit,
        )
        ensureRuntimeStarted()
    }

    /** Keeps the Core service active across task removal and process recreation. */
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        ensureRuntimeStarted()
        when (intent?.action) {
            KeepAliveServiceHost.STOP_WORK_ACTION -> {
                val workId = requireNotNull(intent.getStringExtra(KeepAliveServiceHost.WORK_ID_EXTRA))
                runtimeHost.stopBackgroundWork(workId)
            }
            KeepAliveServiceHost.DISMISS_WORK_ACTION -> {
                val workId = requireNotNull(intent.getStringExtra(KeepAliveServiceHost.WORK_ID_EXTRA))
                KeepAliveServiceHost.dismissWork(workId)
            }
        }
        return START_STICKY
    }

    /** Exposes no binder because Flutter communicates through its platform channel. */
    override fun onBind(intent: Intent?): IBinder? = null

    /** Releases Android event receivers owned by the foreground service. */
    override fun onDestroy() {
        destroyed.set(true)
        HostEventBridge.clear(applicationContext)
        KeepAliveServiceHost.detach(this)
        super.onDestroy()
    }

    /** Starts Runtime initialization once and installs system event receivers. */
    private fun ensureRuntimeStarted() {
        if (!runtimeHost.isStorageConfigured()) {
            Log.i(TAG, "Core Runtime startup waiting for Flutter storage roots")
            return
        }
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
                            runtimeHost::emitRuntimeEvent,
                        )
                    }
                }
            } catch (error: Throwable) {
                Log.e(TAG, "Core Runtime startup failed", error)
                mainHandler.post { stopSelf() }
                NativeCrashActivity.start(
                    applicationContext,
                    "Core Runtime startup failed\n\n${error.stackTraceToString()}",
                )
            }
        }
    }

}
