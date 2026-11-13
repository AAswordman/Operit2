package app.operit

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.IBinder
import android.os.ParcelFileDescriptor
import app.operit.accessibility.IUiAutomationProvider
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.json.JSONObject

/** Adapts the in-repository accessibility companion to the platform-neutral UI Host. */
class AndroidUiAutomation(private val context: Context) {
    private val lock = Any()
    private val operationLock = Any()
    private var connection: ProviderConnection? = null
    private var released = false

    /** Associates every asynchronous service callback with its own binding. */
    private class ProviderConnection : ServiceConnection {
        val ready = CountDownLatch(1)
        @Volatile var service: IUiAutomationProvider? = null

        /** Publishes the companion's versioned Binder endpoint. */
        override fun onServiceConnected(name: ComponentName, binder: IBinder) {
            service = IUiAutomationProvider.Stub.asInterface(binder)
            ready.countDown()
        }

        /** Invalidates a disconnected endpoint. */
        override fun onServiceDisconnected(name: ComponentName) { service = null }

        /** Completes a rejected binding without waiting for the deadline. */
        override fun onNullBinding(name: ComponentName) { ready.countDown() }

        /** Invalidates a binding whose companion was replaced. */
        override fun onBindingDied(name: ComponentName) { service = null; ready.countDown() }
    }

    /** Connects to the Operit2 companion and checks its explicit wire version. */
    private fun requireProvider(): IUiAutomationProvider {
        val binding = synchronized(lock) {
            check(!released) { "UI automation owner has been released" }
            connection?.service?.let { if (it.asBinder().isBinderAlive) return it }
            val pending = ProviderConnection()
            connection?.let { context.unbindService(it) }
            connection = null
            val intent = Intent().setComponent(ComponentName("app.operit.accessibility", "app.operit.accessibility.UiAutomationBinderService"))
            check(context.bindService(intent, pending, Context.BIND_AUTO_CREATE)) {
                "Install the Operit2 Accessibility companion built from tools/android-accessibility"
            }
            connection = pending
            pending
        }
        try {
            check(binding.ready.await(5, TimeUnit.SECONDS)) { "Operit2 Accessibility connection timed out" }
            return synchronized(lock) {
                check(!released && connection === binding) { "UI automation owner has been released" }
                val service = binding.service ?: error("Operit2 Accessibility rejected the connection")
                check(service.protocolVersion == 1) { "Operit2 Accessibility protocol version is incompatible" }
                service
            }
        } catch (error: Exception) {
            synchronized(lock) {
                if (connection === binding) { context.unbindService(binding); connection = null }
            }
            throw error
        }
    }

    /** Reads the full streamed response and preserves native provider failures. */
    fun execute(request: JSONObject): JSONObject = synchronized(operationLock) {
        val descriptor = requireProvider().execute(request.toString())
            ?: error("Operit2 Accessibility returned no response stream")
        val response = ParcelFileDescriptor.AutoCloseInputStream(descriptor).bufferedReader(Charsets.UTF_8)
            .use { JSONObject(it.readText()) }
        check(response.getBoolean("success")) { response.getString("error") }
        response.getJSONObject("data")
    }

    /** Releases only the binding owned by this adapter. */
    fun release() = synchronized(lock) {
        released = true
        connection?.let { context.unbindService(it); it.service = null; it.ready.countDown() }
        connection = null
    }
}
