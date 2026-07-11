package app.operit

import android.content.Context

data class AndroidRuntimeStorageRoots(
    val runtimeRoot: String,
    val workspaceRoot: String,
)

object AndroidRuntimeStorageConfigStore {
    private const val preferencesName = "operit_runtime_storage"
    private const val runtimeRootKey = "runtime_root"
    private const val workspaceRootKey = "workspace_root"

    /** Persists normalized runtime and workspace roots for process restarts. */
    fun write(context: Context, roots: AndroidRuntimeStorageRoots) {
        val committed =
            context.applicationContext
                .getSharedPreferences(preferencesName, Context.MODE_PRIVATE)
                .edit()
                .putString(runtimeRootKey, roots.runtimeRoot)
                .putString(workspaceRootKey, roots.workspaceRoot)
                .commit()
        check(committed) { "failed to persist Android runtime storage roots" }
    }

    /** Reads persisted runtime and workspace roots. */
    fun read(context: Context): AndroidRuntimeStorageRoots? {
        val preferences =
            context.applicationContext.getSharedPreferences(
                preferencesName,
                Context.MODE_PRIVATE,
            )
        val runtimeRoot = preferences.getString(runtimeRootKey, null)
        val workspaceRoot = preferences.getString(workspaceRootKey, null)
        if (runtimeRoot == null && workspaceRoot == null) {
            return null
        }
        check(runtimeRoot != null && workspaceRoot != null) {
            "Android runtime storage roots are incomplete"
        }
        return AndroidRuntimeStorageRoots(
            runtimeRoot = runtimeRoot,
            workspaceRoot = workspaceRoot,
        )
    }
}
