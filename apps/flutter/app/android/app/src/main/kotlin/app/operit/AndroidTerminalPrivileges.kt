package app.operit

import android.content.ComponentName
import android.content.Context
import android.content.ServiceConnection
import android.os.Binder
import android.os.DeadObjectException
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.Parcel
import android.os.ParcelFileDescriptor
import android.util.Log
import app.operit.core.tools.system.AndroidCommandDeadline
import app.operit.core.tools.system.AndroidCommandProcessRunner
import app.operit.core.tools.system.AndroidPrivilegedCommandResult
import app.operit.core.tools.system.AndroidRootShell
import org.json.JSONArray
import org.json.JSONObject
import rikka.shizuku.Shizuku
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

/** Bridges only Android authorization and Binder mechanics; Rust chooses the transport. */
class AndroidTerminalPrivileges private constructor(private val context: Context) {
    companion object {
        @Volatile private var shared: AndroidTerminalPrivileges? = null

        /** One client per process: the privileged service accepts a single owner token. */
        fun shared(context: Context): AndroidTerminalPrivileges =
            shared ?: synchronized(this) {
                shared ?: AndroidTerminalPrivileges(context.applicationContext).also { shared = it }
            }
    }

    private val mainHandler = Handler(Looper.getMainLooper())
    private val owner = Binder()
    private data class RemoteSession(val binder: IBinder, val pid: Int)
    private data class RemoteService(val server: IBinder, val binder: IBinder)
    private val sessions = ConcurrentHashMap<Int, RemoteSession>()
    private val nextSession = AtomicInteger(1)
    private val nextBinding = AtomicInteger(1)
    @Volatile private var service: RemoteService? = null

    /** Executes one native terminal bridge request without requesting new permissions. */
    fun execute(encoded: String): String = try {
        val request = JSONObject(encoded)
        if (request.getString("operation") == "privileges") {
            val rootArguments = if (AndroidPrivilegeAuthorization.hasRootConsent(context)) {
                AndroidRootShell.terminalSuArguments(AndroidPrivilegeAuthorization.rootExecutionSettings(context), 10_000L)
            } else null
            JSONObject()
                .put("rootSuArguments", if (rootArguments == null) JSONObject.NULL else JSONArray(rootArguments))
                .put("shizukuAuthorized", AndroidPrivilegeAuthorization.isShizukuAuthorized())
                .toString()
        } else {
            remoteOperation(request).toString()
        }
    } catch (error: Exception) {
        if (error is InterruptedException) Thread.currentThread().interrupt()
        JSONObject().put("error", error.message ?: error.javaClass.simpleName).toString()
    }

