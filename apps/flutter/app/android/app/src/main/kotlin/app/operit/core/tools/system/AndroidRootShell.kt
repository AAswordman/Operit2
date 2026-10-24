package app.operit.core.tools.system

import com.topjohnwu.superuser.Shell
import java.util.concurrent.ExecutionException
import app.operit.HostTaskExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/** Owns Root shells, separate from libsu's global shell (which may have cached non-Root). */
object AndroidRootShell {
    private val backend = LibsuRootCommandBackend()
    private val router = AndroidRootCommandRouter(backend)

    fun checkAccess(
        settings: AndroidRootExecutionSettings = AndroidRootExecutionSettings(),
        timeoutMillis: Long = 30_000L,
    ): AndroidRootAccessStatus = router.checkAccess(settings, timeoutMillis)

    fun execute(
        target: AndroidPrivilegedCommandTarget,
        command: String,
        timeoutMillis: Long,
        settings: AndroidRootExecutionSettings = AndroidRootExecutionSettings(),
    ): AndroidPrivilegedCommandResult = router.execute(target, command, timeoutMillis, settings)
}

/** Binds each command to the shell whose Root identity was actually verified. */
private class LibsuRootCommandBackend : AndroidRootCommandBackend {
    private val shell = AtomicReference<Shell?>(null)
    private val worker = HostTaskExecutor(serial = true, service = true)

    override fun executeProcess(arguments: List<String>, timeoutMillis: Long): AndroidPrivilegedCommandResult =
        AndroidCommandProcessRunner.execute(arguments, timeoutMillis)

    override fun executeLibsu(command: String, timeoutMillis: Long): AndroidPrivilegedCommandResult {
        val expired = AtomicBoolean(false)
        val ownedSession = AtomicReference<Shell?>(null)
        val closeOwnedSession = { ownedSession.getAndSet(null)?.let { session -> shell.compareAndSet(session, null); session.close() }; Unit }
        val future = worker.submit<AndroidPrivilegedCommandResult> {
            var session = shell.get()
            if (session == null || !session.isAlive || !session.isRoot) {
                shell.getAndSet(null)?.close()
                session = Shell.Builder.create()
                    .setFlags(Shell.FLAG_MOUNT_MASTER)
                    .setTimeout(10)
                    .build()
                if (expired.get()) {
                    session.close()
                    throw TimeoutException("Root shell initialization timed out")
                }
                if (!session.isRoot) {
                    session.close()
                    throw IllegalStateException("libsu did not obtain Root access")
                }
                shell.set(session)
            }
            ownedSession.set(session)
            try {
                check(!expired.get()) { "Root command deadline expired" }
                val stdout = mutableListOf<String>()
                val stderr = mutableListOf<String>()
                // Never use Shell.cmd(): it could select a different globally cached shell.
                val result = session.newJob().add(command).to(stdout, stderr).exec()
                AndroidPrivilegedCommandResult(
                    stdout.joinToString("\n").toByteArray(Charsets.UTF_8),
                    stderr.joinToString("\n").toByteArray(Charsets.UTF_8), result.code,
                )
            } finally {
                if (expired.get()) {
                    closeOwnedSession()
                }
            }
        }
        try {
            return future.get(timeoutMillis, TimeUnit.MILLISECONDS)
        } catch (error: Exception) {
            expired.set(true)
            // Closing this owned shell kills a stuck job; it is never reused after a timeout.
            runCatching { closeOwnedSession() }
            future.cancel(true)
            if (error is InterruptedException) Thread.currentThread().interrupt()
            if (error is ExecutionException) throw (error.cause ?: error)
            throw error
        }
    }
}
