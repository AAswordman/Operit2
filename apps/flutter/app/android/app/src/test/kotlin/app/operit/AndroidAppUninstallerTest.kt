package app.operit

import android.app.Activity
import android.content.Intent
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.net.Uri
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ExecutionException
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import org.junit.Assert.*
import org.junit.Test
import org.mockito.Mockito.*

/** Exercises the real uninstall owner across Android launch, confirmation, and teardown events. */
class AndroidAppUninstallerTest {
    private class Fixture {
        val activity = mock(Activity::class.java)
        val pm = mock(PackageManager::class.java)
        val installed = AtomicBoolean(true)
        val launches = LinkedBlockingQueue<Runnable>()
        var requestCode = 0
        val uninstaller: AndroidAppUninstaller

        init {
            val info = mock(PackageInfo::class.java)
            val missing = mock(PackageManager.NameNotFoundException::class.java)
            `when`(activity.packageManager).thenReturn(pm)
            `when`(pm.getPackageInfo("app.example", 0)).thenAnswer {
                if (installed.get()) info else throw missing
            }
            doAnswer { launches.add(it.getArgument(0)); null }
                .`when`(activity).runOnUiThread(any(Runnable::class.java))
            doAnswer { requestCode = it.getArgument(1); null }
                .`when`(activity).startActivityForResult(any(Intent::class.java), anyInt())
            uninstaller = AndroidAppUninstaller(activity)
        }

        /** Starts the host call on a worker without blocking Android's result delivery. */
        fun start(): CompletableFuture<Void> = CompletableFuture.runAsync {
            uninstaller.uninstall("app.example")
        }

        /** Runs the queued main-thread launch under the test's Android framework mocks. */
        fun launch() {
            requireNotNull(launches.poll(3, TimeUnit.SECONDS)) { "Uninstall did not reach the main thread" }.run()
        }

        /** Obtains the concrete failure returned to the waiting host worker. */
        fun failure(call: CompletableFuture<Void>): Throwable =
            assertThrows(ExecutionException::class.java) { call.get(3, TimeUnit.SECONDS) }.cause!!
    }

    /** Mocks Android intent construction while retaining assertions on its actual contract. */
    private fun withUninstallIntent(test: (List<Intent>) -> Unit) {
        val uri = mock(Uri::class.java)
        mockStatic(Uri::class.java).use { uris ->
            uris.`when`<Uri> { Uri.parse("package:app.example") }.thenReturn(uri)
            mockConstruction(Intent::class.java) { intent, construction ->
                assertEquals(listOf(Intent.ACTION_DELETE, uri), construction.arguments())
                `when`(intent.putExtra(Intent.EXTRA_RETURN_RESULT, true)).thenReturn(intent)
            }.use { intents -> test(intents.constructed()) }
        }
    }

    /** Keeps success pending until Android confirms and PackageManager no longer finds the app. */
    @Test
    fun successRequiresConfirmationAndActualRemoval() = withUninstallIntent { intents ->
        val fixture = Fixture()
        val call = fixture.start()
        fixture.launch()
        assertFalse(call.isDone)
        assertFalse(fixture.uninstaller.onActivityResult(fixture.requestCode + 1, Activity.RESULT_OK))
        assertFalse(call.isDone)
        verify(intents.single()).putExtra(Intent.EXTRA_RETURN_RESULT, true)
        verify(intents.single(), never()).addFlags(anyInt())
        fixture.installed.set(false)
        assertTrue(fixture.uninstaller.onActivityResult(fixture.requestCode, Activity.RESULT_OK))
        call.get(3, TimeUnit.SECONDS)
    }

    /** Rejects cancellation and prevents a late result from completing a subsequent request. */
    @Test
    fun cancellationFailsAndRetryOwnsANewResult() = withUninstallIntent {
        val fixture = Fixture()
        val canceled = fixture.start()
        fixture.launch()
        val previousCode = fixture.requestCode
        fixture.uninstaller.onActivityResult(previousCode, Activity.RESULT_CANCELED)
        assertTrue(fixture.failure(canceled).message!!.contains("canceled or rejected"))

        val retry = fixture.start()
        fixture.launch()
        assertNotEquals(previousCode, fixture.requestCode)
        assertFalse(fixture.uninstaller.onActivityResult(previousCode, Activity.RESULT_OK))
        assertFalse(retry.isDone)
        fixture.installed.set(false)
        fixture.uninstaller.onActivityResult(fixture.requestCode, Activity.RESULT_OK)
        retry.get(3, TimeUnit.SECONDS)
    }

    /** Rejects a successful Activity result when Android still reports the package as installed. */
    @Test
    fun systemSuccessWithoutRemovalFails() = withUninstallIntent {
        val fixture = Fixture()
        val call = fixture.start()
        fixture.launch()
        fixture.uninstaller.onActivityResult(fixture.requestCode, Activity.RESULT_OK)
        assertTrue(fixture.failure(call).message!!.contains("still installed"))
    }

    /** Propagates launch rejection immediately instead of waiting or reporting success. */
    @Test
    fun launchFailureReachesTheHost() = withUninstallIntent {
        val fixture = Fixture()
        doThrow(SecurityException("Uninstall request denied"))
            .`when`(fixture.activity).startActivityForResult(any(Intent::class.java), anyInt())
        val call = fixture.start()
        fixture.launch()
        assertEquals("Uninstall request denied", fixture.failure(call).message)
    }

    /** Teardown unblocks the host and rejects new calls on the detached owner. */
    @Test
    fun releaseFailsPendingAndFutureRequests() = withUninstallIntent {
        val fixture = Fixture()
        val call = fixture.start()
        fixture.launch()
        fixture.uninstaller.release()
        assertTrue(fixture.failure(call).message!!.contains("released"))
        assertFalse(fixture.uninstaller.onActivityResult(fixture.requestCode, Activity.RESULT_OK))
        val error = assertThrows(IllegalStateException::class.java) {
            fixture.uninstaller.uninstall("app.example")
        }
        assertTrue(error.message!!.contains("released"))
    }

    /** Teardown also prevents a queued launch from opening UI after its owner was detached. */
    @Test
    fun releaseBeforeMainThreadLaunchDoesNotOpenUninstaller() = withUninstallIntent { intents ->
        val fixture = Fixture()
        val call = fixture.start()
        val launch = requireNotNull(fixture.launches.poll(3, TimeUnit.SECONDS))
        fixture.uninstaller.release()
        launch.run()
        assertTrue(intents.isEmpty())
        assertTrue(fixture.failure(call).message!!.contains("released"))
    }

    /** Concurrent calls cannot replace the package whose confirmation is already visible. */
    @Test
    fun concurrentRequestCannotStealConfirmation() = withUninstallIntent {
        val fixture = Fixture()
        val call = fixture.start()
        fixture.launch()
        val error = assertThrows(IllegalStateException::class.java) {
            fixture.uninstaller.uninstall("app.example")
        }
        assertTrue(error.message!!.contains("already active"))
        fixture.uninstaller.onActivityResult(fixture.requestCode, Activity.RESULT_CANCELED)
        fixture.failure(call)
    }

    /** A missing package fails before presenting a system uninstall dialog. */
    @Test
    fun missingPackageDoesNotLaunchConfirmation() {
        val fixture = Fixture()
        fixture.installed.set(false)
        assertThrows(PackageManager.NameNotFoundException::class.java) {
            fixture.uninstaller.uninstall("app.example")
        }
        assertTrue(fixture.launches.isEmpty())
    }
}
