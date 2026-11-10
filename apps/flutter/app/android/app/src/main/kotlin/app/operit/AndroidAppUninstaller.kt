package app.operit

import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ExecutionException
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException
import java.util.concurrent.atomic.AtomicInteger

/** Owns system uninstall confirmation and verifies removal before completing the host request. */
internal class AndroidAppUninstaller(private val activity: Activity) {
    private companion object {
        // Keep request codes distinct from permissions and document pickers, including late results.
        private val nextRequestCode = AtomicInteger(49000)
        private const val CONFIRMATION_TIMEOUT_SECONDS = 110L
    }

    private class Request(val code: Int, val result: CompletableFuture<Int> = CompletableFuture())

    private val lock = Any()
    private var pending: Request? = null
    private var released = false

    /** Waits on the owner worker while Android presents confirmation on the main thread. */
    @Suppress("DEPRECATION")
    fun uninstall(packageName: String) {
        activity.packageManager.getPackageInfo(packageName, 0)
        val request = synchronized(lock) {
            check(!released) { "The Android uninstall owner has been released" }
            check(pending == null) { "An application uninstall request is already active" }
            val code = nextRequestCode.getAndIncrement()
            check(code <= 0xffff) { "Android uninstall request codes exhausted; reopen Operit" }
            Request(code).also { pending = it }
        }
        try {
            activity.runOnUiThread {
                synchronized(lock) {
                    if (pending !== request || request.result.isDone) return@runOnUiThread
                    try {
                        val intent = Intent(Intent.ACTION_DELETE, Uri.parse("package:$packageName"))
                            .putExtra(Intent.EXTRA_RETURN_RESULT, true)
                        activity.startActivityForResult(intent, request.code)
                    } catch (error: Exception) {
                        request.result.completeExceptionally(error)
                    }
                }
            }
            val resultCode = request.result.get(CONFIRMATION_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            check(resultCode == Activity.RESULT_OK) {
                "Application uninstallation canceled or rejected by Android (resultCode=$resultCode)"
            }
            val installed = try {
                activity.packageManager.getPackageInfo(packageName, 0)
                true
            } catch (_: PackageManager.NameNotFoundException) {
                false
            }
            check(!installed) { "Android returned from uninstallation but the application is still installed: $packageName" }
        } catch (error: ExecutionException) {
            throw error.cause ?: error
        } catch (error: TimeoutException) {
            throw IllegalStateException("Android uninstall confirmation timed out; application removal has not been verified", error)
        } catch (error: InterruptedException) {
            Thread.currentThread().interrupt()
            throw IllegalStateException("Android uninstall confirmation was interrupted; application removal has not been verified", error)
        } finally {
            synchronized(lock) {
                if (pending === request) pending = null
            }
        }
    }

    /** Completes only the confirmation belonging to this owner and request code. */
    fun onActivityResult(requestCode: Int, resultCode: Int): Boolean = synchronized(lock) {
        val request = pending ?: return false
        if (request.code != requestCode) return false
        request.result.complete(resultCode)
        true
    }

    /** Fails the waiting tool call when its Activity owner is detached. */
    fun release() = synchronized(lock) {
        released = true
        pending?.result?.completeExceptionally(IllegalStateException("Android uninstall owner was released before confirmation"))
        pending = null
    }
}
