package app.operit

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.content.Intent
import android.net.wifi.WifiManager
import org.json.JSONObject

object RuntimeEvents {
    object Domain {
        const val HOST = "host"
        const val APP = "app"
        const val RUNTIME = "runtime"
    }

    object Source {
        const val ANDROID_BROADCAST = "android.broadcast"
    }

    object Platform {
        const val ANDROID = "android"
    }

    object Topic {
        const val SYSTEM_BOOT_COMPLETED = "system.boot.completed"
        const val SYSTEM_POWER_CONNECTED = "system.power.connected"
        const val SYSTEM_POWER_DISCONNECTED = "system.power.disconnected"
        const val SYSTEM_POWER_SLEEP = "system.power.sleep"
        const val SYSTEM_POWER_WAKE = "system.power.wake"
        const val SYSTEM_BATTERY_LOW = "system.battery.low"
        const val SYSTEM_BATTERY_OKAY = "system.battery.okay"
        const val SYSTEM_SCREEN_ON = "system.screen.on"
        const val SYSTEM_SCREEN_OFF = "system.screen.off"
        const val SYSTEM_USER_PRESENT = "system.user.present"
        const val SYSTEM_TIME_TICK = "system.time.tick"
        const val SYSTEM_DATE_CHANGED = "system.date.changed"
        const val SYSTEM_TIMEZONE_CHANGED = "system.timezone.changed"
        const val SYSTEM_AIRPLANE_MODE_CHANGED = "system.airplane_mode.changed"
        const val SYSTEM_HEADSET_PLUG = "system.headset.plug"
        const val SYSTEM_NETWORK_CHANGED = "system.network.changed"
        const val SYSTEM_SESSION_LOCK = "system.session.lock"
        const val SYSTEM_SESSION_UNLOCK = "system.session.unlock"
        const val BLUETOOTH_DEVICE_FOUND = "bluetooth.device.found"
        const val BLUETOOTH_DEVICE_NAME_CHANGED = "bluetooth.device.name_changed"
        const val BLUETOOTH_DEVICE_CONNECTED = "bluetooth.device.connected"
        const val BLUETOOTH_DEVICE_DISCONNECTED = "bluetooth.device.disconnected"
        const val BLUETOOTH_DEVICE_BOND_STATE_CHANGED = "bluetooth.device.bond_state_changed"
        const val BLUETOOTH_ADAPTER_CONNECTION_STATE_CHANGED = "bluetooth.adapter.connection_state_changed"
        const val BLUETOOTH_ADAPTER_POWERED_CHANGED = "bluetooth.adapter.powered_changed"
    }

    fun androidBroadcast(topic: String, data: JSONObject): JSONObject = JSONObject()
        .put("domain", Domain.HOST)
        .put("source", Source.ANDROID_BROADCAST)
        .put("topic", topic)
        .put("platform", Platform.ANDROID)
        .put("payload", data)
        .put("occurredAtMillis", System.currentTimeMillis())
}

object AndroidRuntimeEvents {
    val systemBroadcastActions = setOf(
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

    val bluetoothBroadcastActions = setOf(
        BluetoothDevice.ACTION_FOUND,
        BluetoothDevice.ACTION_NAME_CHANGED,
        BluetoothDevice.ACTION_ACL_CONNECTED,
        BluetoothDevice.ACTION_ACL_DISCONNECTED,
        BluetoothDevice.ACTION_BOND_STATE_CHANGED,
        BluetoothAdapter.ACTION_CONNECTION_STATE_CHANGED,
        BluetoothAdapter.ACTION_STATE_CHANGED,
    )

    fun systemBroadcast(intent: Intent, extras: JSONObject): JSONObject {
        val action = requireNotNull(intent.action) {
            "android broadcast action is required"
        }
        val data = JSONObject()
            .put("action", action)
            .put("extras", extras)
        return RuntimeEvents.androidBroadcast(systemTopic(action), data)
    }

    fun bluetoothBroadcast(intent: Intent, device: BluetoothDevice?, extras: JSONObject): JSONObject {
        val action = requireNotNull(intent.action) {
            "bluetooth broadcast action is required"
        }
        val data = JSONObject()
            .put("action", action)
            .put("deviceName", device?.name)
            .put("deviceAddress", device?.address)
            .put("extras", extras)
        return RuntimeEvents.androidBroadcast(bluetoothTopic(action), data)
    }

    private fun systemTopic(action: String): String = when (action) {
        Intent.ACTION_BOOT_COMPLETED -> RuntimeEvents.Topic.SYSTEM_BOOT_COMPLETED
        Intent.ACTION_POWER_CONNECTED -> RuntimeEvents.Topic.SYSTEM_POWER_CONNECTED
        Intent.ACTION_POWER_DISCONNECTED -> RuntimeEvents.Topic.SYSTEM_POWER_DISCONNECTED
        Intent.ACTION_BATTERY_LOW -> RuntimeEvents.Topic.SYSTEM_BATTERY_LOW
        Intent.ACTION_BATTERY_OKAY -> RuntimeEvents.Topic.SYSTEM_BATTERY_OKAY
        Intent.ACTION_SCREEN_ON -> RuntimeEvents.Topic.SYSTEM_SCREEN_ON
        Intent.ACTION_SCREEN_OFF -> RuntimeEvents.Topic.SYSTEM_SCREEN_OFF
        Intent.ACTION_USER_PRESENT -> RuntimeEvents.Topic.SYSTEM_USER_PRESENT
        Intent.ACTION_TIME_TICK -> RuntimeEvents.Topic.SYSTEM_TIME_TICK
        Intent.ACTION_DATE_CHANGED -> RuntimeEvents.Topic.SYSTEM_DATE_CHANGED
        Intent.ACTION_TIMEZONE_CHANGED -> RuntimeEvents.Topic.SYSTEM_TIMEZONE_CHANGED
        Intent.ACTION_AIRPLANE_MODE_CHANGED -> RuntimeEvents.Topic.SYSTEM_AIRPLANE_MODE_CHANGED
        Intent.ACTION_HEADSET_PLUG -> RuntimeEvents.Topic.SYSTEM_HEADSET_PLUG
        WifiManager.WIFI_STATE_CHANGED_ACTION -> RuntimeEvents.Topic.SYSTEM_NETWORK_CHANGED
        else -> error("Unsupported Android broadcast action: $action")
    }

    private fun bluetoothTopic(action: String): String = when (action) {
        BluetoothDevice.ACTION_FOUND -> RuntimeEvents.Topic.BLUETOOTH_DEVICE_FOUND
        BluetoothDevice.ACTION_NAME_CHANGED -> RuntimeEvents.Topic.BLUETOOTH_DEVICE_NAME_CHANGED
        BluetoothDevice.ACTION_ACL_CONNECTED -> RuntimeEvents.Topic.BLUETOOTH_DEVICE_CONNECTED
        BluetoothDevice.ACTION_ACL_DISCONNECTED -> RuntimeEvents.Topic.BLUETOOTH_DEVICE_DISCONNECTED
        BluetoothDevice.ACTION_BOND_STATE_CHANGED -> RuntimeEvents.Topic.BLUETOOTH_DEVICE_BOND_STATE_CHANGED
        BluetoothAdapter.ACTION_CONNECTION_STATE_CHANGED -> RuntimeEvents.Topic.BLUETOOTH_ADAPTER_CONNECTION_STATE_CHANGED
        BluetoothAdapter.ACTION_STATE_CHANGED -> RuntimeEvents.Topic.BLUETOOTH_ADAPTER_POWERED_CHANGED
        else -> error("Unsupported Bluetooth broadcast action: $action")
    }
}
