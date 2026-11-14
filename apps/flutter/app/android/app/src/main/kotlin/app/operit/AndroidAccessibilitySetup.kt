package app.operit

import android.accessibilityservice.AccessibilityServiceInfo
import android.app.Activity
import android.content.ClipData
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.view.accessibility.AccessibilityManager
import androidx.core.content.FileProvider
import java.io.File

/** Owns the embedded companion's installation and Android accessibility authorization. */
internal class AndroidAccessibilitySetup(private val activity: Activity) : AccessibilitySetupFlow.Actions {
    private val flow = AccessibilitySetupFlow(this)

    /** Reports the companion service's accessibility authorization state. */
    fun status(): String {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return "Unavailable"
        if (!hasCurrentCompanion()) return "Missing"
        val manager = activity.getSystemService(AccessibilityManager::class.java)
        val enabled = manager.getEnabledAccessibilityServiceList(AccessibilityServiceInfo.FEEDBACK_ALL_MASK)
            .any {
                val service = it.resolveInfo.serviceInfo
                ComponentName(service.packageName, service.name) == SERVICE
            }
        return if (enabled) "Satisfied" else "Missing"
    }

    /** Begins the install-and-authorize sequence after an explicit grant action. */
    fun request(complete: (Throwable?) -> Unit) {
        check(Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) { "UI 自动化需要 Android 8.0 或更新版本。" }
        flow.request(complete)
    }

    /** Resumes the matching installation or settings stage. */
    fun onActivityResult(requestCode: Int): Boolean = flow.onActivityResult(requestCode)

    /** Finishes pending authorization when its channel is detached. */
    fun release() {
        flow.release()
    }

    /** Compares the independently versioned provider with the version required by this build. */
    override fun hasCurrentCompanion(): Boolean {
        val info = try {
            activity.packageManager.getPackageInfo(PACKAGE, 0)
        } catch (_: PackageManager.NameNotFoundException) {
            return false
        }
        val installedVersion = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) info.longVersionCode
            else @Suppress("DEPRECATION") info.versionCode.toLong()
        return installedVersion >= BuildConfig.OPERIT_ACCESSIBILITY_VERSION_CODE
    }

    /** Checks the per-source installation authorization on supported Android versions. */
    override fun canInstallPackages(): Boolean = activity.packageManager.canRequestPackageInstalls()

    /** Requests permission to install the APK supplied by this application. */
    override fun authorizeInstallation(requestCode: Int) {
        activity.startActivityForResult(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
            Uri.parse("package:${activity.packageName}")), requestCode)
    }

    /** Extracts the bundled APK and opens Android's installer with a scoped read grant. */
    override fun installCompanion(requestCode: Int) {
        val directory = File(activity.cacheDir, "open_files/accessibility").apply { mkdirs() }
        val apk = File(directory, "operit-accessibility.apk")
        activity.assets.open("companions/operit-accessibility.apk").use { input ->
            apk.outputStream().use { input.copyTo(it) }
        }
        val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.openfiles", apk)
        activity.startActivityForResult(Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, "application/vnd.android.package-archive")
            clipData = ClipData.newRawUri("Operit2 Accessibility", uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            putExtra(Intent.EXTRA_RETURN_RESULT, true)
        }, requestCode)
    }

    /** Leaves the service grant to Android's explicit accessibility settings UI. */
    override fun authorizeAccessibility(requestCode: Int) {
        activity.startActivityForResult(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS), requestCode)
    }

    private companion object {
        private const val PACKAGE = "app.operit.accessibility"
        private val SERVICE = ComponentName(PACKAGE, "$PACKAGE.UiAccessibilityService")
    }
}
