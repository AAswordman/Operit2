package app.operit

/** Supplies bounded worker capacity for JVM process tests without loading the Android JNI library. */
object HostTaskScheduler {
    private val workers = java.util.concurrent.Executors.newFixedThreadPool(16) { task ->
        Thread(task, "host-service-test").apply { isDaemon = true }
    }
    /** Starts all independently blocking process stages on the test scheduler. */
    fun executeServices(tasks: Array<Runnable>) { tasks.forEach { workers.execute(it) } }
}
