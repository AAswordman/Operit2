package app.operit

import android.content.Context

object AndroidCoreRuntime {
    @Volatile private var runtimeHost: AndroidRuntimeHost? = null

    /** Returns the process-level Android Runtime host. */
    fun get(context: Context): AndroidRuntimeHost {
        val existing = runtimeHost
        if (existing != null) {
            return existing
        }
        return synchronized(this) {
            val current = runtimeHost
            if (current != null) {
                current
            } else {
                val created = AndroidRuntimeHost(context.applicationContext)
                val storedRoots = AndroidRuntimeStorageConfigStore.read(context)
                if (storedRoots != null) {
                    created.setStorageRoots(
                        storedRoots.runtimeRoot,
                        storedRoots.workspaceRoot,
                    )
                }
                runtimeHost = created
                created
            }
        }
    }
}
