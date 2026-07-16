package app.operit

import android.Manifest
import android.app.AppOpsManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
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
            "localRuntimeStorageDefaults" -> localRuntimeStorageDefaults(result)
            "localRuntimeStoragePaths" -> localRuntimeStoragePaths(call, result)
            "setLocalRuntimeStorage" -> setLocalRuntimeStorage(call, result)
            "startLocalCoreService" -> startLocalCoreService(result)
            "localRuntimeStartupStatus" -> localRuntimeStartupStatus(result)
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

    /** Returns the platform default runtime and workspace roots. */
    private fun localRuntimeStorageDefaults(result: MethodChannel.Result) {
        result.success(runtimeHost.defaultStoragePathsMap())
    }

    /** Returns local runtime storage paths for requested roots. */
    private fun localRuntimeStoragePaths(call: MethodCall, result: MethodChannel.Result) {
        try {
            result.success(
                runtimeHost.storagePathsMap(
                    call.argument<String>("runtimeRoot"),
                    call.argument<String>("workspaceRoot"),
                ),
            )
        } catch (error: Throwable) {
            result.error("RUNTIME_STORAGE_PATHS_ERROR", error.message, null)
        }
    }

    /** Installs local runtime and workspace roots. */
    private fun setLocalRuntimeStorage(call: MethodCall, result: MethodChannel.Result) {
        val runtimeRoot = call.argument<String>("runtimeRoot")
        val workspaceRoot = call.argument<String>("workspaceRoot")
        runtimeHost.runBackground {
            try {
                runtimeHost.setStorageRoots(runtimeRoot, workspaceRoot)
                activity.runOnUiThread { result.success(null) }
            } catch (error: Throwable) {
                activity.runOnUiThread {
                    result.error("RUNTIME_STORAGE_SET_ERROR", error.message, null)
                }
            }
        }
    }

    /** Starts the process-level local Core foreground service. */
    private fun startLocalCoreService(result: MethodChannel.Result) {
        try {
            OperitCoreService.start(activity.applicationContext)
            result.success(null)
        } catch (error: Throwable) {
            result.error("CORE_SERVICE_START_ERROR", error.message, null)
        }
    }

    /** Returns the latest native local-runtime startup stage. */
    private fun localRuntimeStartupStatus(result: MethodChannel.Result) {
        result.success(runtimeHost.runtimeStartupStatusMap())
    }

    private fun onboardingPermissionSnapshot(result: MethodChannel.Result) {
        result.success(
            mapOf(
                "android.fileManagement" to requirement(
                    "android.fileManagement",
                    hasFileManagementPermission(),
                ),
                "android.notifications" to requirement(
                    "android.notifications",
                    hasNotificationPermission(),
                ),
                "android.appList" to requirement(
                    "android.appList",
                    hasPackageQueryVisibilityPermission(),
                ),
                "android.usageStats" to requirement(
                    "android.usageStats",
                    hasUsageStatsPermission(),
                ),
                "android.writeSettings" to requirement(
                    "android.writeSettings",
                    canWriteSystemSettings(),
                ),
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
            "android.fileManagement" -> requestFileManagementPermission(result)
            "android.notifications" -> requestNotificationPermission(result)
            "android.appList" -> acknowledgeManifestManagedPermission(result)
            "android.usageStats" -> {
                openUsageAccessSettings()
                result.success(null)
            }
            "android.writeSettings" -> {
                openWriteSettings()
                result.success(null)
            }
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

    /** Requests broad shared-storage access for Android file tools. */
    private fun requestFileManagementPermission(result: MethodChannel.Result) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val intent =
                Intent(
                    Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                    Uri.parse("package:${activity.packageName}"),
                )
            activity.startActivity(intent)
            result.success(null)
            return
        }
        val permissions =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                arrayOf(Manifest.permission.READ_EXTERNAL_STORAGE)
            } else {
                arrayOf(
                    Manifest.permission.READ_EXTERNAL_STORAGE,
                    Manifest.permission.WRITE_EXTERNAL_STORAGE,
                )
            }
        requestRuntimePermissions(permissions, result)
    }

    /** Requests notification posting access required by Android task status surfaces. */
    private fun requestNotificationPermission(result: MethodChannel.Result) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
            result.success(null)
            return
        }
        requestRuntimePermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), result)
    }

    /** Acknowledges permissions controlled by the Android manifest rather than a user runtime dialog. */
    private fun acknowledgeManifestManagedPermission(result: MethodChannel.Result) {
        result.success(null)
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

    /** Opens Android usage-access settings for app foreground-time statistics. */
    private fun openUsageAccessSettings() {
        val intent = Intent(Settings.ACTION_USAGE_ACCESS_SETTINGS)
        activity.startActivity(intent)
    }

    /** Opens Android write-settings access for system setting mutations. */
    private fun openWriteSettings() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val intent =
                Intent(
                    Settings.ACTION_MANAGE_WRITE_SETTINGS,
                    Uri.parse("package:${activity.packageName}"),
                )
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

    /** Returns whether Android shared-storage access is available to file tools. */
    private fun hasFileManagementPermission(): Boolean {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            return Environment.isExternalStorageManager()
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            return hasPermission(Manifest.permission.READ_EXTERNAL_STORAGE)
        }
        return hasPermission(Manifest.permission.READ_EXTERNAL_STORAGE) &&
            hasPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE)
    }

    /** Returns whether Android notification posting access is available. */
    private fun hasNotificationPermission(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
            return true
        }
        return hasPermission(Manifest.permission.POST_NOTIFICATIONS)
    }

    /** Returns whether package visibility for Android app listing is granted by the manifest. */
    private fun hasPackageQueryVisibilityPermission(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
            return true
        }
        return hasPermission(Manifest.permission.QUERY_ALL_PACKAGES)
    }

    /** Returns whether Android usage-access statistics are available to this app. */
    private fun hasUsageStatsPermission(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.LOLLIPOP) {
            return true
        }
        val appOps = activity.getSystemService(Context.APP_OPS_SERVICE) as AppOpsManager
        val mode =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                appOps.unsafeCheckOpNoThrow(
                    AppOpsManager.OPSTR_GET_USAGE_STATS,
                    android.os.Process.myUid(),
                    activity.packageName,
                )
            } else {
                appOps.checkOpNoThrow(
                    AppOpsManager.OPSTR_GET_USAGE_STATS,
                    android.os.Process.myUid(),
                    activity.packageName,
                )
            }
        return mode == AppOpsManager.MODE_ALLOWED
    }

    /** Returns whether Android system settings can be modified by this app. */
    private fun canWriteSystemSettings(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) {
            return true
        }
        return Settings.System.canWrite(activity)
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
