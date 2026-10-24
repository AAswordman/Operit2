package app.operit.core.tools.system

import java.io.InputStream
import java.util.concurrent.ExecutionException
import app.operit.HostTaskScheduler
import java.util.concurrent.FutureTask
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException

/** One monotonic budget shared by transport selection, identity checks and execution. */
internal class AndroidCommandDeadline(timeoutMillis: Long) {
    private val startNanos = System.nanoTime()
    private val budgetNanos = TimeUnit.MILLISECONDS.toNanos(timeoutMillis)

    init { require(timeoutMillis > 0) { "command timeout must be positive" } }

    /** Returns the remaining shared process deadline. */
    fun remainingMillis(): Long {
        val remaining = budgetNanos - (System.nanoTime() - startNanos)
        if (remaining <= 0) throw TimeoutException("privileged command timed out")
        return TimeUnit.NANOSECONDS.toMillis(remaining).coerceAtLeast(1)
    }
}

/** Drains both pipes concurrently, bounds all waits, and reaps failed processes. */
internal object AndroidCommandProcessRunner {
    /** Starts a process and collects its output within one deadline. */
    fun execute(arguments: List<String>, timeoutMillis: Long): AndroidPrivilegedCommandResult {
        val deadline = AndroidCommandDeadline(timeoutMillis)
        val process = ProcessBuilder(arguments).start()
        try {
            // These are noninteractive commands. Leaving stdin open can deadlock su/wrappers.
            process.outputStream.close()
            return collect(
                process.inputStream, process.errorStream, { process.waitFor() },
                { process.destroyForcibly() }, deadline.remainingMillis(),
            )
        } catch (error: Exception) {
            process.destroyForcibly()
            throw error
        }
    }

    /** Atomically reserves every blocking stage before starting pipe collection. */
    fun collect(
        stdout: InputStream,
        stderr: InputStream,
        awaitExit: () -> Int,
        destroy: () -> Unit,
        timeoutMillis: Long,
    ): AndroidPrivilegedCommandResult {
        val deadline = AndroidCommandDeadline(timeoutMillis)
        val output = FutureTask<ByteArray> { stdout.use { it.readBytes() } }
        val errors = FutureTask<ByteArray> { stderr.use { it.readBytes() } }
        val exit = FutureTask<Int> { awaitExit() }
        try {
            HostTaskScheduler.executeServices(arrayOf(output, errors, exit))
            val code = exit.get(deadline.remainingMillis(), TimeUnit.MILLISECONDS)
            return AndroidPrivilegedCommandResult(
                output.get(deadline.remainingMillis(), TimeUnit.MILLISECONDS),
                errors.get(deadline.remainingMillis(), TimeUnit.MILLISECONDS), code,
            )
        } catch (error: Exception) {
            runCatching { destroy() }
            if (error is InterruptedException) Thread.currentThread().interrupt()
            if (error is TimeoutException) {
                throw TimeoutException("privileged command timed out after $timeoutMillis ms")
            }
            if (error is ExecutionException) throw (error.cause ?: error)
            throw error
        } finally {
            // Closing pipes after destroy also unblocks readers when a descendant holds a pipe.
            runCatching { stdout.close() }
            runCatching { stderr.close() }
            output.cancel(true)
            errors.cancel(true)
            exit.cancel(true)
        }
    }
}
