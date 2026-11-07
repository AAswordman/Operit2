package app.operit

/** Holds an install continuation until the operating system finishes its authorization UI. */
internal class StoragePermissionRequest(
    private val granted: () -> Boolean,
    private val launch: () -> Unit,
) {
    private var pending: ((Boolean) -> Unit)? = null

    /** Preserves request ownership until completion and clears it on launch failure. */
    fun request(complete: (Boolean) -> Unit) {
        check(pending == null) { "A storage permission request is already active" }
        if (granted()) {
            complete(true)
            return
        }
        pending = complete
        try {
            launch()
        } catch (error: Throwable) {
            pending = null
            throw error
        }
    }

    /** Rechecks the actual grant before allowing installation to resume. */
    fun finish() {
        val complete = pending
        pending = null
        complete?.invoke(granted())
    }
}
