package app.operit

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

class AndroidPlatformChannel(
    private val activity: MainActivity,
    private val runtimeHost: AndroidRuntimeHost,
) {
    private var pendingPermissionResult: MethodChannel.Result? = null

    fun handle(call: MethodCall, result: MethodChannel.Result): Boolean {
        when (call.method) {
            "androidRuntimePaths" -> androidRuntimePaths(result)
            "localRuntimeStoragePaths" -> localRuntimeStoragePaths(call, result)
            "setLocalRuntimeStorage" -> setLocalRuntimeStorage(call, result)
            "hostOnboardingPermissionSnapshot" -> hostOnboardingPermissionSnapshot(call, result)
            "hostOnboardingRequestPermission" -> hostOnboardingRequestPermission(call, result)
            else -> return false
        }
        return true
    }

    private fun hostOnboardingPermissionSnapshot(call: MethodCall, result: MethodChannel.Result) {
        val hostId = call.argument<String>("hostId")
        if (hostId != "android") {
            result.error("INVALID_HOST", "Invalid onboarding host", null)
            return
        }
        onboardingPermissionSnapshot(result)
    }

    private fun hostOnboardingRequestPermission(call: MethodCall, result: MethodChannel.Result) {
        val hostId = call.argument<String>("hostId")
        if (hostId != "android") {
            result.error("INVALID_HOST", "Invalid onboarding host", null)
            return
        }
        onboardingRequestPermission(call, result)
    }

    private fun androidRuntimePaths(result: MethodChannel.Result) {
        Thread {
            try {
                val response = runtimeHost.androidRuntimePathsMap()
                activity.runOnUiThread { result.success(response) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("RUNTIME_BRIDGE_ERROR", error.message, null)
                }
            }
        }.start()
    }

    /** Returns local runtime storage paths for the requested root. */
    private fun localRuntimeStoragePaths(call: MethodCall, result: MethodChannel.Result) {
        try {
            result.success(runtimeHost.storagePathsMap(call.argument<String>("storageRoot")))
        } catch (error: Throwable) {
            result.error("RUNTIME_STORAGE_PATHS_ERROR", error.message, null)
        }
    }

    /** Applies the local runtime storage root before runtime creation. */
    private fun setLocalRuntimeStorage(call: MethodCall, result: MethodChannel.Result) {
        try {
            runtimeHost.setStorageRoot(call.argument<String>("storageRoot"))
            result.success(null)
        } catch (error: Throwable) {
            result.error("RUNTIME_STORAGE_SET_ERROR", error.message, null)
        }
    }

    private fun onboardingPermissionSnapshot(result: MethodChannel.Result) {
        result.success(
            mapOf(
                "android.location" to requirement(
                    "android.location",
                    hasPermission(Manifest.permission.ACCESS_FINE_LOCATION),
                ),
                "android.bluetooth" to requirement(
                    "android.bluetooth",
                    hasBluetoothConnectPermission() && hasBluetoothScanPermission(),
                ),
                "android.overlay" to requirement("android.overlay", canDrawOverlays()),
                "android.batteryOptimization" to requirement(
                    "android.batteryOptimization",
                    isIgnoringBatteryOptimizations(),
                ),
            ),
        )
    }

    private fun onboardingRequestPermission(call: MethodCall, result: MethodChannel.Result) {
        when (call.argument<String>("requirementId")) {
            "android.location" -> requestRuntimePermissions(arrayOf(Manifest.permission.ACCESS_FINE_LOCATION), result)
            "android.bluetooth" -> requestBluetoothPermissions(result)
            "android.overlay" -> {
                openOverlayPermissionSettings()
                result.success(null)
            }
            "android.batteryOptimization" -> {
                openBatteryOptimizationSettings()
                result.success(null)
            }
            else -> {
                result.error("INVALID_ONBOARDING_REQUIREMENT", "Invalid onboarding requirement", null)
                return
            }
        }
    }

    fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ): Boolean {
        if (requestCode != ONBOARDING_PERMISSION_REQUEST_CODE) {
            return false
        }
        pendingPermissionResult?.success(null)
        pendingPermissionResult = null
        return true
    }

    private fun requestBluetoothPermissions(result: MethodChannel.Result) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
            result.success(null)
            return
        }
        requestRuntimePermissions(
            arrayOf(
                Manifest.permission.BLUETOOTH_CONNECT,
                Manifest.permission.BLUETOOTH_SCAN,
            ),
            result,
        )
    }

    private fun requestRuntimePermissions(permissions: Array<String>, result: MethodChannel.Result) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) {
            result.success(null)
            return
        }
        val missing =
            permissions.filter { activity.checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }
        if (missing.isEmpty()) {
            result.success(null)
            return
        }
        if (pendingPermissionResult != null) {
            result.error("PERMISSION_REQUEST_ACTIVE", "An onboarding permission request is already active", null)
            return
        }
        pendingPermissionResult = result
        activity.requestPermissions(missing.toTypedArray(), ONBOARDING_PERMISSION_REQUEST_CODE)
    }

    private fun openOverlayPermissionSettings() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val intent =
                Intent(
                    Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                    Uri.parse("package:${activity.packageName}"),
                )
            activity.startActivity(intent)
        }
    }

    private fun openBatteryOptimizationSettings() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val intent =
                Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
                    data = Uri.parse("package:${activity.packageName}")
                }
            activity.startActivity(intent)
        }
    }

    private fun hasBluetoothConnectPermission(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
            return true
        }
        return hasPermission(Manifest.permission.BLUETOOTH_CONNECT)
    }

    private fun hasBluetoothScanPermission(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
            return true
        }
        return hasPermission(Manifest.permission.BLUETOOTH_SCAN)
    }

    private fun hasPermission(permission: String): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) {
            return true
        }
        return activity.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED
    }

    private fun canDrawOverlays(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) {
            return true
        }
        return Settings.canDrawOverlays(activity)
    }

    private fun isIgnoringBatteryOptimizations(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) {
            return true
        }
        val powerManager = activity.getSystemService(Context.POWER_SERVICE) as PowerManager
        return powerManager.isIgnoringBatteryOptimizations(activity.packageName)
    }

    private fun requirement(id: String, satisfied: Boolean): Map<String, Any> {
        return mapOf(
            "id" to id,
            "status" to if (satisfied) "Satisfied" else "Missing",
        )
    }

    private companion object {
        private const val ONBOARDING_PERMISSION_REQUEST_CODE = 2407
    }
}
