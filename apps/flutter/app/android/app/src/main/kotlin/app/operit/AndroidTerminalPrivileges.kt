package app.operit

import android.content.ComponentName
import android.content.Context
import android.content.ServiceConnection
import android.os.Binder
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.Parcel
import android.os.ParcelFileDescriptor
import app.operit.core.tools.system.AndroidRootShell
import org.json.JSONArray
import org.json.JSONObject
import rikka.shizuku.Shizuku
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

/** Bridges only Android authorization and Binder mechanics; Rust chooses the transport. */
class AndroidTerminalPrivileges(private val context: Context) {
    private val mainHandler = Handler(Looper.getMainLooper())
    private val owner = Binder()
    private data class RemoteSession(val binder: IBinder, val pid: Int)
    private val sessions = ConcurrentHashMap<Int, RemoteSession>()
    private val nextSession = AtomicInteger(1)
    @Volatile private var service: IBinder? = null
    private val args = Shizuku.UserServiceArgs(ComponentName(context, AndroidTerminalUserService::class.java))
        .daemon(false).processNameSuffix("terminal").version(1)

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
        service?.let { if (it.isBinderAlive) return it }
        check(AndroidPrivilegeAuthorization.isShizukuAuthorized()) { "Shizuku authorization is not granted" }
        val ready = CompletableFuture<IBinder>()
        val connection = object : ServiceConnection {
            /** Publishes the privileged Binder on the Android callback thread. */
            override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                service = binder
                ready.complete(binder)
            }
            /** Invalidates the connection without replacing any live terminal session. */
            override fun onServiceDisconnected(name: ComponentName) { service = null }
        }
        mainHandler.post {
            try { Shizuku.bindUserService(args, connection) }
            catch (error: Exception) { ready.completeExceptionally(error) }
        }
        try {
            return ready.get(10, TimeUnit.SECONDS)
        } catch (error: Exception) {
            mainHandler.post { Shizuku.unbindUserService(args, connection, false) }
            throw error
        }
    }

    /** Transfers ordinary pipes while keeping the PTY and child owned by Shizuku. */
    private fun remoteOperation(request: JSONObject): JSONObject {
        val starting = request.getString("operation") == "start"
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
                checkNotNull(reply.readTypedObject(ParcelFileDescriptor.CREATOR)).use { reader ->
                    checkNotNull(reply.readTypedObject(ParcelFileDescriptor.CREATOR)).use { writer ->
                        sessions[handle] = RemoteSession(binder, response.getInt("pid"))
                        response.put("pid", handle)
                        response.put("readFd", reader.detachFd())
                        response.put("writeFd", writer.detachFd())
                    }
                }
            }
            return response
        } finally {
            if (request.getString("operation") == "close") sessions.remove(handle)
            reply.recycle()
            data.recycle()
        }
    }
}
