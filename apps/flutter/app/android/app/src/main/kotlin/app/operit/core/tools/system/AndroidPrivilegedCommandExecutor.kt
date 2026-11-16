package app.operit.core.tools.system

import android.content.Context
import android.content.pm.PackageManager
import app.operit.AndroidTerminalPrivileges
import rikka.shizuku.Shizuku

/** Executes system commands through the selected, verified privileged transport. */
object AndroidPrivilegedCommandExecutor {
    private const val DEFAULT_COMMAND_TIMEOUT_MS = 60_000L

    fun execute(
        context: Context,
        target: AndroidPrivilegedCommandTarget,
        command: String,
        timeoutMillis: Long = DEFAULT_COMMAND_TIMEOUT_MS,
        rootSettings: AndroidRootExecutionSettings = AndroidRootExecutionSettings(),
    ): AndroidPrivilegedCommandResult {
        require(command.isNotBlank()) { "privileged command must not be blank" }
        require(timeoutMillis > 0L) { "privileged command timeout must be positive" }
        return when (target) {
            AndroidPrivilegedCommandTarget.RootAuto,
            AndroidPrivilegedCommandTarget.RootLibsu,
            AndroidPrivilegedCommandTarget.RootExec -> AndroidRootShell.execute(target, command, timeoutMillis, rootSettings)
            AndroidPrivilegedCommandTarget.Shizuku -> executeWithShizuku(context, command, timeoutMillis)
        }
    }

    /** Shizuku children must pass the same shell-identity drop as terminal PTYs. */
    private fun executeWithShizuku(context: Context, command: String, timeoutMillis: Long): AndroidPrivilegedCommandResult {
        val deadline = AndroidCommandDeadline(timeoutMillis)
        check(Shizuku.pingBinder()) { "Shizuku service is not running" }
        check(Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED) {
            "Shizuku permission is not granted"
        }
        return AndroidTerminalPrivileges.shared(context).executePiped(command, deadline.remainingMillis())
    }
}