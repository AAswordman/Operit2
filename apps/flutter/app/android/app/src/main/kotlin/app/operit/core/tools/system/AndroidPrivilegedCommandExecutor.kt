package app.operit.core.tools.system

import android.content.pm.PackageManager
import android.os.ParcelFileDescriptor
import com.topjohnwu.superuser.Shell
import java.io.InputStream
import java.nio.charset.StandardCharsets
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException
import moe.shizuku.server.IShizukuService
import rikka.shizuku.Shizuku

/** Identifies the privileged Android command transport selected by the caller. */
enum class AndroidPrivilegedCommandTarget {
    RootLibsu,
    RootExec,
    Shizuku,
}

/** Contains the raw streams and exit code produced by one privileged command. */
data class AndroidPrivilegedCommandResult(
    val stdout: ByteArray,
    val stderr: ByteArray,
    val exitCode: Int,
) {
    /** Decodes stdout as UTF-8 for text-oriented command callers. */
    fun stdoutText(): String = stdout.toString(StandardCharsets.UTF_8)

    /** Decodes stderr as UTF-8 for text-oriented command callers. */
    fun stderrText(): String = stderr.toString(StandardCharsets.UTF_8)
}

/** Executes Android system commands through one explicitly selected privileged transport. */
object AndroidPrivilegedCommandExecutor {
    private const val DEFAULT_COMMAND_TIMEOUT_MS = 60_000L

    /** Executes one shell command through the requested privileged transport. */
    fun execute(
        target: AndroidPrivilegedCommandTarget,
        command: String,
        timeoutMillis: Long = DEFAULT_COMMAND_TIMEOUT_MS,
    ): AndroidPrivilegedCommandResult {
        require(command.isNotBlank()) { "privileged command must not be blank" }
        require(timeoutMillis > 0L) { "privileged command timeout must be positive" }
        return when (target) {
            AndroidPrivilegedCommandTarget.RootLibsu -> executeWithRootLibsu(command)
            AndroidPrivilegedCommandTarget.RootExec -> executeWithRootExec(command, timeoutMillis)
            AndroidPrivilegedCommandTarget.Shizuku -> executeWithShizuku(command, timeoutMillis)
        }
    }

    /** Runs a command in libsu's verified Root shell. */
    private fun executeWithRootLibsu(command: String): AndroidPrivilegedCommandResult {
        val shell = Shell.getShell()
        check(shell.isRoot) { "libsu did not obtain Root access" }
        val stdout = mutableListOf<String>()
        val stderr = mutableListOf<String>()
        val result = Shell.cmd(command).to(stdout, stderr).exec()
        return AndroidPrivilegedCommandResult(
            stdout = stdout.joinToString("\n").toByteArray(StandardCharsets.UTF_8),
            stderr = stderr.joinToString("\n").toByteArray(StandardCharsets.UTF_8),
            exitCode = result.code,
        )
    }

    /** Runs a command through an independent direct su process. */
    private fun executeWithRootExec(
        command: String,
        timeoutMillis: Long,
    ): AndroidPrivilegedCommandResult {
        val process = ProcessBuilder("su", "-c", command).start()
        return collectProcessResult(
            stdout = process.inputStream,
            stderr = process.errorStream,
            awaitExit = { process.waitFor() },
            destroy = { process.destroyForcibly() },
            timeoutMillis = timeoutMillis,
        )
    }

    /** Runs a command through Shizuku's remote-process service. */
    private fun executeWithShizuku(
        command: String,
        timeoutMillis: Long,
    ): AndroidPrivilegedCommandResult {
        check(Shizuku.pingBinder()) { "Shizuku service is not running" }
        check(Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED) {
            "Shizuku permission is not granted"
        }
        val service =
            IShizukuService.Stub.asInterface(
                requireNotNull(Shizuku.getBinder()) { "Shizuku service binder is unavailable" },
            )
        val process = service.newProcess(arrayOf("/system/bin/sh", "-c", command), null, null)
        val stdout = ParcelFileDescriptor.AutoCloseInputStream(process.inputStream)
        val stderr = ParcelFileDescriptor.AutoCloseInputStream(process.errorStream)
        return collectProcessResult(
            stdout = stdout,
            stderr = stderr,
            awaitExit = { process.waitFor() },
            destroy = { process.destroy() },
            timeoutMillis = timeoutMillis,
        )
    }

    /** Collects both process output streams while the command is running. */
    private fun collectProcessResult(
        stdout: InputStream,
        stderr: InputStream,
        awaitExit: () -> Int,
        destroy: () -> Unit,
        timeoutMillis: Long,
    ): AndroidPrivilegedCommandResult {
        val workers = Executors.newFixedThreadPool(3)
        try {
            val stdoutFuture = workers.submit<ByteArray> { stdout.use { it.readBytes() } }
            val stderrFuture = workers.submit<ByteArray> { stderr.use { it.readBytes() } }
            val exitFuture = workers.submit<Int> { awaitExit() }
            val deadlineNanos = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(timeoutMillis)
            val exitCode = exitFuture.get(remainingWaitMillis(deadlineNanos), TimeUnit.MILLISECONDS)
            return AndroidPrivilegedCommandResult(
                stdout = stdoutFuture.get(remainingWaitMillis(deadlineNanos), TimeUnit.MILLISECONDS),
                stderr = stderrFuture.get(remainingWaitMillis(deadlineNanos), TimeUnit.MILLISECONDS),
                exitCode = exitCode,
            )
        } catch (error: TimeoutException) {
            destroy()
            throw IllegalStateException("privileged command timed out after $timeoutMillis ms", error)
        } finally {
            workers.shutdownNow()
        }
    }

    /** Returns the remaining command deadline as a positive millisecond timeout. */
    private fun remainingWaitMillis(deadlineNanos: Long): Long {
        val remainingNanos = deadlineNanos - System.nanoTime()
        if (remainingNanos <= 0L) {
            throw TimeoutException("privileged command timed out")
        }
        return TimeUnit.NANOSECONDS.toMillis(remainingNanos).coerceAtLeast(1L)
    }
}