    /** Obtains one service connection; an existing session never reconnects to a new process. */
    @Synchronized
    private fun bindService(): IBinder {
        check(AndroidPrivilegeAuthorization.isShizukuAuthorized()) { "Shizuku authorization is not granted" }
        val server = checkNotNull(Shizuku.getBinder()) { "Shizuku disconnected" }
        service?.let { if (it.server === server && it.binder.isBinderAlive) return it.binder }
        val args = Shizuku.UserServiceArgs(ComponentName(context, AndroidTerminalUserService::class.java))
            .daemon(false).processNameSuffix("terminal").version(1).tag("operit-terminal-${nextBinding.getAndIncrement()}")
        val ready = CompletableFuture<IBinder>()
        val connection = object : ServiceConnection {
            private var boundBinder: IBinder? = null

            /** Publishes the privileged Binder on the Android callback thread. */
            override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                if (Shizuku.getBinder() !== server) {
                    ready.completeExceptionally(IllegalStateException("Shizuku backend changed while binding"))
                    return
                }
                boundBinder = binder
                service = RemoteService(server, binder)
                ready.complete(binder)
            }
            /** Invalidates the connection without replacing any live terminal session. */
            override fun onServiceDisconnected(name: ComponentName) {
                if (service?.binder === boundBinder) service = null
            }
        }
        mainHandler.post {
            try {
                check(Shizuku.getBinder() === server) { "Shizuku backend changed before binding" }
                Shizuku.bindUserService(args, connection)
            }
            catch (error: Exception) { ready.completeExceptionally(error) }
        }
        try {
            return ready.get(10, TimeUnit.SECONDS)
        } catch (error: Exception) {
            mainHandler.post {
                try {
                    if (Shizuku.getBinder() === server) Shizuku.unbindUserService(args, connection, false)
                } catch (cleanupError: Exception) {
                    Log.w("OperitTerminal", "Failed to unbind terminal service after binding failed", cleanupError)
                }
            }
            throw error
        }
    }

    /** Transfers ordinary pipes while keeping the PTY and child owned by Shizuku. */
    private fun remoteOperation(request: JSONObject): JSONObject {
        val operation = request.getString("operation")
        val starting = operation == "start" || operation == "exec"
        if (starting) check(AndroidPrivilegeAuthorization.isShizukuAuthorized()) { "Shizuku authorization is not granted" }
        val handle = if (starting) nextSession.getAndIncrement() else request.getInt("pid")
        check(handle > 0) { "Shizuku terminal handle exhausted" }
        val session = if (starting) null else checkNotNull(sessions[handle]) { "Shizuku terminal session does not exist" }
        val binder = session?.binder ?: bindService()
        // A restarted Shizuku service must never receive an old session's recycled PID.
        session?.let { request.put("pid", it.pid) }
        val data = Parcel.obtain()
        val reply = Parcel.obtain()
        try {
            data.writeInterfaceToken(AndroidTerminalUserService.DESCRIPTOR)
            data.writeString(request.toString())
            data.writeStrongBinder(owner)
            check(binder.transact(1, data, reply, 0)) { "Shizuku terminal transaction rejected" }
            reply.readException()
            val response = JSONObject(checkNotNull(reply.readString()))
            if (starting && !response.has("error")) {
                checkNotNull(reply.readTypedObject(ParcelFileDescriptor.CREATOR)).use { first ->
                    checkNotNull(reply.readTypedObject(ParcelFileDescriptor.CREATOR)).use { second ->
                        sessions[handle] = RemoteSession(binder, response.getInt("pid"))
                        response.put("pid", handle)
                        if (operation == "exec") {
                            response.put("stdoutFd", first.detachFd())
                            response.put("stderrFd", second.detachFd())
                        } else {
                            response.put("readFd", first.detachFd())
                            response.put("writeFd", second.detachFd())
                        }
                    }
                }
            }
            return response
        } catch (error: DeadObjectException) {
            if (service?.binder === binder) service = null
            // The owning process is gone. Never send this session's PID to a replacement service.
            return when (operation) {
                "poll" -> JSONObject().put("exitCode", -1)
                "close" -> JSONObject()
                else -> throw error
            }
        } finally {
            if (operation == "close") sessions.remove(handle)
            reply.recycle()
            data.recycle()
        }
    }

    /** Runs one noninteractive command through the same identity-enforcing service as terminals. */
    fun executePiped(command: String, timeoutMillis: Long): AndroidPrivilegedCommandResult {
        val deadline = AndroidCommandDeadline(timeoutMillis)
        val request = JSONObject()
            .put("operation", "exec")
            .put("argv", JSONArray(listOf("/system/bin/sh", "-c", command)))
            .put("env", JSONArray(listOf(
                "PATH=/system/bin:/system/xbin",
                "HOME=/sdcard",
                "TMPDIR=/data/local/tmp",
            )))
            .put("cwd", "/")
        val response = remoteOperation(request)
        if (response.has("error")) throw IllegalStateException(response.getString("error"))
        val handle = response.getInt("pid")
        val stdout = ParcelFileDescriptor.adoptFd(response.getInt("stdoutFd"))
        val stderr = ParcelFileDescriptor.adoptFd(response.getInt("stderrFd"))
        try {
            return AndroidCommandProcessRunner.collect(
                stdout = ParcelFileDescriptor.AutoCloseInputStream(stdout),
                stderr = ParcelFileDescriptor.AutoCloseInputStream(stderr),
                awaitExit = { awaitExit(handle, deadline) },
                destroy = { closeRemote(handle) },
                timeoutMillis = deadline.remainingMillis(),
            )
        } finally {
            closeRemote(handle)
        }
    }

    /** Polls the privileged service until the child exits or the shared budget expires. */
    private fun awaitExit(handle: Int, deadline: AndroidCommandDeadline): Int {
        while (true) {
            val response = remoteOperation(JSONObject().put("operation", "poll").put("pid", handle))
            if (response.has("error")) throw IllegalStateException(response.getString("error"))
            if (!response.isNull("exitCode")) return response.getInt("exitCode")
            deadline.remainingMillis()
            Thread.sleep(20)
        }
    }

    private fun closeRemote(handle: Int) {
        runCatching { remoteOperation(JSONObject().put("operation", "close").put("pid", handle)) }
    }
}
