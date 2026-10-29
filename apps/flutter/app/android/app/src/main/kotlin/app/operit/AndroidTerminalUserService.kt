package app.operit

import android.content.Context
import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import android.os.ParcelFileDescriptor
import android.util.Log
import org.json.JSONObject
import java.io.Closeable
import java.io.IOException

/** Owns real PTYs under Shizuku's UID instead of emulating sessions with one-shot jobs. */
class AndroidTerminalUserService(context: Context) : Binder() {
    companion object {
        const val DESCRIPTOR = "app.operit.AndroidTerminalUserService"
    }

    private val ownerUid = context.applicationInfo.uid
    private var owner: IBinder? = null
    private val sessions = mutableMapOf<Int, Session>()
    private data class Session(
        val descriptor: ParcelFileDescriptor,
        val streams: List<Closeable>,
        var exitCode: Int? = null,
    )

    /** Dispatches only the owning app's requests and Shizuku's reserved destroy call. */
    @Synchronized
    override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
        if (code == 16777115) {
            check(getCallingUid() == 0 || getCallingUid() == 2000) { "Invalid Shizuku destroy caller" }
            closeAll()
            kotlin.system.exitProcess(0)
        }
        if (code != 1) return super.onTransact(code, data, reply, flags)
        check(getCallingUid() == ownerUid) { "Invalid terminal service caller" }
        data.enforceInterface(DESCRIPTOR)
        val response: JSONObject
        var descriptors: List<ParcelFileDescriptor> = emptyList()
        var startedPid: Int? = null
        try {
            val request = JSONObject(checkNotNull(data.readString()))
            val token = checkNotNull(data.readStrongBinder())
            if (owner == null) {
                token.linkToDeath({ synchronized(this) { closeAll() } }, 0)
                owner = token
            }
            check(owner == token) { "Terminal owner changed" }
            val operation = request.getString("operation")
            response = when (operation) {
                "start" -> {
                    val (result, pipes) = startSession(request)
                    descriptors = pipes
                    startedPid = result.getInt("pid")
                    result
                }
                "poll", "resize", "close" -> {
                    val pid = request.getInt("pid")
                    val session = checkNotNull(sessions[pid]) { "Shizuku PTY does not exist: $pid" }
                    when (operation) {
                        "poll" -> {
                            if (session.exitCode == null) {
                                val result = nativeOperation(request)
                                if (!result.isNull("exitCode")) session.exitCode = result.getInt("exitCode")
                            }
                            JSONObject().put("exitCode", session.exitCode ?: JSONObject.NULL)
                        }
                        "resize" -> nativeOperation(request.put("fd", session.descriptor.fd))
                        else -> { sessions.remove(pid); closeSession(pid, session); JSONObject() }
                    }
                }
                else -> throw IllegalArgumentException("Unsupported terminal operation: $operation")
            }
        } catch (error: Exception) {
            reply?.writeNoException()
            reply?.writeString(JSONObject().put("error", error.message ?: error.javaClass.simpleName).toString())
            return true
        }
        try {
            checkNotNull(reply).writeNoException()
            reply.writeString(response.toString())
            descriptors.forEach { reply.writeTypedObject(it, 0) }
        } catch (error: Exception) {
            startedPid?.let { pid -> sessions.remove(pid)?.let { closeSession(pid, it) } }
            throw error
        } finally {
            descriptors.forEach { it.close() }
        }
        return true
    }

    /** Keeps shell-owned devpts private and relays two ordinary pipe FDs across Binder. */
    private fun startSession(request: JSONObject): Pair<JSONObject, List<ParcelFileDescriptor>> {
        val result = nativeOperation(request)
        val pid = result.getInt("pid")
        val master = ParcelFileDescriptor.adoptFd(result.getInt("fd"))
        val resources = mutableListOf<Closeable>(master)
        try {
            val input = ParcelFileDescriptor.createPipe()
            resources.addAll(input)
            val output = ParcelFileDescriptor.createPipe()
            resources.addAll(output)
            val ptyReader = ParcelFileDescriptor.AutoCloseInputStream(ParcelFileDescriptor.dup(master.fileDescriptor))
            resources.add(ptyReader)
            val ptyWriter = ParcelFileDescriptor.AutoCloseOutputStream(ParcelFileDescriptor.dup(master.fileDescriptor))
            resources.add(ptyWriter)
            val pipeReader = ParcelFileDescriptor.AutoCloseInputStream(input[0])
            val pipeWriter = ParcelFileDescriptor.AutoCloseOutputStream(output[1])
            val session = Session(master, listOf(ptyReader, ptyWriter, pipeReader, pipeWriter))
            HostTaskScheduler.executeServices(arrayOf(
                Runnable { relay(pipeReader, ptyWriter) },
                Runnable { relay(ptyReader, pipeWriter) },
            ))
            sessions[pid] = session
            result.remove("fd")
            return result to listOf(output[0], input[1])
        } catch (error: Exception) {
            try { nativeOperation(JSONObject().put("operation", "close").put("pid", pid)) }
            finally { resources.forEach { runCatching { it.close() } } }
            throw error
        }
    }

    /** Ends a relay on pipe EOF or the PTY's normal EIO after its slave exits. */
    private fun relay(input: java.io.InputStream, output: java.io.OutputStream) {
        try { input.use { source -> output.use { destination -> source.copyTo(destination) } } }
        catch (_: IOException) { /* PTY hangup and session closure terminate the relay. */ }
    }

    /** Propagates native process failures rather than fabricating an app-shell session. */
    private fun nativeOperation(request: JSONObject): JSONObject {
        val result = JSONObject(OperitRuntimeNative.privilegedPtyOperation(request.toString()))
        check(!result.has("error")) { result.optString("error") }
        return result
    }

    /** Avoids signaling a recycled PID after its exit was already reaped. */
    private fun closeSession(pid: Int, session: Session) {
        try {
            if (session.exitCode == null) nativeOperation(JSONObject().put("operation", "close").put("pid", pid))
        } finally {
            (session.streams + session.descriptor).forEach { runCatching { it.close() } }
        }
    }

    /** Reclaims all children and descriptors when the app dies or Shizuku stops the service. */
    private fun closeAll() {
        sessions.forEach { (pid, session) ->
            runCatching { closeSession(pid, session) }
                .onFailure { Log.w("OperitTerminal", "Failed to close Shizuku PTY $pid", it) }
        }
        sessions.clear()
        owner = null
    }
}
