package app.operit

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.util.Locale
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicInteger

class AndroidRuntimeHost(context: Context) {
    private companion object {
        private const val TAG = "AndroidRuntimeHost"
    }

    private val applicationContext = context.applicationContext
    private val mainHandler = Handler(Looper.getMainLooper())
    private val runtimeLock = Any()
    private val runtimeThreadIndex = AtomicInteger(0)
    private val runtimeExecutor: ExecutorService =
        Executors.newFixedThreadPool(8) { runnable ->
            Thread(runnable, "operit-runtime-${runtimeThreadIndex.incrementAndGet()}")
        }
    private var runtimeHandle: Long = 0
    private var configuredRuntimeRoot: File? = null
    private var configuredWorkspaceRoot: File? = null
    @Volatile
    private var runtimeStartupState = "preparing"
    @Volatile
    private var runtimeStartupMessage = "正在准备本地运行时"

    /** Installs storage roots and accepts repeated identical configuration. */
    fun setStorageRoots(runtimePath: String?, workspacePath: String?) {
        val runtimeRoot = requiredAbsoluteRoot(runtimePath, "runtimeRoot")
        val workspaceRoot = requiredAbsoluteRoot(workspacePath, "workspaceRoot")
        synchronized(runtimeLock) {
            if (runtimeHandle != 0L) {
                if (configuredRuntimeRoot != runtimeRoot ||
                    configuredWorkspaceRoot != workspaceRoot
                ) {
                    throw IllegalStateException("Runtime and workspace roots cannot change after runtime creation")
                }
            }
            configuredRuntimeRoot = runtimeRoot
            configuredWorkspaceRoot = workspaceRoot
        }
    }

    /** Returns the active native runtime handle, creating it when required. */
    fun ensureRuntimeHandle(): Long {
        synchronized(runtimeLock) {
            if (runtimeHandle != 0L) {
                return runtimeHandle
            }
            val startedAtMillis = System.currentTimeMillis()
            updateRuntimeStartupStatus("preparingAssets", "正在准备本地运行时资源")
            Log.i(TAG, "native runtime create start")
            try {
                val paths = prepareAndroidRuntimePaths()
                updateRuntimeStartupStatus("initializingCore", "正在初始化本地核心服务")
                Log.i(
                    TAG,
                    "native runtime assets ready elapsedMs=" +
                        (System.currentTimeMillis() - startedAtMillis),
                )
                runtimeHandle = OperitRuntimeNative.create(
                    paths.runtimeRoot.absolutePath,
                    paths.workspaceRoot.absolutePath,
                    this,
                )
                if (runtimeHandle == 0L) {
                    updateRuntimeStartupStatus("failed", "本地运行时启动失败")
                    throw IllegalStateException(OperitRuntimeNative.createError())
                }
            } catch (error: Throwable) {
                updateRuntimeStartupStatus("failed", "本地运行时启动失败")
                throw error
            }
            updateRuntimeStartupStatus("ready", "本地运行时已就绪")
            Log.i(
                TAG,
                "native runtime create done elapsedMs=" +
                    (System.currentTimeMillis() - startedAtMillis),
            )
            return runtimeHandle
        }
    }

    /** Returns the current native runtime startup stage for Flutter bootstrap UI. */
    fun runtimeStartupStatusMap(): Map<String, String> {
        return mapOf(
            "state" to runtimeStartupState,
            "message" to runtimeStartupMessage,
        )
    }

    /** Executes a runtime bridge call on the runtime executor. */
    fun <T> runRuntime(result: MethodChannel.Result, block: () -> T) {
        runtimeExecutor.execute {
            try {
                val response = block()
                mainHandler.post { result.success(response) }
            } catch (error: Throwable) {
                mainHandler.post {
                    result.error("RUNTIME_BRIDGE_ERROR", error.message, null)
                }
            }
        }
    }

    /** Executes host work on the runtime executor. */
    fun runBackground(block: () -> Unit) {
        runtimeExecutor.execute(block)
    }

