package app.operit

import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicInteger

class AndroidRuntimeHost(private val activity: MainActivity) {
    private val runtimeLock = Any()
    private val runtimeThreadIndex = AtomicInteger(0)
    private val runtimeExecutor: ExecutorService =
        Executors.newFixedThreadPool(8) { runnable ->
            Thread(runnable, "operit-runtime-${runtimeThreadIndex.incrementAndGet()}")
        }
    private var runtimeHandle: Long = 0

    fun ensureRuntimeHandle(): Long {
        synchronized(runtimeLock) {
            if (runtimeHandle != 0L) {
                return runtimeHandle
            }
            val root = prepareAndroidRuntimePaths().storageRoot
            runtimeHandle = OperitRuntimeNative.create(root.absolutePath, activity)
            if (runtimeHandle == 0L) {
                throw IllegalStateException(OperitRuntimeNative.createError())
            }
            return runtimeHandle
        }
    }

    fun runRuntime(result: MethodChannel.Result, block: () -> String) {
        runtimeExecutor.execute {
            try {
                val response = block()
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("RUNTIME_BRIDGE_ERROR", error.message, null)
                }
            }
        }
    }

    fun runBackground(block: () -> Unit) {
        runtimeExecutor.execute(block)
    }

    fun prepareAndroidRuntimePaths(): AndroidRuntimePaths {
        val root = activity.applicationContext.filesDir
        root.mkdirs()
        return AndroidRuntimeAssets.prepare(activity.applicationContext, root)
    }

    fun androidRuntimePathsMap(): Map<String, String> {
        val paths = prepareAndroidRuntimePaths()
        return mapOf(
            "abi" to paths.abi,
            "runtimeDir" to paths.runtimeDir.absolutePath,
            "rootfsDir" to paths.rootfsDir.absolutePath,
            "busybox" to paths.busybox.absolutePath,
            "bash" to paths.bash.absolutePath,
            "proot" to paths.proot.absolutePath,
            "loader" to paths.loader.absolutePath,
            "nativeLibraryDir" to paths.nativeLibraryDir.absolutePath,
            "storageRoot" to paths.storageRoot.absolutePath,
            "internalRoot" to paths.internalRoot.absolutePath,
            "tmpDir" to paths.tmpDir.absolutePath,
        )
    }

    fun destroy() {
        runtimeExecutor.shutdownNow()
        synchronized(runtimeLock) {
            if (runtimeHandle != 0L) {
                OperitRuntimeNative.destroy(runtimeHandle)
                runtimeHandle = 0
            }
        }
    }
}
