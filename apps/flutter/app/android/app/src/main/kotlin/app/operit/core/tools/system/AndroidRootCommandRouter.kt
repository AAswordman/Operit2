package app.operit.core.tools.system

import java.io.File

/** Inspired by Operit's RootAuthorizer/RootShellExecutor; no Android UI dependencies. */
enum class AndroidRootExecutionMode { Auto, ForceLibsu, ForceExec }

data class AndroidRootExecutionSettings(
    val mode: AndroidRootExecutionMode = AndroidRootExecutionMode.Auto,
    val suCommand: String = "su",
) {
    /** Parse argv, not a shell expression: quoted paths/arguments are supported safely. */
    fun suArguments(): List<String> {
        val command = suCommand.trim().ifEmpty { "su" }
        require(command.none { it == '\u0000' || it == '\n' || it == '\r' }) {
            "su command must not contain NUL or line breaks"
        }
        val result = mutableListOf<String>()
        val token = StringBuilder()
        var quote: Char? = null
        var escaped = false
        var started = false
        for (character in command) {
            when {
                escaped -> { token.append(character); escaped = false; started = true }
                character == '\\' && quote != '\'' -> { escaped = true; started = true }
                quote != null -> if (character == quote) quote = null else token.append(character)
                character == '\'' || character == '"' -> { quote = character; started = true }
                character.isWhitespace() -> if (started) {
                    result.add(token.toString()); token.setLength(0); started = false
                }
                else -> { token.append(character); started = true }
            }
        }
        require(!escaped && quote == null) { "su command has an unfinished quote or escape" }
        if (started) result.add(token.toString())
        require(result.isNotEmpty() && result.first().isNotBlank()) { "su executable must not be blank" }
        return result
    }
}

data class AndroidRootAccessStatus(
    val deviceRooted: Boolean,
    val target: AndroidPrivilegedCommandTarget?,
    val diagnostics: List<String>,
) {
    val granted: Boolean get() = target != null
}

/** Injectable boundary lets Root policy be tested without a rooted Android device. */
internal interface AndroidRootCommandBackend {
    fun executeLibsu(command: String, timeoutMillis: Long): AndroidPrivilegedCommandResult
    fun executeProcess(arguments: List<String>, timeoutMillis: Long): AndroidPrivilegedCommandResult
}

/** Caches transport selection, never a permission decision; never retries a user command. */
internal class AndroidRootCommandRouter(private val backend: AndroidRootCommandBackend) {
    private var selectedTarget: AndroidPrivilegedCommandTarget? = null
    private var selectedSettings: AndroidRootExecutionSettings? = null
    private var selectedSuArguments: List<String>? = null

    @Synchronized
    fun checkAccess(
        settings: AndroidRootExecutionSettings,
        timeoutMillis: Long,
    ): AndroidRootAccessStatus = probe(settings, AndroidCommandDeadline(timeoutMillis))

    /** Verifies the direct su executable needed to own a persistent native PTY. */
    @Synchronized
    fun terminalSuArguments(settings: AndroidRootExecutionSettings, timeoutMillis: Long): List<String>? {
        val status = probe(settings.copy(mode = AndroidRootExecutionMode.ForceExec), AndroidCommandDeadline(timeoutMillis))
        return if (status.granted) requireNotNull(selectedSuArguments).toList() else null
    }

    @Synchronized
    fun execute(
        target: AndroidPrivilegedCommandTarget,
        command: String,
        timeoutMillis: Long,
        settings: AndroidRootExecutionSettings,
    ): AndroidPrivilegedCommandResult {
        require(command.isNotBlank()) { "privileged command must not be blank" }
        val deadline = AndroidCommandDeadline(timeoutMillis)
        val actualTarget = when (target) {
            AndroidPrivilegedCommandTarget.RootAuto -> {
                val status = probe(settings, deadline)
                // Distinguish an exhausted command budget from an actual Root denial.
                deadline.remainingMillis()
                check(status.granted) { "Root access is not granted: ${status.diagnostics.joinToString("; ")}" }
                requireNotNull(status.target)
            }
            AndroidPrivilegedCommandTarget.RootLibsu, AndroidPrivilegedCommandTarget.RootExec -> target
            else -> throw IllegalArgumentException("not a Root transport: $target")
        }
        // Do not fail over after starting a command: it may already have side effects.
        return when (actualTarget) {
            AndroidPrivilegedCommandTarget.RootLibsu -> backend.executeLibsu(command, deadline.remainingMillis())
            AndroidPrivilegedCommandTarget.RootExec -> {
                val arguments = if (settings == selectedSettings) selectedSuArguments else null
                val guardedCommand = "if [ \"\$(id -u)\" != 0 ]; then echo 'Root identity verification failed' >&2; exit 126; fi\n$command"
                backend.executeProcess(
                    (arguments ?: settings.suArguments()) + listOf("-c", guardedCommand),
                    deadline.remainingMillis(),
                )
            }
            else -> error("unexpected Root transport")
        }
    }

