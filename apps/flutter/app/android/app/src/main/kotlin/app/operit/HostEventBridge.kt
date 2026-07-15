package app.operit

import android.bluetooth.BluetoothDevice
import android.app.KeyguardManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.os.Build
import android.os.PowerManager
import org.json.JSONArray
import org.json.JSONObject

object HostEventBridge {
    private val receivers = mutableListOf<BroadcastReceiver>()
    private var connectivityManager: ConnectivityManager? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null

    fun startHostEventReceivers(
        context: Context,
        runtimeHandle: () -> Long,
    ) {
        clear(context)
        registerAndroidBroadcastReceiver(context, runtimeHandle)
        registerBluetoothReceiver(context, runtimeHandle)
        registerNetworkCallback(context, runtimeHandle)
        registerPowerIdleReceiver(context, runtimeHandle)
    }

    fun clear(context: Context) {
        for (receiver in receivers) {
            try {
                context.unregisterReceiver(receiver)
            } catch (_: IllegalArgumentException) {
            }
        }
        receivers.clear()
        val manager = connectivityManager
        val callback = networkCallback
        if (manager != null && callback != null) {
            manager.unregisterNetworkCallback(callback)
        }
        connectivityManager = null
        networkCallback = null
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
                RuntimeEvents.emit(runtimeHandle(), event)
                emitSessionEvent(context, runtimeHandle, intent.action)
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
                RuntimeEvents.emit(runtimeHandle(), event)
            }
        }
        registerReceiver(context, receiver, filter)
        receivers.add(receiver)
    }

    /** Registers Android's active-network callback and emits the shared network payload. */
    private fun registerNetworkCallback(
        context: Context,
        runtimeHandle: () -> Long,
    ) {
        val manager = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val callback = object : ConnectivityManager.NetworkCallback() {
            /** Emits normalized connectivity when Android reports network capabilities. */
            override fun onCapabilitiesChanged(
                network: Network,
                capabilities: NetworkCapabilities,
            ) {
                emitNetworkEvent(runtimeHandle, capabilities)
            }

            /** Emits the shared disconnected state when Android loses its active network. */
            override fun onLost(network: Network) {
                emitNetworkEvent(runtimeHandle, null)
            }
        }
        manager.registerDefaultNetworkCallback(callback)
        connectivityManager = manager
        networkCallback = callback
    }

    /** Registers Android device-idle changes as normalized power sleep and wake events. */
    private fun registerPowerIdleReceiver(
        context: Context,
        runtimeHandle: () -> Long,
    ) {
        val receiver = object : BroadcastReceiver() {
            /** Converts Android Doze state into the shared power suspension topic. */
            override fun onReceive(ctx: Context, intent: Intent) {
                val powerManager = ctx.getSystemService(Context.POWER_SERVICE) as PowerManager
                val sleeping = powerManager.isDeviceIdleMode
                val event = RuntimeEvents.androidBroadcast(
                    if (sleeping) RuntimeEvents.Topic.SYSTEM_POWER_SLEEP else RuntimeEvents.Topic.SYSTEM_POWER_WAKE,
                    JSONObject().put("sleeping", sleeping),
                )
                RuntimeEvents.emit(runtimeHandle(), event)
            }
        }
        registerReceiver(
            context,
            receiver,
            IntentFilter(PowerManager.ACTION_DEVICE_IDLE_MODE_CHANGED),
        )
        receivers.add(receiver)
    }

    /** Emits Android lock and unlock state alongside screen and user-presence broadcasts. */
    private fun emitSessionEvent(
        context: Context,
        runtimeHandle: () -> Long,
        action: String?,
    ) {
        val locked = when (action) {
            Intent.ACTION_SCREEN_OFF -> {
                val keyguard = context.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
                if (!keyguard.isKeyguardLocked) {
                    return
                }
                true
            }
            Intent.ACTION_USER_PRESENT -> false
            else -> return
        }
        val event = RuntimeEvents.androidBroadcast(
            if (locked) RuntimeEvents.Topic.SYSTEM_SESSION_LOCK else RuntimeEvents.Topic.SYSTEM_SESSION_UNLOCK,
            JSONObject().put("locked", locked),
        )
        RuntimeEvents.emit(runtimeHandle(), event)
    }

    /** Converts Android network capabilities and forwards one normalized event to Core. */
    private fun emitNetworkEvent(
        runtimeHandle: () -> Long,
        capabilities: NetworkCapabilities?,
    ) {
        val connected = capabilities?.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) == true
        val networkType = when {
            capabilities == null -> "none"
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_VPN) -> "vpn"
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> "wifi"
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> "cellular"
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) -> "ethernet"
            else -> "other"
        }
        val metered = capabilities?.let {
            !it.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)
        }
        val event = RuntimeEvents.androidBroadcast(
            RuntimeEvents.Topic.SYSTEM_NETWORK_CHANGED,
            AndroidRuntimeEvents.networkChanged(connected, networkType, metered),
        )
        RuntimeEvents.emit(runtimeHandle(), event)
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
            OperitCoreService.startAfterBoot(context)
        }
    }
}
