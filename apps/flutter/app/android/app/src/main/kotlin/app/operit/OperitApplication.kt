package app.operit

import android.app.Activity
import android.app.ActivityManager
import android.app.Application
import android.os.Build
import android.os.Bundle
import android.os.Process
import android.util.Log
import org.json.JSONObject

/** Launches the isolated crash Activity before Android terminates a failed process. */
class OperitApplication : Application() {
    private val isCrashProcess: Boolean by lazy {
        val processName = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            Application.getProcessName()
        } else {
            val activityManager = getSystemService(ACTIVITY_SERVICE) as? ActivityManager
            activityManager?.runningAppProcesses
                ?.firstOrNull { it.pid == Process.myPid() }
                ?.processName
        }
        processName == "$packageName:crash"
    }

    private val isMaintenanceProcess: Boolean by lazy {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            Application.getProcessName() == "$packageName:maintenance"
        } else {
            val manager = getSystemService(ACTIVITY_SERVICE) as ActivityManager
            manager.runningAppProcesses?.firstOrNull { it.pid == Process.myPid() }?.processName == "$packageName:maintenance"
        }
    }

    /** Installs the process-wide Android uncaught-exception handler. */
    override fun onCreate() {
        super.onCreate()
        // The diagnostic process must not relaunch itself or initialize the Core host.
        if (isCrashProcess) return
        if (isMaintenanceProcess) return

        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            Log.e("OperitApplication", "Unhandled Android exception on ${thread.name}", error)
            try {
                NativeCrashActivity.start(
                    applicationContext,
                    "Unhandled Android exception on ${thread.name}\n\n${error.stackTraceToString()}",
                )
            } catch (presentationError: Throwable) {
                Log.e("OperitApplication", "Unable to open the native crash screen", presentationError)
            } finally {
                // Terminate the failed process even if crash presentation also fails.
                Process.killProcess(Process.myPid())
            }
        }
        registerActivityLifecycleCallbacks(object : ActivityLifecycleCallbacks {
            override fun onActivityCreated(activity: Activity, savedInstanceState: Bundle?) {
                emitActivityLifecycleEvent(RuntimeEvents.Topic.ACTIVITY_LIFECYCLE_CREATE, activity)
            }

            override fun onActivityStarted(activity: Activity) {
                emitActivityLifecycleEvent(RuntimeEvents.Topic.ACTIVITY_LIFECYCLE_START, activity)
            }

            override fun onActivityResumed(activity: Activity) {
                emitActivityLifecycleEvent(RuntimeEvents.Topic.ACTIVITY_LIFECYCLE_RESUME, activity)
            }

            override fun onActivityPaused(activity: Activity) {
                emitActivityLifecycleEvent(RuntimeEvents.Topic.ACTIVITY_LIFECYCLE_PAUSE, activity)
            }

            override fun onActivityStopped(activity: Activity) {
                emitActivityLifecycleEvent(RuntimeEvents.Topic.ACTIVITY_LIFECYCLE_STOP, activity)
            }

            override fun onActivitySaveInstanceState(activity: Activity, outState: Bundle) = Unit

            override fun onActivityDestroyed(activity: Activity) {
                emitActivityLifecycleEvent(RuntimeEvents.Topic.ACTIVITY_LIFECYCLE_DESTROY, activity)
            }
        })
    }

    override fun onLowMemory() {
        super.onLowMemory()
        emitLifecycleEvent(RuntimeEvents.Topic.APP_LIFECYCLE_LOW_MEMORY, JSONObject())
    }

    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)
        emitLifecycleEvent(
            RuntimeEvents.Topic.APP_LIFECYCLE_TRIM_MEMORY,
            JSONObject().put("level", level),
        )
    }

    private fun emitActivityLifecycleEvent(topic: String, activity: Activity) {
        emitLifecycleEvent(
            topic,
            JSONObject().put("activityClassName", activity.javaClass.name),
        )
    }

    private fun emitLifecycleEvent(topic: String, payload: JSONObject) {
        if (isCrashProcess) return
        if (isMaintenanceProcess) return
        AndroidCoreRuntime
            .get(applicationContext)
            .emitRuntimeEvent(RuntimeEvents.androidLifecycle(topic, payload))
    }
}
