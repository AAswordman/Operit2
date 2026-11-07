package app.operit

import org.junit.Assert.*
import org.junit.Test

class StoragePermissionRequestTest {
    /** Prevents installation from running before authorization finishes. */
    @Test
    fun installationWaitsForActualGrant() {
        var granted = false
        var launches = 0
        var result: Boolean? = null
        val request = StoragePermissionRequest({ granted }, { launches++ })
        request.request { result = it }
        assertEquals(1, launches)
        assertNull(result)
        granted = true
        request.finish()
        assertEquals(true, result)
    }

    /** Keeps denial from resuming installation and permits an explicit retry. */
    @Test
    fun deniedRequestCanRetryWithoutInstalling() {
        var granted = false
        val decisions = mutableListOf<Boolean>()
        val request = StoragePermissionRequest({ granted }, {})
        request.request { decisions.add(it) }
        request.finish()
        assertEquals(listOf(false), decisions)
        granted = true
        request.request { decisions.add(it) }
        assertEquals(listOf(false, true), decisions)
    }

    /** Makes already granted access avoid the authorization UI entirely. */
    @Test
    fun grantedAccessDoesNotOpenSettings() {
        val request = StoragePermissionRequest({ true }, { fail("Unexpected permission UI") })
        var allowed = false
        request.request { allowed = it }
        assertTrue(allowed)
    }

    /** Prevents duplicates from stealing a continuation and releases ownership after launch failure. */
    @Test
    fun concurrentRequestIsRejectedAndLaunchFailureReleasesOwnership() {
        var failLaunch = true
        val request = StoragePermissionRequest({ false }, {
            if (failLaunch) throw IllegalStateException("No settings activity")
        })
        assertTrue(runCatching { request.request {} }.isFailure)
        failLaunch = false
        var original: Boolean? = null
        request.request { original = it }
        assertTrue(runCatching { request.request { fail("Replaced pending request") } }.isFailure)
        request.finish()
        assertEquals(false, original)
    }
}