    private fun probe(settings: AndroidRootExecutionSettings, deadline: AndroidCommandDeadline): AndroidRootAccessStatus {
        val configuredArguments = settings.suArguments()
        val diagnostics = mutableListOf<String>()
        val sameSettings = settings == selectedSettings
        val cachedTarget = if (sameSettings) selectedTarget else null
        val cachedArguments = if (sameSettings) selectedSuArguments else null
        selectedTarget = null
        selectedSettings = settings
        selectedSuArguments = null
        val knownSuPaths = listOf("/system/bin/su", "/system/xbin/su", "/sbin/su", "/su/bin/su", "/data/adb/ksu/bin/su")
        val existingPaths = knownSuPaths.filter { File(it).canExecute() }
        var deviceRooted = existingPaths.isNotEmpty()
        var preferExec = cachedTarget == AndroidPrivilegedCommandTarget.RootExec
        if (settings.mode == AndroidRootExecutionMode.Auto && cachedTarget == null) {
            attempt(diagnostics, "su version") {
                val version = backend.executeProcess(configuredArguments + "--version", minOf(1_500, deadline.remainingMillis()))
                val text = version.stdoutText() + version.stderrText()
                deviceRooted = deviceRooted || version.exitCode == 0
                // A successful --version alone is not proof of KernelSU or Root authorization.
                preferExec = text.contains("KernelSU", ignoreCase = true) || text.contains("APatch", ignoreCase = true)
            }
        }
        val targets = when (settings.mode) {
            AndroidRootExecutionMode.ForceLibsu -> listOf(AndroidPrivilegedCommandTarget.RootLibsu)
            AndroidRootExecutionMode.ForceExec -> listOf(AndroidPrivilegedCommandTarget.RootExec)
            AndroidRootExecutionMode.Auto -> if (preferExec) {
                listOf(AndroidPrivilegedCommandTarget.RootExec, AndroidPrivilegedCommandTarget.RootLibsu)
            } else listOf(AndroidPrivilegedCommandTarget.RootLibsu, AndroidPrivilegedCommandTarget.RootExec)
        }
        for (target in targets) {
            val candidates = if (target == AndroidPrivilegedCommandTarget.RootExec) {
                // A custom executable must not silently switch to an unrelated su installation.
                if (configuredArguments == listOf("su")) {
                    (listOfNotNull(cachedArguments) + listOf(configuredArguments) + existingPaths.map { listOf(it) }).distinct()
                } else listOf(configuredArguments)
            } else listOf(configuredArguments)
            for (arguments in candidates) {
                var granted = false
                attempt(diagnostics, "$target (${arguments.joinToString(" ")})") {
                    val result = if (target == AndroidPrivilegedCommandTarget.RootLibsu) {
                        backend.executeLibsu("id -u", minOf(10_000, deadline.remainingMillis()))
                    } else backend.executeProcess(arguments + listOf("-c", "id -u"), deadline.remainingMillis())
                    granted = result.exitCode == 0 && result.stdoutText().trim() == "0"
                    if (!granted) diagnostics.add("$target: exit=${result.exitCode}, uid=${result.stdoutText().trim()}, ${result.stderrText().trim()}")
                }
                if (granted) {
                    selectedTarget = target
                    selectedSuArguments = if (target == AndroidPrivilegedCommandTarget.RootExec) arguments else null
                    return AndroidRootAccessStatus(true, target, diagnostics)
                }
            }
        }
        return AndroidRootAccessStatus(deviceRooted, null, diagnostics)
    }

    private fun attempt(diagnostics: MutableList<String>, label: String, action: () -> Unit) {
        try { action() } catch (error: Exception) {
            if (error is InterruptedException) {
                Thread.currentThread().interrupt()
                throw error
            }
            diagnostics.add("$label: ${error.message ?: error.javaClass.simpleName}")
        }
    }
}
