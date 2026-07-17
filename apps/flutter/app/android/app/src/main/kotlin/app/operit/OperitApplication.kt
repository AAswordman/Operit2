package app.operit

import android.app.Application
import android.os.Process

/** Launches the isolated crash Activity before Android terminates a failed process. */
class OperitApplication : Application() {
    /** Installs the process-wide Android uncaught-exception handler. */
    override fun onCreate() {
        super.onCreate()
        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            NativeCrashActivity.start(
                applicationContext,
                "Unhandled Android exception on ${thread.name}\n\n${error.stackTraceToString()}",
            )
            Process.killProcess(Process.myPid())
        }
    }
}
