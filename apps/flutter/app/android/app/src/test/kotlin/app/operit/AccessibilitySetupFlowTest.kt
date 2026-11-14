package app.operit

import org.junit.Assert.*
import org.junit.Test

/** Exercises installation sequencing, permission denial and late Android results. */
class AccessibilitySetupFlowTest {
    private class Fixture : AccessibilitySetupFlow.Actions {
        var providerCurrent = false
        var canInstall = false
        var launchError: Exception? = null
        val launches = mutableListOf<Pair<String, Int>>()
        val completions = mutableListOf<Throwable?>()
        val flow = AccessibilitySetupFlow(this)

        /** Reports whether the simulated installed provider meets the required version. */
        override fun hasCurrentCompanion() = providerCurrent
        /** Reports the simulated unknown-source authorization. */
        override fun canInstallPackages() = canInstall
        /** Records the unknown-source settings Activity. */
        override fun authorizeInstallation(requestCode: Int) = launch("permission", requestCode)
        /** Records the package installer Activity. */
        override fun installCompanion(requestCode: Int) = launch("install", requestCode)
        /** Records accessibility settings. */
        override fun authorizeAccessibility(requestCode: Int) = launch("accessibility", requestCode)
        /** Records native launches or propagates a platform launch error. */
        private fun launch(name: String, code: Int) {
            launchError?.let { throw it }
            launches.add(name to code)
        }
        /** Begins a grant action with a captured completion. */
        fun start() = flow.request { completions.add(it) }
        /** Delivers the current external Activity result. */
        fun resume() = flow.onActivityResult(launches.last().second)
    }

    /** Opens the installer after source authorization and continues to accessibility settings. */
    @Test
    fun firstGrantInstallsAndContinuesToSettings() {
        val fixture = Fixture()
        fixture.start()
        assertEquals("permission", fixture.launches.last().first)
        assertTrue(fixture.completions.isEmpty())
        fixture.canInstall = true
        fixture.resume()
        assertEquals("install", fixture.launches.last().first)
        assertTrue(fixture.completions.isEmpty())
        fixture.resume()
        assertEquals("accessibility", fixture.launches.last().first)
        fixture.resume()
        assertEquals(listOf<Throwable?>(null), fixture.completions)
        assertEquals(3, fixture.launches.map { it.second }.toSet().size)
    }

    /** Opens the bundled installer directly when installation is already authorized. */
    @Test
    fun authorizedSourceOpensInstallerDirectly() {
        val fixture = Fixture().apply { canInstall = true }
        fixture.start()
        assertEquals(listOf("install"), fixture.launches.map { it.first })
    }

    /** Skips installation and its source authorization when the provider is already current. */
    @Test
    fun currentProviderOpensAccessibilityWithoutReinstalling() {
        val fixture = Fixture().apply { providerCurrent = true }
        fixture.start()
        assertEquals(listOf("accessibility"), fixture.launches.map { it.first })
        fixture.resume()
        assertEquals(listOf<Throwable?>(null), fixture.completions)
    }

    /** Stops immediately when the user denies unknown-source authorization. */
    @Test
    fun deniedInstallPermissionDoesNotOpenInstaller() {
        val fixture = Fixture()
        fixture.start()
        fixture.resume()
        assertNotNull(fixture.completions.single())
        assertEquals(listOf("permission"), fixture.launches.map { it.first })
    }

    /** Continues to accessibility settings without inspecting the installed package. */
    @Test
    fun installerReturnOpensAccessibilityWithoutPackageValidation() {
        val fixture = Fixture().apply { canInstall = true }
        fixture.start()
        fixture.resume()
        assertTrue(fixture.completions.isEmpty())
        assertEquals(listOf("install", "accessibility"), fixture.launches.map { it.first })
    }

    /** Prevents stale Android callbacks from resuming a later request or stage. */
    @Test
    fun retriesRejectOldAndUnrelatedResults() {
        val fixture = Fixture()
        fixture.start()
        val oldCode = fixture.launches.last().second
        assertFalse(fixture.flow.onActivityResult(oldCode - 1))
        fixture.resume()
        fixture.start()
        assertFalse(fixture.flow.onActivityResult(oldCode))
        assertEquals(1, fixture.completions.size)
    }

    /** Reports platform failures and permits a new request after failure. */
    @Test
    fun launchFailureCompletesRequest() {
        val error = SecurityException("Installation denied")
        val fixture = Fixture().apply { launchError = error }
        fixture.start()
        assertSame(error, fixture.completions.single())
        fixture.launchError = null
        fixture.start()
        assertEquals("permission", fixture.launches.single().first)
    }

    /** Rejects concurrent grant actions without losing the first completion. */
    @Test
    fun duplicateRequestDoesNotReplacePendingCallback() {
        val fixture = Fixture()
        fixture.start()
        assertThrows(IllegalStateException::class.java) { fixture.start() }
        fixture.flow.release()
        assertEquals(1, fixture.completions.size)
        assertNotNull(fixture.completions.single())
        assertFalse(fixture.resume())
    }
}
