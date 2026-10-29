package app.operit.core.tools.system

/** Supplies exact executable and UID results without launching a Root manager. */
private class TerminalRootBackend : AndroidRootCommandBackend {
    val calls = mutableListOf<List<String>>()
    var granted = true

    /** Persistent PTYs must never borrow libsu's line-oriented global session. */
    override fun executeLibsu(command: String, timeoutMillis: Long): AndroidPrivilegedCommandResult =
        error("Terminal authorization must probe the direct su executable")

    /** Returns an exact identity result and records every literal argv element. */
    override fun executeProcess(arguments: List<String>, timeoutMillis: Long): AndroidPrivilegedCommandResult {
        calls.add(arguments)
        return AndroidPrivilegedCommandResult((if (granted) "0\n" else "2000\n").toByteArray(), byteArrayOf(), 0)
    }
}

/** Exercises the production router's persistent-PTY executable probe. */
fun main() {
    val backend = TerminalRootBackend()
    val router = AndroidRootCommandRouter(backend)
    val settings = AndroidRootExecutionSettings(AndroidRootExecutionMode.ForceLibsu, "'/custom path/su' --flag")
    check(router.terminalSuArguments(settings, 1_000) == listOf("/custom path/su", "--flag"))
    check(backend.calls.single() == listOf("/custom path/su", "--flag", "-c", "id -u"))
    backend.granted = false
    check(router.terminalSuArguments(settings, 1_000) == null)
    backend.granted = true
    check(router.terminalSuArguments(settings, 1_000) == listOf("/custom path/su", "--flag"))
    check(backend.calls.size == 3)
    println("PASS terminal su argv, direct UID verification, revoked permission and reauthorization")
}
