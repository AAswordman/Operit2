package app.operit

import android.content.Context
import android.content.pm.PackageManager
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import io.flutter.plugin.common.MethodChannel
import org.json.JSONObject
import java.io.File
import java.util.ArrayDeque
import java.util.Locale
import java.util.concurrent.ExecutorService

class AndroidRuntimeHost(context: Context) {
    private companion object {
        private const val TAG = "AndroidRuntimeHost"
    }

    private val applicationContext = context.applicationContext
    private val mainHandler = Handler(Looper.getMainLooper())
    // Main-thread callbacks share only the short-lived state lock.
    private val runtimeLock = Any()
    private val runtimeCreationLock = Any()
    private val runtimeExecutor: ExecutorService = HostTaskExecutor()
    private var runtimeHandle: Long = 0
    private var runtimeStarting = false
    private val pendingRuntimeEvents = ArrayDeque<String>()
    private var configuredRuntimeRoot: File? = null
    private var configuredWorkspaceRoot: File? = null
    @Volatile
    private var runtimeStartupState = "preparing"
    @Volatile
    private var runtimeStartupMessage = "正在准备本地运行时"

    private val documentFileSystem by lazy { AndroidDocumentFileSystem(applicationContext) }
    private val terminalPrivileges by lazy { AndroidTerminalPrivileges(applicationContext) }

    /** Serves native terminal privilege probes and remote PTY lifecycle operations. */
    fun terminalPrivilegeOperation(request: String): String = terminalPrivileges.execute(request)

    /** JNI worker entry point for capability-backed filesystem resources. */
    fun fileSystemResourceOperation(request: String): String = documentFileSystem.execute(request)

    /** Keeps Wi-Fi multicast reception enabled for the native discovery daemon's lifetime. */
    fun acquireDiscoveryMulticastLock(): WifiManager.MulticastLock? {
        if (!applicationContext.packageManager.hasSystemFeature(PackageManager.FEATURE_WIFI)) {
            return null
        }
        val manager = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
        return manager.createMulticastLock("operit-service-discovery").apply {
            setReferenceCounted(false)
            acquire()
        }
    }

    /** Installs storage roots and accepts repeated identical configuration. */
    fun setStorageRoots(runtimePath: String?, workspacePath: String?) {
        val runtimeRoot = requiredAbsoluteRoot(runtimePath, "runtimeRoot")
        val workspaceRoot = requiredAbsoluteRoot(workspacePath, "workspaceRoot")
        synchronized(runtimeLock) {
            if (runtimeStarting || runtimeHandle != 0L) {
                if (configuredRuntimeRoot != runtimeRoot ||
                    configuredWorkspaceRoot != workspaceRoot
                ) {
                    throw IllegalStateException("Runtime and workspace roots cannot change after runtime startup begins")
                }
            }
            configuredRuntimeRoot = runtimeRoot
            configuredWorkspaceRoot = workspaceRoot
        }
    }

    /** Publishes Core-owned work revisions even when no Flutter engine is attached. */
    fun publishBackgroundWork(encoded: String) {
        mainHandler.post {
            try {
                app.operit.keep_alive.KeepAliveServiceHost.publishWorkSnapshot(encoded)
            } catch (error: RuntimeException) {
                Log.e(TAG, "Core work presentation failed", error)
                app.operit.keep_alive.KeepAliveServiceHost.reportWorkError(error.toString())
            }
        }
    }

    /** Stops the execution selected by an Android notification action. */
    fun stopBackgroundWork(workId: String) {
        runBackground {
            try {
                val response = JSONObject(OperitRuntimeNative.stopBackgroundWork(ensureRuntimeHandle(), workId))
                check(response.getBoolean("ok")) { response.getString("error") }
            } catch (error: Exception) {
                Log.e(TAG, "Background work stop failed: $workId", error)
                mainHandler.post { app.operit.keep_alive.KeepAliveServiceHost.reportWorkError(error.toString()) }
            }
        }
    }

    /** Returns whether Flutter has installed the roots required by the native Runtime. */
    fun isStorageConfigured(): Boolean {
        synchronized(runtimeLock) {
            return configuredRuntimeRoot != null && configuredWorkspaceRoot != null
        }
    }