    /** Prepares Android runtime assets for the configured storage roots. */
    fun prepareAndroidRuntimePaths(): AndroidRuntimePaths {
        val runtimeRoot = configuredRuntimeRoot
            ?: throw IllegalStateException("runtimeRoot is not configured")
        val workspaceRoot = configuredWorkspaceRoot
            ?: throw IllegalStateException("workspaceRoot is not configured")
        runtimeRoot.mkdirs()
        workspaceRoot.mkdirs()
        return AndroidRuntimeAssets.prepare(
            applicationContext,
            runtimeRoot,
            workspaceRoot,
        )
    }

    /** Returns the platform default runtime and workspace roots. */
    fun defaultStoragePathsMap(): Map<String, String> {
        val base = applicationContext.filesDir
        return mapOf(
            "runtimeRoot" to File(base, "runtime").absolutePath,
            "workspaceRoot" to File(base, "workspaces").absolutePath,
        )
    }

    /** Returns normalized storage paths without creating the runtime. */
    fun storagePathsMap(runtimePath: String?, workspacePath: String?): Map<String, String> {
        val runtimeRoot = requiredAbsoluteRoot(runtimePath, "runtimeRoot")
        val workspaceRoot = requiredAbsoluteRoot(workspacePath, "workspaceRoot")
        return mapOf(
            "runtimeRoot" to runtimeRoot.absolutePath,
            "workspaceRoot" to workspaceRoot.absolutePath,
        )
    }

    /** Returns Android runtime asset and storage diagnostics. */
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
            "runtimeRoot" to paths.runtimeRoot.absolutePath,
            "workspaceRoot" to paths.workspaceRoot.absolutePath,
            "internalRoot" to paths.internalRoot.absolutePath,
            "tmpDir" to paths.tmpDir.absolutePath,
        )
    }

    /** Releases the native runtime and executor. */
    fun destroy() {
        runtimeExecutor.shutdownNow()
        synchronized(runtimeLock) {
            if (runtimeHandle != 0L) {
                OperitRuntimeNative.destroy(runtimeHandle)
                runtimeHandle = 0
            }
        }
    }

    /** Reads host secret bytes for native Runtime calls. */
    fun readHostSecret(key: String): ByteArray? {
        return AndroidHostSecretStore.read(applicationContext, key)
    }

    /** Writes host secret bytes for native Runtime calls. */
    fun writeHostSecret(key: String, content: ByteArray) {
        AndroidHostSecretStore.write(applicationContext, key, content)
    }

    /** Deletes host secret bytes for native Runtime calls. */
    fun deleteHostSecret(key: String) {
        AndroidHostSecretStore.delete(applicationContext, key)
    }

    /** Returns the Android system locale language tag. */
    fun systemLanguageCode(): String = Locale.getDefault().toLanguageTag()

    /** Reconciles ToolPkg timers and intervals with Android AlarmManager. */
    fun replaceHostRuntimeEventSchedules(schedulesJson: String) {
        AndroidHostEventScheduler.replaceSchedules(applicationContext, schedulesJson)
    }

    /** Executes one installed local speech-to-text model request. */
    fun transcribeLocalSpeech(requestJson: String): String {
        return AndroidLocalInference.transcribe(requestJson)
    }

    /** Executes one installed local text-to-speech model request. */
    fun synthesizeLocalSpeech(requestJson: String): String {
        return AndroidLocalInference.synthesize(requestJson)
    }

    /** Updates the startup stage after a real native runtime lifecycle boundary. */
    private fun updateRuntimeStartupStatus(state: String, message: String) {
        runtimeStartupState = state
        runtimeStartupMessage = message
    }

    /** Validates one required absolute storage root. */
    private fun requiredAbsoluteRoot(path: String?, label: String): File {
        val value = path?.trim()
        require(!value.isNullOrEmpty()) { "$label is required" }
        val root = File(value)
        require(root.isAbsolute) { "$label must be an absolute path" }
        return root.absoluteFile.normalize()
    }
}
