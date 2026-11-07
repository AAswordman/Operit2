package app.operit

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.storage.StorageManager
import android.provider.Settings
import java.io.File

/** Owns shared-storage authorization before application runtime roots are committed. */
class AndroidStorageAccess(private val activity: MainActivity) {
    private val authorization = StoragePermissionRequest(::isGranted, ::launchRequest)

    /** Checks Android access rather than the permission declaration in the manifest. */
    fun isGranted(): Boolean = when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.R -> Environment.isExternalStorageManager()
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.M -> legacyPermissions().all {
            activity.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED
        }
        else -> true
    }

    /** Requests access only when either selected root lies in shared storage. */
    fun ensureRoots(runtimeRoot: String, workspaceRoot: String, complete: (Boolean) -> Unit) {
        if (!requiresPermission(runtimeRoot) && !requiresPermission(workspaceRoot)) {
            complete(true)
            return
        }
        request(complete)
    }

    /** Uses canonical directory boundaries to exclude this application's private storage. */
    internal fun requiresPermission(path: String): Boolean {
        val candidate = File(path).canonicalFile
        val owned = listOf(File(activity.applicationInfo.dataDir)) +
            activity.getExternalFilesDirs(null).filterNotNull() +
            activity.externalCacheDirs.filterNotNull() + activity.externalMediaDirs.filterNotNull()
        if (owned.any { inside(candidate, it) }) return false
        val volumes = mutableListOf(Environment.getExternalStorageDirectory())
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val storage = activity.getSystemService(StorageManager::class.java)
            volumes.addAll(storage.storageVolumes.mapNotNull { it.directory })
        }
        return volumes.any { inside(candidate, it) }
    }

    /** Compares full path components, including canonicalized /sdcard aliases. */
    private fun inside(candidate: File, root: File): Boolean {
        val canonical = root.canonicalFile
        return candidate == canonical || candidate.path.startsWith(canonical.path + File.separator)
    }

    /** Waits for Android to return so denial cannot start Core installation. */
    fun request(complete: (Boolean) -> Unit) {
        authorization.request(complete)
    }

    /** Launches the authorization UI supported by the device's storage model. */
    private fun launchRequest() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            activity.startActivityForResult(
                Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                    Uri.parse("package:${activity.packageName}")),
                REQUEST_CODE,
            )
        } else {
            activity.requestPermissions(legacyPermissions(), REQUEST_CODE)
        }
    }

    /** Rechecks all-files access because the settings result code does not represent a grant. */
    fun onActivityResult(requestCode: Int): Boolean {
        if (requestCode != REQUEST_CODE) return false
        authorization.finish()
        return true
    }

    /** Rechecks legacy access after the runtime permission dialog finishes. */
    fun onRequestPermissionsResult(requestCode: Int): Boolean {
        if (requestCode != REQUEST_CODE) return false
        authorization.finish()
        return true
    }

    /** Returns permissions supported by the device's shared-storage model and manifest. */
    private fun legacyPermissions(): Array<String> = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
        arrayOf(Manifest.permission.READ_EXTERNAL_STORAGE)
    } else {
        arrayOf(Manifest.permission.READ_EXTERNAL_STORAGE, Manifest.permission.WRITE_EXTERNAL_STORAGE)
    }

    companion object {
        private const val REQUEST_CODE = 2409
    }
}
