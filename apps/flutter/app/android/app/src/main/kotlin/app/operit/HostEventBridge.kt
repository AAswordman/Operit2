package app.operit

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.wifi.WifiManager
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
        registerAndroidBroadcastReceiver(context, runtimeHandle, commonAndroidBroadcastActions)
        registerBluetoothReceiver(context, runtimeHandle, commonBluetoothBroadcastActions)
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
        actions: Set<String>,
    ) {
        val filter = IntentFilter()
        for (action in actions) {
            filter.addAction(action)
        }
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(ctx: Context, intent: Intent) {
                val action = requireNotNull(intent.action) {
                    "android broadcast action is required"
                }
                val topic = topicForAndroidAction(action)
                val payload = JSONObject()
                    .put("topic", topic)
                    .put("platform", "android")
                    .put(
                        "data",
                        JSONObject()
                            .put("action", action)
                            .put("extras", intentExtrasToJson(intent)),
                    )
                    .put("receivedAtMillis", System.currentTimeMillis())
                OperitRuntimeNative.dispatchHostEvent(
                    runtimeHandle(),
                    "broadcast",
                    payload.toString(),
                )
            }
        }
        registerReceiver(context, receiver, filter)
        receivers.add(receiver)
    }

    private fun registerBluetoothReceiver(
        context: Context,
        runtimeHandle: () -> Long,
        actions: Set<String>,
    ) {
        val filter = IntentFilter().apply {
            for (action in actions) {
                addAction(action)
            }
        }
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(ctx: Context, intent: Intent) {
                val action = requireNotNull(intent.action) {
                    "bluetooth broadcast action is required"
                }
                val topic = topicForBluetoothAction(action)
                val device = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableExtra(
                        BluetoothDevice.EXTRA_DEVICE,
                        BluetoothDevice::class.java,
                    )
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE)
                }
                val payload = JSONObject()
                    .put("topic", topic)
                    .put("platform", "android")
                    .put(
                        "data",
                        JSONObject()
                            .put("action", action)
                            .put("deviceName", device?.name)
                            .put("deviceAddress", device?.address)
                            .put("extras", intentExtrasToJson(intent)),
                    )
                    .put("receivedAtMillis", System.currentTimeMillis())
                OperitRuntimeNative.dispatchHostEvent(
                    runtimeHandle(),
                    "broadcast",
                    payload.toString(),
                )
            }
        }
        registerReceiver(context, receiver, filter)
        receivers.add(receiver)
    }

    private fun topicForAndroidAction(action: String): String = when (action) {
        Intent.ACTION_BOOT_COMPLETED -> "system.boot.completed"
        Intent.ACTION_POWER_CONNECTED -> "system.power.connected"
        Intent.ACTION_POWER_DISCONNECTED -> "system.power.disconnected"
        Intent.ACTION_BATTERY_LOW -> "system.battery.low"
        Intent.ACTION_BATTERY_OKAY -> "system.battery.okay"
        Intent.ACTION_SCREEN_ON -> "system.screen.on"
        Intent.ACTION_SCREEN_OFF -> "system.screen.off"
        Intent.ACTION_USER_PRESENT -> "system.user.present"
        Intent.ACTION_TIME_TICK -> "system.time.tick"
        Intent.ACTION_DATE_CHANGED -> "system.date.changed"
        Intent.ACTION_TIMEZONE_CHANGED -> "system.timezone.changed"
        Intent.ACTION_AIRPLANE_MODE_CHANGED -> "system.airplane_mode.changed"
        Intent.ACTION_HEADSET_PLUG -> "system.headset.plug"
        WifiManager.WIFI_STATE_CHANGED_ACTION -> "system.network.changed"
        else -> error("Unsupported Android broadcast action: $action")
    }

    private fun topicForBluetoothAction(action: String): String = when (action) {
        BluetoothDevice.ACTION_FOUND -> "bluetooth.device.found"
        BluetoothDevice.ACTION_NAME_CHANGED -> "bluetooth.device.name_changed"
        BluetoothDevice.ACTION_ACL_CONNECTED -> "bluetooth.device.connected"
        BluetoothDevice.ACTION_ACL_DISCONNECTED -> "bluetooth.device.disconnected"
        BluetoothDevice.ACTION_BOND_STATE_CHANGED -> "bluetooth.device.bond_state_changed"
        BluetoothAdapter.ACTION_CONNECTION_STATE_CHANGED -> "bluetooth.adapter.connection_state_changed"
        else -> error("Unsupported Bluetooth broadcast action: $action")
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

    val commonAndroidBroadcastActions = setOf(
        Intent.ACTION_BOOT_COMPLETED,
        Intent.ACTION_POWER_CONNECTED,
        Intent.ACTION_POWER_DISCONNECTED,
        Intent.ACTION_BATTERY_LOW,
        Intent.ACTION_BATTERY_OKAY,
        Intent.ACTION_SCREEN_ON,
        Intent.ACTION_SCREEN_OFF,
        Intent.ACTION_USER_PRESENT,
        Intent.ACTION_TIME_TICK,
        Intent.ACTION_DATE_CHANGED,
        Intent.ACTION_TIMEZONE_CHANGED,
        Intent.ACTION_AIRPLANE_MODE_CHANGED,
        WifiManager.WIFI_STATE_CHANGED_ACTION,
        Intent.ACTION_HEADSET_PLUG,
    )

    private val commonBluetoothBroadcastActions = setOf(
        BluetoothDevice.ACTION_FOUND,
        BluetoothDevice.ACTION_NAME_CHANGED,
        BluetoothDevice.ACTION_ACL_CONNECTED,
        BluetoothDevice.ACTION_ACL_DISCONNECTED,
        BluetoothDevice.ACTION_BOND_STATE_CHANGED,
        BluetoothAdapter.ACTION_CONNECTION_STATE_CHANGED,
    )
}

class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        val launchIntent = context.packageManager.getLaunchIntentForPackage(context.packageName)
        if (launchIntent != null) {
            launchIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(launchIntent)
        }
    }
}
