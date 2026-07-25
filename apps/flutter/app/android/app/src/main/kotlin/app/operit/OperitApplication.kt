package app.operit

import android.app.Application
import android.app.Activity
import android.os.Bundle
import android.os.Process
import org.json.JSONObject

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
        AndroidCoreRuntime
            .get(applicationContext)
            .emitRuntimeEvent(RuntimeEvents.androidLifecycle(topic, payload))
    }
}
