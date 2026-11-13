package app.operit.accessibility

import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.os.ParcelFileDescriptor
import android.util.Log
import java.util.concurrent.Executors
import org.json.JSONObject

/** Streams versioned operations for the same-signature Operit2 application. */
class UiAutomationBinderService : Service() {
    private val worker = Executors.newSingleThreadExecutor()
    private val binder = object : IUiAutomationProvider.Stub() {
        /** Reports the exact wire version implemented by this companion. */
        override fun getProtocolVersion(): Int = 1

        /** Returns a pipe immediately and serializes device operations on one worker. */
        override fun execute(requestJson: String): ParcelFileDescriptor {
            val pipe = ParcelFileDescriptor.createPipe()
            try {
                worker.execute {
                    ParcelFileDescriptor.AutoCloseOutputStream(pipe[1]).bufferedWriter(Charsets.UTF_8).use { writer ->
                        val response = try {
                            val service = UiAccessibilityService.connected
                                ?: error("Enable Operit2 Accessibility in Android accessibility settings")
                            JSONObject().put("success", true).put("data", service.execute(JSONObject(requestJson)))
                        } catch (error: Exception) {
                            Log.e("OperitUiAutomation", "UI operation failed", error)
                            JSONObject().put("success", false).put("error", error.message ?: error.javaClass.simpleName)
                        }
                        try { writer.write(response.toString()) }
                        catch (error: Exception) { Log.e("OperitUiAutomation", "UI response stream failed", error) }
                    }
                }
            } catch (error: Exception) {
                pipe.forEach { it.close() }
                throw error
            }
            return pipe[0]
        }
    }

    /** Exposes the protocol only through the manifest-protected explicit service binding. */
    override fun onBind(intent: Intent): IBinder = binder

    /** Finishes already accepted operations before shutting down the worker. */
    override fun onDestroy() {
        worker.shutdown()
        super.onDestroy()
    }
}
