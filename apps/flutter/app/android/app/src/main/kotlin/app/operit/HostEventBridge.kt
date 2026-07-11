package app.operit

import android.bluetooth.BluetoothDevice
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import org.json.JSONArray
import org.json.JSONObject

object HostEventBridge {
    private val receivers = mutableListOf<BroadcastReceiver>()

    fun startHostEventReceivers(
        context: Context,
        runtimeHandle: () -> Long,
    ) {
        clear(context)
        registerAndroidBroadcastReceiver(context, runtimeHandle)
        registerBluetoothReceiver(context, runtimeHandle)
    }

    fun clear(context: Context) {
        for (receiver in receivers) {
            try {
                context.unregisterReceiver(receiver)
            } catch (_: IllegalArgumentException) {
            }
        }
        receivers.clear()
    }

    private fun registerAndroidBroadcastReceiver(
        context: Context,
        runtimeHandle: () -> Long,
    ) {
        val filter = IntentFilter()
        for (action in AndroidRuntimeEvents.systemBroadcastActions) {
            filter.addAction(action)
        }
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(ctx: Context, intent: Intent) {
                val event = AndroidRuntimeEvents.systemBroadcast(intent, intentExtrasToJson(intent))
                OperitRuntimeNative.emitRuntimeEvent(runtimeHandle(), event.toString())
            }
        }
        registerReceiver(context, receiver, filter)
        receivers.add(receiver)
    }

    private fun registerBluetoothReceiver(
        context: Context,
        runtimeHandle: () -> Long,
    ) {
        val filter = IntentFilter().apply {
            for (action in AndroidRuntimeEvents.bluetoothBroadcastActions) {
                addAction(action)
            }
        }
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(ctx: Context, intent: Intent) {
                val device = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableExtra(
                        BluetoothDevice.EXTRA_DEVICE,
                        BluetoothDevice::class.java,
                    )
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE)
                }
                val event = AndroidRuntimeEvents.bluetoothBroadcast(
                    intent,
                    device,
                    intentExtrasToJson(intent),
                )
                OperitRuntimeNative.emitRuntimeEvent(runtimeHandle(), event.toString())
            }
        }
        registerReceiver(context, receiver, filter)
        receivers.add(receiver)
    }

    private fun registerReceiver(context: Context, receiver: BroadcastReceiver, filter: IntentFilter) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            context.registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            context.registerReceiver(receiver, filter)
        }
    }

    private fun intentExtrasToJson(intent: Intent): JSONObject {
        val json = JSONObject()
        val extras = intent.extras
        if (extras == null) {
            return json
        }
        for (key in extras.keySet()) {
            val value = extras.get(key)
            when (value) {
                null -> json.put(key, JSONObject.NULL)
                is String -> json.put(key, value)
                is Int -> json.put(key, value)
                is Long -> json.put(key, value)
                is Double -> json.put(key, value)
                is Float -> json.put(key, value.toDouble())
                is Boolean -> json.put(key, value)
                is Short -> json.put(key, value.toInt())
                is Byte -> json.put(key, value.toInt())
                is IntArray -> json.put(key, JSONArray(value.toList()))
                is LongArray -> json.put(key, JSONArray(value.toList()))
                is BooleanArray -> json.put(key, JSONArray(value.toList()))
                is Array<*> -> json.put(key, JSONArray(value.toList()))
                else -> json.put(key, value.toString())
            }
        }
        return json
    }
}

class BootReceiver : BroadcastReceiver() {
    /** Starts the persistent Core service after boot when storage is configured. */
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        if (AndroidRuntimeStorageConfigStore.read(context) != null) {
            OperitCoreService.start(context)
        }
    }
}
