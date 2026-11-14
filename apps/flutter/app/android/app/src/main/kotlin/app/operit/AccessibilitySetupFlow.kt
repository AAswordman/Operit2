package app.operit

import java.util.concurrent.atomic.AtomicInteger

/** Sequences installation authorization, package installation and accessibility settings. */
internal class AccessibilitySetupFlow(private val actions: Actions) {
    interface Actions {
        /** Checks the installed companion's version and signature. */
        fun hasCurrentCompanion(): Boolean
        /** Checks whether Android permits this application to launch package installation. */
        fun canInstallPackages(): Boolean
        /** Opens the application's unknown-source authorization screen. */
        fun authorizeInstallation(requestCode: Int)
        /** Opens the system installer for the embedded companion. */
        fun installCompanion(requestCode: Int)
        /** Opens Android's accessibility authorization screen. */
        fun authorizeAccessibility(requestCode: Int)
    }

    private enum class Stage { InstallPermission, Install, Accessibility }
    private var stage: Stage? = null
    private var requestCode = 0
    private var complete: ((Throwable?) -> Unit)? = null

    /** Starts one explicit user request and rejects overlapping requests. */
    fun request(callback: (Throwable?) -> Unit) {
        check(complete == null) { "Accessibility authorization is already active" }
        complete = callback
        advance {
            when {
                actions.hasCurrentCompanion() -> launch(Stage.Accessibility)
                actions.canInstallPackages() -> launch(Stage.Install)
                else -> launch(Stage.InstallPermission)
            }
        }
    }

    /** Rechecks actual platform state instead of trusting settings Activity result codes. */
    fun onActivityResult(code: Int): Boolean {
        if (complete == null || code != requestCode) return false
        advance {
            when (stage) {
                Stage.InstallPermission -> {
                    check(actions.canInstallPackages()) { "请允许 Operit 安装应用后重试。" }
                    launch(Stage.Install)
                }
                Stage.Install -> {
                    check(actions.hasCurrentCompanion()) { "UI 自动化配套包未安装成功，请完成系统安装后重试。" }
                    launch(Stage.Accessibility)
                }
                Stage.Accessibility -> finish(null)
                null -> error("Accessibility authorization has no active stage")
            }
        }
        return true
    }

    /** Cancels an outstanding channel call when its Activity owner is detached. */
    fun release() {
        finish(IllegalStateException("Accessibility authorization owner was released"))
    }

    /** Assigns a unique result slot to each external Activity. */
    private fun launch(next: Stage) {
        stage = next
        requestCode = nextRequestCode.getAndIncrement()
        when (next) {
            Stage.InstallPermission -> actions.authorizeInstallation(requestCode)
            Stage.Install -> actions.installCompanion(requestCode)
            Stage.Accessibility -> actions.authorizeAccessibility(requestCode)
        }
    }

    /** Propagates native launch failures and clears the active request. */
    private fun advance(action: () -> Unit) {
        try { action() } catch (error: Exception) { finish(error) }
    }

    /** Clears state before notifying Flutter so another request can begin. */
    private fun finish(error: Throwable?) {
        val callback = complete
        complete = null
        stage = null
        callback?.invoke(error)
    }

    private companion object {
        private val nextRequestCode = AtomicInteger(53000)
    }
}
