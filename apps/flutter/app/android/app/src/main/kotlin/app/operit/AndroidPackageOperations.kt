package app.operit

import android.app.AppOpsManager
import android.app.usage.UsageStatsManager
import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.net.Uri
import android.os.Build
import android.os.Process
import android.provider.Settings
import app.operit.core.tools.system.AndroidPrivilegedCommandExecutor
import app.operit.core.tools.system.AndroidPrivilegedCommandTarget
import java.io.File
import org.json.JSONArray
import org.json.JSONObject

/** Android package operations invoked through the owner interaction channel. */
internal class AndroidPackageOperations(private val context: Context) {
    private val pm = context.packageManager

    fun execute(operation: String, params: JSONObject): JSONObject = when (operation) {
        "install_app" -> install(params.getString("path"))
        "uninstall_app" -> uninstall(packageName(params))
        "list_installed_apps" -> list(params.getBoolean("includeSystemApps"))
        "start_app" -> start(packageName(params), params.optString("activity", "").takeIf { it.isNotBlank() && it != "null" })
        "stop_app" -> stop(packageName(params))
        "get_app_usage_time" -> usage(params)
        else -> throw IllegalArgumentException("Unsupported Android package operation: $operation")
    }

    private fun packageName(params: JSONObject): String {
        val name = params.getString("packageName")
        require(name.matches(Regex("[A-Za-z0-9_]+(\\.[A-Za-z0-9_]+)+"))) { "Invalid Android package name" }
        return name
    }

    private fun result(operation: String, name: String, details: String) = JSONObject()
        .put("operationType", operation).put("packageName", name)
        .put("success", true).put("details", details)

    private fun install(path: String): JSONObject {
        val file = File(path)
        require(file.isAbsolute && file.isFile && file.canRead()) { "APK is not a readable absolute file: $path" }
        val info = pm.getPackageArchiveInfo(file.absolutePath, 0)
            ?: throw IllegalArgumentException("File is not a valid APK: $path")
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !pm.canRequestPackageInstalls()) {
            context.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                Uri.parse("package:${context.packageName}")).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            return result("install_app", info.packageName,
                "请允许 Operit 安装应用，然后返回点击继续安装。安装包已保留，无需重新下载。")
                .put("installState", "permission_required")
        }
        AndroidFileOpener(context).open(file, null, null)
        return result("install_app", info.packageName, "安装器已打开，请在系统窗口确认安装。")
            .put("installState", "installer_opened")
    }

    private fun uninstall(name: String): JSONObject {
        pm.getPackageInfo(name, 0)
        val intent = Intent(Intent.ACTION_DELETE, Uri.parse("package:$name"))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        context.startActivity(intent)
        return result("uninstall_app", name, "Uninstallation requested; system confirmation is required")
    }

    private fun list(includeSystem: Boolean): JSONObject {
        val packages = pm.getInstalledApplications(0).asSequence()
            .filter { includeSystem || !isSystem(it) }
            .map { it.packageName }.sorted().toList()
        return JSONObject().put("includesSystemApps", includeSystem).put("packages", JSONArray(packages))
    }

    private fun start(name: String, activity: String?): JSONObject {
        val intent = if (activity == null) {
            pm.getLaunchIntentForPackage(name)
                ?: throw IllegalStateException("No launchable activity found for $name")
        } else {
            Intent(Intent.ACTION_MAIN).setClassName(name, if (activity.startsWith('.')) name + activity else activity)
        }
        context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        return result("start_app", name, "Launch requested")
    }

    private fun stop(name: String): JSONObject {
        pm.getPackageInfo(name, 0)
        val target = when {
            AndroidPrivilegeAuthorization.isShizukuAuthorized() -> AndroidPrivilegedCommandTarget.Shizuku
            AndroidPrivilegeAuthorization.hasRootConsent(context) -> AndroidPrivilegedCommandTarget.RootAuto
            else -> throw SecurityException("Force stopping apps requires authorized Shizuku or Root")
        }
        val response = AndroidPrivilegedCommandExecutor.execute(
            target, "am force-stop '$name'", rootSettings = AndroidPrivilegeAuthorization.rootExecutionSettings(context),
        )
        check(response.exitCode == 0) { "Force stop failed: ${response.stderrText()}" }
        return result("stop_app", name, "Force stop completed")
    }

    private fun usage(params: JSONObject): JSONObject {
        val hours = params.getInt("sinceHours")
        val limit = params.getInt("limit")
        val includeSystem = params.getBoolean("includeSystemApps")
        val requested = params.getString("packageName").trim().takeIf { it.isNotEmpty() }
        require(hours > 0 && limit > 0) { "Usage hours and limit must be positive" }
        val appOps = context.getSystemService(Context.APP_OPS_SERVICE) as AppOpsManager
        @Suppress("DEPRECATION")
        val mode = appOps.checkOpNoThrow(AppOpsManager.OPSTR_GET_USAGE_STATS, Process.myUid(), context.packageName)
        check(mode == AppOpsManager.MODE_ALLOWED) { "Android usage access is not granted; enable it in system settings" }
        val end = System.currentTimeMillis()
        val start = end - hours.toLong() * 60L * 60L * 1000L
        val manager = context.getSystemService(Context.USAGE_STATS_SERVICE) as UsageStatsManager
        val entries = manager.queryAndAggregateUsageStats(start, end).values.mapNotNull { stats ->
            if (requested != null && stats.packageName != requested) return@mapNotNull null
            val info = try { pm.getApplicationInfo(stats.packageName, 0) } catch (_: android.content.pm.PackageManager.NameNotFoundException) { return@mapNotNull null }
            val system = isSystem(info)
            if (!includeSystem && system && requested == null) return@mapNotNull null
            JSONObject().put("packageName", stats.packageName)
                .put("appName", pm.getApplicationLabel(info).toString())
                .put("totalForegroundTimeMs", stats.totalTimeInForeground)
                .put("lastTimeUsed", stats.lastTimeUsed).put("isSystemApp", system)
        }.sortedByDescending { it.getLong("totalForegroundTimeMs") }
        return JSONObject().put("startTime", start).put("endTime", end).put("sinceHours", hours)
            .put("requestedPackageName", requested ?: JSONObject.NULL).put("includesSystemApps", includeSystem)
            .put("totalEntries", entries.size).put("entries", JSONArray(entries.take(limit)))
    }

    private fun isSystem(info: ApplicationInfo): Boolean =
        info.flags and (ApplicationInfo.FLAG_SYSTEM or ApplicationInfo.FLAG_UPDATED_SYSTEM_APP) != 0
}