    /** Serializes native creation without blocking main-thread storage and lifecycle callbacks. */
    fun ensureRuntimeHandle(): Long {
        synchronized(runtimeLock) {
            if (runtimeHandle != 0L) {
                return runtimeHandle
            }
        }
        synchronized(runtimeCreationLock) {
            val storageRoots = synchronized(runtimeLock) {
                if (runtimeHandle != 0L) {
                    return runtimeHandle
                }
                val roots = configuredStorageRootsLocked()
                runtimeStarting = true
                roots
            }
            val startedAtMillis = System.currentTimeMillis()
            updateRuntimeStartupStatus("preparingAssets", "正在准备本地运行时资源")
            Log.i(TAG, "native runtime create start")
            try {
                val leaseResponse = OperitRuntimeNative.dataRescue(
                    defaultRuntimeRootPath(),
                    "{\"operation\":\"runtimeLease\"}",
                    this,
                )
                if (!JSONObject(leaseResponse).optBoolean("ok", false)) {
                    throw IllegalStateException("Runtime storage lease was not acquired: $leaseResponse")
                }
                val paths = prepareAndroidRuntimePaths(storageRoots.first, storageRoots.second)
                updateRuntimeStartupStatus("initializingCore", "正在初始化本地核心服务")
                Log.i(
                    TAG,
                    "native runtime assets ready elapsedMs=" +
                        (System.currentTimeMillis() - startedAtMillis),
                )
                val createdHandle = OperitRuntimeNative.create(
                    paths.runtimeRoot.absolutePath,
                    paths.workspaceRoot.absolutePath,
                    Build.MODEL,
                    this,
                )
                if (createdHandle == 0L) {
                    throw IllegalStateException(OperitRuntimeNative.createError())
                }
                synchronized(runtimeLock) {
                    runtimeHandle = createdHandle
                    runtimeStarting = false
                    flushPendingRuntimeEventsLocked()
                }
                updateRuntimeStartupStatus("ready", "本地运行时已就绪")
                Log.i(
                    TAG,
                    "native runtime create done elapsedMs=" +
                        (System.currentTimeMillis() - startedAtMillis),
                )
                return createdHandle
            } catch (error: Throwable) {
                synchronized(runtimeLock) {
                    runtimeStarting = false
                }
                updateRuntimeStartupStatus("failed", "本地运行时启动失败")
                throw error
            }
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

    /** Queues Android host and lifecycle events without waiting for native Core processing. */
    fun emitRuntimeEvent(event: org.json.JSONObject) {
        val eventJson = event.toString()
        synchronized(runtimeLock) {
            val handle = runtimeHandle
            if (handle == 0L) {
                pendingRuntimeEvents.addLast(eventJson)
                return
            }
            scheduleRuntimeEventLocked(handle, eventJson)
        }
    }

    /** Schedules queued host events while the caller holds the state lock. */
    private fun flushPendingRuntimeEventsLocked() {
        while (pendingRuntimeEvents.isNotEmpty()) {
            scheduleRuntimeEventLocked(runtimeHandle, pendingRuntimeEvents.removeFirst())
        }
    }

    /** Dispatches one host event to the worker pool without running JNI under the state lock. */
    private fun scheduleRuntimeEventLocked(handle: Long, eventJson: String) {
        runtimeExecutor.execute {
            try {
                val response = OperitRuntimeNative.emitRuntimeEvent(handle, eventJson)
                val result = org.json.JSONObject(response)
                check(result.getBoolean("ok")) {
                    result.optString("error", "Core rejected Android runtime event")
                }
            } catch (error: Throwable) {
                Log.e(TAG, "failed to deliver Android runtime event", error)
            }
        }
    }

    /** Prepares Android runtime assets for the configured storage roots. */
    fun prepareAndroidRuntimePaths(): AndroidRuntimePaths {
        val roots = synchronized(runtimeLock) { configuredStorageRootsLocked() }
        return prepareAndroidRuntimePaths(roots.first, roots.second)
    }

    /** Captures both configured roots while the caller holds the state lock. */
    private fun configuredStorageRootsLocked(): Pair<File, File> {
        val runtimeRoot = configuredRuntimeRoot
            ?: throw IllegalStateException("runtimeRoot is not configured")
        val workspaceRoot = configuredWorkspaceRoot
            ?: throw IllegalStateException("workspaceRoot is not configured")
        return Pair(runtimeRoot, workspaceRoot)
    }

    /** Prepares filesystem assets outside the state lock using one immutable root snapshot. */
    private fun prepareAndroidRuntimePaths(
        runtimeRoot: File,
        workspaceRoot: File,
    ): AndroidRuntimePaths {
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

    /** Returns the platform default Runtime root used to locate client bootstrap state. */
    fun defaultRuntimeRootPath(): String {
        return File(applicationContext.filesDir, "runtime").absolutePath
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
        synchronized(runtimeCreationLock) {
            val handle = synchronized(runtimeLock) {
                val activeHandle = runtimeHandle
                runtimeHandle = 0
                activeHandle
            }
            if (handle != 0L) {
                OperitRuntimeNative.destroy(handle)
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
