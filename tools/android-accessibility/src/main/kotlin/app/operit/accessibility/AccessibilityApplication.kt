package app.operit.accessibility

import android.app.ActivityManager
import android.app.Application
import android.os.Build
import android.os.Process
import android.util.Log
import java.util.concurrent.atomic.AtomicBoolean

/** Captures fatal JVM exceptions from the companion's setup and service threads. */
class AccessibilityApplication : Application() {
    /** Installs the handler in the companion process without intercepting the report viewer. */
    override fun onCreate() {
        super.onCreate()
        if (processName() != packageName) return

        val androidHandler = checkNotNull(Thread.getDefaultUncaughtExceptionHandler())
        val reporting = AtomicBoolean()
        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            if (reporting.compareAndSet(false, true)) {
                try {
                    CrashReportStore(this).write(thread, error)
                    CrashReportActivity.open(this)
                } catch (reportingError: Exception) {
                    Log.e("OperitAccessibility", "Unable to publish the crash report", reportingError)
                } finally {
                    androidHandler.uncaughtException(thread, error)
                }
            }
        }
    }

    /** Identifies this process on every supported Android version. */
    private fun processName(): String? {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) return Application.getProcessName()
        val manager = getSystemService(ACTIVITY_SERVICE) as ActivityManager
        return manager.runningAppProcesses?.firstOrNull { it.pid == Process.myPid() }?.processName
    }
}
