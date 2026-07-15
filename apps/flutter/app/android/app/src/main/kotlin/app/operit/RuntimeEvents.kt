package app.operit

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.content.Intent
import android.os.BatteryManager
import java.util.TimeZone
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

    /** Delivers one normalized event and requires Core to accept its canonical payload. */
    fun emit(runtimeHandle: Long, event: JSONObject) {
        val response = JSONObject(OperitRuntimeNative.emitRuntimeEvent(runtimeHandle, event.toString()))
        check(response.getBoolean("ok")) {
            response.optString("error", "Core rejected Android runtime event")
        }
    }
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
        val data = systemData(action, extras)
        return RuntimeEvents.androidBroadcast(systemTopic(action), data)
    }

    fun bluetoothBroadcast(intent: Intent, device: BluetoothDevice?, extras: JSONObject): JSONObject {
        val action = requireNotNull(intent.action) {
            "bluetooth broadcast action is required"
        }
        val data = bluetoothData(action, device, extras)
        return RuntimeEvents.androidBroadcast(bluetoothTopic(action), data)
    }

    /** Builds one platform-independent network change payload. */
    fun networkChanged(
        connected: Boolean,
        networkType: String,
        metered: Boolean?,
    ): JSONObject = JSONObject()
        .put("connected", connected)
        .put("networkType", networkType)
        .put("metered", metered)
        .put("interfaceName", JSONObject.NULL)

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

    /** Converts one Android system broadcast into its canonical topic data. */
    private fun systemData(action: String, extras: JSONObject): JSONObject = when (action) {
        Intent.ACTION_BOOT_COMPLETED -> JSONObject().put("bootCompleted", true)
        Intent.ACTION_POWER_CONNECTED -> JSONObject()
            .put("connected", true)
            .put("source", powerSource(extras))
            .put("batteryLevel", batteryLevel(extras))
        Intent.ACTION_POWER_DISCONNECTED -> JSONObject()
            .put("connected", false)
            .put("source", "battery")
            .put("batteryLevel", batteryLevel(extras))
        Intent.ACTION_BATTERY_LOW -> JSONObject()
            .put("low", true)
            .put("level", batteryLevel(extras))
            .put("charging", batteryCharging(extras))
        Intent.ACTION_BATTERY_OKAY -> JSONObject()
            .put("low", false)
            .put("level", batteryLevel(extras))
            .put("charging", batteryCharging(extras))
        Intent.ACTION_SCREEN_ON -> JSONObject().put("screenOn", true)
        Intent.ACTION_SCREEN_OFF -> JSONObject().put("screenOn", false)
        Intent.ACTION_USER_PRESENT -> JSONObject().put("present", true)
        Intent.ACTION_TIME_TICK,
        Intent.ACTION_DATE_CHANGED -> JSONObject()
            .put("timestampMillis", System.currentTimeMillis())
            .put("timezone", TimeZone.getDefault().id)
        Intent.ACTION_TIMEZONE_CHANGED -> JSONObject()
            .put("timestampMillis", System.currentTimeMillis())
            .put("timezone", extras.optString("time-zone", TimeZone.getDefault().id))
        Intent.ACTION_AIRPLANE_MODE_CHANGED -> JSONObject()
            .put("enabled", extras.getBoolean("state"))
        Intent.ACTION_HEADSET_PLUG -> JSONObject()
            .put("connected", extras.getInt("state") == 1)
            .put("deviceName", extras.optString("name").takeIf { it.isNotEmpty() })
            .put("hasMicrophone", extras.optInt("microphone", -1).takeIf { it >= 0 }?.let { it == 1 })
        else -> error("Unsupported Android system broadcast action: $action")
    }

    /** Converts one Android Bluetooth broadcast into its canonical topic data. */
    private fun bluetoothData(
        action: String,
        device: BluetoothDevice?,
        extras: JSONObject,
    ): JSONObject {
        if (action == BluetoothAdapter.ACTION_CONNECTION_STATE_CHANGED) {
            val state = extras.getInt(BluetoothAdapter.EXTRA_CONNECTION_STATE)
            return JSONObject()
                .put("powered", JSONObject.NULL)
                .put("connected", state == BluetoothAdapter.STATE_CONNECTED)
        }
        if (action == BluetoothAdapter.ACTION_STATE_CHANGED) {
            val state = extras.getInt(BluetoothAdapter.EXTRA_STATE)
            return JSONObject()
                .put("powered", state == BluetoothAdapter.STATE_ON)
                .put("connected", JSONObject.NULL)
        }
        val data = JSONObject()
            .put("deviceName", device?.name)
            .put("deviceAddress", device?.address)
            .put("connected", JSONObject.NULL)
            .put("bonded", JSONObject.NULL)
            .put("rssi", extras.optInt(BluetoothDevice.EXTRA_RSSI, Int.MIN_VALUE).takeIf { it != Int.MIN_VALUE })
        when (action) {
            BluetoothDevice.ACTION_ACL_CONNECTED -> data.put("connected", true)
            BluetoothDevice.ACTION_ACL_DISCONNECTED -> data.put("connected", false)
            BluetoothDevice.ACTION_BOND_STATE_CHANGED -> {
                val bondState = extras.getInt(BluetoothDevice.EXTRA_BOND_STATE)
                data.put("bonded", bondState == BluetoothDevice.BOND_BONDED)
            }
        }
        return data
    }

    /** Reads a normalized Android battery percentage from broadcast extras. */
    private fun batteryLevel(extras: JSONObject): Double? {
        val level = extras.optInt(BatteryManager.EXTRA_LEVEL, -1)
        val scale = extras.optInt(BatteryManager.EXTRA_SCALE, -1)
        return if (level >= 0 && scale > 0) level * 100.0 / scale else null
    }

    /** Reads Android battery charging state when the broadcast provides it. */
    private fun batteryCharging(extras: JSONObject): Boolean? {
        val status = extras.optInt(BatteryManager.EXTRA_STATUS, -1)
        return when (status) {
            BatteryManager.BATTERY_STATUS_CHARGING,
            BatteryManager.BATTERY_STATUS_FULL -> true
            BatteryManager.BATTERY_STATUS_DISCHARGING,
            BatteryManager.BATTERY_STATUS_NOT_CHARGING -> false
            else -> null
        }
    }

    /** Maps Android's plugged source code to the shared power source vocabulary. */
    private fun powerSource(extras: JSONObject): String? = when (extras.optInt(BatteryManager.EXTRA_PLUGGED, -1)) {
        BatteryManager.BATTERY_PLUGGED_AC -> "ac"
        BatteryManager.BATTERY_PLUGGED_USB -> "usb"
        BatteryManager.BATTERY_PLUGGED_WIRELESS -> "wireless"
        0 -> "battery"
        -1 -> null
        else -> "unknown"
    }
}
